use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::errors::IpcError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySource {
    pub display_id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowSource {
    pub window_id: String,
    pub title: String,
    pub app_name: String,
    pub app_icon: Option<String>,
    pub is_visible: bool,
    pub is_minimized: bool,
    pub is_recordable: bool,
    pub last_used_at: Option<u64>,
}

pub fn list_displays(app: &AppHandle) -> Result<Vec<DisplaySource>, IpcError> {
    let monitors = app
        .available_monitors()
        .map_err(|error| IpcError::internal(format!("Could not enumerate displays: {error}")))?;
    let primary_geometry = app
        .primary_monitor()
        .map_err(|error| {
            IpcError::internal(format!("Could not identify the primary display: {error}"))
        })?
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            (position.x, position.y, size.width, size.height)
        });

    let mut displays = monitors
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let position = monitor.position();
            let size = monitor.size();
            let name = monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Display {}", index + 1));
            let display_id = monitor.name().map_or_else(
                || {
                    format!(
                        "display:{}:{}:{}:{}",
                        position.x, position.y, size.width, size.height
                    )
                },
                |name| format!("{name}@{},{}", position.x, position.y),
            );

            DisplaySource {
                display_id,
                primary: primary_geometry
                    == Some((position.x, position.y, size.width, size.height)),
                name,
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
                scale_factor: monitor.scale_factor(),
            }
        })
        .collect::<Vec<_>>();

    displays.sort_by_key(|display| (!display.primary, display.x, display.y));
    Ok(displays)
}

pub fn list_windows() -> Result<Vec<WindowSource>, IpcError> {
    #[cfg(windows)]
    {
        crate::capture::platform::windows::list_windows()
    }
    #[cfg(not(windows))]
    {
        Err(window_sources_unsupported())
    }
}

pub fn find_recordable_window(window_id: &str) -> Result<WindowSource, IpcError> {
    #[cfg(windows)]
    {
        crate::capture::platform::windows::find_recordable_window(window_id)
    }
    #[cfg(not(windows))]
    {
        let _ = window_id;
        Err(window_sources_unsupported())
    }
}

pub fn capture_window_thumbnail(window_id: &str) -> Result<Option<String>, IpcError> {
    #[cfg(windows)]
    {
        crate::capture::platform::windows::capture_window_thumbnail(window_id)
    }
    #[cfg(not(windows))]
    {
        let _ = window_id;
        Err(window_sources_unsupported())
    }
}

#[cfg(not(windows))]
fn window_sources_unsupported() -> IpcError {
    IpcError::new(
        "window_sources_unsupported",
        "Native window selection and preview are currently implemented on Windows only. macOS and Linux require their platform capture and permission adapters.",
        true,
    )
}

pub fn find_display(
    displays: &[DisplaySource],
    display_id: &str,
) -> Result<DisplaySource, IpcError> {
    displays
        .iter()
        .find(|display| display.display_id == display_id)
        .cloned()
        .ok_or_else(|| {
            IpcError::new(
                "display_unavailable",
                "The selected display is no longer available. Refresh the source list and choose another display.",
                true,
            )
        })
}

#[cfg(all(test, windows))]
mod tests {
    use super::list_windows;

    #[test]
    fn native_window_enumerator_runs_on_the_windows_test_host() {
        assert!(list_windows().is_ok());
    }
}
