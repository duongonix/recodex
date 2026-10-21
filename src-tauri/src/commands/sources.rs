use tauri::{AppHandle, WebviewWindow};

use crate::{
    capture::sources::{self, DisplaySource},
    errors::IpcError,
};

#[tauri::command]
pub fn list_displays(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Vec<DisplaySource>, IpcError> {
    require_main_window(&window)?;
    sources::list_displays(&app)
}

fn require_main_window(window: &WebviewWindow) -> Result<(), IpcError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "Capture sources may only be listed by the main window.",
            false,
        ))
    }
}
