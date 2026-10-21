use image::{
    RgbImage,
    imageops::{self, FilterType},
};

use crate::{
    errors::IpcError,
    webcam::settings::{
        WebcamBackgroundMode, WebcamPosition, WebcamSettings, WebcamShape, parse_hex_color,
    },
};

use super::capture::WebcamFrame;

pub struct WebcamCompositor {
    settings: WebcamSettings,
    frame_width: u32,
    frame_height: u32,
    overlay_width: u32,
    overlay_height: u32,
    overlay_x: u32,
    overlay_y: u32,
    background_image: Option<RgbImage>,
    rgba: Vec<u8>,
    shape_mask: Vec<u8>,
    border_color: [u8; 3],
    opacity: u8,
    underlay: Vec<u8>,
    underlay_valid: bool,
    last_sequence: Option<u64>,
    image_warning: Option<String>,
}

impl WebcamCompositor {
    pub fn new(
        settings: WebcamSettings,
        frame_width: u32,
        frame_height: u32,
        camera_width: u32,
        camera_height: u32,
    ) -> Result<Self, IpcError> {
        settings.validate()?;
        let width = (f32::from(settings.width) * frame_height as f32 / 1080.0).round() as u32;
        let mut overlay_width = width.clamp(1, frame_width.max(1));
        let mut overlay_height = if settings.shape == WebcamShape::Circle {
            overlay_width
        } else {
            let ratio = camera_width.max(1) as f32 / camera_height.max(1) as f32;
            (overlay_width as f32 / ratio).round().max(1.0) as u32
        };
        if overlay_height > frame_height {
            let scale = frame_height as f32 / overlay_height as f32;
            overlay_width = (overlay_width as f32 * scale).round().max(1.0) as u32;
            overlay_height = frame_height.max(1);
        }
        let (overlay_x, overlay_y) = overlay_origin(
            &settings,
            frame_width,
            frame_height,
            overlay_width,
            overlay_height,
        );
        let (background_image, image_warning) = load_background_image(&settings);
        let rgba = vec![0; byte_len(overlay_width, overlay_height)?];
        let shape_mask = build_shape_mask(&settings, frame_height, overlay_width, overlay_height)?;
        let border_color = parse_hex_color(&settings.border_color).unwrap_or([255, 48, 56]);
        let opacity = (u16::from(settings.opacity) * 255 / 100) as u8;
        let underlay = vec![0; byte_len(overlay_width, overlay_height)?];

        Ok(Self {
            settings,
            frame_width,
            frame_height,
            overlay_width,
            overlay_height,
            overlay_x,
            overlay_y,
            background_image,
            rgba,
            shape_mask,
            border_color,
            opacity,
            underlay,
            underlay_valid: false,
            last_sequence: None,
            image_warning,
        })
    }

    pub fn image_warning(&self) -> Option<&str> {
        self.image_warning.as_deref()
    }

    pub fn discard_underlay(&mut self) {
        self.underlay_valid = false;
    }

    pub fn move_by(&mut self, delta_x: i32, delta_y: i32) {
        let max_x = self.frame_width.saturating_sub(self.overlay_width);
        let max_y = self.frame_height.saturating_sub(self.overlay_height);
        self.overlay_x =
            (i64::from(self.overlay_x) + i64::from(delta_x)).clamp(0, i64::from(max_x)) as u32;
        self.overlay_y =
            (i64::from(self.overlay_y) + i64::from(delta_y)).clamp(0, i64::from(max_y)) as u32;
        self.settings.position = WebcamPosition::Custom;
        self.settings.x = if max_x == 0 {
            0.0
        } else {
            self.overlay_x as f32 / max_x as f32
        };
        self.settings.y = if max_y == 0 {
            0.0
        } else {
            self.overlay_y as f32 / max_y as f32
        };
        self.underlay_valid = false;
    }

