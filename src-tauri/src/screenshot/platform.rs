use std::{
    borrow::Cow,
    path::Path,
    time::{Duration, Instant},
};

use arboard::{Clipboard, ImageData};

use crate::{
    capture::{
        platform::windows::{CapturedFrame, WindowsCapture},
        sources::list_displays,
    },
    errors::IpcError,
};

use super::{ScreenshotRequest, ScreenshotResult, format, output};

pub(super) fn capture(
    app: &tauri::AppHandle,
    request: ScreenshotRequest,
    output_directory: &Path,
) -> Result<ScreenshotResult, IpcError> {
    let frame = capture_frame(app, &request.target)?;
    deliver(frame, request, output_directory)
}

pub(super) fn capture_frame(
    app: &tauri::AppHandle,
    target: &crate::recording::CaptureTarget,
) -> Result<CapturedFrame, IpcError> {
    let displays = list_displays(app)?;
    let mut capture = WindowsCapture::start(target, &displays, 30)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let frame = loop {
        if Instant::now() >= deadline {
            return Err(IpcError::new(
                "screenshot_capture_timeout",
                "Windows Graphics Capture did not return a frame within 10 seconds. Refresh the selected source and try again.",
                true,
            ));
        }
        if let Some(frame) = capture.wait_for_frame(Duration::from_millis(100))? {
            break frame;
        }
    };
    capture.stop()?;
    Ok(frame)
}

fn deliver(
    frame: crate::capture::platform::windows::CapturedFrame,
    request: ScreenshotRequest,
    output_directory: &Path,
) -> Result<ScreenshotResult, IpcError> {
    let mut output_path = None;
    let mut copied_to_clipboard = false;
    let mut failures = Vec::new();

    if request.save_to_file {
        match format::encode(&frame, request.format)
            .and_then(|bytes| output::save(output_directory, request.format, &bytes))
        {
            Ok(path) => output_path = Some(path.to_string_lossy().into_owned()),
            Err(error) => failures.push(format!("Save failed: {}", error.message)),
        }
    }

    if request.copy_to_clipboard {
        match copy_image(&frame) {
            Ok(()) => copied_to_clipboard = true,
            Err(error) => failures.push(format!("Clipboard failed: {}", error.message)),
        }
    }

    if output_path.is_none() && !copied_to_clipboard {
        return Err(IpcError::new(
            "screenshot_delivery_failed",
            failures.join(" "),
            true,
        ));
    }

    Ok(ScreenshotResult {
        width: frame.width,
        height: frame.height,
        output_path,
        copied_to_clipboard,
        warning: (!failures.is_empty()).then(|| failures.join(" ")),
        editor_session_id: None,
    })
}

fn copy_image(frame: &crate::capture::platform::windows::CapturedFrame) -> Result<(), IpcError> {
    let pixels = format::rgba_pixels(frame)?;
    let image = ImageData {
        width: frame.width as usize,
        height: frame.height as usize,
        bytes: Cow::Owned(pixels),
    };
    let mut clipboard = Clipboard::new().map_err(|error| {
        IpcError::new(
            "screenshot_clipboard_unavailable",
            format!("Could not open the system clipboard: {error}"),
            true,
        )
    })?;
    clipboard.set_image(image).map_err(|error| {
        IpcError::new(
            "screenshot_clipboard_failed",
            format!("Could not copy the screenshot to the clipboard: {error}"),
            true,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{recording::CaptureTarget, settings::ScreenshotFormat};

    fn frame() -> crate::capture::platform::windows::CapturedFrame {
        crate::capture::platform::windows::CapturedFrame {
            width: 2,
            height: 2,
            timestamp_100ns: 1,
            pixels_bgra: vec![
                0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
            ],
        }
    }

    #[test]
    fn saves_selected_format_and_reports_dimensions() {
        let directory = tempfile::tempdir().unwrap();
        let request = ScreenshotRequest {
            target: CaptureTarget::Display {
                display_id: "display".into(),
            },
            format: ScreenshotFormat::Png,
            copy_to_clipboard: false,
            save_to_file: true,
        };
        let result = deliver(frame(), request, directory.path()).unwrap();

        assert_eq!((result.width, result.height), (2, 2));
        assert!(result.output_path.unwrap().ends_with(".png"));
        assert!(!result.copied_to_clipboard);
        assert!(result.warning.is_none());
    }
}
