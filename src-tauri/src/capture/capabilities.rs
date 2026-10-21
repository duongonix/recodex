use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FeatureSupport {
    pub status: FeatureStatus,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FeatureStatus {
    Available,
    Limited,
    Unavailable,
    NotImplemented,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureCapabilities {
    pub platform: String,
    pub supported_capture_modes: Vec<String>,
    pub video_codecs: Vec<String>,
    pub available_fps: Vec<u16>,
    pub max_resolution_height: Option<u16>,
    pub video_capture: FeatureSupport,
    pub screenshot_capture: FeatureSupport,
    pub display_sources: FeatureSupport,
    pub window_sources: FeatureSupport,
    pub region_selection: FeatureSupport,
    pub microphone_capture: FeatureSupport,
    pub system_audio_capture: FeatureSupport,
    pub capture_exclusion: FeatureSupport,
    pub zoom_follow_cursor: FeatureSupport,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionKind {
    ScreenRecording,
    Microphone,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotRequired,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionInfo {
    pub kind: PermissionKind,
    pub status: PermissionStatus,
    pub message: String,
}

pub fn get_capabilities() -> CaptureCapabilities {
    CaptureCapabilities {
        platform: std::env::consts::OS.to_owned(),
        supported_capture_modes: video_capture_modes(),
        video_codecs: video_codecs(),
        available_fps: video_fps(),
        max_resolution_height: if cfg!(windows) { Some(2160) } else { None },
        video_capture: video_capture_support(),
        screenshot_capture: screenshot_support(),
        display_sources: available("Displays are enumerated through Tauri's native monitor API."),
        window_sources: window_source_support(),
        region_selection: available("Region selection is available through the native overlay."),
        microphone_capture: not_implemented("Microphone capability has not been checked."),
        system_audio_capture: not_implemented("System-audio capability has not been checked."),
        capture_exclusion: if cfg!(windows) {
            FeatureSupport {
                status: FeatureStatus::Limited,
                message: "The recording toolbar is assigned WDA_EXCLUDEFROMCAPTURE after Windows Graphics Capture starts, and Windows confirms the affinity. Captured-video behavior still needs device smoke testing.".to_owned(),
            }
        } else {
            not_implemented(
                "Recording-toolbar exclusion is currently implemented only for Windows.",
            )
        },
        zoom_follow_cursor: if cfg!(windows) {
            FeatureSupport {
                status: FeatureStatus::Limited,
                message: "Hold-to-zoom follows the native cursor during Windows display and window recordings. D3D11 GPU scaling is used when supported, with a Rust image-processing fallback; region capture does not zoom.".to_owned(),
            }
        } else {
            not_implemented(
                "Hold-to-zoom is currently implemented only for Windows display and window recordings.",
            )
        },
    }
}

pub fn set_audio_capabilities(
    capabilities: &mut CaptureCapabilities,
    microphone_available: bool,
    system_audio_available: bool,
) {
    capabilities.microphone_capture = audio_support(
        microphone_available,
        "WASAPI microphone capture is available through an active input device. Windows privacy settings may still deny access at recording start.",
        "No active microphone input device was found.",
    );
    capabilities.system_audio_capture = audio_support(
        system_audio_available,
        "WASAPI render-endpoint loopback is available. Audio begins only when System Audio is selected.",
        "No active audio output device was found for loopback capture.",
    );
}

pub fn get_permissions() -> Vec<PermissionInfo> {
    vec![
        permission_info(PermissionKind::ScreenRecording),
        permission_info(PermissionKind::Microphone),
    ]
}

pub fn permission_info(kind: PermissionKind) -> PermissionInfo {
    match (std::env::consts::OS, kind) {
        ("windows", PermissionKind::ScreenRecording) => PermissionInfo {
            kind,
            status: PermissionStatus::Unknown,
            message: "The capture target is selected explicitly in RecoDex and Windows Graphics Capture uses the OS default capture-border behavior. Windows does not expose a separate permission status for this target-based flow.".to_owned(),
        },
        ("windows", PermissionKind::Microphone) => PermissionInfo {
            kind,
            status: PermissionStatus::Unknown,
            message: "Windows does not expose a reliable app-level microphone permission query here. RecoDex checks access when you start a recording; if it fails, review Windows Settings > Privacy & security > Microphone.".to_owned(),
        },
        (_, PermissionKind::ScreenRecording) => PermissionInfo {
            kind,
            status: PermissionStatus::Unknown,
            message: "Screen-recording permission checks and requests are not implemented for this platform yet.".to_owned(),
        },
        (_, PermissionKind::Microphone) => PermissionInfo {
            kind,
            status: PermissionStatus::Unknown,
            message: "Microphone permission checks and requests are not implemented for this platform yet.".to_owned(),
        },
    }
}

fn window_source_support() -> FeatureSupport {
    if cfg!(windows) {
        available("Visible titled Windows top-level windows are enumerated with the Win32 API.")
    } else {
        FeatureSupport {
            status: FeatureStatus::Unavailable,
            message: "Native window enumeration is currently implemented only for Windows."
                .to_owned(),
        }
    }
}

fn screenshot_support() -> FeatureSupport {
    if cfg!(windows) {
        FeatureSupport {
            status: FeatureStatus::Limited,
            message: "Windows Graphics Capture supports explicit display, window, and selected-region screenshots in PNG or JPG; Windows version and driver support are checked when capture starts, and pixels remain in Rust for saving or clipboard delivery.".to_owned(),
        }
    } else {
        not_implemented(
            "Native screenshot capture is currently implemented for Windows only; macOS and Linux adapters are pending.",
        )
    }
}

fn video_capture_support() -> FeatureSupport {
    if cfg!(windows) {
        FeatureSupport {
            status: FeatureStatus::Limited,
            message: "Windows Graphics Capture returns native display/window/region frames and timestamps, and Media Foundation encodes H.264 MP4. The recording start/pause/stop workflow is scheduled for Phase 12; macOS/Linux adapters are not implemented.".to_owned(),
        }
    } else {
        not_implemented(
            "Native video capture is currently implemented for Windows only; macOS and Linux adapters are pending.",
        )
    }
}

fn video_capture_modes() -> Vec<String> {
    if cfg!(windows) {
        vec![
            "display".to_owned(),
            "window".to_owned(),
            "region".to_owned(),
        ]
    } else {
        Vec::new()
    }
}

fn video_codecs() -> Vec<String> {
    if cfg!(windows) {
        vec!["h264".to_owned()]
    } else {
        Vec::new()
    }
}

fn video_fps() -> Vec<u16> {
    if cfg!(windows) {
        vec![30, 60]
    } else {
        Vec::new()
    }
}

fn available(message: &str) -> FeatureSupport {
    FeatureSupport {
        status: FeatureStatus::Available,
        message: message.to_owned(),
    }
}

fn not_implemented(message: &str) -> FeatureSupport {
    FeatureSupport {
        status: FeatureStatus::NotImplemented,
        message: message.to_owned(),
    }
}

fn audio_support(
    available: bool,
    available_message: &str,
    missing_message: &str,
) -> FeatureSupport {
    if cfg!(windows) && available {
        FeatureSupport {
            status: FeatureStatus::Limited,
            message: available_message.to_owned(),
        }
    } else if cfg!(windows) {
        FeatureSupport {
            status: FeatureStatus::Unavailable,
            message: missing_message.to_owned(),
        }
    } else {
        FeatureSupport {
            status: FeatureStatus::NotImplemented,
            message: "Native audio capture is currently implemented only for Windows.".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FeatureStatus, PermissionKind, PermissionStatus, get_capabilities, permission_info,
    };

    #[test]
    fn capture_implementation_status_is_distinct_from_source_enumeration() {
        let capabilities = get_capabilities();

        assert_eq!(
            capabilities.video_capture.status,
            if cfg!(windows) {
                FeatureStatus::Limited
            } else {
                FeatureStatus::NotImplemented
            }
        );
        assert_eq!(
            capabilities.screenshot_capture.status,
            if cfg!(windows) {
                FeatureStatus::Limited
            } else {
                FeatureStatus::NotImplemented
            }
        );
        assert_eq!(
            capabilities.display_sources.status,
            FeatureStatus::Available
        );
        assert_eq!(
            capabilities.window_sources.status,
            if cfg!(windows) {
                FeatureStatus::Available
            } else {
                FeatureStatus::Unavailable
            }
        );
    }

    #[test]
    fn windows_capture_permission_is_not_falsely_reported_as_granted() {
        let permission = permission_info(PermissionKind::ScreenRecording);

        assert_eq!(permission.status, PermissionStatus::Unknown);
        if cfg!(windows) {
            assert!(
                permission
                    .message
                    .contains("OS default capture-border behavior")
            );
        }
    }
}