    pub fn restore_underlay(&mut self, screen_bgra: &mut [u8]) {
        if !self.underlay_valid
            || screen_bgra.len() != (self.frame_width as usize * self.frame_height as usize * 4)
        {
            return;
        }
        let row_length = self.overlay_width as usize * 4;
        for y in 0..self.overlay_height as usize {
            let screen_start = ((self.overlay_y as usize + y) * self.frame_width as usize
                + self.overlay_x as usize)
                * 4;
            let overlay_start = y * row_length;
            screen_bgra[screen_start..screen_start + row_length]
                .copy_from_slice(&self.underlay[overlay_start..overlay_start + row_length]);
        }
        self.underlay_valid = false;
    }

    pub fn clear(&mut self, screen_bgra: &mut [u8]) {
        self.restore_underlay(screen_bgra);
        self.rgba.fill(0);
        self.last_sequence = None;
    }

    pub fn composite(
        &mut self,
        screen_bgra: &mut [u8],
        camera: Option<&WebcamFrame>,
    ) -> Result<(), IpcError> {
        let expected = byte_len(self.frame_width, self.frame_height)?;
        if screen_bgra.len() != expected {
            return Err(IpcError::new(
                "frame_layout_invalid",
                "The webcam compositor received an invalid screen frame buffer.",
                true,
            ));
        }
        self.restore_underlay(screen_bgra);
        if let Some(camera) = camera
            && self.last_sequence != Some(camera.sequence)
        {
            self.render_camera(camera)?;
            self.last_sequence = Some(camera.sequence);
        }
        if self.last_sequence.is_none() {
            return Ok(());
        }

        let row_length = self.overlay_width as usize * 4;
        for local_y in 0..self.overlay_height as usize {
            let screen_start = ((self.overlay_y as usize + local_y) * self.frame_width as usize
                + self.overlay_x as usize)
                * 4;
            let overlay_start = local_y * row_length;
            let screen_row = &mut screen_bgra[screen_start..screen_start + row_length];
            let underlay_row = &mut self.underlay[overlay_start..overlay_start + row_length];
            let overlay_row = &self.rgba[overlay_start..overlay_start + row_length];
            let (destination_pixels, destination_tail) = screen_row.as_chunks_mut::<4>();
            let (underlay_pixels, underlay_tail) = underlay_row.as_chunks_mut::<4>();
            let (source_pixels, source_tail) = overlay_row.as_chunks::<4>();
            debug_assert!(destination_tail.is_empty());
            debug_assert!(underlay_tail.is_empty());
            debug_assert!(source_tail.is_empty());
            for ((destination, underlay), source) in destination_pixels
                .iter_mut()
                .zip(underlay_pixels)
                .zip(source_pixels)
            {
                underlay.copy_from_slice(destination);
                let alpha = u32::from(source[3]);
                if alpha == 0 {
                    continue;
                }
                if alpha == 255 {
                    destination[0] = source[2];
                    destination[1] = source[1];
                    destination[2] = source[0];
                    destination[3] = 255;
                    continue;
                }
                let inverse = 255 - alpha;
                destination[0] = blend(source[2], destination[0], alpha, inverse);
                destination[1] = blend(source[1], destination[1], alpha, inverse);
                destination[2] = blend(source[0], destination[2], alpha, inverse);
                destination[3] = 255;
            }
        }
        self.underlay_valid = true;
        Ok(())
    }

