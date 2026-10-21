use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tauri::{AppHandle, State, WebviewWindow};

use crate::{
    app_state::AppState,
    errors::IpcError,
    recording::{
        AudioSource, RecordingOptions, RecordingSnapshot, SessionStatus, emit_state_changed,
        runtime::{WorkerCommand, spawn_recording_worker},
    },
};

#[tauri::command]
pub fn get_recording_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<RecordingSnapshot, IpcError> {
    require_window(&window, &["main", "recording-toolbar"])?;
    state
        .recording
        .lock()
        .map(|session| session.snapshot())
        .map_err(|_| IpcError::internal("Recording state is unavailable because its lock failed."))
}

#[tauri::command]
pub async fn get_recording_preflight(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    options: RecordingOptions,
) -> Result<crate::recording::preflight::RecordingPreflightReport, IpcError> {
    require_window(&window, &["main"])?;
    let options = validate_options(options)?;
    let webcam = state.settings.get()?.webcam;
    tauri::async_runtime::spawn_blocking(move || {
        crate::recording::preflight::inspect(&app, &options, &webcam)
    })
    .await
    .map_err(|error| IpcError::internal(format!("Recording preflight failed: {error}")))
}

#[tauri::command]
pub async fn start_recording(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    options: RecordingOptions,
) -> Result<String, IpcError> {
    require_window(&window, &["main"])?;
    let options = validate_options(options)?;
    let webcam = state.webcam.clone();
    tauri::async_runtime::spawn_blocking(move || webcam.stop_preview())
        .await
        .map_err(|error| IpcError::internal(format!("Could not stop webcam preview: {error}")))??;
    state
        .zoom_hotkey_held
        .store(false, std::sync::atomic::Ordering::Release);
    if let Ok(mut toolbar_position) = state.toolbar_last_position.lock() {
        *toolbar_position = None;
    }
    let session_id = new_session_id();
    let shared_session = state.recording.clone();

    {
        let mut session = shared_session.lock().map_err(|_| {
            IpcError::internal("Recording state is unavailable because its lock failed.")
        })?;
        let current = session.snapshot();
        match current.status {
            SessionStatus::Idle => {}
            SessionStatus::Completed | SessionStatus::Failed => {
                let reset = session.transition(
                    SessionStatus::Idle,
                    current.session_id.as_deref(),
                    current.recorded_duration_ms,
                    current.output_path,
                    None,
                )?;
                emit_state_changed(&app, &reset)?;
            }
            _ => {
                return Err(IpcError::invalid_state(
                    "A recording session is already active.",
                ));
            }
        }
        let preparing =
            session.transition(SessionStatus::Preparing, Some(&session_id), 0, None, None)?;
        emit_state_changed(&app, &preparing)?;
    }

    let (control, ready) = match spawn_recording_worker(
        app.clone(),
        shared_session.clone(),
        session_id.clone(),
        options,
    ) {
        Ok(worker) => worker,
        Err(error) => {
            crate::recording::runtime::fail_session(
                &app,
                &shared_session,
                &session_id,
                error.clone(),
            );
            crate::commands::webcam::restore_after_recording(&app);
            return Err(error);
        }
    };
    *state.recording_control.lock().map_err(|_| {
        IpcError::internal("Recording controls are unavailable because their lock failed.")
    })? = Some(control.clone());

    if let Err(mut error) = await_worker_response(ready, Duration::from_secs(15)).await {
        if error.code == "recording_worker_timeout" {
            let _ = control.send(WorkerCommand::Cancel);
            error
                .message
                .push_str(" Startup cancellation was requested.");
        }
        return Err(error);
    }
    Ok(session_id)
}

