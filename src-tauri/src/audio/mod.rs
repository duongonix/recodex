use serde::{Deserialize, Serialize};

use crate::{errors::IpcError, recording::AudioSource};

mod levels;
#[cfg(windows)]
mod mixer;
#[cfg(windows)]
mod windows;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub device_id: String,
    pub name: String,
    pub source: AudioSource,
    pub is_default: bool,
}

pub use levels::{AudioInputTestResult, AudioLevel, measure_s16le};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioAvailability {
    pub microphone: bool,
    pub system: bool,
}

pub fn list_audio_devices() -> Result<Vec<AudioDevice>, IpcError> {
    #[cfg(windows)]
    {
        windows::list_audio_devices()
    }
    #[cfg(not(windows))]
    {
        Err(unsupported_audio())
    }
}

pub fn availability() -> Result<AudioAvailability, IpcError> {
    let devices = list_audio_devices()?;
    Ok(AudioAvailability {
        microphone: devices
            .iter()
            .any(|device| device.source == AudioSource::Microphone),
        system: devices
            .iter()
            .any(|device| device.source == AudioSource::System),
    })
}

#[cfg(not(windows))]
fn unsupported_audio() -> IpcError {
    IpcError::new(
        "audio_capture_unsupported",
        "Native microphone and system-audio capture are currently implemented only for Windows.",
        true,
    )
}

#[cfg(windows)]
pub use mixer::{AudioMixSample, AudioMixer, AudioMixerDiagnostics};
#[cfg(windows)]
pub use windows::{AudioCapture, AudioMessage, AudioSample};
