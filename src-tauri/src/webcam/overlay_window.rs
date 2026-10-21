use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Position, Runtime, Size,
    WebviewWindow,
};

use crate::errors::IpcError;

use super::settings::WebcamSettings;

pub const OVERLAY_SETTINGS_EVENT: &str = "webcam://overlay-settings-changed";
const OVERLAY_WINDOW: &str = "webcam-overlay";
const TOOLBAR_WINDOW: &str = "recording-toolbar";
const MAIN_WINDOW: &str = "main";
const MARGIN: i64 = 20;
const GAP: i64 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackPositions {
    pub toolbar: (i32, i32),
    pub webcam: (i32, i32),
}

pub fn emit_settings<R: Runtime>(
    app: &AppHandle<R>,
    settings: &WebcamSettings,
) -> Result<(), IpcError> {
    app.emit(OVERLAY_SETTINGS_EVENT, settings)
        .map_err(|error| IpcError::internal(format!("Could not update webcam overlay: {error}")))
}

pub fn configure_size<R: Runtime>(
    window: &WebviewWindow<R>,
    settings: &WebcamSettings,
    camera_dimensions: Option<(u32, u32)>,
) -> Result<(), IpcError> {
    let width = f64::from(settings.width);
    let height = if settings.shape == super::settings::WebcamShape::Circle {
        width
    } else {
        camera_dimensions
            .filter(|(w, h)| *w > 0 && *h > 0)
            .map(|(w, h)| width * f64::from(h) / f64::from(w))
            .unwrap_or(width * 0.75)
    };
    window
        .set_size(Size::Logical(LogicalSize::new(width, height.max(1.0))))
        .map_err(|error| IpcError::internal(format!("Could not size the webcam overlay: {error}")))
}

pub fn show_idle<R: Runtime>(app: &AppHandle<R>) -> Result<(), IpcError> {
    let window = overlay_window(app)?;
    configure_interaction(&window)?;
    let main = app.get_webview_window(MAIN_WINDOW).ok_or_else(|| {
        IpcError::new(
            "main_window_unavailable",
            "The main window is unavailable.",
            true,
        )
    })?;
    let monitor = main
        .current_monitor()
        .map_err(|error| {
            IpcError::internal(format!("Could not inspect the current monitor: {error}"))
        })?
        .or(main.primary_monitor().map_err(|error| {
            IpcError::internal(format!("Could not inspect the primary monitor: {error}"))
        })?);
    let monitor = monitor.ok_or_else(|| {
        IpcError::new(
            "display_unavailable",
            "No display is available for the webcam overlay.",
            true,
        )
    })?;
    let position = monitor.position();
    let size = monitor.size();
    let overlay_size = window.outer_size().map_err(|error| {
        IpcError::internal(format!("Could not read webcam overlay size: {error}"))
    })?;
    let margin = (MARGIN as f64 * monitor.scale_factor()).round() as i64;
    let x = i64::from(position.x) + i64::from(size.width) - i64::from(overlay_size.width) - margin;
    let y =
        i64::from(position.y) + i64::from(size.height) - i64::from(overlay_size.height) - margin;
    window
        .set_position(Position::Physical(PhysicalPosition::new(
            clamp_i32(x),
            clamp_i32(y),
        )))
        .map_err(|error| {
            IpcError::internal(format!("Could not place the webcam overlay: {error}"))
        })?;
    window
        .show()
        .map_err(|error| IpcError::internal(format!("Could not show the webcam overlay: {error}")))
}

pub fn hide<R: Runtime>(app: &AppHandle<R>) -> Result<(), IpcError> {
    if let Some(window) = app.get_webview_window(OVERLAY_WINDOW) {
        window.hide().map_err(|error| {
            IpcError::internal(format!("Could not hide the webcam overlay: {error}"))
        })?;
    }
    Ok(())
}

pub fn place_recording_stack<R: Runtime>(
    app: &AppHandle<R>,
    toolbar: &WebviewWindow<R>,
    target: (i32, i32, u32, u32),
    webcam_enabled: bool,
) -> Result<(), IpcError> {
    let overlay = overlay_window(app)?;
    configure_interaction(&overlay)?;
    let toolbar_size = toolbar
        .outer_size()
        .map_err(|error| IpcError::internal(format!("Could not read toolbar size: {error}")))?;
    let webcam_size = if webcam_enabled {
        overlay.outer_size().map_err(|error| {
            IpcError::internal(format!("Could not read webcam overlay size: {error}"))
        })?
    } else {
        tauri::PhysicalSize::new(0, 0)
    };
    let positions = recording_stack_positions(
        target,
        (toolbar_size.width, toolbar_size.height),
        (webcam_size.width, webcam_size.height),
        webcam_enabled,
        MARGIN,
        GAP,
    );
    let toolbar_position = positions.toolbar;
    let webcam_position = positions.webcam;
    set_toolbar_baseline(app, toolbar_position)?;
    toolbar
        .set_position(Position::Physical(PhysicalPosition::new(
            toolbar_position.0,
            toolbar_position.1,
        )))
        .map_err(|error| {
            IpcError::internal(format!("Could not position the recording toolbar: {error}"))
        })?;
    if webcam_enabled {
        overlay
            .set_position(Position::Physical(PhysicalPosition::new(
                webcam_position.0,
                webcam_position.1,
            )))
            .map_err(|error| {
                IpcError::internal(format!("Could not position the webcam overlay: {error}"))
            })?;
        overlay.show().map_err(|error| {
            IpcError::internal(format!("Could not show the webcam overlay: {error}"))
        })?;
    } else {
        overlay.hide().map_err(|error| {
            IpcError::internal(format!("Could not hide the webcam overlay: {error}"))
        })?;
    }
    Ok(())
}

