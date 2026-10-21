mod capture;
mod com;
mod devices;
mod stream;

pub use capture::{AudioCapture, AudioMessage, AudioSample};
pub(super) use devices::list_audio_devices;

use crate::errors::IpcError;

pub(super) fn native_audio_error(
    operation: &'static str,
    error: impl std::fmt::Display,
) -> IpcError {
    IpcError::new(
        "audio_capture_failed",
        format!(
            "Could not {operation}: {error}. Check Windows sound privacy settings and the selected audio device."
        ),
        true,
    )
}
