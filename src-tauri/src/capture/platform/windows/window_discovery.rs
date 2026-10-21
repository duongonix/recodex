use std::{io, os::windows::ffi::OsStringExt, path::PathBuf};

use windows_sys::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        System::Threading::GetCurrentProcessId,
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowModuleFileNameW, GetWindowRect, GetWindowTextW,
            GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
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
    if unsafe { IsWindow(hwnd) } == 0 {
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

    let is_visible = unsafe { IsWindowVisible(hwnd) } != 0;
    let is_minimized = unsafe { IsIconic(hwnd) } != 0;
    let (app_name, app_icon) = application_details(hwnd);
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

fn application_details(hwnd: HWND) -> (String, Option<String>) {
    let mut path_buffer = vec![0u16; 32_768];
    let length = unsafe {
        GetWindowModuleFileNameW(hwnd, path_buffer.as_mut_ptr(), path_buffer.len() as u32)
    };
    if length == 0 || length as usize > path_buffer.len() {
        return ("Unknown application".to_owned(), None);
    }
    path_buffer.truncate(length as usize);
    let path = PathBuf::from(std::ffi::OsString::from_wide(&path_buffer));
    let app_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Unknown application")
        .to_owned();
    let app_icon = application_icon(&path);
    (app_name, app_icon)
}

fn window_unavailable() -> IpcError {
    IpcError::new(
        "window_unavailable",
        "The selected window was closed, hidden, minimized, or replaced. Refresh the list and choose a visible window.",
        true,
    )
}
