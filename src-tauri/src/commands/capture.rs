use tauri::WebviewWindow;

use crate::{
    capture::capabilities::{self, CaptureCapabilities, PermissionInfo, PermissionKind},
    errors::IpcError,
};

#[tauri::command]
pub fn get_capture_capabilities(window: WebviewWindow) -> Result<CaptureCapabilities, IpcError> {
    require_main_window(&window)?;
    let mut capabilities = capabilities::get_capabilities();
    match crate::audio::availability() {
        Ok(availability) => capabilities::set_audio_capabilities(
            &mut capabilities,
            availability.microphone,
            availability.system,
        ),
        Err(error) => {
            let unsupported = crate::capture::capabilities::FeatureSupport {
                status: crate::capture::capabilities::FeatureStatus::Unavailable,
                message: error.message,
            };
            capabilities.microphone_capture = unsupported.clone();
            capabilities.system_audio_capture = unsupported;
        }
    }
    Ok(capabilities)
}

#[tauri::command]
pub fn get_capture_permissions(window: WebviewWindow) -> Result<Vec<PermissionInfo>, IpcError> {
    require_main_window(&window)?;
    Ok(capabilities::get_permissions())
}

#[tauri::command]
pub fn request_capture_permission(
    window: WebviewWindow,
    kind: PermissionKind,
) -> Result<PermissionInfo, IpcError> {
    require_main_window(&window)?;
    let info = capabilities::permission_info(kind);

    Err(IpcError::new(
        "permission_request_unavailable",
        info.message,
        true,
    ))
}

fn require_main_window(window: &WebviewWindow) -> Result<(), IpcError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(IpcError::new(
            "window_not_authorized",
            "This window is not allowed to query or request capture permissions.",
            false,
        ))
    }
}
