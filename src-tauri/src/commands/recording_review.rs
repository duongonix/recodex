use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewWindow};

use crate::{
    app_state::AppState,
    errors::IpcError,
    recording::{
        SessionStatus, editor::ExportSegment, emit_state_changed,
        encoding::editor_render::EditorRenderSettings, review::RecordingReviewInfo,
    },
};

pub const REVIEW_READY_EVENT: &str = "recording-review://ready";
pub const REVIEW_ERROR_EVENT: &str = "recording-review://error";
pub const EXPORT_PROGRESS_EVENT: &str = "recording-review://export-progress";

struct VideoExportGuard {
    active: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for VideoExportGuard {
    fn drop(&mut self) {
        self.cancelled.store(false, Ordering::Release);
        self.active.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub fn get_pending_recording_review(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<RecordingReviewInfo>, IpcError> {
    require_window(&window, "recording-review")?;
    #[cfg(windows)]
    {
        state.recording_review.current()
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        Err(unsupported())
    }
}

#[tauri::command]
pub async fn get_recording_review_assets(
    window: WebviewWindow,
    state: State<'_, AppState>,
    review_id: String,
    bin_count: usize,
    thumbnail_count: usize,
) -> Result<crate::recording::editor::EditorTimelineAssets, IpcError> {
    require_window(&window, "recording-review")?;
    #[cfg(windows)]
    {
        ensure_reviewing(&state, Some(&review_id))?;
        let review = state
            .recording_review
            .current()?
            .ok_or_else(|| IpcError::invalid_state("There is no recording waiting for review."))?;
        let path = PathBuf::from(review.video_path);
        let duration_ms = review.duration_ms;
        tauri::async_runtime::spawn_blocking(move || {
            let audio_waveform =
                crate::recording::encoding::windows_media_reader::extract_audio_waveform(
                    &path,
                    duration_ms,
                    bin_count,
                )?;
            let video_thumbnails =
                crate::recording::encoding::windows_media_reader::extract_video_thumbnails(
                    &path,
                    duration_ms,
                    thumbnail_count,
                )?;
            Ok(crate::recording::editor::EditorTimelineAssets {
                audio_waveform,
                video_thumbnails,
            })
        })
        .await
        .map_err(|error| IpcError::internal(format!("The audio waveform task failed: {error}")))?
    }
    #[cfg(not(windows))]
    {
        let _ = (state, review_id, bin_count, thumbnail_count);
        Err(unsupported())
    }
}

#[tauri::command]
pub fn save_recording_review(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    review_id: String,
) -> Result<String, IpcError> {
    require_window(&window, "recording-review")?;
    finish_review(&app, &state, &review_id, true)?.ok_or_else(|| {
        IpcError::internal("Saving the pending recording did not return an output path.")
    })
}

#[tauri::command]
pub fn discard_recording_review(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    review_id: String,
) -> Result<(), IpcError> {
    require_window(&window, "recording-review")?;
    finish_review(&app, &state, &review_id, false).map(|_| ())
}

#[tauri::command]
pub async fn trim_recording_review(
    window: WebviewWindow,
    app: AppHandle,
    review_id: String,
    start_ms: u64,
    end_ms: u64,
) -> Result<String, IpcError> {
    require_window(&window, "recording-review")?;
    #[cfg(windows)]
    {
        let app = app.clone();
        let mut paths = tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<AppState>();
            finish_review_edit(
                &app,
                &state,
                &review_id,
                ReviewEdit::Trim { start_ms, end_ms },
                &mut |_| {},
            )
        })
        .await
        .map_err(|error| IpcError::internal(format!("The trim export task failed: {error}")))??;
        paths.pop().ok_or_else(|| {
            IpcError::internal("The trim export completed without returning its output path.")
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (app, review_id, start_ms, end_ms);
        Err(unsupported())
    }
}

#[tauri::command]
pub async fn split_recording_review(
    window: WebviewWindow,
    app: AppHandle,
    review_id: String,
    start_ms: u64,
    split_ms: u64,
    end_ms: u64,
) -> Result<Vec<String>, IpcError> {
    require_window(&window, "recording-review")?;
    #[cfg(windows)]
    {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<AppState>();
            finish_review_edit(
                &app,
                &state,
                &review_id,
                ReviewEdit::Split {
                    start_ms,
                    split_ms,
                    end_ms,
                },
                &mut |_| {},
            )
        })
        .await
        .map_err(|error| IpcError::internal(format!("The split export task failed: {error}")))?
    }
    #[cfg(not(windows))]
    {
        let _ = (app, review_id, start_ms, split_ms, end_ms);
        Err(unsupported())
    }
}

#[tauri::command]
pub async fn export_recording_timeline(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    review_id: String,
    segments: Vec<ExportSegment>,
    settings: EditorRenderSettings,
    output_path: String,
) -> Result<String, IpcError> {
    require_window(&window, "recording-review")?;
    let output_path = PathBuf::from(output_path);
    if !output_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"))
        || !output_path.parent().is_some_and(std::path::Path::is_dir)
        || output_path.file_name().is_none()
    {
        return Err(IpcError::new(
            "output_path_invalid",
            "Choose an MP4 file inside an existing folder.",
            true,
        ));
    }
    #[cfg(windows)]
    {
        let active = Arc::clone(&state.video_editor_export_active);
        let cancelled = Arc::clone(&state.video_editor_export_cancelled);
        if active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(IpcError::new(
                "video_export_in_progress",
                "A video timeline export is already running.",
                true,
            ));
        }
        cancelled.store(false, Ordering::Release);
        let app = app.clone();
        let task_cancelled = Arc::clone(&cancelled);
        let export_guard = VideoExportGuard {
            active: Arc::clone(&active),
            cancelled: Arc::clone(&cancelled),
        };
        let result = tauri::async_runtime::spawn_blocking(move || {
            let _export_guard = export_guard;
            let state = app.state::<AppState>();
            let mut emit_progress = |progress: f32| {
                let _ = app.emit_to("recording-review", EXPORT_PROGRESS_EVENT, progress);
            };
            finish_review_edit(
                &app,
                &state,
                &review_id,
                ReviewEdit::Timeline {
                    segments,
                    settings,
                    output_path,
                    cancelled: task_cancelled,
                },
                &mut emit_progress,
            )
        })
        .await
        .map_err(|error| IpcError::internal(format!("The video export task failed: {error}")))
        .and_then(|result| result);
        let mut paths = result?;
        paths
            .pop()
            .ok_or_else(|| IpcError::internal("The video export completed without an output path."))
    }
    #[cfg(not(windows))]
    {
        let _ = (app, review_id, segments, settings, output_path);
        Err(unsupported())
    }
}

#[tauri::command]
pub fn cancel_recording_timeline_export(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    require_window(&window, "recording-review")?;
    if state.video_editor_export_active.load(Ordering::Acquire) {
        state
            .video_editor_export_cancelled
            .store(true, Ordering::Release);
    }
    Ok(())
}

#[tauri::command]
pub fn focus_recording_review(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    require_window(&window, "main")?;
    ensure_reviewing(&state, None)?;
    show_review_window(&app)
}

pub fn save_pending_for_quit<R: Runtime>(app: &AppHandle<R>) -> Result<(), IpcError> {
    #[cfg(windows)]
    {
        let state = app.state::<AppState>();
        let Some(review) = state.recording_review.current()? else {
            return Ok(());
        };
        finish_review(app, &state, &review.review_id, true).map(|_| ())
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(())
    }
}

pub fn save_pending_on_window_close<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
) -> Result<bool, IpcError> {
    #[cfg(windows)]
    {
        let Some(review) = state.recording_review.current()? else {
            return Ok(false);
        };
        finish_review(app, state, &review.review_id, true)?;
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, state);
        Ok(false)
    }
}

