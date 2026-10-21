use serde::Deserialize;

use crate::errors::IpcError;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorRenderSettings {
    pub scale: f32,
    pub position_x: i32,
    pub position_y: i32,
    pub rotation: f32,
    pub speed: f32,
    pub opacity: f32,
    pub crop_enabled: bool,
    pub crop_left: u32,
    pub crop_right: u32,
    pub crop_top: u32,
    pub crop_bottom: u32,
    pub volume: f32,
    pub muted: bool,
    pub canvas_aspect: String,
    pub background_color: String,
    pub fit: String,
}

impl Default for EditorRenderSettings {
    fn default() -> Self {
        Self {
            scale: 100.0,
            position_x: 0,
            position_y: 0,
            rotation: 0.0,
            speed: 1.0,
            opacity: 100.0,
            crop_enabled: false,
            crop_left: 0,
            crop_right: 0,
            crop_top: 0,
            crop_bottom: 0,
            volume: 100.0,
            muted: false,
            canvas_aspect: "source".to_owned(),
            background_color: "#08090c".to_owned(),
            fit: "contain".to_owned(),
        }
    }
}

impl EditorRenderSettings {
    pub fn validate(&self) -> Result<(), IpcError> {
        let finite = [
            self.scale,
            self.rotation,
            self.speed,
            self.opacity,
            self.volume,
        ]
        .into_iter()
        .all(f32::is_finite);
        if !finite
            || !(25.0..=300.0).contains(&self.scale)
            || !(-180.0..=180.0).contains(&self.rotation)
            || !(0.25..=2.0).contains(&self.speed)
            || !(0.0..=100.0).contains(&self.opacity)
            || !(0.0..=100.0).contains(&self.volume)
            || !matches!(
                self.canvas_aspect.as_str(),
                "source" | "16:9" | "9:16" | "1:1"
            )
            || !matches!(self.fit.as_str(), "contain" | "cover")
        {
            return Err(IpcError::new(
                "video_editor_settings_invalid",
                "One or more video editor settings are outside the supported range.",
                true,
            ));
        }
        parse_color(&self.background_color)?;
        Ok(())
    }

    pub fn output_dimensions(
        &self,
        source_width: u32,
        source_height: u32,
    ) -> Result<(u32, u32), IpcError> {
        self.validate()?;
        if source_width < 2 || source_height < 2 {
            return Err(invalid_frame());
        }
        if self.canvas_aspect == "source" {
            return Ok((source_width, source_height));
        }
        let ratio = match self.canvas_aspect.as_str() {
            "16:9" => 16.0 / 9.0,
            "9:16" => 9.0 / 16.0,
            "1:1" => 1.0,
            _ => return Err(invalid_frame()),
        };
        let width = ((source_height as f64 * ratio).round() as u32).max(2) & !1;
        let height = source_height.max(2) & !1;
        let pixels = u64::from(width) * u64::from(height);
        if width > 8192 || height > 8192 || pixels > 67_108_864 {
            return Err(IpcError::new(
                "video_editor_canvas_too_large",
                "The selected output canvas is larger than the supported 64 megapixel limit.",
                true,
            ));
        }
        Ok((width, height))
    }
}

pub fn transform_frame_bgra(
    input: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    settings: &EditorRenderSettings,
) -> Result<Vec<u8>, IpcError> {
    let expected = frame_bytes(source_width, source_height)?;
    if input.len() != expected {
        return Err(invalid_frame());
    }
    if source_width == output_width
        && source_height == output_height
        && settings.scale == 100.0
        && settings.position_x == 0
        && settings.position_y == 0
        && settings.rotation == 0.0
        && settings.opacity == 100.0
        && !settings.crop_enabled
        && settings.fit == "contain"
    {
        return Ok(input.to_vec());
    }

    let [bg_b, bg_g, bg_r] = parse_color(&settings.background_color)?;
    let output_length = frame_bytes(output_width, output_height)?;
    let mut output = vec![0_u8; output_length];
    for pixel in output.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&[bg_b, bg_g, bg_r, 255]);
    }

    let (crop_left, crop_right, crop_top, crop_bottom) = if settings.crop_enabled {
        (
            settings.crop_left.min(source_width.saturating_sub(2)),
            settings.crop_right.min(source_width.saturating_sub(2)),
            settings.crop_top.min(source_height.saturating_sub(2)),
            settings.crop_bottom.min(source_height.saturating_sub(2)),
        )
    } else {
        (0, 0, 0, 0)
    };
    let crop_width = source_width.saturating_sub(crop_left + crop_right).max(2);
    let crop_height = source_height.saturating_sub(crop_top + crop_bottom).max(2);
    let width_scale = output_width as f32 / crop_width as f32;
    let height_scale = output_height as f32 / crop_height as f32;
    let fit_scale = if settings.fit == "cover" {
        width_scale.max(height_scale)
    } else {
        width_scale.min(height_scale)
    } * (settings.scale / 100.0);
    let radians = settings.rotation.to_radians();
    let cosine = radians.cos();
    let sine = radians.sin();
    let crop_center_x = crop_left as f32 + crop_width as f32 * 0.5;
    let crop_center_y = crop_top as f32 + crop_height as f32 * 0.5;
    let opacity = settings.opacity / 100.0;
    let out_center_x = output_width as f32 * 0.5 + settings.position_x as f32;
    let out_center_y = output_height as f32 * 0.5 + settings.position_y as f32;

    for y in 0..output_height {
        for x in 0..output_width {
            let dx = x as f32 + 0.5 - out_center_x;
            let dy = y as f32 + 0.5 - out_center_y;
            let unrotated_x = cosine * dx + sine * dy;
            let unrotated_y = -sine * dx + cosine * dy;
            let source_x = crop_center_x + unrotated_x / fit_scale;
            let source_y = crop_center_y + unrotated_y / fit_scale;
            if source_x < crop_left as f32
                || source_y < crop_top as f32
                || source_x >= (crop_left + crop_width) as f32
                || source_y >= (crop_top + crop_height) as f32
            {
                continue;
            }
            let [b, g, r] =
                sample_bgra_bilinear(input, source_width, source_height, source_x, source_y);
            let offset = ((y * output_width + x) * 4) as usize;
            output[offset] = blend(b, bg_b, opacity);
            output[offset + 1] = blend(g, bg_g, opacity);
            output[offset + 2] = blend(r, bg_r, opacity);
            output[offset + 3] = 255;
        }
    }
    Ok(output)
}

