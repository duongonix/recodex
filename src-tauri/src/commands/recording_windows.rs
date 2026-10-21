use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewWindow, Window, WindowEvent};

use crate::{app_state::AppState, errors::IpcError, recording::SessionStatus};

#[tauri::command]
pub fn show_recording_toolbar(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    require_window(&window, "main")?;
    ensure_active(&state)?;
    let toolbar = app.get_webview_window("recording-toolbar").ok_or_else(|| {
        IpcError::new(
            "toolbar_window_unavailable",
            "The recording toolbar window is unavailable.",
            true,
        )
    })?;
    toolbar.set_always_on_top(true).map_err(|error| {
        IpcError::new(
            "toolbar_topmost_failed",
            format!("Could not keep the recording toolbar above other windows: {error}"),
            true,
        )
    })?;
    window.hide().map_err(|error| {
        IpcError::new(
            "main_window_hide_failed",
            format!("Could not hide the main window before reopening the toolbar: {error}"),
            true,
        )
    })?;
    if let Err(error) = toolbar.show() {
        let _ = window.show();
        return Err(IpcError::new(
            "toolbar_show_failed",
            format!("Could not show the recording toolbar: {error}"),
            true,
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn hide_recording_toolbar(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    require_window(&window, "recording-toolbar")?;
    ensure_active(&state)?;
    let main = app.get_webview_window("main").ok_or_else(|| {
        IpcError::new(
            "main_window_unavailable",
            "The main window is unavailable.",
            true,
        )
    })?;
    window.hide().map_err(|error| {
        IpcError::new(
            "toolbar_hide_failed",
            format!("Could not hide the toolbar: {error}"),
            true,
        )
    })?;
    if let Err(error) = main.show() {
        let _ = window.show();
        return Err(IpcError::new(
            "main_window_show_failed",
            format!("Could not open the main window: {error}"),
            true,
        ));
    }
    main.set_focus().map_err(|error| {
        IpcError::new(
            "main_window_focus_failed",
            format!("Could not focus the main window: {error}"),
            true,
        )
    })?;
    Ok(())
}

pub fn handle_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::Moved(position) = event {
        if window.label() == "recording-toolbar"
            && let Some(state) = window.try_state::<AppState>()
        {
            handle_toolbar_moved(window.app_handle(), &state, position.x, position.y);
        }
        return;
    }
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    let Some(state) = window.try_state::<AppState>() else {
        return;
    };
    match window.label() {
        "main" => handle_main_close(window, api, &state),
        "recording-toolbar" => handle_toolbar_close(window, api, &state),
        "recording-review" => handle_review_close(window, api, &state),
        "window-picker" => {
            let _ = window.app_handle().emit_to(
                "main",
                crate::commands::window_picker::WINDOW_PICKER_CANCELLED_EVENT,
                (),
            );
        }
        _ => {}
    }
}

fn handle_toolbar_moved<R: Runtime>(app: &tauri::AppHandle<R>, state: &AppState, x: i32, y: i32) {
    if !has_active_session(state) {
        return;
    }
    let Ok(mut last_position) = state.toolbar_last_position.lock() else {
        return;
    };
    let Some((last_x, last_y)) = *last_position else {
        *last_position = Some((x, y));
        return;
    };
    let delta_x = x.saturating_sub(last_x);
    let delta_y = y.saturating_sub(last_y);
    if delta_x == 0 && delta_y == 0 {
        return;
    }
    let Ok(control) = state.recording_control.lock() else {
        return;
    };
    let Some(control) = control.as_ref() else {
        return;
    };
    match control
        .try_send(crate::recording::runtime::WorkerCommand::MoveWebcam { delta_x, delta_y })
    {
        Ok(()) | Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
            *last_position = Some((x, y));
        }
        // Keep the old baseline so the next move event includes any delta not queued here.
        Err(std::sync::mpsc::TrySendError::Full(_)) => {}
    }
    drop(last_position);
    let webcam_enabled = state
        .recording
        .lock()
        .is_ok_and(|session| session.snapshot().webcam_enabled);
    if webcam_enabled && let Err(error) = crate::webcam::overlay_window::follow_toolbar(app, x, y) {
        let _ = app.emit(
            crate::webcam::manager::RECORDING_WARNING_EVENT,
            format!(
                "Webcam overlay could not follow the recording toolbar: {}",
                error.message
            ),
        );
    }
}

fn handle_main_close<R: Runtime>(
    window: &Window<R>,
    api: &tauri::CloseRequestApi,
    state: &AppState,
) {
    let status = state
        .recording
        .lock()
        .ok()
        .map(|session| session.snapshot().status);
    if status == Some(SessionStatus::Reviewing) {
        api.prevent_close();
        if let Some(review) = window.app_handle().get_webview_window("recording-review") {
            let _ = review.show();
            let _ = review.set_focus();
        }
        return;
    }
    if !has_active_session(state) {
        let minimize_to_tray = state
            .settings
            .get()
            .is_ok_and(|settings| settings.minimize_to_tray);
        api.prevent_close();
        if minimize_to_tray {
            let _ = window.hide();
        } else {
            window.app_handle().exit(0);
        }
        return;
    }

    api.prevent_close();
    let app = window.app_handle();
    let Some(toolbar) = app.get_webview_window("recording-toolbar") else {
        return;
    };
    if toolbar.set_always_on_top(true).is_ok() && window.hide().is_ok() {
        if toolbar.show().is_ok() {
            let _ = toolbar.set_focus();
        } else {
            let _ = window.show();
        }
    }
}

fn handle_review_close<R: Runtime>(
    window: &Window<R>,
    api: &tauri::CloseRequestApi,
    state: &AppState,
) {
    api.prevent_close();
    match crate::commands::recording_review::save_pending_on_window_close(
        window.app_handle(),
        state,
    ) {
        Ok(true) => {}
        Ok(false) => {
            let _ = window.hide();
            if let Some(main) = window.app_handle().get_webview_window("main") {
                let _ = main.show();
                let _ = main.set_focus();
            }
        }
        Err(error) => {
            crate::commands::recording_review::report_review_error(
                window.app_handle(),
                &error.message,
            );
            eprintln!("[RecoDex][recording-review] Could not save on window close: {error:?}");
        }
    }
}

fn handle_toolbar_close<R: Runtime>(
    window: &Window<R>,
    api: &tauri::CloseRequestApi,
    state: &AppState,
) {
    if !has_active_session(state) {
        return;
    }
    api.prevent_close();

    let app = window.app_handle();
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    if window.hide().is_ok() {
        if main.show().is_ok() {
            let _ = main.set_focus();
        } else {
            let _ = window.show();
        }
    }
}

fn ensure_active(state: &AppState) -> Result<(), IpcError> {
    if has_active_session(state) {
        Ok(())
    } else {
        Err(IpcError::invalid_state(
            "The recording toolbar is available only while a recording session is active.",
        ))
    }
}

fn has_active_session(state: &AppState) -> bool {
    state.recording.lock().is_ok_and(|session| {
        matches!(
            session.snapshot().status,
            SessionStatus::Preparing
                | SessionStatus::Recording
                | SessionStatus::Paused
                | SessionStatus::Stopping
                | SessionStatus::Finalizing
        )
    })
}

fn require_window(window: &WebviewWindow, label: &str) -> Result<(), IpcError> {
    if window.label() == label {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "This window is not allowed to run the requested command.",
            false,
        ))
    }
}
