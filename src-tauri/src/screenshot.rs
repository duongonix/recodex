use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{errors::IpcError, recording::CaptureTarget, settings::ScreenshotFormat};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenshotRequest {
    pub target: CaptureTarget,
    pub format: ScreenshotFormat,
    pub copy_to_clipboard: bool,
    pub save_to_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotResult {
    pub width: u32,
    pub height: u32,
    pub output_path: Option<String>,
    pub copied_to_clipboard: bool,
    pub warning: Option<String>,
    pub editor_session_id: Option<String>,
}

pub struct EditorScreenshot {
    pub width: u32,
    pub height: u32,
    pub source_png: Vec<u8>,
}

#[cfg(windows)]
mod format;
#[cfg(windows)]
mod output;
#[cfg(windows)]
mod platform;

pub fn capture(
    app: &tauri::AppHandle,
    request: ScreenshotRequest,
    output_directory: &Path,
) -> Result<ScreenshotResult, IpcError> {
    if !request.copy_to_clipboard && !request.save_to_file {
        return Err(IpcError::new(
            "screenshot_destination_required",
            "Choose Copy to clipboard, Save to file, or both.",
            true,
        ));
    }

    #[cfg(windows)]
    {
        platform::capture(app, request, output_directory)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, request, output_directory);
        Err(IpcError::new(
            "screenshot_unavailable",
            "Native screenshot capture is currently implemented only for Windows.",
            true,
        ))
    }
}

pub fn capture_for_editor(
    app: &tauri::AppHandle,
    target: CaptureTarget,
) -> Result<EditorScreenshot, IpcError> {
    #[cfg(windows)]
    {
        let frame = platform::capture_frame(app, &target)?;
        let source_png = format::encode(&frame, ScreenshotFormat::Png)?;
        Ok(EditorScreenshot {
            width: frame.width,
            height: frame.height,
            source_png,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (app, target);
        Err(IpcError::new(
            "screenshot_unavailable",
            "Native screenshot capture and the screenshot editor are currently implemented only for Windows.",
            true,
        ))
    }
}
