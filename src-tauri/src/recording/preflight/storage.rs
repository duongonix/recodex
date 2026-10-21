use std::path::Path;

use crate::recording::RecordingOptions;

use super::{PreflightCheck, PreflightStatus, check};

const LOW_DISK_BYTES: u64 = 1_073_741_824;
const CRITICAL_DISK_BYTES: u64 = 157_286_400;
const LOW_DISK_MINUTES: u64 = 5;
const AAC_BITRATE_BITS_PER_SECOND: u64 = 96_000;

pub(super) struct StorageResult {
    pub(super) check: PreflightCheck,
    pub(super) free_disk_bytes: Option<u64>,
    pub(super) estimated_minutes_remaining: Option<u64>,
}

pub(super) fn inspect_storage(
    options: &RecordingOptions,
    dimensions: Option<(u32, u32)>,
) -> StorageResult {
    let output_directory = Path::new(&options.output_directory);
    if !output_directory.is_dir() {
        return StorageResult {
            check: check(
                "storage",
                "Save location",
                PreflightStatus::Error,
                "The recording folder is unavailable. Choose an existing folder in Settings."
                    .to_owned(),
            ),
            free_disk_bytes: None,
            estimated_minutes_remaining: None,
        };
    }
    if let Err(error) = tempfile::NamedTempFile::new_in(output_directory) {
        return StorageResult {
            check: check(
                "storage",
                "Save location",
                PreflightStatus::Error,
                format!("The recording folder is not writable: {error}"),
            ),
            free_disk_bytes: None,
            estimated_minutes_remaining: None,
        };
    }

    match query_disk_free_bytes(output_directory) {
        Ok(Some(available)) => {
            let estimated_minutes = dimensions.map(|(width, height)| {
                let video_bitrate = estimate_video_bitrate(width, height, options.fps);
                let audio_bitrate = if options.microphone_enabled || options.system_audio {
                    AAC_BITRATE_BITS_PER_SECOND
                } else {
                    0
                };
                available
                    .saturating_mul(8)
                    .checked_div(video_bitrate.saturating_add(audio_bitrate).max(1))
                    .unwrap_or_default()
                    / 60
            });

            let status = if available < CRITICAL_DISK_BYTES
                || estimated_minutes.is_some_and(|minutes| minutes == 0)
            {
                PreflightStatus::Error
            } else if available < LOW_DISK_BYTES
                || estimated_minutes.is_some_and(|minutes| minutes < LOW_DISK_MINUTES)
            {
                PreflightStatus::Warning
            } else {
                PreflightStatus::Ready
            };
            let remaining = estimated_minutes
                .map(|minutes| format!("Estimated time at this quality: about {minutes} min."))
                .unwrap_or_else(|| {
                    "Select an available capture source to estimate recording time.".to_owned()
                });
            let detail = match status {
                PreflightStatus::Error => format!(
                    "Only {} free. Free disk space before starting. {remaining}",
                    format_bytes(available)
                ),
                PreflightStatus::Warning => format!(
                    "{} free. A longer recording may run out of space. {remaining}",
                    format_bytes(available)
                ),
                _ => format!("{} free. {remaining}", format_bytes(available)),
            };

            StorageResult {
                check: check("storage", "Save location", status, detail),
                free_disk_bytes: Some(available),
                estimated_minutes_remaining: estimated_minutes,
            }
        }
        Ok(None) => StorageResult {
            check: check(
                "storage",
                "Save location",
                PreflightStatus::Warning,
                "Folder is writable, but free space cannot be estimated on this platform."
                    .to_owned(),
            ),
            free_disk_bytes: None,
            estimated_minutes_remaining: None,
        },
        Err(message) => StorageResult {
            check: check(
                "storage",
                "Save location",
                PreflightStatus::Warning,
                format!("Folder is writable, but Windows could not read free space: {message}"),
            ),
            free_disk_bytes: None,
            estimated_minutes_remaining: None,
        },
    }
}

fn estimate_video_bitrate(width: u32, height: u32, fps: u16) -> u64 {
    u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(u64::from(fps))
        .saturating_div(8)
        .clamp(2_000_000, 60_000_000)
}

fn format_bytes(bytes: u64) -> String {
    const GIB: u64 = 1_073_741_824;
    const MIB: u64 = 1_048_576;
    if bytes >= GIB {
        format!("{:.1} GB", bytes as f64 / GIB as f64)
    } else {
        format!("{} MB", bytes / MIB)
    }
}

#[cfg(windows)]
fn query_disk_free_bytes(path: &Path) -> Result<Option<u64>, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut available = 0_u64;
    let succeeded = unsafe {
        GetDiskFreeSpaceExW(
            path_wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if succeeded == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(Some(available))
    }
}

#[cfg(not(windows))]
fn query_disk_free_bytes(_path: &Path) -> Result<Option<u64>, String> {
    Ok(None)
}

pub fn free_bytes_for_path(path: &Path) -> Option<u64> {
    query_disk_free_bytes(path).ok().flatten()
}
