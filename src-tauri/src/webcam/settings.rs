use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::IpcError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WebcamPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WebcamShape {
    Rectangle,
    Circle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WebcamBackgroundMode {
    None,
    Blur,
    Color,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebcamSettings {
    pub enabled: bool,
    pub camera_id: Option<String>,
    pub position: WebcamPosition,
    /// Normalized top-left position within the available overlay travel area.
    pub x: f32,
    pub y: f32,
    /// Width in pixels at a 1080p output height.
    pub width: u16,
    pub shape: WebcamShape,
    pub corner_radius: u8,
    pub border_enabled: bool,
    pub border_color: String,
    pub border_width: u8,
    pub background_mode: WebcamBackgroundMode,
    pub background_color: String,
    pub background_image_path: Option<String>,
    pub opacity: u8,
    pub mirror: bool,
}

impl Default for WebcamSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            camera_id: None,
            position: WebcamPosition::BottomRight,
            x: 1.0,
            y: 1.0,
            width: 180,
            shape: WebcamShape::Rectangle,
            corner_radius: 12,
            border_enabled: true,
            border_color: "#FF3038".to_owned(),
            border_width: 2,
            background_mode: WebcamBackgroundMode::None,
            background_color: "#17191F".to_owned(),
            background_image_path: None,
            opacity: 100,
            mirror: false,
        }
    }
}

impl WebcamSettings {
    pub fn validate(&self) -> Result<(), IpcError> {
        if !self.x.is_finite()
            || !(0.0..=1.0).contains(&self.x)
            || !self.y.is_finite()
            || !(0.0..=1.0).contains(&self.y)
        {
            return Err(invalid("Webcam position must be between 0 and 1."));
        }
        if !(80..=640).contains(&self.width) {
            return Err(invalid("Webcam width must be between 80 and 640 pixels."));
        }
        if self.corner_radius > 50 {
            return Err(invalid(
                "Webcam corner radius must be between 0 and 50 pixels.",
            ));
        }
        if self.border_width > 12 {
            return Err(invalid(
                "Webcam border width must be between 0 and 12 pixels.",
            ));
        }
        if self.opacity > 100 {
            return Err(invalid("Webcam opacity must be between 0 and 100 percent."));
        }
        if !is_hex_color(&self.border_color) || !is_hex_color(&self.background_color) {
            return Err(invalid("Webcam colors must use #RRGGBB format."));
        }
        if self.camera_id.as_ref().is_some_and(|id| {
            id.trim().is_empty() || id.len() > 256 || id.chars().any(char::is_control)
        }) {
            return Err(invalid("The selected camera ID is invalid."));
        }
        if let Some(path) = &self.background_image_path
            && (path.trim().is_empty()
                || path.chars().any(char::is_control)
                || !Path::new(path).is_absolute())
        {
            return Err(invalid(
                "Webcam background image must be an absolute local path.",
            ));
        }
        Ok(())
    }
}

pub fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}

fn is_hex_color(value: &str) -> bool {
    parse_hex_color(value).is_some()
}

fn invalid(message: &str) -> IpcError {
    IpcError::new("webcam_settings_invalid", message, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_and_1080p_sized() {
        let settings = WebcamSettings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.width, 180);
        assert_eq!(settings.opacity, 100);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_geometry_colors_and_paths() {
        let mut settings = WebcamSettings {
            x: f32::NAN,
            ..WebcamSettings::default()
        };
        assert_eq!(
            settings.validate().unwrap_err().code,
            "webcam_settings_invalid"
        );
        settings.x = 0.5;
        settings.border_color = "red".to_owned();
        assert!(settings.validate().is_err());
        settings.border_color = "#FF3038".to_owned();
        settings.background_image_path = Some("relative.png".to_owned());
        assert!(settings.validate().is_err());
    }

    #[test]
    fn parses_palette_color() {
        assert_eq!(parse_hex_color("#FF3038"), Some([255, 48, 56]));
        assert_eq!(parse_hex_color("#xyzxyz"), None);
    }
}
