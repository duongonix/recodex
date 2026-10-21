use tauri::{AppHandle, Manager, State, WebviewWindow};

use crate::{
    app_state::AppState,
    errors::IpcError,
    settings::{AppSettings, SettingsPatch},
    webcam::overlay_window,
};

#[tauri::command]
pub fn get_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<AppSettings, IpcError> {
    require_main_window(&window)?;
    state.settings.get()
}

#[tauri::command]
pub async fn update_settings(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<AppSettings, IpcError> {
    require_main_window(&window)?;
    let _update_guard = state.hold_zoom_shortcut_update.lock().map_err(|_| {
        IpcError::internal("Settings could not be updated because the shortcut lock failed.")
    })?;
    let old = state.settings.get()?;
    let capture_active = state
        .recording
        .lock()
        .map_err(|_| IpcError::internal("Recording status is unavailable."))?
        .snapshot()
        .status;
    if matches!(
        capture_active,
        crate::recording::SessionStatus::Preparing
            | crate::recording::SessionStatus::Recording
            | crate::recording::SessionStatus::Paused
            | crate::recording::SessionStatus::Stopping
            | crate::recording::SessionStatus::Finalizing
    ) && patch
        .webcam
        .as_ref()
        .is_some_and(|webcam| webcam.enabled != old.webcam.enabled)
    {
        return Err(IpcError::invalid_state(
            "Use the recording toolbar to toggle the webcam during recording.",
        ));
    }
    if patch
        .webcam
        .as_ref()
        .is_some_and(|webcam| webcam.enabled && webcam.camera_id.is_none())
    {
        return Err(IpcError::new(
            "webcam_camera_unselected",
            "Choose a camera in Webcam Overlay settings before enabling the desktop overlay.",
            true,
        ));
    }
    if let Some(accelerator) = patch.zoom_on_hold_shortcut.as_deref() {
        crate::shortcut::validate_hold_zoom_shortcut(accelerator)?;
    }
    let updated = state.settings.update(patch)?;
    if (old.zoom_on_hold_enabled != updated.zoom_on_hold_enabled
        || old.zoom_on_hold_shortcut != updated.zoom_on_hold_shortcut)
        && let Err(error) = crate::shortcut::update_hold_zoom_shortcut(
            &app,
            old.zoom_on_hold_enabled,
            &old.zoom_on_hold_shortcut,
            updated.zoom_on_hold_enabled,
            &updated.zoom_on_hold_shortcut,
        )
    {
        let rollback = state.settings.update(SettingsPatch {
            zoom_on_hold_enabled: Some(old.zoom_on_hold_enabled),
            zoom_on_hold_shortcut: Some(old.zoom_on_hold_shortcut.clone()),
            ..SettingsPatch::default()
        });
        let webcam_rollback = if old.webcam != updated.webcam {
            state.settings.update(SettingsPatch {
                webcam: Some(old.webcam.clone()),
                ..SettingsPatch::default()
            })
        } else {
            Ok(old.clone())
        };
        if let Err(webcam_error) = webcam_rollback {
            return Err(IpcError::internal(format!(
                "Could not restore webcam settings after shortcut registration failed: {}",
                webcam_error.message
            )));
        }
        if let Err(rollback_error) = rollback {
            return Err(IpcError::internal(format!(
                "Could not register the shortcut ({}) or restore saved settings ({}).",
                error.message, rollback_error.message
            )));
        }
        return Err(error);
    }

    drop(_update_guard);
    if old.webcam != updated.webcam
        && !matches!(
            capture_active,
            crate::recording::SessionStatus::Preparing
                | crate::recording::SessionStatus::Recording
                | crate::recording::SessionStatus::Paused
                | crate::recording::SessionStatus::Stopping
                | crate::recording::SessionStatus::Finalizing
        )
    {
        sync_idle_webcam_overlay(&app, &state, &old, &updated).await?;
    }
    Ok(updated)
}

async fn sync_idle_webcam_overlay(
    app: &AppHandle,
    state: &State<'_, AppState>,
    old: &AppSettings,
    updated: &AppSettings,
) -> Result<(), IpcError> {
    if updated.webcam.enabled {
        let camera_id = updated.webcam.camera_id.clone().ok_or_else(|| {
            IpcError::new(
                "webcam_camera_unselected",
                "Choose a camera in Webcam Overlay settings before enabling the desktop overlay.",
                true,
            )
        })?;
        let manager = state.webcam.clone();
        let app_for_preview = app.clone();
        if let Err(error) = tauri::async_runtime::spawn_blocking(move || {
            manager.start_preview(app_for_preview, camera_id)
        })
        .await
        .map_err(|error| IpcError::internal(format!("Camera startup task failed: {error}")))?
        {
            rollback_webcam_settings(state, old)?;
            restore_idle_webcam_overlay(app, state, old).await;
            return Err(error);
        }

        let overlay = match app.get_webview_window("webcam-overlay") {
            Some(overlay) => overlay,
            None => {
                let manager = state.webcam.clone();
                let _ = tauri::async_runtime::spawn_blocking(move || manager.stop_preview()).await;
                rollback_webcam_settings(state, old)?;
                restore_idle_webcam_overlay(app, state, old).await;
                return Err(IpcError::new(
                    "webcam_overlay_unavailable",
                    "The webcam overlay window is unavailable.",
                    true,
                ));
            }
        };
        let dimensions = state.webcam.latest_dimensions();
        if let Err(error) = overlay_window::configure_size(&overlay, &updated.webcam, dimensions)
            .and_then(|()| overlay_window::emit_settings(app, &updated.webcam))
            .and_then(|()| overlay_window::show_idle(app))
        {
            let manager = state.webcam.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || manager.stop_preview()).await;
            rollback_webcam_settings(state, old)?;
            restore_idle_webcam_overlay(app, state, old).await;
            return Err(error);
        }
    } else if old.webcam.enabled {
        let manager = state.webcam.clone();
        let stop_result = tauri::async_runtime::spawn_blocking(move || manager.stop_preview())
            .await
            .map_err(|error| IpcError::internal(format!("Camera stop task failed: {error}")))?;
        let hide_result = overlay_window::hide(app);
        stop_result?;
        hide_result?;
        overlay_window::emit_settings(app, &updated.webcam)?;
    } else {
        overlay_window::emit_settings(app, &updated.webcam)?;
    }
    Ok(())
}

