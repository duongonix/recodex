use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tempfile::{Builder, TempDir};
use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};

use crate::{
    audio::AudioMixSample, capture::platform::windows::CapturedFrame, errors::IpcError,
    recording::encoding::windows_media::MediaFoundationWriter,
};

/// A native H.264/MP4 output transaction. Incomplete output is retained in a temporary folder
/// after a write/finalize failure so the user can recover it.
pub struct VideoOutput {
    writer: Option<MediaFoundationWriter>,
    transaction: OutputTransaction,
    input_width: u32,
    input_height: u32,
}

pub struct PendingVideoOutput {
    transaction: OutputTransaction,
}

impl VideoOutput {
    pub fn create(
        output_directory: &Path,
        input_width: u32,
        input_height: u32,
        max_height: u16,
        fps: u16,
    ) -> Result<Self, IpcError> {
        Self::create_with_audio(
            output_directory,
            input_width,
            input_height,
            max_height,
            fps,
            false,
            false,
        )
    }

    pub fn create_mixed_audio(
        output_directory: &Path,
        input_width: u32,
        input_height: u32,
        max_height: u16,
        fps: u16,
        audio_enabled: bool,
    ) -> Result<Self, IpcError> {
        Self::create_with_audio(
            output_directory,
            input_width,
            input_height,
            max_height,
            fps,
            audio_enabled,
            false,
        )
    }

    pub fn create_with_audio(
        output_directory: &Path,
        input_width: u32,
        input_height: u32,
        max_height: u16,
        fps: u16,
        microphone: bool,
        system_audio: bool,
    ) -> Result<Self, IpcError> {
        let transaction = OutputTransaction::begin(output_directory)?;
        let writer = MediaFoundationWriter::create_with_audio(
            transaction.temporary_file(),
            input_width,
            input_height,
            max_height,
            fps,
            microphone,
            system_audio,
        )?;
        Ok(Self {
            writer: Some(writer),
            transaction,
            input_width,
            input_height,
        })
    }

    pub fn write_frame(&mut self, frame: &CapturedFrame) -> Result<(), IpcError> {
        self.write_frame_pixels(
            frame.width,
            frame.height,
            &frame.pixels_bgra,
            frame.timestamp_100ns,
        )
    }

    pub fn write_frame_pixels(
        &mut self,
        width: u32,
        height: u32,
        pixels_bgra: &[u8],
        timestamp_100ns: i64,
    ) -> Result<(), IpcError> {
        if width != self.input_width || height != self.input_height {
            return Err(IpcError::new(
                "frame_layout_invalid",
                "Captured frame dimensions changed during this recording session.",
                true,
            ));
        }
        self.transaction.preserve_on_drop = true;
        self.writer
            .as_mut()
            .ok_or_else(|| IpcError::internal("The MP4 writer is already finalized."))?
            .write_frame(pixels_bgra, timestamp_100ns)
            .map_err(|error| self.recovery_error(error))
    }

    pub fn write_mixed_audio(&mut self, sample: &AudioMixSample) -> Result<(), IpcError> {
        self.write_audio_track(
            crate::recording::AudioSource::Microphone,
            &sample.pcm_s16le,
            sample.frame_count,
            sample.timestamp_100ns,
        )
    }

    pub fn write_audio_track(
        &mut self,
        source: crate::recording::AudioSource,
        pcm_s16le: &[u8],
        frame_count: u32,
        timestamp_100ns: i64,
    ) -> Result<(), IpcError> {
        self.transaction.preserve_on_drop = true;
        self.writer
            .as_mut()
            .ok_or_else(|| IpcError::internal("The MP4 writer is already finalized."))?
            .write_audio(source, pcm_s16le, frame_count, timestamp_100ns)
            .map_err(|error| self.recovery_error(error))
    }

    pub fn finalize(mut self) -> Result<PathBuf, IpcError> {
        let writer = self
            .writer
            .take()
            .ok_or_else(|| IpcError::internal("The MP4 writer is already finalized."))?;
        writer
            .finalize()
            .map_err(|error| self.transaction.attach_recovery_location(error))?;
        self.transaction.commit()
    }

