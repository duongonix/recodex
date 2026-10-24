use std::path::PathBuf;

use tauri::{State, WebviewWindow};

use crate::{
    app_state::AppState,
    errors::IpcError,
    library::{LibraryItem, LibraryItemKind},
};

#[tauri::command]
pub fn list_library_items(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<LibraryItem>, IpcError> {
    require_main_window(&window)?;
    state.capture_library.list(&state.settings.get()?)
}

#[tauri::command]
pub async fn get_library_thumbnail(
    window: WebviewWindow,
    state: State<'_, AppState>,
    path: String,
) -> Result<Option<String>, IpcError> {
    require_main_window(&window)?;
    let item_path = state
        .capture_library
        .validate_item_path(&PathBuf::from(path))?;
    let kind = state.capture_library.kind_for_path(&item_path)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::library::thumbnail_data_url(&item_path, kind)
    })
    .await
    .map_err(|error| IpcError::internal(format!("Library thumbnail worker failed: {error}")))?
}

#[tauri::command]
pub fn open_library_item(
    window: WebviewWindow,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), IpcError> {
    require_main_window(&window)?;
    let path = state
        .capture_library
        .validate_item_path(&PathBuf::from(path))?;
    crate::library::open_item(&path)
}

#[tauri::command]
pub fn reveal_library_item(
    window: WebviewWindow,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), IpcError> {
    require_main_window(&window)?;
    let path = state
        .capture_library
        .validate_item_path(&PathBuf::from(path))?;
    crate::library::reveal_item(&path)
}

#[tauri::command]
pub fn open_library_folder(
    window: WebviewWindow,
    state: State<'_, AppState>,
    kind: LibraryItemKind,
) -> Result<(), IpcError> {
    require_main_window(&window)?;
    let settings = state.settings.get()?;
    let path = match kind {
        LibraryItemKind::Video => settings.video_output_directory,
        LibraryItemKind::Screenshot => settings.screenshot_output_directory,
    };
    crate::library::open_output_folder(&PathBuf::from(path))
}

fn require_main_window(window: &WebviewWindow) -> Result<(), IpcError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "The capture library is available only from the main window.",
            false,
        ))
    }
}
