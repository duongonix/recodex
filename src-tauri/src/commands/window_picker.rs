use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::{
    app_state::AppState,
    capture::sources::{self, WindowSource},
    errors::IpcError,
};

pub const WINDOW_SELECTED_EVENT: &str = "capture://window-selected";
pub const WINDOW_PICKER_CANCELLED_EVENT: &str = "capture://window-picker-cancelled";

#[tauri::command]
pub async fn list_windows(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<WindowSource>, IpcError> {
    require_source_window(&window)?;
    let mut windows = tauri::async_runtime::spawn_blocking(sources::list_windows)
        .await
        .map_err(|error| IpcError::internal(format!("Window listing task failed: {error}")))??;
    state
        .window_selection
        .lock()
        .map_err(|_| IpcError::internal("Window selection state is unavailable."))?
        .annotate_last_used(&mut windows);
    Ok(windows)
}

#[tauri::command]
pub async fn get_window_thumbnail(
    window: WebviewWindow,
    window_id: String,
) -> Result<Option<String>, IpcError> {
    require_picker_window(&window)?;
    tauri::async_runtime::spawn_blocking(move || sources::capture_window_thumbnail(&window_id))
        .await
        .map_err(|error| IpcError::internal(format!("Window preview task failed: {error}")))?
}

#[tauri::command]
pub fn get_selected_window(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<WindowSource>, IpcError> {
    require_source_window(&window)?;
    state
        .window_selection
        .lock()
        .map(|selection| selection.selected())
        .map_err(|_| IpcError::internal("Window selection state is unavailable."))
}

#[tauri::command]
pub async fn open_window_picker(app: AppHandle, window: WebviewWindow) -> Result<(), IpcError> {
    require_main_window(&window)?;
    if let Some(picker) = app.get_webview_window("window-picker") {
        picker.show().map_err(|error| {
            IpcError::new(
                "window_picker_show_failed",
                format!("Could not show the window picker: {error}"),
                true,
            )
        })?;
        return picker.set_focus().map_err(|error| {
            IpcError::new(
                "window_picker_focus_failed",
                format!("Could not focus the window picker: {error}"),
                true,
            )
        });
    }

    let (width, height) = picker_size(&window);
    let picker = WebviewWindowBuilder::new(
        &app,
        "window-picker",
        WebviewUrl::App("window-picker".into()),
    )
    .title("Choose a window")
    .inner_size(width, height)
    .resizable(true)
    .decorations(false)
    .skip_taskbar(true)
    .visible(false)
    .center()
    .build()
    .map_err(|error| {
        IpcError::new(
            "window_picker_create_failed",
            format!("Could not create the window picker: {error}"),
            true,
        )
    })?;

    picker.show().map_err(|error| {
        IpcError::new(
            "window_picker_show_failed",
            format!("Could not show the window picker: {error}"),
            true,
        )
    })?;
    picker.set_focus().map_err(|error| {
        IpcError::new(
            "window_picker_focus_failed",
            format!("Could not focus the window picker: {error}"),
            true,
        )
    })
}

#[tauri::command]
pub fn close_window_picker(app: AppHandle, window: WebviewWindow) -> Result<(), IpcError> {
    require_picker_window(&window)?;
    app.emit_to("main", WINDOW_PICKER_CANCELLED_EVENT, ())
        .map_err(|error| {
            IpcError::internal(format!("Could not restore the previous source: {error}"))
        })?;
    window.destroy().map_err(|error| {
        IpcError::new(
            "window_picker_close_failed",
            format!("Could not close the window picker: {error}"),
            true,
        )
    })
}

#[tauri::command]
pub fn select_capture_window(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    window_id: String,
) -> Result<WindowSource, IpcError> {
    require_picker_window(&window)?;
    let source = sources::find_recordable_window(&window_id)?;
    let selected = state
        .window_selection
        .lock()
        .map_err(|_| IpcError::internal("Window selection state is unavailable."))?
        .select(source);

    app.emit_to("main", WINDOW_SELECTED_EVENT, selected.clone())
        .map_err(|error| {
            IpcError::internal(format!("Could not apply the selected window: {error}"))
        })?;
    window.destroy().map_err(|error| {
        IpcError::new(
            "window_picker_close_failed",
            format!("The window was selected but the picker could not close: {error}"),
            true,
        )
    })?;
    Ok(selected)
}

fn picker_size(window: &WebviewWindow) -> (f64, f64) {
    let Some(monitor) = window.current_monitor().ok().flatten() else {
        return (1000.0, 900.0);
    };
    let scale = monitor.scale_factor().max(1.0);
    let available_width = f64::from(monitor.size().width) / scale - 48.0;
    let available_height = f64::from(monitor.size().height) / scale - 96.0;
    (
        1000.0_f64.min(available_width.max(320.0)),
        900.0_f64.min(available_height.max(360.0)),
    )
}

fn require_main_window(window: &WebviewWindow) -> Result<(), IpcError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(unauthorized_window())
    }
}

fn require_picker_window(window: &WebviewWindow) -> Result<(), IpcError> {
    if window.label() == "window-picker" {
        Ok(())
    } else {
        Err(unauthorized_window())
    }
}

fn require_source_window(window: &WebviewWindow) -> Result<(), IpcError> {
    if matches!(window.label(), "main" | "window-picker") {
        Ok(())
    } else {
        Err(unauthorized_window())
    }
}

fn unauthorized_window() -> IpcError {
    IpcError::new(
        "window_not_authorized",
        "This window is not allowed to manage capture window sources.",
        false,
    )
}
