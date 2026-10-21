use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        Media::Audio::{
            AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, AUDCLNT_BUFFERFLAGS_SILENT,
            AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR, AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, EDataFlow,
            IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, WAVE_FORMAT_PCM, WAVEFORMATEX,
            eCapture, eMultimedia, eRender,
        },
        System::{
            Com::{CLSCTX_ALL, CoCreateInstance},
            Threading::{CreateEventW, WaitForSingleObject},
        },
    },
    core::{GUID, PCWSTR},
};

use crate::{errors::IpcError, recording::AudioSource};

use super::{AudioSample, native_audio_error};

const AUDIO_SAMPLE_RATE: u32 = 48_000;
const AUDIO_CHANNELS: u16 = 2;
const AUDIO_BYTES_PER_FRAME: usize = 4;

pub(super) struct AudioStream {
    client: IAudioClient,
    capture: IAudioCaptureClient,
    event: EventHandle,
    enabled: bool,
}

impl AudioStream {
    pub(super) fn open(source: AudioSource, device_id: Option<&str>) -> Result<Self, IpcError> {
        const MMDEVICE_ENUMERATOR_CLSID: GUID =
            GUID::from_u128(0xbcde0395_e52f_467c_8e3d_c4579291692e);
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDEVICE_ENUMERATOR_CLSID, None, CLSCTX_ALL)
                .map_err(|error| native_audio_error("open Windows audio endpoints", error))?
        };
        let flow: EDataFlow = match source {
            AudioSource::Microphone => eCapture,
            AudioSource::System => eRender,
        };
        let device = if let Some(device_id) = device_id {
            let device_id: Vec<u16> = device_id.encode_utf16().chain(Some(0)).collect();
            unsafe { enumerator.GetDevice(PCWSTR(device_id.as_ptr())) }
        } else {
            unsafe { enumerator.GetDefaultAudioEndpoint(flow, eMultimedia) }
        }
        .map_err(|error| {
            native_audio_error(
                match source {
                    AudioSource::Microphone => "open the selected microphone",
                    AudioSource::System => "open the default audio output for loopback capture",
                },
                error,
            )
        })?;
        let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
            .map_err(|error| native_audio_error("create a WASAPI audio client", error))?;
        let format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nChannels: AUDIO_CHANNELS,
            nSamplesPerSec: AUDIO_SAMPLE_RATE,
            nAvgBytesPerSec: (AUDIO_SAMPLE_RATE as usize * AUDIO_BYTES_PER_FRAME) as u32,
            nBlockAlign: AUDIO_BYTES_PER_FRAME as u16,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let mut flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK
            | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
            | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
        if source == AudioSource::System {
            flags |= AUDCLNT_STREAMFLAGS_LOOPBACK;
        }
        unsafe {
            client
                .Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 400_000, 0, &format, None)
                .map_err(|error| {
                    native_audio_error("initialize shared-mode WASAPI capture", error)
                })?;
        }
        let event = EventHandle::create()?;
        unsafe {
            client
                .SetEventHandle(event.0)
                .map_err(|error| native_audio_error("create an audio capture event", error))?;
        }
        let capture = unsafe { client.GetService::<IAudioCaptureClient>() }
            .map_err(|error| native_audio_error("access audio samples", error))?;
        unsafe {
            client
                .Start()
                .map_err(|error| native_audio_error("start audio capture", error))?;
        }
        Ok(Self {
            client,
            capture,
            event,
            enabled: true,
        })
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) -> Result<(), IpcError> {
        if self.enabled == enabled {
            return Ok(());
        }
        let result = unsafe {
            if enabled {
                self.client.Start()
            } else {
                self.client.Stop()
            }
        };
        result.map_err(|error| {
            native_audio_error(
                if enabled {
                    "resume audio capture"
                } else {
                    "pause audio capture"
                },
                error,
            )
        })?;
        self.enabled = enabled;
        Ok(())
    }

    pub(super) fn wait_for_data(&self) -> Result<bool, IpcError> {
        let wait = unsafe { WaitForSingleObject(self.event.0, 100) };
        if wait.0 == 258 {
            return Ok(false);
        }
        if wait != WAIT_OBJECT_0 {
            return Err(native_audio_error(
                "wait for audio samples",
                std::io::Error::last_os_error(),
            ));
        }
        Ok(true)
    }

    pub(super) fn next_sample(&self, source: AudioSource) -> Result<Option<AudioSample>, IpcError> {
        let frames = unsafe { self.capture.GetNextPacketSize() }
            .map_err(|error| native_audio_error("inspect the audio packet", error))?;
        if frames == 0 {
            return Ok(None);
        }
        read_audio_packet(&self.capture, source, frames)
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        if self.enabled {
            let _ = unsafe { self.client.Stop() };
            self.enabled = false;
        }
    }
}

struct EventHandle(HANDLE);

impl EventHandle {
    fn create() -> Result<Self, IpcError> {
        unsafe { CreateEventW(None, false, false, PCWSTR::null()) }
            .map(Self)
            .map_err(|error| native_audio_error("create an audio capture event", error))
    }
}

