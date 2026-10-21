pub mod editor;
pub mod encoding;
#[cfg(windows)]
pub mod output;
#[cfg(windows)]
mod output_edit;
pub mod preflight;
pub mod review;
pub mod runtime;
mod state;

pub use state::{
    AudioSource, CaptureTarget, RecordingEvent, RecordingOptions, RecordingQuality,
    RecordingSession, RecordingSnapshot, STATE_CHANGED_EVENT, SessionStatus,
};

pub fn emit_state_changed<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    event: &RecordingEvent,
) -> Result<(), crate::errors::IpcError> {
    use tauri::Emitter;

    app.emit(STATE_CHANGED_EVENT, event)
        .map_err(|error| crate::errors::IpcError::internal(error.to_string()))
}

pub fn emit_progress<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    event: &RecordingEvent,
) -> Result<(), crate::errors::IpcError> {
    use tauri::Emitter;

    app.emit(PROGRESS_EVENT, event)
        .map_err(|error| crate::errors::IpcError::internal(error.to_string()))
}

pub const PROGRESS_EVENT: &str = "recording://progress";
