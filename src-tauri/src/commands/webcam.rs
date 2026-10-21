use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

use crate::{
    app_state::AppState,
    errors::IpcError,
    recording::SessionStatus,
    webcam::{devices, manager::PREVIEW_ERROR_EVENT, overlay_window},
};

#[tauri::command]
pub async fn list_webcam_devices(
    window: WebviewWindow,
) -> Result<Vec<devices::WebcamDevice>, IpcError> {
    require_main(&window)?;
    tauri::async_runtime::spawn_blocking(devices::enumerate)
        .await
        .map_err(|error| IpcError::internal(format!("Camera enumeration task failed: {error}")))?
}

#[tauri::command]
pub fn get_webcam_overlay_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<crate::webcam::settings::WebcamSettings, IpcError> {
    require_settings_window(&window)?;
    Ok(state.settings.get()?.webcam)
}

#[tauri::command]
pub async fn start_webcam_preview(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    camera_id: String,
) -> Result<(), IpcError> {
    require_main(&window)?;
    let status = state
        .recording
        .lock()
        .map_err(|_| IpcError::internal("Recording status is unavailable."))?
        .snapshot()
        .status;
    if !matches!(
        status,
        SessionStatus::Idle | SessionStatus::Completed | SessionStatus::Failed
    ) {
        return Err(IpcError::invalid_state(
            "Stop recording before opening the webcam preview.",
        ));
    }
    let manager = state.webcam.clone();
    let app_for_preview = app.clone();
    tauri::async_runtime::spawn_blocking(move || manager.start_preview(app_for_preview, camera_id))
        .await
        .map_err(|error| IpcError::internal(format!("Camera preview task failed: {error}")))??;
    let settings = state.settings.get()?.webcam;
    if settings.enabled {
        if let Some(overlay) = app.get_webview_window("webcam-overlay") {
            overlay_window::configure_size(&overlay, &settings, state.webcam.latest_dimensions())?;
        }
        overlay_window::emit_settings(&app, &settings)?;
        overlay_window::show_idle(&app)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_webcam_preview(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    require_main(&window)?;
    if state.settings.get()?.webcam.enabled {
        return Ok(());
    }
    let manager = state.webcam.clone();
    tauri::async_runtime::spawn_blocking(move || manager.stop_preview())
        .await
        .map_err(|error| IpcError::internal(format!("Camera preview stop task failed: {error}")))?
}

pub(crate) fn restore_after_recording(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Ok(settings) = state.settings.get() else {
        return;
    };
    if !settings.webcam.enabled {
        let _ = overlay_window::hide(app);
        return;
    }
    let Some(camera_id) = settings.webcam.camera_id.clone() else {
        let _ = overlay_window::hide(app);
        return;
    };

    if let Err(error) = state.webcam.start_preview(app.clone(), camera_id) {
        let _ = app.emit(PREVIEW_ERROR_EVENT, error.message);
        let _ = overlay_window::hide(app);
        return;
    }
    if let Some(overlay) = app.get_webview_window("webcam-overlay") {
        let _ = overlay_window::configure_size(
            &overlay,
            &settings.webcam,
            state.webcam.latest_dimensions(),
        );
    }
    let _ = overlay_window::emit_settings(app, &settings.webcam);
    if let Err(error) = overlay_window::show_idle(app) {
        let _ = app.emit(PREVIEW_ERROR_EVENT, error.message);
        let _ = overlay_window::hide(app);
    }
}

#[tauri::command]
pub async fn get_webcam_background_preview(
    window: WebviewWindow,
    path: String,
) -> Result<String, IpcError> {
    require_settings_window(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::webcam::preview::encode_background_preview(&path)
    })
    .await
    .map_err(|error| IpcError::internal(format!("Background preview task failed: {error}")))?
}

fn require_main(window: &WebviewWindow) -> Result<(), IpcError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "Webcam settings are available only from the main window.",
            false,
        ))
    }
}

fn require_settings_window(window: &WebviewWindow) -> Result<(), IpcError> {
    if matches!(window.label(), "main" | "webcam-overlay") {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "Webcam settings are available only from the main or webcam overlay window.",
            false,
        ))
    }
}
