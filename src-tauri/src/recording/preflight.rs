use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{
    audio,
    recording::{AudioSource, RecordingOptions},
    webcam::{devices, settings::WebcamSettings},
};

const HIGH_PIXEL_RATE: u64 = 250_000_000;

mod storage;
mod target;

pub use storage::free_bytes_for_path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightStatus {
    Ready,
    Warning,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreflightCheck {
    pub key: String,
    pub label: String,
    pub status: PreflightStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingPreflightReport {
    pub can_start: bool,
    pub fps: u16,
    pub max_height: u16,
    pub output_width: Option<u32>,
    pub output_height: Option<u32>,
    pub workload_pixels_per_second: Option<u64>,
    pub free_disk_bytes: Option<u64>,
    pub estimated_minutes_remaining: Option<u64>,
    pub checks: Vec<PreflightCheck>,
}

pub fn inspect(
    app: &AppHandle,
    options: &RecordingOptions,
    webcam: &WebcamSettings,
) -> RecordingPreflightReport {
    let mut checks = Vec::new();
    let target = target::inspect_target(app, &options.target);
    push_result_check(
        &mut checks,
        "capture",
        "Capture source",
        target
            .as_ref()
            .map(|target| target.detail.clone())
            .map_err(Clone::clone),
    );

    let dimensions = target.as_ref().ok().and_then(|target| {
        target::output_dimensions(target.width, target.height, options.max_height)
    });
    let workload = dimensions.map(|(width, height)| {
        u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(u64::from(options.fps))
    });
    if let Some((width, height)) = dimensions {
        let megapixels_per_second = workload.unwrap_or_default() / 1_000_000;
        if workload.is_some_and(|rate| rate >= HIGH_PIXEL_RATE) {
            let webcam_note = if webcam.enabled {
                " Webcam composition adds more per-frame work."
            } else {
                ""
            };
            checks.push(check(
                "workload",
                "Video workload",
                PreflightStatus::Warning,
                format!(
                    "{width} × {height} at {} FPS is about {megapixels_per_second} million pixels/s; some PCs may drop frames.{webcam_note}",
                    options.fps
                ),
            ));
        } else {
            checks.push(check(
                "workload",
                "Video workload",
                PreflightStatus::Ready,
                format!("{width} × {height} at {} FPS.", options.fps),
            ));
        }
    } else {
        checks.push(check(
            "workload",
            "Video workload",
            PreflightStatus::Skipped,
            "Resolve the capture source before estimating the video workload.".to_owned(),
        ));
    }

    check_audio_sources(options, &mut checks);
    check_webcam(webcam, &mut checks);
    let storage = storage::inspect_storage(options, dimensions);
    checks.push(storage.check);

    let can_start = !checks
        .iter()
        .any(|item| item.status == PreflightStatus::Error);
    RecordingPreflightReport {
        can_start,
        fps: options.fps,
        max_height: options.max_height,
        output_width: dimensions.map(|(width, _)| width),
        output_height: dimensions.map(|(_, height)| height),
        workload_pixels_per_second: workload,
        free_disk_bytes: storage.free_disk_bytes,
        estimated_minutes_remaining: storage.estimated_minutes_remaining,
        checks,
    }
}

fn check_audio_sources(options: &RecordingOptions, checks: &mut Vec<PreflightCheck>) {
    if !options.microphone_enabled && !options.system_audio {
        checks.push(check(
            "audio",
            "Audio inputs",
            PreflightStatus::Skipped,
            "Microphone and system audio are off.".to_owned(),
        ));
        return;
    }

    match audio::list_audio_devices() {
        Ok(devices) => {
            let microphone_missing = options.microphone_enabled
                && !devices.iter().any(|device| {
                    device.source == AudioSource::Microphone
                        && options
                            .microphone_device_id
                            .as_ref()
                            .is_none_or(|id| device.device_id == *id)
                });
            let system_missing = options.system_audio
                && !devices
                    .iter()
                    .any(|device| device.source == AudioSource::System);
            let status = if microphone_missing || system_missing {
                PreflightStatus::Error
            } else {
                PreflightStatus::Ready
            };
            let mut selected = Vec::new();
            if options.microphone_enabled {
                selected.push(if microphone_missing {
                    "microphone unavailable"
                } else {
                    "microphone ready"
                });
            }
            if options.system_audio {
                selected.push(if system_missing {
                    "system audio unavailable"
                } else {
                    "system audio ready"
                });
            }
            let note = if microphone_missing || system_missing {
                "Turn off unavailable sources or reconnect the device before recording."
            } else {
                "Device access is verified again when recording starts. Use Test audio to check for a signal."
            };
            checks.push(check(
                "audio",
                "Audio inputs",
                status,
                format!("{} · {note}", selected.join(" · ")),
            ));
        }
        Err(error) => checks.push(check(
            "audio",
            "Audio inputs",
            PreflightStatus::Error,
            error.message,
        )),
    }
}

fn check_webcam(webcam: &WebcamSettings, checks: &mut Vec<PreflightCheck>) {
    if !webcam.enabled {
        checks.push(check(
            "webcam",
            "Webcam overlay",
            PreflightStatus::Skipped,
            "Off · the recording will not start a camera session.".to_owned(),
        ));
        return;
    }

    let Some(camera_id) = webcam.camera_id.as_deref() else {
        checks.push(check(
            "webcam",
            "Webcam overlay",
            PreflightStatus::Error,
            "Choose a camera in Webcam Overlay settings before recording.".to_owned(),
        ));
        return;
    };

    match devices::enumerate() {
        Ok(cameras) if cameras.iter().any(|camera| camera.id == camera_id) => checks.push(check(
            "webcam",
            "Webcam overlay",
            PreflightStatus::Ready,
            "Selected camera is connected. Camera access is confirmed when capture starts.".to_owned(),
        )),
        Ok(_) => checks.push(check(
            "webcam",
            "Webcam overlay",
            PreflightStatus::Error,
            "The selected camera is disconnected. Choose another camera or turn off Webcam Overlay.".to_owned(),
        )),
        Err(error) => checks.push(check(
            "webcam",
            "Webcam overlay",
            PreflightStatus::Error,
            format!("Camera check failed: {}", error.message),
        )),
    }
}

fn check(key: &str, label: &str, status: PreflightStatus, detail: String) -> PreflightCheck {
    PreflightCheck {
        key: key.to_owned(),
        label: label.to_owned(),
        status,
        detail,
    }
}

fn push_result_check(
    checks: &mut Vec<PreflightCheck>,
    key: &str,
    label: &str,
    detail: Result<String, String>,
) {
    match detail {
        Ok(detail) => checks.push(check(key, label, PreflightStatus::Ready, detail)),
        Err(detail) => checks.push(check(key, label, PreflightStatus::Error, detail)),
    }
}
