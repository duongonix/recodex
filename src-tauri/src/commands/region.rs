use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, State,
    WebviewWindow,
};

use crate::{
    app_state::AppState,
    capture::{
        region::{self, RegionPickerState},
        sources,
    },
    errors::IpcError,
    recording::CaptureTarget,
};

const REGION_PICKER_OPENED_EVENT: &str = "capture://region-picker-opened";
const REGION_SELECTED_EVENT: &str = "capture://region-selected";

#[tauri::command]
pub fn open_region_picker(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    display_id: String,
) -> Result<(), IpcError> {
    require_window(&window, "main")?;
    let display = sources::find_display(&sources::list_displays(&app)?, &display_id)?;
    let overlay = get_overlay(&app)?;

    overlay
        .set_position(Position::Physical(PhysicalPosition::new(
            display.x, display.y,
        )))
        .map_err(|error| {
            IpcError::internal(format!("Could not position the region picker: {error}"))
        })?;
    overlay
        .set_size(Size::Physical(PhysicalSize::new(
            display.width,
            display.height,
        )))
        .map_err(|error| {
            IpcError::internal(format!("Could not size the region picker: {error}"))
        })?;

    let picker_state = RegionPickerState { display };
    set_picker_state(&state, Some(picker_state.clone()))?;
    if let Err(error) = app.emit_to("region-overlay", REGION_PICKER_OPENED_EVENT, &picker_state) {
        set_picker_state(&state, None)?;
        return Err(IpcError::internal(format!(
            "Could not initialize the region picker: {error}"
        )));
    }
    if let Err(error) = overlay.show() {
        set_picker_state(&state, None)?;
        return Err(IpcError::internal(format!(
            "Could not show the region picker: {error}"
        )));
    }
    if let Err(error) = overlay.set_focus() {
        let _ = overlay.hide();
        set_picker_state(&state, None)?;
        return Err(IpcError::internal(format!(
            "Could not focus the region picker: {error}"
        )));
    }
    if let Err(error) = window.hide() {
        let _ = overlay.hide();
        set_picker_state(&state, None)?;
        return Err(IpcError::internal(format!(
            "Could not hide the main window: {error}"
        )));
    }

    Ok(())
}

#[tauri::command]
pub fn get_region_picker_state(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<RegionPickerState, IpcError> {
    require_window(&window, "region-overlay")?;
    state
        .region_picker
        .lock()
        .map_err(|_| {
            IpcError::internal("Region picker state is unavailable because its lock failed.")
        })?
        .clone()
        .ok_or_else(|| IpcError::invalid_state("No region selection is currently active."))
}

#[tauri::command]
pub fn complete_region_selection(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
) -> Result<CaptureTarget, IpcError> {
    require_window(&window, "region-overlay")?;
    let active = get_picker_state(&state)?;
    let target = region::normalize_region(&active.display, start_x, start_y, end_x, end_y)?;

    close_overlay_and_restore_main(&app)?;
    set_picker_state(&state, None)?;
    app.emit_to("main", REGION_SELECTED_EVENT, target.clone())
        .map_err(|error| {
            IpcError::internal(format!(
                "Region was selected, but the main window could not be notified: {error}"
            ))
        })?;
    Ok(target)
}

#[tauri::command]
pub fn cancel_region_picker(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    require_window(&window, "region-overlay")?;
    close_overlay_and_restore_main(&app)?;
    set_picker_state(&state, None)
}

fn get_overlay(app: &AppHandle) -> Result<WebviewWindow, IpcError> {
    app.get_webview_window("region-overlay")
        .ok_or_else(|| IpcError::internal("The region-overlay window is not configured."))
}

fn close_overlay_and_restore_main(app: &AppHandle) -> Result<(), IpcError> {
    let overlay = get_overlay(app)?;
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| IpcError::internal("The main window is not configured."))?;
    overlay.hide().map_err(|error| {
        IpcError::internal(format!("Could not hide the region picker: {error}"))
    })?;
    if let Err(error) = main.show() {
        let _ = overlay.show();
        let _ = overlay.set_focus();
        return Err(IpcError::internal(format!(
            "Could not restore the main window: {error}"
        )));
    }
    main.set_focus()
        .map_err(|error| IpcError::internal(format!("Could not focus the main window: {error}")))
}

fn get_picker_state(state: &State<'_, AppState>) -> Result<RegionPickerState, IpcError> {
    state
        .region_picker
        .lock()
        .map_err(|_| {
            IpcError::internal("Region picker state is unavailable because its lock failed.")
        })?
        .clone()
        .ok_or_else(|| IpcError::invalid_state("No region selection is currently active."))
}

fn set_picker_state(
    state: &State<'_, AppState>,
    next: Option<RegionPickerState>,
) -> Result<(), IpcError> {
    *state.region_picker.lock().map_err(|_| {
        IpcError::internal("Region picker state is unavailable because its lock failed.")
    })? = next;
    Ok(())
}

fn require_window(window: &WebviewWindow, allowed: &str) -> Result<(), IpcError> {
    if window.label() == allowed {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "This window is not allowed to run the requested region-picker command.",
            false,
        ))
    }
}
