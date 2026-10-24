use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageReader, codecs::jpeg::JpegEncoder};

use crate::{errors::IpcError, library::LibraryItemKind};

pub fn thumbnail_data_url(path: &Path, kind: LibraryItemKind) -> Result<Option<String>, IpcError> {
    let bytes = match kind {
        LibraryItemKind::Screenshot => image_thumbnail(path)?,
        LibraryItemKind::Video => video_thumbnail(path)?,
    };
    Ok(bytes.map(|bytes| format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes))))
}

fn image_thumbnail(path: &Path) -> Result<Option<Vec<u8>>, IpcError> {
    let reader = ImageReader::open(path)
        .map_err(|error| thumbnail_error("Could not open the screenshot thumbnail", error))?
        .with_guessed_format()
        .map_err(|error| thumbnail_error("Could not identify the screenshot format", error))?;
    let image = reader
        .decode()
        .map_err(|error| thumbnail_error("Could not decode the screenshot thumbnail", error))?;
    encode_jpeg(image.thumbnail(192, 108))
}

fn encode_jpeg(image: DynamicImage) -> Result<Option<Vec<u8>>, IpcError> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 72)
        .encode_image(&image)
        .map_err(|error| {
            IpcError::internal(format!("Could not encode a library thumbnail: {error}"))
        })?;
    Ok(Some(bytes))
}

#[cfg(windows)]
fn video_thumbnail(path: &Path) -> Result<Option<Vec<u8>>, IpcError> {
    crate::recording::encoding::windows_media_reader::extract_library_video_thumbnail(path)
        .map(Some)
}

#[cfg(not(windows))]
fn video_thumbnail(_path: &Path) -> Result<Option<Vec<u8>>, IpcError> {
    Ok(None)
}

fn thumbnail_error(context: &str, error: impl std::fmt::Display) -> IpcError {
    IpcError::new(
        "library_thumbnail_failed",
        format!("{context}: {error}"),
        true,
    )
}