async fn restore_idle_webcam_overlay(
    app: &AppHandle,
    state: &State<'_, AppState>,
    settings: &AppSettings,
) {
    let webcam = &settings.webcam;
    let Some(camera_id) = webcam.camera_id.clone().filter(|_| webcam.enabled) else {
        let _ = overlay_window::hide(app);
        let _ = overlay_window::emit_settings(app, webcam);
        return;
    };

    let manager = state.webcam.clone();
    let app_for_preview = app.clone();
    let started = tauri::async_runtime::spawn_blocking(move || {
        manager.start_preview(app_for_preview, camera_id)
    })
    .await;
    let Ok(Ok(())) = started else {
        let _ = overlay_window::hide(app);
        return;
    };

    let result = app
        .get_webview_window("webcam-overlay")
        .ok_or_else(|| {
            IpcError::new(
                "webcam_overlay_unavailable",
                "The webcam overlay window is unavailable.",
                true,
            )
        })
        .and_then(|overlay| {
            overlay_window::configure_size(&overlay, webcam, state.webcam.latest_dimensions())
        })
        .and_then(|()| overlay_window::emit_settings(app, webcam))
        .and_then(|()| overlay_window::show_idle(app));
    if result.is_err() {
        let manager = state.webcam.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || manager.stop_preview()).await;
        let _ = overlay_window::hide(app);
    }
}

fn rollback_webcam_settings(
    state: &State<'_, AppState>,
    old: &AppSettings,
) -> Result<(), IpcError> {
    state.settings.update(SettingsPatch {
        webcam: Some(old.webcam.clone()),
        ..SettingsPatch::default()
    })?;
    Ok(())
}

fn require_main_window(window: &WebviewWindow) -> Result<(), IpcError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "Settings may only be changed from the main window.",
            false,
        ))
    }
}