    pub fn finalize_for_review(mut self) -> Result<PendingVideoOutput, IpcError> {
        let writer = self
            .writer
            .take()
            .ok_or_else(|| IpcError::internal("The MP4 writer is already finalized."))?;
        writer
            .finalize()
            .map_err(|error| self.transaction.attach_recovery_location(error))?;
        self.transaction
            .validate()
            .map_err(|error| self.transaction.attach_recovery_location(error))?;
        Ok(PendingVideoOutput {
            transaction: self.transaction,
        })
    }

    pub fn recoverable_path(&self) -> &Path {
        self.transaction.temporary_directory_path()
    }

    pub fn recovery_error(&self, error: IpcError) -> IpcError {
        self.transaction.attach_recovery_location(error)
    }

    pub fn discard_incomplete(&mut self) {
        self.transaction.preserve_on_drop = false;
    }
}

impl PendingVideoOutput {
    pub fn preview_path(&self) -> &Path {
        &self.transaction.temporary_file
    }

    pub(crate) fn output_directory(&self) -> &Path {
        &self.transaction.output_directory
    }

    pub fn save(&mut self) -> Result<PathBuf, IpcError> {
        self.transaction.commit()
    }

    pub fn save_as(
        &mut self,
        target_path: &Path,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(f32),
    ) -> Result<PathBuf, IpcError> {
        self.transaction.commit_as(target_path, cancelled, progress)
    }

    pub fn discard(&mut self) {
        self.transaction.preserve_on_drop = false;
    }
}

struct OutputTransaction {
    output_directory: PathBuf,
    temporary_directory: Option<TempDir>,
    temporary_file: PathBuf,
    preserve_on_drop: bool,
}

impl OutputTransaction {
    fn begin(output_directory: &Path) -> Result<Self, IpcError> {
        if !output_directory.is_dir() {
            return Err(IpcError::new(
                "output_directory_unavailable",
                "The selected recording folder does not exist or is not a directory.",
                true,
            ));
        }
        let temporary_directory = Builder::new()
            .prefix(".recodex-recording-")
            .tempdir_in(output_directory)
            .map_err(|error| {
                storage_io_error(
                    "Could not create a temporary recording file in the selected folder",
                    error,
                )
            })?;
        let temporary_file = temporary_directory.path().join("recording.mp4");
        Ok(Self {
            output_directory: output_directory.to_owned(),
            temporary_directory: Some(temporary_directory),
            temporary_file,
            preserve_on_drop: false,
        })
    }

    fn temporary_file(&self) -> &Path {
        &self.temporary_file
    }

    fn temporary_directory_path(&self) -> &Path {
        match self.temporary_directory.as_ref() {
            Some(directory) => directory.path(),
            None => self.output_directory.as_path(),
        }
    }

    fn attach_recovery_location(&self, mut error: IpcError) -> IpcError {
        error.message.push_str(&format!(
            " Recoverable temporary output: {}.",
            self.temporary_directory_path().display()
        ));
        error
    }

    fn validate(&self) -> Result<(), IpcError> {
        validate_mp4(&self.temporary_file)
    }

    fn commit(&mut self) -> Result<PathBuf, IpcError> {
        if let Err(error) = self.validate() {
            return Err(self.attach_recovery_location(error));
        }

        let base_name = format!("RecoDex-{}.mp4", unix_timestamp_millis());
        let final_path = self
            .publish_collision_safe_file(&base_name)
            .map_err(|error| self.attach_recovery_location(error))?;
        self.preserve_on_drop = false;
        if let Some(temporary_directory) = self.temporary_directory.take() {
            let _ = temporary_directory.close();
        }
        Ok(final_path)
    }

