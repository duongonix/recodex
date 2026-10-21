use std::path::PathBuf;

use base64::{Engine, engine::general_purpose::STANDARD};
use tauri::{
    AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    ipc::{InvokeBody, Request, Response},
};

use crate::{app_state::AppState, errors::IpcError};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorExportResult {
    pub width: u32,
    pub height: u32,
    pub output_path: Option<String>,
    pub copied_to_clipboard: bool,
}

pub async fn open_editor_window(
    app: &AppHandle,
    main_window: &WebviewWindow,
    session_id: &str,
) -> Result<(), IpcError> {
    let label = window_label(session_id);
    if let Some(editor) = app.get_webview_window(&label) {
        editor.show().map_err(|error| window_error("show", error))?;
        return editor
            .set_focus()
            .map_err(|error| window_error("focus", error));
    }

    let (width, height, min_width, min_height) = window_size(main_window);
    let editor = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App(format!("screenshot-editor?session={session_id}").into()),
    )
    .title("RecoDex Screenshot Editor")
    .inner_size(width, height)
    .min_inner_size(min_width, min_height)
    .resizable(true)
    .decorations(false)
    .center()
    .visible(false)
    .build()
    .map_err(|error| window_error("create", error))?;
    editor.show().map_err(|error| window_error("show", error))?;
    editor
        .set_focus()
        .map_err(|error| window_error("focus", error))
}

#[tauri::command]
pub fn get_screenshot_editor_metadata(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<super::EditorSessionMetadata, IpcError> {
    require_editor_window(&window, &session_id)?;
    state.screenshot_editor_sessions.metadata(&session_id)
}

#[tauri::command]
pub fn get_screenshot_editor_source(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Response, IpcError> {
    require_editor_window(&window, &session_id)?;
    let bytes = state.screenshot_editor_sessions.source(&session_id)?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub fn save_screenshot_editor(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<EditorExportResult, IpcError> {
    let session_id = required_header(&request, "x-session-id")?;
    require_editor_window(&window, session_id)?;
    let operation = required_header(&request, "x-operation")?;
    let (session, output_directory) = state
        .screenshot_editor_sessions
        .export_location(session_id)?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(IpcError::new(
            "editor_export_invalid",
            "The edited screenshot must be sent as a binary image.",
            true,
        ));
    };
    let (width, height) = super::export::validate_image(bytes, session.format)?;

    let output_path = match operation {
        "save" => Some(super::export::save_unique(
            &output_directory,
            session.format,
            bytes,
        )?),
        "save-as" => {
            let encoded_path = required_header(&request, "x-output-path")?;
            let path_bytes = STANDARD.decode(encoded_path).map_err(|error| {
                IpcError::new(
                    "editor_export_path_invalid",
                    format!("The selected Save As path is malformed: {error}"),
                    true,
                )
            })?;
            let path = String::from_utf8(path_bytes).map_err(|error| {
                IpcError::new(
                    "editor_export_path_invalid",
                    format!("The selected Save As path is not valid UTF-8: {error}"),
                    true,
                )
            })?;
            Some(super::export::save_as(
                &PathBuf::from(path),
                session.format,
                bytes,
            )?)
        }
        "copy" => {
            super::export::copy_to_clipboard(bytes, session.format)?;
            None
        }
        _ => {
            return Err(IpcError::new(
                "editor_export_operation_invalid",
                "Choose Save, Save As, or Copy.",
                true,
            ));
        }
    };

    Ok(EditorExportResult {
        width,
        height,
        output_path: output_path.map(|path| path.to_string_lossy().into_owned()),
        copied_to_clipboard: operation == "copy",
    })
}

#[tauri::command]
pub fn close_screenshot_editor(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), IpcError> {
    require_editor_window(&window, &session_id)?;
    state.screenshot_editor_sessions.remove(&session_id)
}

fn window_label(session_id: &str) -> String {
    format!("screenshot-editor-{session_id}")
}

fn require_editor_window(window: &WebviewWindow, session_id: &str) -> Result<(), IpcError> {
    if !session_id.is_empty() && window.label() == window_label(session_id) {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "This window is not allowed to access the requested screenshot editor session.",
            false,
        ))
    }
}

fn required_header<'a>(request: &'a Request<'_>, name: &str) -> Result<&'a str, IpcError> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            IpcError::new(
                "editor_export_metadata_missing",
                format!("The export request is missing {name}."),
                true,
            )
        })
}

fn window_size(window: &WebviewWindow) -> (f64, f64, f64, f64) {
    let Some(monitor) = window.current_monitor().ok().flatten() else {
        return (1280.0, 800.0, 960.0, 620.0);
    };
    let scale = monitor.scale_factor().max(1.0);
    let available_width = (f64::from(monitor.size().width) / scale - 48.0).max(320.0);
    let available_height = (f64::from(monitor.size().height) / scale - 96.0).max(240.0);
    let width = 1440.0_f64.min(available_width);
    let height = 900.0_f64.min(available_height);
    (width, height, 960.0_f64.min(width), 620.0_f64.min(height))
}

fn window_error(action: &str, error: impl std::fmt::Display) -> IpcError {
    IpcError::new(
        "editor_window_failed",
        format!("Could not {action} the screenshot editor window: {error}"),
        true,
    )
}
