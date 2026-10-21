use std::{fs, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{ImageEncoder, RgbImage, codecs::jpeg::JpegEncoder, imageops::FilterType};

use crate::errors::IpcError;

const MAX_BACKGROUND_FILE_BYTES: u64 = 25 * 1024 * 1024;

pub fn encode_background_preview(path: &str) -> Result<String, IpcError> {
    let image = load_background_image(path)?;
    let image = image::DynamicImage::ImageRgb8(image)
        .resize_to_fill(640, 360, FilterType::Triangle)
        .to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 76)
        .write_image(image.as_raw(), 640, 360, image::ExtendedColorType::Rgb8)
        .map_err(|_| IpcError::internal("Could not prepare the webcam background preview."))?;
    Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg)))
}

pub fn load_background_image(path: &str) -> Result<RgbImage, IpcError> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err(IpcError::new(
            "webcam_background_invalid",
            "Choose an image using the local file picker.",
            true,
        ));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
        return Err(IpcError::new(
            "webcam_background_format_unsupported",
            "Webcam backgrounds must be PNG or JPEG images.",
            true,
        ));
    }
    let metadata = fs::metadata(path).map_err(|_| {
        IpcError::new(
            "webcam_background_unavailable",
            "The selected background image could not be read.",
            true,
        )
    })?;
    if !metadata.is_file() || metadata.len() > MAX_BACKGROUND_FILE_BYTES {
        return Err(IpcError::new(
            "webcam_background_too_large",
            "Choose an image file smaller than 25 MB.",
            true,
        ));
    }
    let reader = image::ImageReader::open(path)
        .map_err(|_| invalid_image())?
        .with_guessed_format()
        .map_err(|_| invalid_image())?;
    let (width, height) = reader.into_dimensions().map_err(|_| invalid_image())?;
    if width > 16_384 || height > 16_384 || width.saturating_mul(height) > 16_000_000 {
        return Err(IpcError::new(
            "webcam_background_too_large",
            "Choose an image with no more than 16 megapixels.",
            true,
        ));
    }
    image::open(path)
        .map(|image| image.to_rgb8())
        .map_err(|_| invalid_image())
}

fn invalid_image() -> IpcError {
    IpcError::new(
        "webcam_background_invalid",
        "The selected file is not a readable PNG or JPEG image.",
        true,
    )
}
