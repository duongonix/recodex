use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, State};

use crate::{
    app_state::AppState,
    errors::IpcError,
    recording::{SessionStatus, runtime::WorkerCommand},
};

use super::recording::issue_worker_command;

pub async fn stop_before_quit(app: &AppHandle) -> Result<(), IpcError> {
    let state = app.state::<AppState>();
    let snapshot = state
        .recording
        .lock()
        .map_err(|_| IpcError::internal("Recording state is unavailable because its lock failed."))?
        .snapshot();

    match snapshot.status {
        SessionStatus::Preparing => {
            let worker = state
                .recording_control
                .lock()
                .map_err(|_| {
                    IpcError::internal(
                        "Recording controls are unavailable because their lock failed.",
                    )
                })?
                .as_ref()
                .cloned()
                .ok_or_else(|| {
                    IpcError::invalid_state(
                        "Recording startup is still initializing. Try Quit again shortly.",
                    )
                })?;
            worker.send(WorkerCommand::Cancel).map_err(|_| {
                IpcError::invalid_state("The recording worker is no longer available.")
            })?;
            wait_for_quit_completion(
                app,
                &state,
                snapshot.session_id.as_deref().unwrap_or_default(),
            )
            .await
        }
        SessionStatus::Recording | SessionStatus::Paused => {
            let session_id = snapshot.session_id.as_deref().ok_or_else(|| {
                IpcError::invalid_state("The active recording has no session ID.")
            })?;
            issue_worker_command(
                &state,
                session_id.to_owned(),
                Duration::from_secs(120),
                WorkerCommand::Stop,
            )
            .await?;
            crate::commands::recording_review::save_pending_for_quit(app)
        }
        SessionStatus::Stopping | SessionStatus::Finalizing => {
            wait_for_quit_completion(
                app,
                &state,
                snapshot.session_id.as_deref().unwrap_or_default(),
            )
            .await
        }
        SessionStatus::Reviewing => crate::commands::recording_review::save_pending_for_quit(app),
        SessionStatus::Idle | SessionStatus::Completed | SessionStatus::Failed => Ok(()),
    }
}

async fn wait_for_quit_completion(
    app: &AppHandle,
    state: &State<'_, AppState>,
    session_id: &str,
) -> Result<(), IpcError> {
    let deadline = Instant::now() + Duration::from_secs(125);
    loop {
        let snapshot = state
            .recording
            .lock()
            .map_err(|_| {
                IpcError::internal("Recording state is unavailable because its lock failed.")
            })?
            .snapshot();
        match snapshot.status {
            SessionStatus::Completed | SessionStatus::Idle => return Ok(()),
            SessionStatus::Reviewing => {
                return crate::commands::recording_review::save_pending_for_quit(app);
            }
            SessionStatus::Failed => {
                let error = snapshot.error.ok_or_else(|| {
                    IpcError::internal("Recording stopped without a recoverable error message.")
                })?;
                if error.code == "recording_start_cancelled"
                    && !error.message.contains("Recoverable temporary output:")
                {
                    return Ok(());
                }
                return Err(IpcError::new(&error.code, error.message, error.recoverable));
            }
            SessionStatus::Recording | SessionStatus::Paused => {
                issue_worker_command(
                    state,
                    session_id.to_owned(),
                    Duration::from_secs(120),
                    WorkerCommand::Stop,
                )
                .await?;
                return crate::commands::recording_review::save_pending_for_quit(app);
            }
            SessionStatus::Preparing | SessionStatus::Stopping | SessionStatus::Finalizing => {}
        }

        if Instant::now() >= deadline {
            return Err(IpcError::new(
                "recording_shutdown_timeout",
                "The recording did not finish in time. RecoDex remains open so you can check its status and recover any temporary output.",
                true,
            ));
        }
        tauri::async_runtime::spawn_blocking(|| std::thread::sleep(Duration::from_millis(100)))
            .await
            .map_err(|error| {
                IpcError::internal(format!("Recording shutdown wait failed: {error}"))
            })?;
    }
}
