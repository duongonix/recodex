use tauri::{Runtime, WebviewWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowDisplayAffinity, SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE,
};

use crate::errors::IpcError;

/// Reapply the Windows 10+ exclusion after WGC has started and confirm the OS accepted it.
pub(crate) fn exclude_window_from_capture<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<(), IpcError> {
    let hwnd = window
        .hwnd()
        .map_err(|error| IpcError::internal(format!("Could not access toolbar HWND: {error}")))?;
    apply_capture_exclusion(hwnd.0 as _)
}

fn apply_capture_exclusion(hwnd: windows_sys::Win32::Foundation::HWND) -> Result<(), IpcError> {
    unsafe {
        // Reset while the toolbar is still hidden, then apply the exclusion to the active session.
        if SetWindowDisplayAffinity(hwnd, 0) == 0 {
            return Err(display_affinity_error("reset"));
        }
        if SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) == 0 {
            return Err(display_affinity_error("exclude"));
        }

        let mut affinity = 0;
        if GetWindowDisplayAffinity(hwnd, &mut affinity) == 0 {
            return Err(display_affinity_error("verify"));
        }
        if affinity != WDA_EXCLUDEFROMCAPTURE {
            return Err(IpcError::new(
                "capture_exclusion_unavailable",
                format!(
                    "Windows reported unexpected toolbar capture affinity {affinity}; the toolbar cannot be safely excluded from the recording."
                ),
                true,
            ));
        }
        eprintln!(
            "[RecoDex][capture] verified toolbar WDA_EXCLUDEFROMCAPTURE affinity=0x{affinity:08X}"
        );
    }

    Ok(())
}

fn display_affinity_error(action: &str) -> IpcError {
    let error = std::io::Error::last_os_error();
    IpcError::new(
        "capture_exclusion_unavailable",
        format!("Could not {action} Windows toolbar capture exclusion: {error}."),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::apply_capture_exclusion;
    use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_POPUP};

    #[test]
    fn native_capture_exclusion_round_trips_for_a_hidden_top_level_window() {
        let class_name = "STATIC\0".encode_utf16().collect::<Vec<_>>();
        let title = "RecoDex affinity test\0".encode_utf16().collect::<Vec<_>>();
        let window = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                0,
                0,
                16,
                16,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(
            !window.is_null(),
            "Windows should create the hidden test window"
        );

        let result = apply_capture_exclusion(window);
        unsafe { DestroyWindow(window) };
        result.expect("Windows should store WDA_EXCLUDEFROMCAPTURE on a top-level window");
    }
}
