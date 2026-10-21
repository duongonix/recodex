use std::ffi::c_void;

use windows::Win32::{
    Foundation::{LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO},
};
use windows::core::BOOL;
use windows_capture::{monitor::Monitor, window::Window};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
};

use crate::{capture::sources::DisplaySource, errors::IpcError, recording::CaptureTarget};

use super::cursor::CursorTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CropRect {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub(super) enum CaptureItem {
    Monitor(Monitor),
    Window(Window),
}

#[derive(Debug)]
pub(super) struct ResolvedTarget {
    pub item: CaptureItem,
    pub crop: Option<CropRect>,
    pub cursor_target: CursorTarget,
}

pub(super) fn resolve_target(
    target: &CaptureTarget,
    displays: &[DisplaySource],
) -> Result<ResolvedTarget, IpcError> {
    match target {
        CaptureTarget::Display { display_id } => {
            let display = find_display(displays, display_id)?;
            Ok(ResolvedTarget {
                item: CaptureItem::Monitor(find_monitor(display)?),
                crop: None,
                cursor_target: CursorTarget::Screen {
                    origin_x: display.x,
                    origin_y: display.y,
                },
            })
        }
        CaptureTarget::Region {
            display_id,
            x,
            y,
            width,
            height,
        } => {
            let display = find_display(displays, display_id)?;
            let crop = region_crop(display, *x, *y, *width, *height)?;
            let monitor = find_monitor(display)?;
            Ok(ResolvedTarget {
                item: CaptureItem::Monitor(monitor),
                crop: Some(crop),
                cursor_target: CursorTarget::Screen {
                    origin_x: *x,
                    origin_y: *y,
                },
            })
        }
        CaptureTarget::Window { window_id } => {
            let (window, hwnd) = resolve_window(window_id)?;
            Ok(ResolvedTarget {
                item: CaptureItem::Window(window),
                crop: None,
                cursor_target: CursorTarget::Window { hwnd },
            })
        }
    }
}

fn region_crop(
    display: &DisplaySource,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<CropRect, IpcError> {
    let left = i64::from(x) - i64::from(display.x);
    let top = i64::from(y) - i64::from(display.y);
    let right = left.checked_add(i64::from(width));
    let bottom = top.checked_add(i64::from(height));
    if left < 0
        || top < 0
        || width < 2
        || height < 2
        || right.is_none_or(|right| right > i64::from(display.width))
        || bottom.is_none_or(|bottom| bottom > i64::from(display.height))
    {
        return Err(IpcError::new(
            "invalid_region",
            "The selected physical-pixel region no longer fits its display. Select it again.",
            true,
        ));
    }
    Ok(CropRect {
        left: u32::try_from(left).map_err(|_| invalid_region())?,
        top: u32::try_from(top).map_err(|_| invalid_region())?,
        width,
        height,
    })
}

fn find_display<'a>(
    displays: &'a [DisplaySource],
    display_id: &str,
) -> Result<&'a DisplaySource, IpcError> {
    displays
        .iter()
        .find(|display| display.display_id == display_id)
        .ok_or_else(|| {
            IpcError::new(
                "display_unavailable",
                "The selected display is no longer available. Refresh the source list and choose another display.",
                true,
            )
        })
}

fn find_monitor(display: &DisplaySource) -> Result<Monitor, IpcError> {
    let mut context = MonitorMatch {
        x: display.x,
        y: display.y,
        width: display.width,
        height: display.height,
        monitor: None,
    };
    let result = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(find_monitor_callback),
            LPARAM((&raw mut context) as isize),
        )
    };
    if !result.as_bool() {
        return Err(IpcError::internal(format!(
            "Could not enumerate Windows monitors: {}",
            windows::core::Error::from_thread()
        )));
    }
    context
        .monitor
        .map(|monitor| Monitor::from_raw_hmonitor(monitor.0))
        .ok_or_else(|| {
            IpcError::new(
                "display_unavailable",
                "Windows no longer reports the selected display at its physical bounds. Refresh the source list.",
                true,
            )
        })
}

struct MonitorMatch {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    monitor: Option<HMONITOR>,
}

unsafe extern "system" fn find_monitor_callback(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    context: LPARAM,
) -> BOOL {
    let match_context = unsafe { &mut *(context.0 as *mut MonitorMatch) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..MONITORINFO::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return BOOL(1);
    }
    let rect = info.rcMonitor;
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    if rect.left == match_context.x
        && rect.top == match_context.y
        && width == i64::from(match_context.width)
        && height == i64::from(match_context.height)
    {
        match_context.monitor = Some(monitor);
    }
    BOOL(1)
}

fn resolve_window(window_id: &str) -> Result<(Window, isize), IpcError> {
    let mut parts = window_id.split(':');
    let (Some("hwnd"), Some(process_id), Some(handle), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(invalid_window());
    };
    let expected_process_id = u32::from_str_radix(process_id, 16).map_err(|_| invalid_window())?;
    let handle = usize::from_str_radix(handle, 16).map_err(|_| invalid_window())?;
    if handle == 0 || expected_process_id == 0 {
        return Err(invalid_window());
    }

    let hwnd = handle as windows_sys::Win32::Foundation::HWND;
    if unsafe { IsWindow(hwnd) } == 0
        || unsafe { IsWindowVisible(hwnd) } == 0
        || unsafe { IsIconic(hwnd) } != 0
    {
        return Err(invalid_window());
    }
    let mut actual_process_id = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut actual_process_id) };
    if actual_process_id != expected_process_id {
        return Err(invalid_window());
    }

    let window = Window::from_raw_hwnd(hwnd.cast::<c_void>());
    if !window.is_valid() {
        return Err(invalid_window());
    }
    Ok((window, hwnd as isize))
}

fn invalid_window() -> IpcError {
    IpcError::new(
        "window_unavailable",
        "The selected window was closed, minimized, or replaced. Refresh the source list and choose another window.",
        true,
    )
}

fn invalid_region() -> IpcError {
    IpcError::new(
        "invalid_region",
        "The selected region is outside the supported display coordinate range.",
        true,
    )
}

#[cfg(test)]
mod tests {
    use crate::capture::sources::DisplaySource;

    use super::{CropRect, find_display, region_crop};

    fn display() -> DisplaySource {
        DisplaySource {
            display_id: "test-display".to_owned(),
            name: "Test Display".to_owned(),
            x: -1920,
            y: -100,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
            primary: false,
        }
    }

    #[test]
    fn region_coordinates_are_converted_from_global_physical_bounds() {
        let selected = display();
        assert_eq!(
            region_crop(&selected, -1800, 20, 800, 500).unwrap(),
            CropRect {
                left: 120,
                top: 120,
                width: 800,
                height: 500,
            }
        );
    }

    #[test]
    fn region_must_stay_inside_the_selected_display() {
        let selected = display();
        assert_eq!(
            region_crop(&selected, -1800, 20, 2000, 500)
                .unwrap_err()
                .code,
            "invalid_region"
        );
        assert_eq!(
            find_display(&[selected], "removed").unwrap_err().code,
            "display_unavailable"
        );
    }

    #[test]
    fn refreshed_window_identifiers_bind_pid_and_hwnd() {
        assert_eq!(
            super::resolve_window("hwnd:xyz:100").unwrap_err().code,
            "window_unavailable"
        );
        assert_eq!(
            super::resolve_window("hwnd:123").unwrap_err().code,
            "window_unavailable"
        );
    }
}