impl Drop for EventHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

struct CaptureBuffer<'a> {
    client: &'a IAudioCaptureClient,
    frame_count: u32,
}

impl Drop for CaptureBuffer<'_> {
    fn drop(&mut self) {
        let _ = unsafe { self.client.ReleaseBuffer(self.frame_count) };
    }
}

/// WASAPI leaves DATA_DISCONTINUITY undefined on the first packet while device position is zero.
fn has_audio_data_discontinuity(flags: u32, device_position: u64) -> bool {
    device_position != 0 && flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0
}

fn read_audio_packet(
    client: &IAudioCaptureClient,
    source: AudioSource,
    requested_frames: u32,
) -> Result<Option<AudioSample>, IpcError> {
    let byte_count = usize::try_from(requested_frames)
        .ok()
        .and_then(|frames| frames.checked_mul(AUDIO_BYTES_PER_FRAME))
        .ok_or_else(|| {
            IpcError::new(
                "audio_packet_too_large",
                "The audio packet exceeds supported memory bounds.",
                true,
            )
        })?;
    let mut data = std::ptr::null_mut();
    let mut frame_count = 0;
    let mut flags = 0;
    let mut device_position = 0;
    let mut timestamp_100ns = 0;
    unsafe {
        client
            .GetBuffer(
                &mut data,
                &mut frame_count,
                &mut flags,
                Some(&mut device_position),
                Some(&mut timestamp_100ns),
            )
            .map_err(|error| native_audio_error("read audio samples", error))?;
    }
    let _buffer = CaptureBuffer {
        client,
        frame_count,
    };
    if flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR.0 as u32 != 0 {
        return Err(IpcError::new(
            "audio_timestamp_invalid",
            "Windows did not provide a reliable audio timestamp, so the recording was stopped to avoid unsynchronized sound.",
            true,
        ));
    }
    let discontinuity = has_audio_data_discontinuity(flags, device_position);
    let actual_bytes = usize::try_from(frame_count)
        .ok()
        .and_then(|frames| frames.checked_mul(AUDIO_BYTES_PER_FRAME))
        .filter(|length| *length <= byte_count)
        .ok_or_else(|| {
            IpcError::new(
                "audio_packet_invalid",
                "Windows returned an audio packet larger than the advertised buffer.",
                true,
            )
        })?;
    if frame_count == 0 {
        return Ok(None);
    }
    let pcm = unsafe {
        copy_audio_bytes(
            data,
            actual_bytes,
            flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0,
        )?
    };
    let timestamp_100ns = i64::try_from(timestamp_100ns).map_err(|_| {
        IpcError::new(
            "audio_timestamp_invalid",
            "Windows returned an audio timestamp outside the supported range.",
            true,
        )
    })?;
    Ok(Some(AudioSample {
        source,
        timestamp_100ns,
        discontinuity,
        frame_count,
        pcm_s16le: pcm,
    }))
}

/// Copies a WASAPI packet; `data` must point to `byte_count` readable bytes unless `silent` is true.
unsafe fn copy_audio_bytes(
    data: *const u8,
    byte_count: usize,
    silent: bool,
) -> Result<Vec<u8>, IpcError> {
    if silent {
        return Ok(vec![0; byte_count]);
    }
    if data.is_null() {
        return Err(IpcError::new(
            "audio_packet_invalid",
            "Windows returned an empty pointer for a non-silent audio packet.",
            true,
        ));
    }
    let mut pcm = vec![0; byte_count];
    unsafe { std::ptr::copy_nonoverlapping(data, pcm.as_mut_ptr(), byte_count) };
    Ok(pcm)
}

#[cfg(test)]
mod tests {
    use super::{
        AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, copy_audio_bytes, has_audio_data_discontinuity,
    };

    #[test]
    fn ignores_the_undefined_first_packet_discontinuity_flag() {
        let discontinuity = AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32;
        assert!(!has_audio_data_discontinuity(discontinuity, 0));
    }

    #[test]
    fn detects_discontinuity_after_device_position_is_available() {
        let discontinuity = AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32;
        assert!(has_audio_data_discontinuity(discontinuity, 1));
        assert!(!has_audio_data_discontinuity(0, 1));
    }

    #[test]
    fn silent_wasapi_packet_does_not_dereference_a_null_buffer() {
        let pcm = unsafe { copy_audio_bytes(std::ptr::null(), 8, true) }.unwrap();
        assert_eq!(pcm, vec![0; 8]);
    }

    #[test]
    fn non_silent_wasapi_packet_rejects_a_null_buffer() {
        let error = unsafe { copy_audio_bytes(std::ptr::null(), 8, false) }.unwrap_err();
        assert_eq!(error.code, "audio_packet_invalid");
    }

    #[test]
    fn non_silent_wasapi_packet_copies_pcm_bytes() {
        let input = [1_u8, 2, 3, 4];
        let pcm = unsafe { copy_audio_bytes(input.as_ptr(), input.len(), false) }.unwrap();
        assert_eq!(pcm, input);
    }
}