    fn commit_as(
        &mut self,
        target_path: &Path,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(f32),
    ) -> Result<PathBuf, IpcError> {
        self.validate()
            .map_err(|error| self.attach_recovery_location(error))?;
        let parent = target_path.parent().ok_or_else(|| {
            IpcError::new(
                "output_path_invalid",
                "Choose an MP4 file inside an existing folder.",
                true,
            )
        })?;
        if !parent.is_dir() || target_path.file_name().is_none() {
            return Err(IpcError::new(
                "output_path_invalid",
                "Choose an MP4 destination inside an existing folder.",
                true,
            ));
        }
        if target_path.exists() {
            return Err(IpcError::new(
                "output_file_exists",
                "A file already exists at the selected destination. Choose a new filename to avoid overwriting it.",
                true,
            ));
        }
        if cancelled.load(Ordering::Relaxed) {
            self.preserve_on_drop = false;
            return Err(export_cancelled());
        }
        let (staged_file, staging_directory) = if parent == self.output_directory {
            (self.temporary_file.clone(), None)
        } else {
            let staging_directory = Builder::new()
                .prefix(".recodex-export-")
                .tempdir_in(parent)
                .map_err(|error| {
                    self.attach_recovery_location(storage_io_error(
                        "Could not create a temporary export beside the selected destination",
                        error,
                    ))
                })?;
            let staged_file = staging_directory.path().join("export.mp4");
            if let Err(error) =
                copy_export_with_progress(&self.temporary_file, &staged_file, cancelled, progress)
            {
                if error.code == "video_export_cancelled" {
                    self.preserve_on_drop = false;
                    return Err(error);
                }
                return Err(self.attach_recovery_location(error));
            }
            validate_mp4(&staged_file).map_err(|error| self.attach_recovery_location(error))?;
            (staged_file, Some(staging_directory))
        };
        if cancelled.load(Ordering::Relaxed) {
            self.preserve_on_drop = false;
            return Err(export_cancelled());
        }
        let source = wide_null(staged_file.as_os_str());
        let destination = wide_null(target_path.as_os_str());
        let moved = unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved == 0 {
            return Err(self.attach_recovery_location(storage_io_error(
                "Could not save the finalized MP4 to the selected destination without replacing an existing file",
                std::io::Error::last_os_error(),
            )));
        }
        self.preserve_on_drop = false;
        if let Some(staging_directory) = staging_directory {
            let _ = staging_directory.close();
        }
        if let Some(directory) = self.temporary_directory.take() {
            let _ = directory.close();
        }
        Ok(target_path.to_owned())
    }

    fn publish_collision_safe_file(&self, base_name: &str) -> Result<PathBuf, IpcError> {
        let (stem, extension) = base_name.rsplit_once('.').ok_or_else(|| {
            IpcError::internal("The generated MP4 filename is missing its extension.")
        })?;
        let source = wide_null(self.temporary_file.as_os_str());
        for suffix in 0..10_000_u32 {
            let file_name = if suffix == 0 {
                base_name.to_owned()
            } else {
                format!("{stem}-{suffix}.{extension}")
            };
            let candidate = self.output_directory.join(file_name);
            let destination = wide_null(candidate.as_os_str());
            let moved = unsafe {
                MoveFileExW(
                    source.as_ptr(),
                    destination.as_ptr(),
                    MOVEFILE_WRITE_THROUGH,
                )
            };
            if moved != 0 {
                return Ok(candidate);
            }
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                continue;
            }
            return Err(storage_io_error(
                "Could not atomically move the finalized MP4 without replacing an existing file",
                error,
            ));
        }
        Err(IpcError::new(
            "output_name_exhausted",
            "Could not find an unused MP4 filename in the selected output folder.",
            true,
        ))
    }
}

