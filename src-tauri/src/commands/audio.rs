#[cfg(windows)]
use std::{
    thread,
    time::{Duration, Instant},
};

use tauri::WebviewWindow;

use crate::{
    audio::{AudioDevice, AudioInputTestResult},
    errors::IpcError,
};

#[cfg(windows)]
use crate::audio::AudioLevel;
#[cfg(windows)]
use crate::recording::AudioSource;

#[tauri::command]
pub fn list_audio_devices(window: WebviewWindow) -> Result<Vec<AudioDevice>, IpcError> {
    if window.label() != "main" {
        return Err(IpcError::new(
            "window_not_authorized",
            "Only the main window can enumerate audio devices.",
            false,
        ));
    }
    crate::audio::list_audio_devices()
}

#[tauri::command]
pub async fn test_audio_inputs(
    window: WebviewWindow,
    microphone_enabled: bool,
    system_audio_enabled: bool,
    microphone_device_id: Option<String>,
) -> Result<AudioInputTestResult, IpcError> {
    if window.label() != "main" {
        return Err(IpcError::new(
            "window_not_authorized",
            "Only the main window can test audio inputs.",
            false,
        ));
    }
    if !microphone_enabled && !system_audio_enabled {
        return Err(IpcError::new(
            "audio_test_no_sources",
            "Select a microphone or system audio source before testing.",
            true,
        ));
    }

    #[cfg(windows)]
    {
        tauri::async_runtime::spawn_blocking(move || {
            run_audio_input_test(
                microphone_device_id.as_deref(),
                microphone_enabled,
                system_audio_enabled,
            )
        })
        .await
        .map_err(|error| IpcError::internal(format!("Audio input test failed: {error}")))?
    }
    #[cfg(not(windows))]
    {
        let _ = (
            microphone_device_id,
            microphone_enabled,
            system_audio_enabled,
        );
        Err(IpcError::new(
            "audio_capture_unsupported",
            "Native audio input testing is currently implemented only for Windows.",
            true,
        ))
    }
}

#[cfg(windows)]
fn run_audio_input_test(
    microphone_device_id: Option<&str>,
    microphone_enabled: bool,
    system_audio_enabled: bool,
) -> Result<AudioInputTestResult, IpcError> {
    let capture = crate::audio::AudioCapture::start(
        microphone_device_id,
        microphone_enabled,
        system_audio_enabled,
    )?;
    let started = Instant::now();
    let duration = Duration::from_secs(2);
    let mut result = AudioInputTestResult {
        duration_ms: duration.as_millis() as u64,
        microphone: microphone_enabled.then_some(AudioLevel::default()),
        system: system_audio_enabled.then_some(AudioLevel::default()),
        microphone_packets: 0,
        system_packets: 0,
    };

    while started.elapsed() < duration {
        accumulate_audio_levels(&mut result, capture.drain()?);
        thread::sleep(Duration::from_millis(20));
    }
    accumulate_audio_levels(&mut result, capture.stop()?);
    Ok(result)
}

#[cfg(windows)]
fn accumulate_audio_levels(
    result: &mut AudioInputTestResult,
    samples: impl IntoIterator<Item = crate::audio::AudioSample>,
) {
    for sample in samples {
        let level = crate::audio::measure_s16le(&sample.pcm_s16le);
        match sample.source {
            AudioSource::Microphone => {
                result.microphone_packets = result.microphone_packets.saturating_add(1);
                if let Some(current) = result.microphone {
                    result.microphone = Some(current.max(level));
                }
            }
            AudioSource::System => {
                result.system_packets = result.system_packets.saturating_add(1);
                if let Some(current) = result.system {
                    result.system = Some(current.max(level));
                }
            }
        }
    }
}
