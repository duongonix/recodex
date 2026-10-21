use std::{
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, SyncSender, TryRecvError},
    },
    thread,
};

use tauri::{AppHandle, Manager};

#[cfg(windows)]
mod clock;
#[cfg(windows)]
mod webcam;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use clock::ActiveClock;
#[cfg(windows)]
use windows::run_platform;

use crate::{
    errors::IpcError,
    recording::{
        AudioSource, RecordingEvent, RecordingOptions, RecordingQuality, RecordingSession,
        RecordingSnapshot, SessionStatus, emit_state_changed,
    },
};

pub enum WorkerCommand {
    Pause(SyncSender<Result<RecordingSnapshot, IpcError>>),
    Resume(SyncSender<Result<RecordingSnapshot, IpcError>>),
    Stop(SyncSender<Result<RecordingSnapshot, IpcError>>),
    Cancel,
    SetAudio {
        source: AudioSource,
        enabled: bool,
        response: SyncSender<Result<RecordingSnapshot, IpcError>>,
    },
    SetWebcam {
        enabled: bool,
        response: SyncSender<Result<RecordingSnapshot, IpcError>>,
    },
    MoveWebcam {
        delta_x: i32,
        delta_y: i32,
    },
}

pub(super) fn check_startup_cancellation(
    commands: &Receiver<WorkerCommand>,
) -> Result<(), IpcError> {
    match commands.try_recv() {
        Ok(WorkerCommand::Cancel) => Err(IpcError::new(
            "recording_start_cancelled",
            "Recording startup was cancelled before the session became active.",
            true,
        )),
        Ok(WorkerCommand::Pause(response)) | Ok(WorkerCommand::Resume(response)) => {
            let _ = response.send(Err(IpcError::invalid_state(
                "Recording controls are unavailable until capture is ready.",
            )));
            Ok(())
        }
        Ok(WorkerCommand::Stop(response)) => {
            let _ = response.send(Err(IpcError::invalid_state(
                "Recording cannot be stopped until capture is ready.",
            )));
            Ok(())
        }
        Ok(WorkerCommand::SetAudio { response, .. }) => {
            let _ = response.send(Err(IpcError::invalid_state(
                "Audio controls are unavailable until capture is ready.",
            )));
            Ok(())
        }
        Ok(WorkerCommand::SetWebcam { response, .. }) => {
            let _ = response.send(Err(IpcError::invalid_state(
                "Webcam controls are unavailable until capture is ready.",
            )));
            Ok(())
        }
        Ok(WorkerCommand::MoveWebcam { .. }) => Ok(()),
        Err(TryRecvError::Disconnected) => Err(IpcError::new(
            "recording_control_lost",
            "Recording controls disconnected during startup; capture was cancelled.",
            true,
        )),
        Err(TryRecvError::Empty) => Ok(()),
    }
}

pub type RecordingControl = SyncSender<WorkerCommand>;
type WorkerReady = Receiver<Result<(), IpcError>>;

pub fn spawn_recording_worker(
    app: AppHandle,
    session: Arc<Mutex<RecordingSession>>,
    session_id: String,
    options: RecordingOptions,
) -> Result<(RecordingControl, WorkerReady), IpcError> {
    let (command_tx, command_rx) = std::sync::mpsc::sync_channel(8);
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    thread::Builder::new()
        .name("recodex-recording".to_owned())
        .spawn(move || {
            let mut ready_tx = Some(ready_tx);
            let result = run_platform(
                app.clone(),
                session.clone(),
                &session_id,
                options,
                command_rx,
                &mut ready_tx,
            );
            if let Err(error) = result {
                if let Some(response) = ready_tx.take() {
                    let _ = response.send(Err(error.clone()));
                }
                fail_session(&app, &session, &session_id, error);
            }
            let should_restore_overlay = session.lock().is_ok_and(|session| {
                matches!(
                    session.snapshot().status,
                    SessionStatus::Reviewing | SessionStatus::Completed | SessionStatus::Failed
                )
            });
            if should_restore_overlay {
                crate::commands::webcam::restore_after_recording(&app);
            }
        })
        .map_err(|error| {
            IpcError::new(
                "recording_worker_start_failed",
                format!("Could not start the native recording worker: {error}"),
                true,
            )
        })?;
    Ok((command_tx, ready_rx))
}

#[cfg(not(windows))]
fn run_platform(
    _app: AppHandle,
    _session: Arc<Mutex<RecordingSession>>,
    _session_id: &str,
    _options: RecordingOptions,
    _commands: Receiver<WorkerCommand>,
    _ready: &mut Option<SyncSender<Result<(), IpcError>>>,
) -> Result<(), IpcError> {
    Err(IpcError::new(
        "capture_unavailable",
        "Native video recording is currently implemented only for Windows.",
        true,
    ))
}

pub(super) fn transition_and_emit(
    app: &AppHandle,
    session: &Arc<Mutex<RecordingSession>>,
    session_id: &str,
    status: SessionStatus,
    duration: u64,
    output_path: Option<String>,
    error: Option<crate::recording::state::RecordingError>,
) -> Result<RecordingEvent, IpcError> {
    let event = session
        .lock()
        .map_err(|_| IpcError::internal("The recording state lock failed."))?
        .transition(status, Some(session_id), duration, output_path, error)?;
    emit_state_changed(app, &event)?;
    Ok(event)
}

pub(super) fn update_progress(
    session: &Arc<Mutex<RecordingSession>>,
    session_id: &str,
    duration: u64,
    quality: RecordingQuality,
) -> Result<RecordingEvent, IpcError> {
    session
        .lock()
        .map_err(|_| IpcError::internal("The recording state lock failed."))?
        .update_progress_with_quality(session_id, duration, Some(quality))
}

pub(super) fn respond_with_snapshot(
    session: &Arc<Mutex<RecordingSession>>,
    response: SyncSender<Result<RecordingSnapshot, IpcError>>,
    result: Result<RecordingEvent, IpcError>,
) {
    let result = result.and_then(|_| {
        session
            .lock()
            .map(|session| session.snapshot())
            .map_err(|_| IpcError::internal("The recording state lock failed."))
    });
    let _ = response.send(result);
}

pub(crate) fn fail_session(
    app: &AppHandle,
    session: &Arc<Mutex<RecordingSession>>,
    session_id: &str,
    error: IpcError,
) {
    if let Some(toolbar) = app.get_webview_window("recording-toolbar") {
        let _ = toolbar.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
    }
    let Ok(mut session) = session.lock() else {
        return;
    };
    let duration = session.snapshot().recorded_duration_ms;
    let failed = session.transition(
        SessionStatus::Failed,
        Some(session_id),
        duration,
        None,
        Some(crate::recording::state::RecordingError {
            code: error.code,
            message: error.message,
            recoverable: error.recoverable,
        }),
    );
    if let Ok(event) = failed {
        let _ = emit_state_changed(app, &event);
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::{WorkerCommand, check_startup_cancellation};
    use std::sync::mpsc::sync_channel;

    #[test]
    fn startup_cancel_is_reported_to_the_recording_worker() {
        let (sender, receiver) = sync_channel(1);
        sender.send(WorkerCommand::Cancel).unwrap();

        let error = check_startup_cancellation(&receiver).unwrap_err();
        assert_eq!(error.code, "recording_start_cancelled");
    }

    #[test]
    fn startup_continues_when_no_cancellation_is_pending() {
        let (_sender, receiver) = sync_channel(1);
        assert!(check_startup_cancellation(&receiver).is_ok());
    }
}