pub fn retime_audio_pcm(
    bytes: &[u8],
    speed: f32,
    volume: f32,
    muted: bool,
) -> Result<Vec<u8>, IpcError> {
    if !bytes.len().is_multiple_of(4) || !speed.is_finite() || speed <= 0.0 {
        return Err(IpcError::new(
            "video_editor_audio_invalid",
            "The decoded audio packet does not match interleaved 48 kHz stereo PCM.",
            true,
        ));
    }
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if muted {
        return Ok(vec![0; bytes.len()]);
    }
    let input_frames = bytes.len() / 4;
    let output_frames = ((input_frames as f64 / f64::from(speed)).ceil() as usize).max(1);
    let gain = volume.clamp(0.0, 100.0) / 100.0;
    let mut output = Vec::with_capacity(output_frames * 4);
    for output_frame in 0..output_frames {
        let source_position =
            (output_frame as f32 * speed).min(input_frames.saturating_sub(1) as f32);
        let first = source_position.floor() as usize;
        let second = (first + 1).min(input_frames.saturating_sub(1));
        let fraction = source_position - first as f32;
        for channel in 0..2 {
            let first_offset = first * 4 + channel * 2;
            let second_offset = second * 4 + channel * 2;
            let first_sample =
                i16::from_le_bytes([bytes[first_offset], bytes[first_offset + 1]]) as f32;
            let second_sample =
                i16::from_le_bytes([bytes[second_offset], bytes[second_offset + 1]]) as f32;
            let interpolated = (first_sample + (second_sample - first_sample) * fraction) * gain;
            output.extend_from_slice(
                &(interpolated.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16)
                    .to_le_bytes(),
            );
        }
    }
    Ok(output)
}

fn sample_bgra_bilinear(input: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 3] {
    let fx = (x - 0.5).clamp(0.0, width.saturating_sub(1) as f32);
    let fy = (y - 0.5).clamp(0.0, height.saturating_sub(1) as f32);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let mut channels = [0_u8; 3];
    for (channel, value) in channels.iter_mut().enumerate() {
        let at = |column: u32, row: u32| input[((row * width + column) * 4) as usize + channel];
        let top = f32::from(at(x0, y0)) * (1.0 - tx) + f32::from(at(x1, y0)) * tx;
        let bottom = f32::from(at(x0, y1)) * (1.0 - tx) + f32::from(at(x1, y1)) * tx;
        *value = (top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0) as u8;
    }
    channels
}

fn blend(foreground: u8, background: u8, opacity: f32) -> u8 {
    (f32::from(foreground) * opacity + f32::from(background) * (1.0 - opacity))
        .round()
        .clamp(0.0, 255.0) as u8
}

fn parse_color(value: &str) -> Result<[u8; 3], IpcError> {
    let hex = value.strip_prefix('#').ok_or_else(invalid_color)?;
    if hex.len() != 6 {
        return Err(invalid_color());
    }
    let red = u8::from_str_radix(&hex[0..2], 16).map_err(|_| invalid_color())?;
    let green = u8::from_str_radix(&hex[2..4], 16).map_err(|_| invalid_color())?;
    let blue = u8::from_str_radix(&hex[4..6], 16).map_err(|_| invalid_color())?;
    Ok([blue, green, red])
}

fn frame_bytes(width: u32, height: u32) -> Result<usize, IpcError> {
    usize::try_from(u64::from(width) * u64::from(height) * 4).map_err(|_| invalid_frame())
}

fn invalid_color() -> IpcError {
    IpcError::new(
        "video_editor_color_invalid",
        "The background color must be a six-digit hex color.",
        true,
    )
}

fn invalid_frame() -> IpcError {
    IpcError::new(
        "recording_frame_layout_invalid",
        "The source or output video frame layout is invalid.",
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::{EditorRenderSettings, retime_audio_pcm, transform_frame_bgra};

    #[test]
    fn volume_and_mute_are_applied_to_audio_samples() {
        let pcm = [0x10, 0x27, 0x10, 0x27]; // 10,000 in both stereo channels
        let half = retime_audio_pcm(&pcm, 1.0, 50.0, false).expect("retime");
        assert_eq!(i16::from_le_bytes([half[0], half[1]]), 5_000);
        assert_eq!(
            retime_audio_pcm(&pcm, 1.0, 100.0, true).expect("mute"),
            [0; 4]
        );
    }

    #[test]
    fn transform_preserves_frame_when_settings_are_default() {
        let source = [30_u8, 20, 10, 255].repeat(4);
        let output = transform_frame_bgra(&source, 2, 2, 2, 2, &EditorRenderSettings::default())
            .expect("transform");
        assert_eq!(output, source);
    }

    #[test]
    fn square_canvas_dimensions_keep_even_height() {
        let settings = EditorRenderSettings {
            canvas_aspect: "1:1".to_owned(),
            ..EditorRenderSettings::default()
        };
        assert_eq!(
            settings.output_dimensions(1920, 1080).expect("dimensions"),
            (1080, 1080)
        );
    }
}
