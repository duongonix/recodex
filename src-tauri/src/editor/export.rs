use std::{
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use image::{ImageFormat, ImageReader};
use tempfile::NamedTempFile;

use crate::{errors::IpcError, settings::ScreenshotFormat};

static NEXT_FILE_ID: AtomicU64 = AtomicU64::new(1);
const MAX_SCREENSHOT_PIXELS: u64 = 64_000_000;
const MAX_EXPORT_BYTES: usize = 512 * 1024 * 1024;

pub(super) fn validate_image(
    bytes: &[u8],
    format: ScreenshotFormat,
) -> Result<(u32, u32), IpcError> {
    if bytes.is_empty() || bytes.len() > MAX_EXPORT_BYTES {
        return Err(IpcError::new(
            "editor_export_invalid",
            "The exported screenshot is empty or exceeds the supported size.",
            true,
        ));
    }
    let expected_format = match format {
        ScreenshotFormat::Png => ImageFormat::Png,
        ScreenshotFormat::Jpg => ImageFormat::Jpeg,
    };
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| invalid_export(error.to_string()))?;
    if reader.format() != Some(expected_format) {
        return Err(invalid_export(
            "Image format does not match the session format.",
        ));
    }
    let image = reader
        .decode()
        .map_err(|error| invalid_export(error.to_string()))?;
    let (width, height) = (image.width(), image.height());
    if width < 2
        || height < 2
        || u64::from(width).saturating_mul(u64::from(height)) > MAX_SCREENSHOT_PIXELS
    {
        return Err(invalid_export(
            "Exported dimensions are outside the supported range.",
        ));
    }
    Ok((width, height))
}

pub(super) fn save_unique(
    directory: &Path,
    format: ScreenshotFormat,
    bytes: &[u8],
) -> Result<PathBuf, IpcError> {
    if !directory.is_absolute() || !directory.is_dir() {
        return Err(IpcError::new(
            "screenshot_output_unavailable",
            "The screenshot output folder is unavailable. Choose an existing folder in Settings.",
            true,
        ));
    }
    let extension = extension(format);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut temporary = write_temporary(directory, bytes)?;
    for _ in 0..100 {
        let id = NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!("RecoDex-{timestamp}-{id}.{extension}"));
        match temporary.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temporary = error.file;
            }
            Err(error) => return Err(save_error(error.error)),
        }
    }
    Err(IpcError::new(
        "screenshot_filename_exhausted",
        "Could not find an unused screenshot filename after 100 attempts.",
        true,
    ))
}

pub(super) fn save_as(
    requested_path: &Path,
    format: ScreenshotFormat,
    bytes: &[u8],
) -> Result<PathBuf, IpcError> {
    if !requested_path.is_absolute() {
        return Err(IpcError::new(
            "editor_export_path_invalid",
            "Save As requires an absolute file path.",
            true,
        ));
    }
    let parent = requested_path
        .parent()
        .filter(|path| path.is_dir())
        .ok_or_else(|| {
            IpcError::new(
                "editor_export_path_invalid",
                "The selected screenshot folder is unavailable.",
                true,
            )
        })?;
    let file_name = requested_path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            IpcError::new(
                "editor_export_path_invalid",
                "Choose a screenshot file name.",
                true,
            )
        })?;
    let extension_matches = requested_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|file_extension| file_extension.eq_ignore_ascii_case(extension(format)));
    if !extension_matches {
        return Err(IpcError::new(
            "editor_export_format_mismatch",
            format!("Choose a .{} file for this screenshot.", extension(format)),
            true,
        ));
    }

    let canonical_parent = parent.canonicalize().map_err(|error| {
        IpcError::new(
            "editor_export_path_invalid",
            format!("Could not resolve the selected screenshot folder: {error}"),
            true,
        )
    })?;
    let destination = canonical_parent.join(file_name);
    let temporary = write_temporary(&canonical_parent, bytes)?;
    temporary
        .persist(&destination)
        .map_err(|error| save_error(error.error))?;
    Ok(destination)
}