#[tauri::command]
pub async fn pause_recording(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<RecordingSnapshot, IpcError> {
    require_window(&window, &["recording-toolbar"])?;
    issue_worker_command(&state, session_id, Duration::from_secs(8), |response| {
        WorkerCommand::Pause(response)
    })
    .await
}

#[tauri::command]
pub async fn resume_recording(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<RecordingSnapshot, IpcError> {
    require_window(&window, &["recording-toolbar"])?;
    issue_worker_command(&state, session_id, Duration::from_secs(8), |response| {
        WorkerCommand::Resume(response)
    })
    .await
}

#[tauri::command]
pub async fn stop_recording(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<RecordingSnapshot, IpcError> {
    require_window(&window, &["recording-toolbar"])?;
    issue_worker_command(
        &state,
        session_id,
        Duration::from_secs(120),
        WorkerCommand::Stop,
    )
    .await
}

#[tauri::command]
pub async fn set_recording_audio(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    source: AudioSource,
    enabled: bool,
) -> Result<RecordingSnapshot, IpcError> {
    require_window(&window, &["recording-toolbar"])?;
    issue_worker_command(&state, session_id, Duration::from_secs(8), |response| {
        WorkerCommand::SetAudio {
            source,
            enabled,
            response,
        }
    })
    .await
}

#[tauri::command]
pub async fn set_recording_webcam(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    enabled: bool,
) -> Result<RecordingSnapshot, IpcError> {
    require_window(&window, &["recording-toolbar"])?;
    let snapshot = issue_worker_command(&state, session_id, Duration::from_secs(12), |response| {
        WorkerCommand::SetWebcam { enabled, response }
    })
    .await?;
    let current = state.settings.get()?;
    if current.webcam.enabled != snapshot.webcam_enabled {
        let mut webcam = current.webcam;
        webcam.enabled = snapshot.webcam_enabled;
        state.settings.update(crate::settings::SettingsPatch {
            webcam: Some(webcam.clone()),
            ..crate::settings::SettingsPatch::default()
        })?;
        crate::webcam::overlay_window::emit_settings(&app, &webcam)?;
    }
    Ok(snapshot)
}

pub(super) async fn issue_worker_command(
    state: &State<'_, AppState>,
    session_id: String,
    timeout: Duration,
    build: impl FnOnce(SyncSender<Result<RecordingSnapshot, IpcError>>) -> WorkerCommand,
) -> Result<RecordingSnapshot, IpcError> {
    let status = state
        .recording
        .lock()
        .map_err(|_| IpcError::internal("Recording state is unavailable because its lock failed."))?
        .snapshot();
    if status.session_id.as_deref() != Some(session_id.as_str()) {
        return Err(IpcError::invalid_state(
            "The session ID does not match the active recording.",
        ));
    }

    let worker = state
        .recording_control
        .lock()
        .map_err(|_| {
            IpcError::internal("Recording controls are unavailable because their lock failed.")
        })?
        .as_ref()
        .cloned()
        .ok_or_else(|| IpcError::invalid_state("There is no active recording worker."))?;
    let (response_tx, response_rx) = std::sync::mpsc::sync_channel(1);
    worker
        .send(build(response_tx))
        .map_err(|_| IpcError::invalid_state("The recording worker is no longer running."))?;
    await_worker_response(response_rx, timeout).await
}

async fn await_worker_response<T: Send + 'static>(
    receiver: Receiver<Result<T, IpcError>>,
    timeout: Duration,
) -> Result<T, IpcError> {
    tauri::async_runtime::spawn_blocking(move || receiver.recv_timeout(timeout))
        .await
        .map_err(|error| IpcError::internal(format!("Recording worker wait failed: {error}")))?
        .map_err(|error| match error {
            std::sync::mpsc::RecvTimeoutError::Timeout => IpcError::new(
                "recording_worker_timeout",
                "The recording worker did not respond in time. Check the recording status before trying again.",
                true,
            ),
            std::sync::mpsc::RecvTimeoutError::Disconnected => IpcError::new(
                "recording_worker_disconnected",
                "The recording worker stopped before confirming the request.",
                true,
            ),
        })?
}

fn validate_options(mut options: RecordingOptions) -> Result<RecordingOptions, IpcError> {
    if !matches!(options.fps, 30 | 60) || !matches!(options.max_height, 720 | 1080 | 2160) {
        return Err(IpcError::new(
            "recording_options_invalid",
            "Recording requires 30 or 60 FPS and a 720p, 1080p, or 2160p maximum height.",
            true,
        ));
    }
    if options
        .microphone_device_id
        .as_ref()
        .is_some_and(|id| id.trim().is_empty())
    {
        return Err(IpcError::new(
            "audio_device_invalid",
            "A selected microphone device ID cannot be empty.",
            true,
        ));
    }
    if options.output_directory.trim().is_empty() {
        return Err(IpcError::new(
            "output_directory_unavailable",
            "Choose an existing folder for the recording.",
            true,
        ));
    }
    let path = Path::new(&options.output_directory);
    if !path.is_absolute() || !path.is_dir() {
        return Err(IpcError::new(
            "output_directory_unavailable",
            "The selected recording folder does not exist or is not an absolute path.",
            true,
        ));
    }
    options.output_directory = path
        .canonicalize()
        .map_err(|error| {
            IpcError::new(
                "output_directory_unavailable",
                format!("Could not resolve the recording folder: {error}"),
                true,
            )
        })?
        .to_string_lossy()
        .into_owned();
    Ok(options)
}

fn new_session_id() -> String {
    static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "recording-{nanos}-{}",
        NEXT_SESSION.fetch_add(1, Ordering::Relaxed)
    )
}

fn require_window(window: &WebviewWindow, allowed: &[&str]) -> Result<(), IpcError> {
    if allowed.contains(&window.label()) {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "This window is not allowed to run the requested command.",
            false,
        ))
    }
}
