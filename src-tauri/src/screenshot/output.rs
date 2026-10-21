use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tempfile::NamedTempFile;

use crate::{errors::IpcError, settings::ScreenshotFormat};

static NEXT_FILE_ID: AtomicU64 = AtomicU64::new(1);

pub(super) fn save(
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

    let extension = match format {
        ScreenshotFormat::Png => "png",
        ScreenshotFormat::Jpg => "jpg",
    };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut temporary = NamedTempFile::new_in(directory).map_err(|error| {
        IpcError::new(
            "screenshot_save_failed",
            format!("Could not create a temporary screenshot file: {error}"),
            true,
        )
    })?;
    temporary.write_all(bytes).map_err(|error| {
        IpcError::new(
            "screenshot_save_failed",
            format!("Could not write screenshot data: {error}"),
            true,
        )
    })?;
    temporary.as_file().sync_all().map_err(|error| {
        IpcError::new(
            "screenshot_save_failed",
            format!("Could not flush screenshot data to disk: {error}"),
            true,
        )
    })?;

    for _ in 0..100 {
        let file_id = NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let destination = directory.join(format!("RecoDex-{timestamp}-{file_id}.{extension}"));
        match temporary.persist_noclobber(&destination) {
            Ok(_) => return Ok(destination),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temporary = error.file;
            }
            Err(error) => {
                return Err(IpcError::new(
                    "screenshot_save_failed",
                    format!("Could not finalize screenshot file: {}", error.error),
                    true,
                ));
            }
        }
    }

    Err(IpcError::new(
        "screenshot_filename_exhausted",
        "Could not find an unused screenshot filename after 100 attempts.",
        true,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile::tempdir;

    #[test]
    fn saves_unique_png_files_without_overwriting_existing_files() {
        let directory = tempdir().unwrap();
        let first = save(directory.path(), ScreenshotFormat::Png, b"first").unwrap();
        let second = save(directory.path(), ScreenshotFormat::Png, b"second").unwrap();

        assert_ne!(first, second);
        assert_eq!(fs::read(&first).unwrap(), b"first");
        assert_eq!(fs::read(&second).unwrap(), b"second");
    }

    #[test]
    fn rejects_nonexistent_output_directories() {
        let directory = tempdir().unwrap();
        let missing = directory.path().join("missing");
        let error = save(&missing, ScreenshotFormat::Png, b"bytes").unwrap_err();

        assert_eq!(error.code, "screenshot_output_unavailable");
        assert!(!missing.exists());
    }
}