fn copy_export_with_progress(
    source_path: &Path,
    destination_path: &Path,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(f32),
) -> Result<(), IpcError> {
    let mut source = File::open(source_path).map_err(|error| {
        storage_io_error("Could not open the finalized MP4 for export staging", error)
    })?;
    let total_bytes = source
        .metadata()
        .map_err(|error| storage_io_error("Could not inspect the finalized MP4", error))?
        .len()
        .max(1);
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination_path)
        .map_err(|error| {
            storage_io_error(
                "Could not create the staged MP4 in the destination folder",
                error,
            )
        })?;
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut copied = 0_u64;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(export_cancelled());
        }
        let count = source.read(&mut buffer).map_err(|error| {
            storage_io_error(
                "Could not read the finalized MP4 during export staging",
                error,
            )
        })?;
        if count == 0 {
            break;
        }
        destination.write_all(&buffer[..count]).map_err(|error| {
            storage_io_error(
                "Could not write the staged MP4 in the destination folder",
                error,
            )
        })?;
        copied = copied.saturating_add(count as u64);
        progress(0.95 + 0.04 * (copied as f32 / total_bytes as f32).clamp(0.0, 1.0));
    }
    destination.flush().map_err(|error| {
        storage_io_error(
            "Could not flush the staged MP4 in the destination folder",
            error,
        )
    })?;
    destination.sync_all().map_err(|error| {
        storage_io_error(
            "Could not finalize the staged MP4 in the destination folder",
            error,
        )
    })?;
    Ok(())
}

fn export_cancelled() -> IpcError {
    IpcError::new(
        "video_export_cancelled",
        "The video export was canceled. The original recording is still available.",
        true,
    )
}

fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn storage_io_error(context: &str, error: io::Error) -> IpcError {
    let is_disk_full = error.kind() == io::ErrorKind::StorageFull
        || matches!(error.raw_os_error(), Some(28 | 39 | 112));
    if is_disk_full {
        IpcError::new(
            "output_disk_full",
            format!("{context}: {error}. Free space on the selected recording disk and try again."),
            true,
        )
    } else {
        IpcError::new("output_create_failed", format!("{context}: {error}"), true)
    }
}

impl Drop for OutputTransaction {
    fn drop(&mut self) {
        if self.preserve_on_drop
            && let Some(directory) = self.temporary_directory.take()
        {
            let _ = directory.keep();
        }
    }
}

fn validate_mp4(path: &Path) -> Result<(), IpcError> {
    let mut file = File::open(path).map_err(|error| {
        IpcError::new(
            "mp4_invalid",
            format!("The Media Foundation output file could not be opened: {error}"),
            true,
        )
    })?;
    let file_length = file
        .metadata()
        .map_err(|error| {
            IpcError::new(
                "mp4_invalid",
                format!("Could not inspect the MP4 output: {error}"),
                true,
            )
        })?
        .len();
    if file_length < 24 {
        return Err(invalid_mp4());
    }

    let mut has_ftyp = false;
    let mut has_moov = false;
    let mut has_mdat = false;
    let mut position = 0_u64;
    while position < file_length {
        if file_length - position < 8 {
            return Err(invalid_mp4());
        }
        file.seek(SeekFrom::Start(position)).map_err(mp4_io_error)?;
        let mut header = [0_u8; 16];
        file.read_exact(&mut header[..8]).map_err(mp4_io_error)?;
        let size32 = u32::from_be_bytes(header[..4].try_into().map_err(|_| invalid_mp4())?);
        let box_type: [u8; 4] = header[4..8].try_into().map_err(|_| invalid_mp4())?;
        let (box_size, header_size) = match size32 {
            0 => (file_length - position, 8_u64),
            1 => {
                if file_length - position < 16 {
                    return Err(invalid_mp4());
                }
                file.read_exact(&mut header[8..16]).map_err(mp4_io_error)?;
                (
                    u64::from_be_bytes(header[8..16].try_into().map_err(|_| invalid_mp4())?),
                    16_u64,
                )
            }
            size => (u64::from(size), 8_u64),
        };
        let next = position.checked_add(box_size).ok_or_else(invalid_mp4)?;
        if box_size < header_size || next > file_length {
            return Err(invalid_mp4());
        }
        has_ftyp |= box_type == *b"ftyp";
        has_moov |= box_type == *b"moov";
        has_mdat |= box_type == *b"mdat";
        if size32 == 0 {
            position = file_length;
        } else {
            position = next;
        }
    }

    if has_ftyp && has_moov && has_mdat {
        Ok(())
    } else {
        Err(invalid_mp4())
    }
}

fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn mp4_io_error(error: std::io::Error) -> IpcError {
    IpcError::new(
        "mp4_invalid",
        format!("Could not read finalized MP4 metadata: {error}"),
        true,
    )
}

