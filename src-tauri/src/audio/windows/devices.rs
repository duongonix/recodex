use crate::{audio::AudioDevice, errors::IpcError, recording::AudioSource};

use super::com::ComApartment;

pub fn list_audio_devices() -> Result<Vec<AudioDevice>, IpcError> {
    std::thread::Builder::new()
        .name("recodex-audio-enumeration".to_owned())
        .spawn(enumerate_audio_devices)
        .map_err(|error| {
            IpcError::new(
                "audio_enumeration_failed",
                format!("Could not start audio-device enumeration: {error}"),
                true,
            )
        })?
        .join()
        .map_err(|_| {
            IpcError::new(
                "audio_enumeration_failed",
                "The audio-device enumeration worker stopped unexpectedly.",
                true,
            )
        })?
}

fn enumerate_audio_devices() -> Result<Vec<AudioDevice>, IpcError> {
    let _apartment = ComApartment::initialize()?;
    let enumerator =
        wasapi::DeviceEnumerator::new().map_err(wasapi_error("enumerate audio devices"))?;
    let default_microphone = enumerator
        .get_default_device(&wasapi::Direction::Capture)
        .ok()
        .and_then(|device| device.get_id().ok());
    let default_output = enumerator
        .get_default_device(&wasapi::Direction::Render)
        .ok()
        .and_then(|device| device.get_id().ok());
    let mut devices = Vec::new();
    append_devices(
        &enumerator,
        wasapi::Direction::Capture,
        AudioSource::Microphone,
        default_microphone.as_deref(),
        &mut devices,
    )?;
    append_devices(
        &enumerator,
        wasapi::Direction::Render,
        AudioSource::System,
        default_output.as_deref(),
        &mut devices,
    )?;
    Ok(devices)
}

fn append_devices(
    enumerator: &wasapi::DeviceEnumerator,
    direction: wasapi::Direction,
    source: AudioSource,
    default_id: Option<&str>,
    output: &mut Vec<AudioDevice>,
) -> Result<(), IpcError> {
    let collection = enumerator
        .get_device_collection(&direction)
        .map_err(wasapi_error("list audio endpoints"))?;
    for index in 0..collection
        .get_nbr_devices()
        .map_err(wasapi_error("count audio endpoints"))?
    {
        let device = collection
            .get_device_at_index(index)
            .map_err(wasapi_error("read audio endpoint"))?;
        let device_id = device
            .get_id()
            .map_err(wasapi_error("read audio endpoint ID"))?;
        let name = device
            .get_friendlyname()
            .map_err(wasapi_error("read audio endpoint name"))?;
        output.push(AudioDevice {
            is_default: default_id == Some(device_id.as_str()),
            device_id,
            name,
            source,
        });
    }
    Ok(())
}

pub(super) fn resolve_endpoint_name(
    source: AudioSource,
    device_id: Option<&str>,
) -> Option<String> {
    let enumerator = wasapi::DeviceEnumerator::new().ok()?;
    let direction = match source {
        AudioSource::Microphone => wasapi::Direction::Capture,
        AudioSource::System => wasapi::Direction::Render,
    };
    match device_id {
        Some(device_id) => find_endpoint_name(&enumerator, &direction, device_id),
        None => enumerator
            .get_default_device(&direction)
            .ok()
            .and_then(|device| device.get_friendlyname().ok()),
    }
}

fn find_endpoint_name(
    enumerator: &wasapi::DeviceEnumerator,
    direction: &wasapi::Direction,
    selected_id: &str,
) -> Option<String> {
    let collection = enumerator.get_device_collection(direction).ok()?;
    for index in 0..collection.get_nbr_devices().ok()? {
        let device = collection.get_device_at_index(index).ok()?;
        if device.get_id().ok().as_deref() == Some(selected_id) {
            return device.get_friendlyname().ok();
        }
    }
    None
}

pub(super) fn is_playback_mix_endpoint(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    ["stereo mix", "what u hear", "wave out mix", "loopback"]
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn wasapi_error(operation: &'static str) -> impl FnOnce(wasapi::WasapiError) -> IpcError {
    move |error| {
        IpcError::new(
            "audio_capture_failed",
            format!(
                "Could not {operation}: {error}. Check Windows sound privacy settings and the selected audio device."
            ),
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::is_playback_mix_endpoint;

    #[test]
    fn identifies_common_playback_mix_inputs_case_insensitively() {
        for name in [
            "Stereo Mix (Realtek Audio)",
            "SOUND BLASTER What U Hear",
            "Wave Out Mix",
            "Virtual Loopback Capture",
        ] {
            assert!(is_playback_mix_endpoint(name), "{name}");
        }
        assert!(!is_playback_mix_endpoint(
            "Microphone Array (Realtek Audio)"
        ));
    }
}