pub fn follow_toolbar<R: Runtime>(app: &AppHandle<R>, x: i32, y: i32) -> Result<(), IpcError> {
    let Some(toolbar) = app.get_webview_window(TOOLBAR_WINDOW) else {
        return Ok(());
    };
    let Some(overlay) = app.get_webview_window(OVERLAY_WINDOW) else {
        return Ok(());
    };
    let toolbar_size = toolbar
        .outer_size()
        .map_err(|error| IpcError::internal(format!("Could not read toolbar size: {error}")))?;
    let overlay_size = overlay.outer_size().map_err(|error| {
        IpcError::internal(format!("Could not read webcam overlay size: {error}"))
    })?;
    let webcam_x = i64::from(x) + i64::from(toolbar_size.width) - i64::from(overlay_size.width);
    let webcam_y = i64::from(y) + i64::from(toolbar_size.height) + GAP;
    overlay
        .set_position(Position::Physical(PhysicalPosition::new(
            clamp_i32(webcam_x),
            clamp_i32(webcam_y),
        )))
        .map_err(|error| {
            IpcError::internal(format!(
                "Could not move the webcam overlay with its toolbar: {error}"
            ))
        })
}

pub fn recording_stack_positions(
    target: (i32, i32, u32, u32),
    toolbar_size: (u32, u32),
    webcam_size: (u32, u32),
    webcam_enabled: bool,
    margin: i64,
    gap: i64,
) -> StackPositions {
    let (left, top, width, height) = target;
    let right = i64::from(left) + i64::from(width);
    let bottom = i64::from(top) + i64::from(height);
    let toolbar_x = (right - i64::from(toolbar_size.0) - margin).max(i64::from(left));
    let stack_height = i64::from(toolbar_size.1)
        + if webcam_enabled {
            gap + i64::from(webcam_size.1)
        } else {
            0
        };
    let toolbar_y = (bottom - stack_height - margin).max(i64::from(top));
    let webcam_x = toolbar_x + i64::from(toolbar_size.0) - i64::from(webcam_size.0);
    let webcam_y = toolbar_y + i64::from(toolbar_size.1) + gap;
    StackPositions {
        toolbar: (clamp_i32(toolbar_x), clamp_i32(toolbar_y)),
        webcam: (clamp_i32(webcam_x), clamp_i32(webcam_y)),
    }
}

fn overlay_window<R: Runtime>(app: &AppHandle<R>) -> Result<WebviewWindow<R>, IpcError> {
    app.get_webview_window(OVERLAY_WINDOW).ok_or_else(|| {
        IpcError::new(
            "webcam_overlay_unavailable",
            "The webcam overlay window is unavailable.",
            true,
        )
    })
}

fn configure_interaction<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), IpcError> {
    window.set_always_on_top(true).map_err(|error| {
        IpcError::internal(format!(
            "Could not keep the webcam overlay above other windows: {error}"
        ))
    })?;
    window.set_ignore_cursor_events(true).map_err(|error| {
        IpcError::internal(format!(
            "Could not make the webcam overlay click-through: {error}"
        ))
    })
}

fn set_toolbar_baseline<R: Runtime>(
    app: &AppHandle<R>,
    position: (i32, i32),
) -> Result<(), IpcError> {
    if let Some(state) = app.try_state::<crate::app_state::AppState>() {
        *state
            .toolbar_last_position
            .lock()
            .map_err(|_| IpcError::internal("Toolbar position state is unavailable."))? =
            Some(position);
    }
    Ok(())
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::recording_stack_positions;

    #[test]
    fn recording_stack_places_webcam_under_toolbar_at_target_bottom_right() {
        let positions =
            recording_stack_positions((0, 0, 1920, 1080), (620, 58), (180, 135), true, 20, 8);
        assert_eq!(positions.webcam.0 + 180, positions.toolbar.0 + 620);
        assert_eq!(positions.webcam.1, positions.toolbar.1 + 58 + 8);
        assert_eq!(positions.webcam.1 + 135, 1060);
    }
}