fn invalid_mp4() -> IpcError {
    IpcError::new(
        "mp4_invalid",
        "The video encoder did not produce a complete MP4 file (ftyp, moov, and mdat boxes are required).",
        true,
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, sync::atomic::AtomicBool};

    use super::{OutputTransaction, validate_mp4};

    fn box_bytes(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(payload.len() + 8).unwrap();
        [size.to_be_bytes().as_slice(), kind, payload].concat()
    }

    fn valid_mp4_bytes() -> Vec<u8> {
        [
            box_bytes(b"ftyp", b"isom"),
            box_bytes(b"mdat", &[1, 2, 3]),
            box_bytes(b"moov", &[]),
        ]
        .concat()
    }

    #[test]
    fn finalized_mp4_requires_valid_top_level_boxes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("valid.mp4");
        fs::write(&path, valid_mp4_bytes()).unwrap();
        assert!(validate_mp4(&path).is_ok());

        fs::write(&path, box_bytes(b"ftyp", b"isom")).unwrap();
        assert_eq!(validate_mp4(&path).unwrap_err().code, "mp4_invalid");
    }

    #[test]
    fn output_publication_never_overwrites_an_existing_mp4() {
        let directory = tempfile::tempdir().unwrap();
        let mut transaction = OutputTransaction::begin(directory.path()).unwrap();
        transaction.preserve_on_drop = true;
        fs::File::create(transaction.temporary_file())
            .unwrap()
            .write_all(&valid_mp4_bytes())
            .unwrap();
        let existing = directory.path().join("capture.mp4");
        fs::write(&existing, b"keep-existing-file").unwrap();

        // Exercise the same atomic move primitive with a deterministic colliding name.
        let published = transaction
            .publish_collision_safe_file("capture.mp4")
            .unwrap();
        assert_eq!(published, existing.with_file_name("capture-1.mp4"));
        assert_eq!(fs::read(existing).unwrap(), b"keep-existing-file");
        assert_eq!(fs::read(published).unwrap(), valid_mp4_bytes());
        transaction.preserve_on_drop = false;
    }

    #[test]
    fn valid_output_is_published_and_temporary_directory_is_removed() {
        let directory = tempfile::tempdir().unwrap();
        let mut transaction = OutputTransaction::begin(directory.path()).unwrap();
        transaction.preserve_on_drop = true;
        let recovery_path = transaction.temporary_directory_path().to_owned();
        fs::write(transaction.temporary_file(), valid_mp4_bytes()).unwrap();

        let final_path = transaction.commit().unwrap();

        assert!(final_path.exists());
        assert_eq!(final_path.parent(), Some(directory.path()));
        assert_eq!(
            final_path.extension().and_then(|value| value.to_str()),
            Some("mp4")
        );
        assert!(!recovery_path.exists());
    }

    #[test]
    fn save_as_supports_another_folder_and_publishes_without_overwrite() {
        let root = tempfile::tempdir().unwrap();
        let source_directory = root.path().join("recordings");
        let destination_directory = root.path().join("exports");
        fs::create_dir(&source_directory).unwrap();
        fs::create_dir(&destination_directory).unwrap();
        let mut transaction = OutputTransaction::begin(&source_directory).unwrap();
        transaction.preserve_on_drop = true;
        let recovery_path = transaction.temporary_directory_path().to_owned();
        fs::write(transaction.temporary_file(), valid_mp4_bytes()).unwrap();
        let destination = destination_directory.join("edited.mp4");

        let saved = transaction
            .commit_as(&destination, &AtomicBool::new(false), &mut |_| {})
            .unwrap();

        assert_eq!(saved, destination);
        assert_eq!(fs::read(saved).unwrap(), valid_mp4_bytes());
        assert!(!recovery_path.exists());
    }

    #[test]
    fn save_as_publishes_in_place_without_copying_the_source_transaction() {
        let directory = tempfile::tempdir().unwrap();
        let mut transaction = OutputTransaction::begin(directory.path()).unwrap();
        transaction.preserve_on_drop = true;
        let recovery_path = transaction.temporary_directory_path().to_owned();
        fs::write(transaction.temporary_file(), valid_mp4_bytes()).unwrap();
        let destination = directory.path().join("edited.mp4");

        let saved = transaction
            .commit_as(&destination, &AtomicBool::new(false), &mut |_| {})
            .unwrap();

        assert_eq!(saved, destination);
        assert_eq!(fs::read(saved).unwrap(), valid_mp4_bytes());
        assert!(!recovery_path.exists());
    }

    #[test]
    fn save_as_preserves_an_existing_destination_and_pending_source() {
        let root = tempfile::tempdir().unwrap();
        let mut transaction = OutputTransaction::begin(root.path()).unwrap();
        transaction.preserve_on_drop = true;
        fs::write(transaction.temporary_file(), valid_mp4_bytes()).unwrap();
        let destination = root.path().join("edited.mp4");
        fs::write(&destination, b"keep-existing-file").unwrap();
        let recovery_file = transaction.temporary_file().to_owned();

        let error = transaction
            .commit_as(&destination, &AtomicBool::new(false), &mut |_| {})
            .unwrap_err();

        assert_eq!(error.code, "output_file_exists");
        assert_eq!(fs::read(destination).unwrap(), b"keep-existing-file");
        assert_eq!(fs::read(recovery_file).unwrap(), valid_mp4_bytes());
    }

    #[test]
    fn canceled_cross_folder_publish_removes_partial_export_and_source_transaction() {
        let root = tempfile::tempdir().unwrap();
        let source_directory = root.path().join("recordings");
        let destination_directory = root.path().join("exports");
        fs::create_dir(&source_directory).unwrap();
        fs::create_dir(&destination_directory).unwrap();
        let mut transaction = OutputTransaction::begin(&source_directory).unwrap();
        transaction.preserve_on_drop = true;
        let recovery_path = transaction.temporary_directory_path().to_owned();
        fs::write(transaction.temporary_file(), valid_mp4_bytes()).unwrap();
        let destination = destination_directory.join("edited.mp4");
        let cancelled = AtomicBool::new(true);

        let error = transaction
            .commit_as(&destination, &cancelled, &mut |_| {})
            .unwrap_err();

        assert_eq!(error.code, "video_export_cancelled");
        assert!(!destination.exists());
        drop(transaction);
        assert!(!recovery_path.exists());
    }

    #[test]
    fn disk_full_is_reported_as_a_recoverable_output_error() {
        let error = super::storage_io_error(
            "Could not write the recording",
            std::io::Error::from_raw_os_error(112),
        );
        assert_eq!(error.code, "output_disk_full");
        assert!(error.recoverable);
    }

    #[test]
    fn unstarted_output_transaction_is_removed_on_drop() {
        let directory = tempfile::tempdir().unwrap();
        let recovery_path;
        {
            let transaction = OutputTransaction::begin(directory.path()).unwrap();
            recovery_path = transaction.temporary_directory_path().to_owned();
        }
        assert!(!recovery_path.exists());
    }

    #[test]
    fn recording_failures_include_the_preserved_recovery_folder() {
        let directory = tempfile::tempdir().unwrap();
        let mut output = super::OutputTransaction::begin(directory.path()).unwrap();
        output.preserve_on_drop = true;
        let recovery_path = output.temporary_directory_path().display().to_string();
        let output = super::VideoOutput {
            writer: None,
            transaction: output,
            input_width: 1,
            input_height: 1,
        };
        let error = output.recovery_error(crate::errors::IpcError::new(
            "capture_device_lost",
            "The selected source stopped providing frames.",
            true,
        ));
        assert!(error.message.contains(&recovery_path));
    }

    #[test]
    fn unfinalized_output_is_preserved_for_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let recovery_path;
        {
            let mut transaction = OutputTransaction::begin(directory.path()).unwrap();
            transaction.preserve_on_drop = true;
            recovery_path = transaction.temporary_directory_path().to_owned();
            fs::write(transaction.temporary_file(), b"partial MP4").unwrap();
        }
        assert!(recovery_path.join("recording.mp4").exists());
        fs::remove_dir_all(recovery_path).unwrap();
    }
}