    fn render_camera(&mut self, frame: &WebcamFrame) -> Result<(), IpcError> {
        if frame.width == 0
            || frame.height == 0
            || frame.rgb.dimensions() != (frame.width, frame.height)
        {
            return Err(IpcError::new(
                "webcam_frame_invalid",
                "The camera returned an invalid frame size.",
                true,
            ));
        }
        self.rgba.fill(0);
        let background_active = self.settings.background_mode != WebcamBackgroundMode::None
            && !(self.settings.background_mode == WebcamBackgroundMode::Image
                && self.background_image.is_none());

        match self.settings.background_mode {
            WebcamBackgroundMode::None => {}
            WebcamBackgroundMode::Blur => {
                let mut background = fit_fill(&frame.rgb, self.overlay_width, self.overlay_height);
                if self.settings.mirror {
                    background = imageops::flip_horizontal(&background);
                }
                let blurred =
                    imageops::blur(&background, 8.0_f32.max(self.overlay_width as f32 / 80.0));
                self.write_rgb(&blurred, 0, 0);
            }
            WebcamBackgroundMode::Color => {
                let color =
                    parse_hex_color(&self.settings.background_color).unwrap_or([23, 25, 31]);
                self.fill_rgb(color);
            }
            WebcamBackgroundMode::Image => {
                if let Some(background) = &self.background_image {
                    let scaled = fit_fill(background, self.overlay_width, self.overlay_height);
                    self.write_rgb(&scaled, 0, 0);
                }
            }
        }

        let inset = if background_active {
            (self.overlay_width / 20).max(2)
        } else {
            0
        };
        let content_width = self.overlay_width.saturating_sub(inset * 2).max(1);
        let content_height = self.overlay_height.saturating_sub(inset * 2).max(1);
        let mut foreground = fit_fill(&frame.rgb, content_width, content_height);
        if self.settings.mirror {
            foreground = imageops::flip_horizontal(&foreground);
        }
        self.write_rgb(&foreground, inset, inset);
        self.apply_precomputed_style();
        Ok(())
    }

    fn write_rgb(&mut self, image: &RgbImage, offset_x: u32, offset_y: u32) {
        for (x, y, pixel) in image.enumerate_pixels() {
            let target_x = x + offset_x;
            let target_y = y + offset_y;
            if target_x >= self.overlay_width || target_y >= self.overlay_height {
                continue;
            }
            let index = ((target_y * self.overlay_width + target_x) * 4) as usize;
            self.rgba[index..index + 3].copy_from_slice(&pixel.0);
            self.rgba[index + 3] = 255;
        }
    }

    fn fill_rgb(&mut self, color: [u8; 3]) {
        let (pixels, remainder) = self.rgba.as_chunks_mut::<4>();
        debug_assert!(remainder.is_empty());
        for pixel in pixels {
            pixel[..3].copy_from_slice(&color);
            pixel[3] = 255;
        }
    }

    fn apply_precomputed_style(&mut self) {
        let (pixels, remainder) = self.rgba.as_chunks_mut::<4>();
        debug_assert!(remainder.is_empty());
        for (mask, pixel) in self.shape_mask.iter().zip(pixels) {
            match *mask {
                0 => pixel[3] = 0,
                1 => pixel[3] = self.opacity,
                2 => {
                    pixel[..3].copy_from_slice(&self.border_color);
                    pixel[3] = self.opacity;
                }
                _ => {}
            }
        }
    }
}