#[cfg(windows)]
pub(super) fn copy_to_clipboard(bytes: &[u8], format: ScreenshotFormat) -> Result<(), IpcError> {
    use std::borrow::Cow;

    use arboard::{Clipboard, ImageData};

    let decoded = image::load_from_memory_with_format(bytes, image_format(format))
        .map_err(|error| invalid_export(error.to_string()))?
        .to_rgba8();
    let image = ImageData {
        width: decoded.width() as usize,
        height: decoded.height() as usize,
        bytes: Cow::Owned(decoded.into_raw()),
    };
    let mut clipboard = Clipboard::new().map_err(|error| {
        IpcError::new(
            "screenshot_clipboard_unavailable",
            format!("Could not open the system clipboard: {error}"),
            true,
        )
    })?;
    clipboard.set_image(image).map_err(|error| {
        IpcError::new(
            "screenshot_clipboard_failed",
            format!("Could not copy the edited screenshot to the clipboard: {error}"),
            true,
        )
    })
}

#[cfg(not(windows))]
pub(super) fn copy_to_clipboard(_bytes: &[u8], _format: ScreenshotFormat) -> Result<(), IpcError> {
    Err(IpcError::new(
        "screenshot_clipboard_unavailable",
        "Native screenshot editor clipboard export is currently available only on Windows.",
        true,
    ))
}

fn write_temporary(directory: &Path, bytes: &[u8]) -> Result<NamedTempFile, IpcError> {
    let mut temporary = NamedTempFile::new_in(directory).map_err(save_error)?;
    temporary.write_all(bytes).map_err(save_error)?;
    temporary.as_file().sync_all().map_err(save_error)?;
    Ok(temporary)
}

fn extension(format: ScreenshotFormat) -> &'static str {
    match format {
        ScreenshotFormat::Png => "png",
        ScreenshotFormat::Jpg => "jpg",
    }
}

fn image_format(format: ScreenshotFormat) -> ImageFormat {
    match format {
        ScreenshotFormat::Png => ImageFormat::Png,
        ScreenshotFormat::Jpg => ImageFormat::Jpeg,
    }
}

fn invalid_export(message: impl Into<String>) -> IpcError {
    IpcError::new(
        "editor_export_invalid",
        format!(
            "The edited screenshot could not be validated: {}",
            message.into()
        ),
        true,
    )
}

fn save_error(error: std::io::Error) -> IpcError {
    IpcError::new(
        "editor_export_save_failed",
        format!("Could not save the edited screenshot: {error}"),
        true,
    )
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, Rgba, RgbaImage};

    use super::*;

    fn png() -> Vec<u8> {
        let image = RgbaImage::from_pixel(2, 2, Rgba([255, 48, 56, 255]));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    #[test]
    fn validates_real_png_dimensions_and_rejects_format_mismatch() {
        assert_eq!(
            validate_image(&png(), ScreenshotFormat::Png).unwrap(),
            (2, 2)
        );
        assert_eq!(
            validate_image(&png(), ScreenshotFormat::Jpg)
                .unwrap_err()
                .code,
            "editor_export_invalid"
        );
    }

    #[test]
    fn saves_unique_outputs_and_save_as_paths() {
        let directory = tempfile::tempdir().unwrap();
        let first = save_unique(directory.path(), ScreenshotFormat::Png, &png()).unwrap();
        let second = save_unique(directory.path(), ScreenshotFormat::Png, &png()).unwrap();
        assert_ne!(first, second);

        let custom = directory.path().join("edited.png");
        assert_eq!(
            save_as(&custom, ScreenshotFormat::Png, &png()).unwrap(),
            custom.canonicalize().unwrap()
        );
        assert!(
            image::ImageReader::open(directory.path().join("edited.png"))
                .unwrap()
                .decode()
                .is_ok()
        );
    }
}
