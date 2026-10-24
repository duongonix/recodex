use std::{io, os::windows::ffi::OsStringExt, path::PathBuf};

use windows_sys::{
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM, RECT},
        Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
        System::Threading::{
            GetCurrentProcessId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GW_OWNER, GWL_EXSTYLE, GetClassNameW, GetWindow, GetWindowLongPtrW,
            GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
            IsWindowVisible, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        },
    },
    core::BOOL,
};

use crate::{capture::sources::WindowSource, errors::IpcError};

use super::window_icon::application_icon;

pub fn list_windows() -> Result<Vec<WindowSource>, IpcError> {
    let mut windows: Vec<WindowSource> = Vec::new();
    let context = &mut windows as *mut Vec<WindowSource>;
    let succeeded = unsafe { EnumWindows(Some(collect_window), context as LPARAM) };

    if succeeded == 0 {
        return Err(IpcError::internal(format!(
            "Windows could not enumerate open windows: {}",
            io::Error::last_os_error()
        )));
    }

    windows.sort_by(|left, right| {
        left.app_name
            .to_lowercase()
            .cmp(&right.app_name.to_lowercase())
            .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
    });
    Ok(windows)
}

pub fn find_recordable_window(window_id: &str) -> Result<WindowSource, IpcError> {
    let mut parts = window_id.split(':');
    let (Some("hwnd"), Some(process_id), Some(handle), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(window_unavailable());
    };
    let expected_process_id =
        u32::from_str_radix(process_id, 16).map_err(|_| window_unavailable())?;
    let handle = usize::from_str_radix(handle, 16).map_err(|_| window_unavailable())?;
    if expected_process_id == 0 || handle == 0 {
        return Err(window_unavailable());
    }

    let hwnd = handle as HWND;
    if unsafe { IsWindow(hwnd) } == 0 {
        return Err(window_unavailable());
    }
    let mut actual_process_id = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut actual_process_id) };
    if actual_process_id != expected_process_id
        || actual_process_id == unsafe { GetCurrentProcessId() }
    {
        return Err(window_unavailable());
    }

    source_for_window(hwnd)
        .filter(|source| source.is_recordable)
        .ok_or_else(window_unavailable)
}

pub fn recordable_window_bounds(window_id: &str) -> Result<(i32, i32, u32, u32), IpcError> {
    find_recordable_window(window_id)?;
    let handle = window_id
        .split(':')
        .nth(2)
        .and_then(|value| usize::from_str_radix(value, 16).ok())
        .filter(|handle| *handle != 0)
        .ok_or_else(window_unavailable)?;
    let mut bounds = RECT::default();
    if unsafe { GetWindowRect(handle as HWND, &mut bounds) } == 0
        || bounds.right <= bounds.left
        || bounds.bottom <= bounds.top
    {
        return Err(window_unavailable());
    }
    Ok((
        bounds.left,
        bounds.top,
        (bounds.right - bounds.left) as u32,
        (bounds.bottom - bounds.top) as u32,
    ))
}

unsafe extern "system" fn collect_window(hwnd: HWND, context: LPARAM) -> BOOL {
    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
    if process_id == 0 || process_id == unsafe { GetCurrentProcessId() } {
        return 1;
    }

    let Some(source) = source_for_window(hwnd) else {
        return 1;
    };
    let windows = unsafe { &mut *(context as *mut Vec<WindowSource>) };
    windows.push(source);
    1
}

fn source_for_window(hwnd: HWND) -> Option<WindowSource> {
    if unsafe { IsWindow(hwnd) } == 0 || !is_user_application_window(hwnd) {
        return None;
    }
    let mut bounds = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut bounds) } == 0
        || bounds.right <= bounds.left
        || bounds.bottom <= bounds.top
    {
        return None;
    }

    let mut title_buffer = [0u16; 2048];
    let title_length =
        unsafe { GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32) };
    if title_length <= 0 {
        return None;
    }
    let title = String::from_utf16_lossy(&title_buffer[..title_length as usize])
        .trim()
        .to_owned();
    if title.is_empty() {
        return None;
    }

    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
    if process_id == 0 {
        return None;
    }

    let is_cloaked = is_window_cloaked(hwnd);
    let is_visible = unsafe { IsWindowVisible(hwnd) } != 0 && !is_cloaked;
    let is_minimized = unsafe { IsIconic(hwnd) } != 0;
    let (app_name, app_icon) = application_details(process_id);
    Some(WindowSource {
        window_id: format!("hwnd:{process_id:x}:{:x}", hwnd as usize),
        title,
        app_name,
        app_icon,
        is_visible,
        is_minimized,
        is_recordable: is_visible && !is_minimized,
        last_used_at: None,
    })
}

pub(super) fn is_user_application_window(hwnd: HWND) -> bool {
    let extended_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    let is_app_window = extended_style & WS_EX_APPWINDOW != 0;
    let is_tool_window = extended_style & WS_EX_TOOLWINDOW != 0;
    let has_owner = !unsafe { GetWindow(hwnd, GW_OWNER) }.is_null();
    if is_tool_window || (has_owner && !is_app_window) {
        return false;
    }

    let mut class_name = [0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, class_name.as_mut_ptr(), class_name.len() as i32) };
    if length > 0 {
        let class_name = String::from_utf16_lossy(&class_name[..length as usize]);
        if matches!(
            class_name.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return false;
        }
    }
    true
}

pub(super) fn is_window_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            (&mut cloaked as *mut u32).cast(),
            std::mem::size_of_val(&cloaked) as u32,
        )
    };
    result >= 0 && cloaked != 0
}

fn application_details(process_id: u32) -> (String, Option<String>) {
    let Some(path) = process_image_path(process_id) else {
        return ("Unknown application".to_owned(), None);
    };
    let app_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Unknown application")
        .to_owned();
    let app_icon = application_icon(&path);
    (app_name, app_icon)
}

fn process_image_path(process_id: u32) -> Option<PathBuf> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return None;
    }

    let mut path_buffer = vec![0u16; 32_768];
    let mut length = path_buffer.len() as u32;
    let succeeded =
        unsafe { QueryFullProcessImageNameW(process, 0, path_buffer.as_mut_ptr(), &mut length) };
    unsafe { CloseHandle(process) };
    if succeeded == 0 || length == 0 || length as usize > path_buffer.len() {
        return None;
    }
    path_buffer.truncate(length as usize);
    Some(PathBuf::from(std::ffi::OsString::from_wide(&path_buffer)))
}

fn window_unavailable() -> IpcError {
    IpcError::new(
        "window_unavailable",
        "The selected window was closed, hidden, minimized, or replaced. Refresh the list and choose a visible window.",
        true,
    )
}
