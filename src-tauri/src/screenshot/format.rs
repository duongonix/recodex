use image::{
    ExtendedColorType, ImageEncoder,
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
};

use crate::{
    capture::platform::windows::CapturedFrame, errors::IpcError, settings::ScreenshotFormat,
};

const MAX_SCREENSHOT_PIXELS: u64 = 64_000_000;

pub(super) fn rgba_pixels(frame: &CapturedFrame) -> Result<Vec<u8>, IpcError> {
    validate_frame(frame)?;
    let mut pixels = Vec::with_capacity(frame.pixels_bgra.len());
    for &[blue, green, red, alpha] in frame.pixels_bgra.as_chunks::<4>().0 {
        pixels.extend_from_slice(&[red, green, blue, alpha]);
    }
    Ok(pixels)
}

pub(super) fn encode(frame: &CapturedFrame, format: ScreenshotFormat) -> Result<Vec<u8>, IpcError> {
    let rgba = rgba_pixels(frame)?;
    let mut encoded = Vec::new();
    let result = match format {
        ScreenshotFormat::Png => PngEncoder::new(&mut encoded).write_image(
            &rgba,
            frame.width,
            frame.height,
            ExtendedColorType::Rgba8,
        ),
        ScreenshotFormat::Jpg => {
            let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
            for &[red, green, blue, _] in rgba.as_chunks::<4>().0 {
                rgb.extend_from_slice(&[red, green, blue]);
            }
            JpegEncoder::new_with_quality(&mut encoded, 92).write_image(
                &rgb,
                frame.width,
                frame.height,
                ExtendedColorType::Rgb8,
            )
        }
    };
    result.map_err(|error| {
        IpcError::new(
            "screenshot_encode_failed",
            format!("Could not encode the screenshot: {error}"),
            true,
        )
    })?;
    Ok(encoded)
}

fn validate_frame(frame: &CapturedFrame) -> Result<(), IpcError> {
    let pixel_count = u64::from(frame.width)
        .checked_mul(u64::from(frame.height))
        .filter(|count| *count <= MAX_SCREENSHOT_PIXELS);
    let expected_bytes = pixel_count.and_then(|count| count.checked_mul(4));
    if frame.width < 2 || frame.height < 2 || expected_bytes != Some(frame.pixels_bgra.len() as u64)
    {
        return Err(IpcError::new(
            "screenshot_frame_invalid",
            "The native capture returned an invalid or excessively large screenshot frame.",
            true,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use image::{GenericImageView, ImageReader};
    use std::io::Cursor;

    use super::*;
    use crate::capture::platform::windows::CapturedFrame;

    fn test_frame() -> CapturedFrame {
        CapturedFrame {
            width: 2,
            height: 2,
            timestamp_100ns: 1,
            // BGRA: opaque red, opaque blue, repeated on a second row.
            pixels_bgra: vec![
                0, 0, 255, 255, 255, 0, 0, 255, 0, 0, 255, 255, 255, 0, 0, 255,
            ],
        }
    }

    #[test]
    fn png_encoding_preserves_dimensions_and_bgra_channel_order() {
        let encoded = encode(&test_frame(), ScreenshotFormat::Png).unwrap();
        let decoded = ImageReader::new(Cursor::new(encoded))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap()
            .to_rgba8();

        assert_eq!(decoded.dimensions(), (2, 2));
        assert_eq!(decoded.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(decoded.get_pixel(1, 0).0, [0, 0, 255, 255]);
    }

    #[test]
    fn jpeg_encoding_produces_a_decodable_rgb_image() {
        let encoded = encode(&test_frame(), ScreenshotFormat::Jpg).unwrap();
        let decoded = ImageReader::new(Cursor::new(encoded))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();

        assert_eq!(decoded.dimensions(), (2, 2));
    }

    #[test]
    fn rejects_inconsistent_or_oversized_frames() {
        let mut frame = test_frame();
        frame.pixels_bgra.pop();
        assert_eq!(
            rgba_pixels(&frame).unwrap_err().code,
            "screenshot_frame_invalid"
        );

        frame.width = 100_001;
        frame.height = 1_000;
        frame.pixels_bgra.clear();
        assert_eq!(
            rgba_pixels(&frame).unwrap_err().code,
            "screenshot_frame_invalid"
        );
    }
}