fn build_shape_mask(
    settings: &WebcamSettings,
    frame_height: u32,
    overlay_width: u32,
    overlay_height: u32,
) -> Result<Vec<u8>, IpcError> {
    let width = overlay_width as f32;
    let height = overlay_height as f32;
    let border = if settings.border_enabled {
        (f32::from(settings.border_width) * frame_height as f32 / 1080.0)
            .round()
            .max(0.0)
    } else {
        0.0
    };
    let radius = (f32::from(settings.corner_radius) * frame_height as f32 / 1080.0)
        .min(width.min(height) / 2.0);
    let pixel_count = usize::try_from(overlay_width)
        .ok()
        .and_then(|width| {
            usize::try_from(overlay_height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or_else(|| {
            IpcError::new(
                "webcam_frame_too_large",
                "Webcam output dimensions are too large.",
                true,
            )
        })?;
    let mut mask = vec![0; pixel_count];

    for y in 0..overlay_height {
        for x in 0..overlay_width {
            let index = (y * overlay_width + x) as usize;
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            if shape_contains(settings.shape, px, py, width, height, radius) {
                mask[index] = if border > 0.0
                    && is_border_pixel(settings.shape, px, py, width, height, radius, border)
                {
                    2
                } else {
                    1
                };
            }
        }
    }
    Ok(mask)
}

fn overlay_origin(
    settings: &WebcamSettings,
    width: u32,
    height: u32,
    overlay_width: u32,
    overlay_height: u32,
) -> (u32, u32) {
    let max_x = width.saturating_sub(overlay_width);
    let max_y = height.saturating_sub(overlay_height);
    let margin = (24.0 * height as f32 / 1080.0).round() as u32;
    match settings.position {
        WebcamPosition::TopLeft => (margin.min(max_x), margin.min(max_y)),
        WebcamPosition::TopCenter => (max_x / 2, margin.min(max_y)),
        WebcamPosition::TopRight => (max_x.saturating_sub(margin), margin.min(max_y)),
        WebcamPosition::BottomLeft => (margin.min(max_x), max_y.saturating_sub(margin)),
        WebcamPosition::BottomCenter => (max_x / 2, max_y.saturating_sub(margin)),
        WebcamPosition::BottomRight => (max_x.saturating_sub(margin), max_y.saturating_sub(margin)),
        WebcamPosition::Custom => (
            (max_x as f32 * settings.x).round() as u32,
            (max_y as f32 * settings.y).round() as u32,
        ),
    }
}

fn shape_contains(
    shape: WebcamShape,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> bool {
    match shape {
        WebcamShape::Circle => {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            let circle_radius = width.min(height) / 2.0;
            (x - center_x).powi(2) + (y - center_y).powi(2) <= circle_radius.powi(2)
        }
        WebcamShape::Rectangle => rounded_rect_contains(x, y, width, height, radius),
    }
}

fn is_border_pixel(
    shape: WebcamShape,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    border: f32,
) -> bool {
    let inner_width = (width - border * 2.0).max(0.0);
    let inner_height = (height - border * 2.0).max(0.0);
    match shape {
        WebcamShape::Circle => {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            let inner_radius = (width.min(height) / 2.0 - border).max(0.0);
            (x - center_x).powi(2) + (y - center_y).powi(2) >= inner_radius.powi(2)
        }
        WebcamShape::Rectangle => !rounded_rect_contains(
            x - border,
            y - border,
            inner_width,
            inner_height,
            (radius - border).max(0.0),
        ),
    }
}

fn rounded_rect_contains(x: f32, y: f32, width: f32, height: f32, radius: f32) -> bool {
    if x < 0.0 || y < 0.0 || x >= width || y >= height {
        return false;
    }
    let radius = radius.min(width.min(height) / 2.0);
    let nearest_x = x.clamp(radius, width - radius);
    let nearest_y = y.clamp(radius, height - radius);
    (x - nearest_x).powi(2) + (y - nearest_y).powi(2) <= radius.powi(2)
}

fn fit_fill(image: &RgbImage, width: u32, height: u32) -> RgbImage {
    let scale = (width as f32 / image.width() as f32).max(height as f32 / image.height() as f32);
    let scaled_width = (image.width() as f32 * scale).ceil().max(width as f32) as u32;
    let scaled_height = (image.height() as f32 * scale).ceil().max(height as f32) as u32;
    let scaled = imageops::resize(image, scaled_width, scaled_height, FilterType::Triangle);
    let left = scaled_width.saturating_sub(width) / 2;
    let top = scaled_height.saturating_sub(height) / 2;
    imageops::crop_imm(&scaled, left, top, width, height).to_image()
}

fn load_background_image(settings: &WebcamSettings) -> (Option<RgbImage>, Option<String>) {
    if settings.background_mode != WebcamBackgroundMode::Image {
        return (None, None);
    }
    let Some(path) = &settings.background_image_path else {
        return (
            None,
            Some("Choose a local image for the webcam background.".to_owned()),
        );
    };
    match super::preview::load_background_image(path) {
        Ok(image) => (Some(image), None),
        Err(error) => (None, Some(error.message)),
    }
}

fn byte_len(width: u32, height: u32) -> Result<usize, IpcError> {
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| {
            IpcError::new(
                "webcam_frame_too_large",
                "Webcam output dimensions are too large.",
                true,
            )
        })
}

fn blend(source: u8, destination: u8, alpha: u32, inverse: u32) -> u8 {
    ((u32::from(source) * alpha + u32::from(destination) * inverse + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;
    use std::time::Instant;

    #[test]
    fn bottom_right_overlay_is_scaled_and_composited_into_bgra() {
        let settings = WebcamSettings {
            width: 180,
            border_enabled: false,
            position: WebcamPosition::BottomRight,
            ..WebcamSettings::default()
        };
        let mut compositor = WebcamCompositor::new(settings, 1920, 1080, 4, 3).unwrap();
        let camera = WebcamFrame {
            sequence: 1,
            width: 4,
            height: 3,
            rgb: RgbImage::from_pixel(4, 3, Rgb([240, 20, 10])),
            captured_at: Instant::now(),
        };
        let mut screen = vec![0_u8; 1920 * 1080 * 4];
        compositor.composite(&mut screen, Some(&camera)).unwrap();
        let y = (1080 - 24 - 135 + 20) * 1920 * 4;
        let x = (1920 - 24 - 180 + 20) * 4;
        assert_eq!(&screen[y + x..y + x + 3], &[10, 20, 240]);
        assert_eq!(screen[3], 0);
        compositor.restore_underlay(&mut screen);
        assert_eq!(&screen[y + x..y + x + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn toolbar_motion_moves_overlay_and_clamps_it_to_the_frame() {
        let mut compositor =
            WebcamCompositor::new(WebcamSettings::default(), 1920, 1080, 4, 3).unwrap();

        compositor.move_by(-90, -60);
        assert_eq!((compositor.overlay_x, compositor.overlay_y), (1626, 861));
        assert_eq!(compositor.settings.position, WebcamPosition::Custom);

        compositor.move_by(-5_000, 5_000);
        assert_eq!((compositor.overlay_x, compositor.overlay_y), (0, 945));
    }

    #[test]
    fn circle_leaves_corner_pixels_transparent_and_respects_opacity() {
        let settings = WebcamSettings {
            width: 180,
            shape: WebcamShape::Circle,
            border_enabled: false,
            opacity: 50,
            ..WebcamSettings::default()
        };
        let mut compositor = WebcamCompositor::new(settings, 1920, 1080, 2, 2).unwrap();
        let camera = WebcamFrame {
            sequence: 7,
            width: 2,
            height: 2,
            rgb: RgbImage::from_pixel(2, 2, Rgb([200, 0, 0])),
            captured_at: Instant::now(),
        };
        compositor.render_camera(&camera).unwrap();
        assert_eq!(compositor.rgba[3], 0);
        assert_eq!(compositor.rgba[(90 * 180 + 90) * 4 + 3], 127);
    }

    #[test]
    fn partial_opacity_blends_over_the_saved_screen_row() {
        let settings = WebcamSettings {
            position: WebcamPosition::Custom,
            x: 0.0,
            y: 0.0,
            corner_radius: 0,
            border_enabled: false,
            opacity: 50,
            ..WebcamSettings::default()
        };
        let mut compositor = WebcamCompositor::new(settings, 1920, 1080, 4, 3).unwrap();
        let camera = WebcamFrame {
            sequence: 1,
            width: 4,
            height: 3,
            rgb: RgbImage::from_pixel(4, 3, Rgb([200, 100, 0])),
            captured_at: Instant::now(),
        };
        let mut screen = vec![100_u8; 1920 * 1080 * 4];

        compositor.composite(&mut screen, Some(&camera)).unwrap();

        let pixel = (50 * 1920 + 50) * 4;
        assert_eq!(&screen[pixel..pixel + 4], &[50, 100, 150, 255]);
    }
}
