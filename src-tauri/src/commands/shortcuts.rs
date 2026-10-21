use tauri::{State, WebviewWindow};

use crate::{
    app_state::AppState,
    errors::IpcError,
    shortcut::{HoldZoomShortcutStatus, QuickAccessShortcutStatus},
};

#[tauri::command]
pub fn get_quick_access_shortcut_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<QuickAccessShortcutStatus, IpcError> {
    if window.label() != "main" {
        return Err(IpcError::new(
            "window_not_authorized",
            "Shortcut status may only be read from the main window.",
            false,
        ));
    }
    state
        .quick_access_shortcut
        .lock()
        .map(|status| status.clone())
        .map_err(|_| IpcError::internal("Shortcut status is unavailable because its lock failed."))
}

#[tauri::command]
pub fn get_hold_zoom_shortcut_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<HoldZoomShortcutStatus, IpcError> {
    if window.label() != "main" {
        return Err(IpcError::new(
            "window_not_authorized",
            "Shortcut status may only be read from the main window.",
            false,
        ));
    }
    state
        .hold_zoom_shortcut
        .lock()
        .map(|status| status.clone())
        .map_err(|_| IpcError::internal("Hold-to-zoom shortcut status is unavailable."))
}
