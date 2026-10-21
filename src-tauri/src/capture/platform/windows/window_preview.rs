use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, codecs::jpeg::JpegEncoder, imageops::FilterType};

use crate::{errors::IpcError, recording::CaptureTarget};

use super::WindowsCapture;

const PREVIEW_WIDTH: u32 = 480;
const PREVIEW_HEIGHT: u32 = 270;

pub fn capture_window_thumbnail(window_id: &str) -> Result<Option<String>, IpcError> {
    let target = CaptureTarget::Window {
        window_id: window_id.to_owned(),
    };
    let mut capture = WindowsCapture::start(&target, &[], 30)?;
    let frame_result = capture.wait_for_frame(Duration::from_secs(2));
    let stop_result = capture.stop();
    let frame = frame_result?;
    stop_result?;
    let Some(mut frame) = frame else {
        return Ok(None);
    };

    for pixel in frame.pixels_bgra.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    let rgba = image::RgbaImage::from_raw(frame.width, frame.height, frame.pixels_bgra)
        .ok_or_else(|| {
            IpcError::internal("The native window thumbnail had an invalid pixel buffer.")
        })?;
    let rgb = DynamicImage::ImageRgba8(rgba)
        .resize(PREVIEW_WIDTH, PREVIEW_HEIGHT, FilterType::Triangle)
        .to_rgb8();

    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 76)
        .encode_image(&DynamicImage::ImageRgb8(rgb))
        .map_err(|error| {
            IpcError::internal(format!(
                "Could not encode the native window thumbnail: {error}"
            ))
        })?;
    Ok(Some(format!(
        "data:image/jpeg;base64,{}",
        STANDARD.encode(bytes)
    )))
}
