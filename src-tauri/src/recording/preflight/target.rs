use tauri::AppHandle;

use crate::{
    capture::sources::{self, DisplaySource},
    recording::CaptureTarget,
};

pub(super) struct TargetInfo {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) detail: String,
}

pub(super) fn inspect_target(
    app: &AppHandle,
    target: &CaptureTarget,
) -> Result<TargetInfo, String> {
    match target {
        CaptureTarget::Display { display_id } => {
            let displays = sources::list_displays(app).map_err(|error| error.message)?;
            let display =
                sources::find_display(&displays, display_id).map_err(|error| error.message)?;
            Ok(TargetInfo {
                width: display.width,
                height: display.height,
                detail: format!(
                    "{} · {} × {} px",
                    display.name, display.width, display.height
                ),
            })
        }
        CaptureTarget::Window { window_id } => {
            #[cfg(windows)]
            {
                let (x, y, width, height) =
                    crate::capture::platform::windows::recordable_window_bounds(window_id)
                        .map_err(|error| error.message)?;
                let _ = (x, y);
                Ok(TargetInfo {
                    width,
                    height,
                    detail: format!("Selected window · {width} × {height} px"),
                })
            }
            #[cfg(not(windows))]
            {
                let _ = window_id;
                Err("Native window capture checks are not implemented on this platform.".to_owned())
            }
        }
        CaptureTarget::Region {
            display_id,
            x,
            y,
            width,
            height,
        } => {
            let displays = sources::list_displays(app).map_err(|error| error.message)?;
            let display =
                sources::find_display(&displays, display_id).map_err(|error| error.message)?;
            if !region_fits_display(&display, *x, *y, *width, *height) {
                return Err(
                    "The selected region no longer fits its display. Select the region again."
                        .to_owned(),
                );
            }
            Ok(TargetInfo {
                width: *width,
                height: *height,
                detail: format!("Selected region · {width} × {height} px"),
            })
        }
    }
}

fn region_fits_display(display: &DisplaySource, x: i32, y: i32, width: u32, height: u32) -> bool {
    if width < 2 || height < 2 || x < display.x || y < display.y {
        return false;
    }
    let right = i64::from(x) + i64::from(width);
    let bottom = i64::from(y) + i64::from(height);
    let display_right = i64::from(display.x) + i64::from(display.width);
    let display_bottom = i64::from(display.y) + i64::from(display.height);
    right <= display_right && bottom <= display_bottom
}

pub(super) fn output_dimensions(width: u32, height: u32, max_height: u16) -> Option<(u32, u32)> {
    if width < 2 || height < 2 || max_height < 2 {
        return None;
    }
    let scale = (f64::from(max_height) / f64::from(height)).min(1.0);
    let output_width = ((f64::from(width) * scale).round() as u32) & !1;
    let output_height = ((f64::from(height) * scale).round() as u32) & !1;
    (output_width >= 2 && output_height >= 2).then_some((output_width, output_height))
}