#[cfg(windows)]
fn finish_review<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    review_id: &str,
    save: bool,
) -> Result<Option<String>, IpcError> {
    ensure_reviewing(state, Some(review_id))?;
    let outcome = if save {
        state.recording_review.save(review_id)?
    } else {
        state.recording_review.discard(review_id)?
    };
    if let Err(error) = app.asset_protocol_scope().forbid_file(&outcome.video_path) {
        eprintln!("[RecoDex][recording-review] Could not revoke preview file scope: {error}");
    }

    let output_path = outcome
        .output_path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    if let Some(path) = outcome.output_path.as_deref()
        && let Err(error) = state.capture_library.register(
            path,
            crate::library::LibraryItemKind::Video,
            Some(outcome.duration_ms),
        )
    {
        eprintln!(
            "[RecoDex][library] Could not register the saved recording: {}",
            error.message
        );
    }
    let event = state
        .recording
        .lock()
        .map_err(|_| IpcError::internal("Recording state is unavailable because its lock failed."))?
        .transition(
            SessionStatus::Completed,
            Some(&outcome.review_id),
            outcome.duration_ms,
            output_path.clone(),
            None,
        )?;
    let _ = emit_state_changed(app, &event);

    if let Some(review_window) = app.get_webview_window("recording-review") {
        let _ = review_window.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
    Ok(output_path)
}

#[cfg(windows)]
enum ReviewEdit {
    Trim {
        start_ms: u64,
        end_ms: u64,
    },
    Split {
        start_ms: u64,
        split_ms: u64,
        end_ms: u64,
    },
    Timeline {
        segments: Vec<ExportSegment>,
        settings: EditorRenderSettings,
        output_path: PathBuf,
        cancelled: Arc<AtomicBool>,
    },
}

#[cfg(windows)]
fn finish_review_edit<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    review_id: &str,
    edit: ReviewEdit,
    progress: &mut dyn FnMut(f32),
) -> Result<Vec<String>, IpcError> {
    ensure_reviewing(state, Some(review_id))?;
    let outcome = match edit {
        ReviewEdit::Trim { start_ms, end_ms } => {
            state.recording_review.trim(review_id, start_ms, end_ms)?
        }
        ReviewEdit::Split {
            start_ms,
            split_ms,
            end_ms,
        } => state
            .recording_review
            .split(review_id, start_ms, split_ms, end_ms)?,
        ReviewEdit::Timeline {
            segments,
            settings,
            output_path,
            cancelled,
        } => state.recording_review.export_timeline(
            review_id,
            &segments,
            &settings,
            &output_path,
            cancelled.as_ref(),
            progress,
        )?,
    };
    if let Err(error) = app.asset_protocol_scope().forbid_file(&outcome.video_path) {
        eprintln!("[RecoDex][recording-review] Could not revoke preview file scope: {error}");
    }
    let output_paths: Vec<String> = outcome
        .output_paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    for path in &outcome.output_paths {
        if let Err(error) =
            state
                .capture_library
                .register(path, crate::library::LibraryItemKind::Video, None)
        {
            eprintln!(
                "[RecoDex][library] Could not register an edited recording: {}",
                error.message
            );
        }
    }
    let primary_output = output_paths.first().ok_or_else(|| {
        IpcError::internal("The recording edit completed without an output file.")
    })?;
    let event = state
        .recording
        .lock()
        .map_err(|_| IpcError::internal("Recording state is unavailable because its lock failed."))?
        .transition(
            SessionStatus::Completed,
            Some(&outcome.review_id),
            outcome.duration_ms,
            Some(primary_output.clone()),
            None,
        )?;
    let _ = emit_state_changed(app, &event);

    if let Some(review_window) = app.get_webview_window("recording-review") {
        let _ = review_window.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
    Ok(output_paths)
}

#[cfg(not(windows))]
fn finish_review<R: Runtime>(
    _app: &AppHandle<R>,
    _state: &AppState,
    _review_id: &str,
    _save: bool,
) -> Result<Option<String>, IpcError> {
    Err(unsupported())
}

fn ensure_reviewing(state: &AppState, review_id: Option<&str>) -> Result<(), IpcError> {
    let snapshot = state
        .recording
        .lock()
        .map_err(|_| IpcError::internal("Recording state is unavailable because its lock failed."))?
        .snapshot();
    if snapshot.status != SessionStatus::Reviewing {
        return Err(IpcError::invalid_state(
            "There is no recording waiting for review.",
        ));
    }
    if review_id.is_some_and(|id| snapshot.session_id.as_deref() != Some(id)) {
        return Err(IpcError::invalid_state(
            "The review ID does not match the active recording session.",
        ));
    }
    Ok(())
}

fn show_review_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), IpcError> {
    let window = app.get_webview_window("recording-review").ok_or_else(|| {
        IpcError::new(
            "recording_review_window_unavailable",
            "The recording preview window is unavailable.",
            true,
        )
    })?;
    window.show().map_err(|error| {
        IpcError::new(
            "recording_review_show_failed",
            format!("Could not show the recording preview: {error}"),
            true,
        )
    })?;
    window.set_focus().map_err(|error| {
        IpcError::new(
            "recording_review_focus_failed",
            format!("Could not focus the recording preview: {error}"),
            true,
        )
    })
}

fn require_window(window: &WebviewWindow, label: &str) -> Result<(), IpcError> {
    if window.label() == label {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "This window is not allowed to manage the recording preview.",
            false,
        ))
    }
}

#[cfg(not(windows))]
fn unsupported() -> IpcError {
    IpcError::new(
        "recording_review_unsupported",
        "Recording review is available only on Windows while native video recording is supported.",
        true,
    )
}

pub fn emit_review_ready<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit_to("recording-review", REVIEW_READY_EVENT, ());
}

pub fn show_review_after_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), IpcError> {
    show_review_window(app)
}

pub fn report_review_error<R: Runtime>(app: &AppHandle<R>, message: &str) {
    let _ = app.emit_to("recording-review", REVIEW_ERROR_EVENT, message.to_owned());
}
