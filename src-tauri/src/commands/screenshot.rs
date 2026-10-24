use std::path::PathBuf;

use tauri::{AppHandle, Manager, State, WebviewWindow};

use crate::{
    app_state::AppState,
    errors::IpcError,
    recording::SessionStatus,
    screenshot::{ScreenshotRequest, ScreenshotResult},
};

struct MainWindowRestore {
    window: WebviewWindow,
    hidden_editor_windows: Vec<WebviewWindow>,
    restored: bool,
    content_protected: bool,
}

impl MainWindowRestore {
    fn hide_for_capture(&mut self, app: &AppHandle) -> Result<(), IpcError> {
        self.window.set_content_protected(true).map_err(|error| {
            IpcError::new(
                "screenshot_exclusion_failed",
                format!("Could not exclude RecoDex from its screenshot: {error}"),
                true,
            )
        })?;
        self.content_protected = true;
        self.window.hide().map_err(|error| {
            IpcError::new(
                "main_window_hide_failed",
                format!("Could not hide the main window before the screenshot: {error}"),
                true,
            )
        })?;
        for editor in app
            .webview_windows()
            .into_values()
            .filter(|window| window.label().starts_with("screenshot-editor-"))
        {
            let visible = editor.is_visible().map_err(|error| {
                IpcError::new(
                    "screenshot_exclusion_failed",
                    format!("Could not inspect an open screenshot editor window: {error}"),
                    true,
                )
            })?;
            if visible {
                editor.hide().map_err(|error| {
                    IpcError::new(
                        "screenshot_exclusion_failed",
                        format!("Could not hide an open screenshot editor window: {error}"),
                        true,
                    )
                })?;
                self.hidden_editor_windows.push(editor);
            }
        }
        Ok(())
    }

    fn restore(&mut self) -> Result<(), IpcError> {
        let mut failures = Vec::new();
        if let Err(error) = self.window.show() {
            failures.push(format!("Could not restore the main window: {error}"));
        }
        for editor in &self.hidden_editor_windows {
            if let Err(error) = editor.show() {
                failures.push(format!(
                    "Could not restore a screenshot editor window: {error}"
                ));
            }
        }
        if self.content_protected {
            match self.window.set_content_protected(false) {
                Ok(()) => self.content_protected = false,
                Err(error) => failures.push(format!(
                    "Could not restore screen-capture visibility: {error}"
                )),
            }
        }
        if failures.is_empty() {
            self.restored = true;
            Ok(())
        } else {
            Err(IpcError::new(
                "main_window_restore_failed",
                failures.join(" "),
                true,
            ))
        }
    }
}

impl Drop for MainWindowRestore {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.window.show();
            for editor in &self.hidden_editor_windows {
                let _ = editor.show();
            }
            if self.content_protected {
                let _ = self.window.set_content_protected(false);
            }
        }
    }
}

#[tauri::command]
pub async fn capture_screenshot(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: ScreenshotRequest,
) -> Result<ScreenshotResult, IpcError> {
    if window.label() != "main" {
        return Err(IpcError::new(
            "window_not_authorized",
            "Only the main window can request a screenshot.",
            false,
        ));
    }
    let session = state
        .recording
        .lock()
        .map_err(|_| IpcError::internal("Recording state is unavailable because its lock failed."))?
        .snapshot();
    if !matches!(
        session.status,
        SessionStatus::Idle | SessionStatus::Completed | SessionStatus::Failed
    ) {
        return Err(IpcError::new(
            "recording_active",
            "Stop the current recording before taking a screenshot.",
            true,
        ));
    }
    let settings = state.settings.get()?;
    let output_directory = PathBuf::from(&settings.screenshot_output_directory);

    if settings.show_screenshot_preview {
        let target = request.target;
        let capture_app = app.clone();
        let captured = tauri::async_runtime::spawn_blocking(move || {
            capture_with_main_hidden(&capture_app, || {
                crate::screenshot::capture_for_editor(&capture_app, target)
            })
        })
        .await
        .map_err(|error| {
            IpcError::internal(format!("Screenshot capture worker failed: {error}"))
        })??;
        let cache_directory = app.path().app_cache_dir().map_err(|error| {
            IpcError::new(
                "editor_session_storage_unavailable",
                format!("Could not locate temporary screenshot storage: {error}"),
                true,
            )
        })?;
        let metadata = state.screenshot_editor_sessions.create(
            &cache_directory,
            output_directory,
            settings.screenshot_format,
            captured.width,
            captured.height,
            &captured.source_png,
        )?;
        if let Err(error) =
            crate::editor::open_editor_window(&app, &window, &metadata.session_id).await
        {
            let _ = state
                .screenshot_editor_sessions
                .remove(&metadata.session_id);
            return Err(error);
        }
        return Ok(ScreenshotResult {
            width: captured.width,
            height: captured.height,
            output_path: None,
            copied_to_clipboard: false,
            warning: None,
            editor_session_id: Some(metadata.session_id),
        });
    }

    if !request.copy_to_clipboard && !request.save_to_file {
        return Err(IpcError::new(
            "screenshot_destination_required",
            "Choose Copy to clipboard, Save to file, or both.",
            true,
        ));
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        capture_with_main_hidden(&app, || {
            crate::screenshot::capture(&app, request, &output_directory)
        })
    })
    .await
    .map_err(|error| IpcError::internal(format!("Screenshot capture worker failed: {error}")))??;
    if let Some(path) = result.output_path.as_deref()
        && let Err(error) = state.capture_library.register(
            &PathBuf::from(path),
            crate::library::LibraryItemKind::Screenshot,
            None,
        )
    {
        eprintln!(
            "[RecoDex][library] Could not register the saved screenshot: {}",
            error.message
        );
    }
    Ok(result)
}

fn capture_with_main_hidden<T>(
    app: &AppHandle,
    capture: impl FnOnce() -> Result<T, IpcError>,
) -> Result<T, IpcError> {
    let main = app.get_webview_window("main").ok_or_else(|| {
        IpcError::new(
            "main_window_unavailable",
            "The main window is unavailable.",
            true,
        )
    })?;
    let mut visibility = MainWindowRestore {
        window: main,
        hidden_editor_windows: Vec::new(),
        restored: false,
        content_protected: false,
    };
    visibility.hide_for_capture(app)?;
    let result = capture();
    let restore = visibility.restore();
    match (result, restore) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(capture_error), Ok(())) => Err(capture_error),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(capture_error), Err(restore_error)) => Err(IpcError::new(
            "screenshot_restore_failed",
            format!("{} {}", capture_error.message, restore_error.message),
            true,
        )),
    }
}
