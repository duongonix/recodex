use serde::{Deserialize, Serialize};

use crate::{capture::sources::DisplaySource, errors::IpcError, recording::CaptureTarget};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegionPickerState {
    pub display: DisplaySource,
}

pub fn normalize_region(
    display: &DisplaySource,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
) -> Result<CaptureTarget, IpcError> {
    if !display.scale_factor.is_finite()
        || display.scale_factor <= 0.0
        || display.width == 0
        || display.height == 0
        || ![start_x, start_y, end_x, end_y]
            .into_iter()
            .all(f64::is_finite)
    {
        return Err(IpcError::new(
            "invalid_region",
            "The selected region or display scale is invalid. Choose a rectangular area on the display.",
            true,
        ));
    }

    let logical_width = display.width as f64 / display.scale_factor;
    let logical_height = display.height as f64 / display.scale_factor;
    let left = clamp_local(start_x.min(end_x), logical_width, display.scale_factor);
    let right = clamp_local(start_x.max(end_x), logical_width, display.scale_factor);
    let top = clamp_local(start_y.min(end_y), logical_height, display.scale_factor);
    let bottom = clamp_local(start_y.max(end_y), logical_height, display.scale_factor);

    let region_x = i64::from(display.x) + i64::from(left);
    let region_y = i64::from(display.y) + i64::from(top);
    let region_width = right.saturating_sub(left);
    let region_height = bottom.saturating_sub(top);
    if region_width < 2 || region_height < 2 {
        return Err(IpcError::new(
            "region_too_small",
            "Select a region at least two physical pixels wide and high.",
            true,
        ));
    }

    let x = i32::try_from(region_x).map_err(|_| invalid_region_bounds())?;
    let y = i32::try_from(region_y).map_err(|_| invalid_region_bounds())?;
    Ok(CaptureTarget::Region {
        display_id: display.display_id.clone(),
        x,
        y,
        width: region_width,
        height: region_height,
    })
}

fn clamp_local(value: f64, logical_limit: f64, scale_factor: f64) -> u32 {
    (value.clamp(0.0, logical_limit) * scale_factor)
        .round()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}

fn invalid_region_bounds() -> IpcError {
    IpcError::new(
        "invalid_region",
        "The selected region falls outside the supported desktop coordinate range.",
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::normalize_region;
    use crate::{capture::sources::DisplaySource, recording::CaptureTarget};

    fn display(x: i32, y: i32, width: u32, height: u32, scale_factor: f64) -> DisplaySource {
        DisplaySource {
            display_id: "DISPLAY_TEST".to_owned(),
            name: "Test display".to_owned(),
            x,
            y,
            width,
            height,
            scale_factor,
            primary: false,
        }
    }

    #[test]
    fn converts_scaled_local_points_to_negative_desktop_coordinates() {
        let selected = normalize_region(
            &display(-1920, -100, 2560, 1440, 1.5),
            10.2,
            5.1,
            100.2,
            55.2,
        )
        .expect("valid region");

        assert_eq!(
            selected,
            CaptureTarget::Region {
                display_id: "DISPLAY_TEST".to_owned(),
                x: -1905,
                y: -92,
                width: 135,
                height: 75,
            }
        );
    }

    #[test]
    fn clamps_reverse_drag_to_monitor_physical_bounds() {
        let selected = normalize_region(
            &display(-100, 50, 1000, 500, 1.25),
            900.0,
            500.0,
            -10.0,
            -10.0,
        )
        .expect("drag is clamped to the display");

        assert_eq!(
            selected,
            CaptureTarget::Region {
                display_id: "DISPLAY_TEST".to_owned(),
                x: -100,
                y: 50,
                width: 1000,
                height: 500,
            }
        );
    }

    #[test]
    fn rejects_non_finite_scale_and_tiny_regions() {
        assert_eq!(
            normalize_region(&display(0, 0, 100, 100, f64::NAN), 0.0, 0.0, 10.0, 10.0)
                .unwrap_err()
                .code,
            "invalid_region"
        );
        assert_eq!(
            normalize_region(&display(0, 0, 100, 100, 1.0), 10.0, 10.0, 11.0, 10.0)
                .unwrap_err()
                .code,
            "region_too_small"
        );
    }
}
