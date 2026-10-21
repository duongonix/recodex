use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};

use windows::{
    Win32::{
        Media::MediaFoundation::{
            IMFAttributes, MF_MT_AAC_PAYLOAD_TYPE, MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
            MF_MT_AUDIO_BITS_PER_SAMPLE, MF_MT_AUDIO_BLOCK_ALIGNMENT, MF_MT_AUDIO_CHANNEL_MASK,
            MF_MT_AUDIO_NUM_CHANNELS, MF_MT_AUDIO_SAMPLES_PER_SECOND, MF_MT_AVG_BITRATE,
            MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE,
            MF_MT_MAJOR_TYPE, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE,
            MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, MF_VERSION, MFAudioFormat_AAC,
            MFAudioFormat_PCM, MFCreateAttributes, MFCreateMemoryBuffer, MFCreateSample,
            MFCreateSinkWriterFromURL, MFMediaType_Audio, MFMediaType_Video, MFSTARTUP_FULL,
            MFShutdown, MFStartup, MFVideoFormat_H264, MFVideoFormat_RGB32,
            MFVideoInterlace_Progressive,
        },
        System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize},
    },
    core::PCWSTR,
};

use crate::errors::IpcError;
use crate::recording::AudioSource;

pub struct MediaFoundationWriter {
    sink_writer: windows::Win32::Media::MediaFoundation::IMFSinkWriter,
    _runtime: MediaFoundationRuntime,
    stream_index: u32,
    microphone_audio_stream_index: Option<u32>,
    system_audio_stream_index: Option<u32>,
    input_width: u32,
    input_height: u32,
    frame_duration_100ns: i64,
    last_sample_time_100ns: Option<i64>,
    sample_count: u64,
    microphone_audio_last_time_100ns: Option<i64>,
    system_audio_last_time_100ns: Option<i64>,
}

impl MediaFoundationWriter {
    pub fn create(
        output_path: &Path,
        input_width: u32,
        input_height: u32,
        max_height: u16,
        fps: u16,
    ) -> Result<Self, IpcError> {
        Self::create_with_audio(
            output_path,
            input_width,
            input_height,
            max_height,
            fps,
            false,
            false,
        )
    }

    pub fn create_with_audio(
        output_path: &Path,
        input_width: u32,
        input_height: u32,
        max_height: u16,
        fps: u16,
        microphone: bool,
        system_audio: bool,
    ) -> Result<Self, IpcError> {
        validate_dimensions(input_width, input_height, fps)?;
        let runtime = MediaFoundationRuntime::initialize()?;
        let (output_width, output_height) =
            output_dimensions(input_width, input_height, max_height)?;
        let wide_path = wide_null(output_path.as_os_str());
        let sink_writer_attributes = create_sink_writer_attributes()?;

        let sink_writer = unsafe {
            MFCreateSinkWriterFromURL(
                PCWSTR(wide_path.as_ptr()),
                None::<&windows::Win32::Media::MediaFoundation::IMFByteStream>,
                Some(&sink_writer_attributes),
            )
            .map_err(|error| mf_error("Could not create the MP4 sink writer", error))?
        };

        let output_type = create_video_type(output_width, output_height, fps, true)?;
        let stream_index = unsafe {
            sink_writer
                .AddStream(&output_type)
                .map_err(|error| mf_error("Could not configure the H.264 output stream", error))?
        };

        let input_type = create_video_type(input_width, input_height, fps, false)?;
        let (microphone_audio_stream_index, system_audio_stream_index) = unsafe {
            sink_writer
                .SetInputMediaType(stream_index, &input_type, None::<&IMFAttributes>)
                .map_err(|error| {
                    mf_error("Windows could not configure the video input format", error)
                })?;
            let microphone_audio_stream_index = add_audio_stream(&sink_writer, microphone)?;
            let system_audio_stream_index = add_audio_stream(&sink_writer, system_audio)?;
            sink_writer
                .BeginWriting()
                .map_err(|error| mf_error("Windows could not start the MP4 writer", error))?;
            (microphone_audio_stream_index, system_audio_stream_index)
        };

        Ok(Self {
            _runtime: runtime,
            sink_writer,
            stream_index,
            microphone_audio_stream_index,
            system_audio_stream_index,
            input_width,
            input_height,
            frame_duration_100ns: 10_000_000 / i64::from(fps),
            last_sample_time_100ns: None,
            sample_count: 0,
            microphone_audio_last_time_100ns: None,
            system_audio_last_time_100ns: None,
        })
    }

    pub fn write_frame(&mut self, pixels: &[u8], sample_time_100ns: i64) -> Result<(), IpcError> {
        validate_sample_timestamp(self.last_sample_time_100ns, sample_time_100ns)?;
        let expected_length = frame_byte_length(self.input_width, self.input_height)?;
        if pixels.len() != expected_length {
            return Err(IpcError::new(
                "frame_layout_invalid",
                format!(
                    "Expected {expected_length} BGRA bytes, received {}.",
                    pixels.len()
                ),
                true,
            ));
        }
        if sample_time_100ns < 0 {
            return Err(IpcError::new(
                "frame_timestamp_invalid",
                "Video frame timestamps must be non-negative.",
                true,
            ));
        }

        let byte_length = u32::try_from(pixels.len()).map_err(|_| {
            IpcError::new(
                "frame_too_large",
                "The selected frame is too large to encode on this system.",
                true,
            )
        })?;
        let buffer = unsafe {
            MFCreateMemoryBuffer(byte_length)
                .map_err(|error| mf_error("Could not allocate a video sample buffer", error))?
        };
        let mut destination = std::ptr::null_mut();
        let mut maximum_length = 0;
        unsafe {
            buffer
                .Lock(&mut destination, Some(&mut maximum_length), None)
                .map_err(|error| mf_error("Could not lock a video sample buffer", error))?;
        }
        if maximum_length < byte_length || destination.is_null() {
            let _ = unsafe { buffer.Unlock() };
            return Err(IpcError::new(
                "frame_buffer_invalid",
                "Windows returned a video buffer smaller than the captured frame.",
                true,
            ));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), destination, pixels.len());
            buffer
                .Unlock()
                .map_err(|error| mf_error("Could not unlock a video sample buffer", error))?;
            buffer
                .SetCurrentLength(byte_length)
                .map_err(|error| mf_error("Could not set the video sample size", error))?;
        }

        let sample = unsafe {
            MFCreateSample().map_err(|error| mf_error("Could not create a video sample", error))?
        };
        unsafe {
            sample
                .AddBuffer(&buffer)
                .map_err(|error| mf_error("Could not attach the captured frame", error))?;
            sample
                .SetSampleTime(sample_time_100ns)
                .map_err(|error| mf_error("Could not timestamp the captured frame", error))?;
            sample
                .SetSampleDuration(self.frame_duration_100ns)
                .map_err(|error| mf_error("Could not set the video frame duration", error))?;
            self.sink_writer
                .WriteSample(self.stream_index, &sample)
                .map_err(|error| mf_error("Windows could not encode the video frame", error))?;
        }
        self.last_sample_time_100ns = Some(sample_time_100ns);
        self.sample_count = self.sample_count.saturating_add(1);
        Ok(())
    }

    pub fn finalize(self) -> Result<(), IpcError> {
        if self.sample_count == 0 {
            return Err(IpcError::new(
                "recording_empty",
                "The MP4 writer cannot finalize a recording without video frames.",
                true,
            ));
        }
        unsafe {
            self.sink_writer
                .Finalize()
                .map_err(|error| mf_error("Windows could not finalize the MP4 file", error))
        }
    }

    pub fn write_audio(
        &mut self,
        source: AudioSource,
        pcm_s16le: &[u8],
        frame_count: u32,
        sample_time_100ns: i64,
    ) -> Result<(), IpcError> {
        let (stream_index, previous_time) = match source {
            AudioSource::Microphone => (
                self.microphone_audio_stream_index,
                self.microphone_audio_last_time_100ns,
            ),
            AudioSource::System => (
                self.system_audio_stream_index,
                self.system_audio_last_time_100ns,
            ),
        };
        let stream_index = stream_index.ok_or_else(|| {
            IpcError::new(
                "audio_source_unavailable",
                format!("The {source:?} audio stream was not configured when recording started."),
                true,
            )
        })?;
        if sample_time_100ns < 0 || previous_time.is_some_and(|last| sample_time_100ns <= last) {
            return Err(IpcError::new(
                "audio_timestamp_invalid",
                "Audio track sample timestamps must be non-negative and strictly increasing.",
                true,
            ));
        }
        let expected_bytes = usize::try_from(frame_count)
            .ok()
            .and_then(|frames| frames.checked_mul(4))
            .ok_or_else(|| {
                IpcError::new(
                    "audio_packet_too_large",
                    "The audio packet exceeds supported memory bounds.",
                    true,
                )
            })?;
        if frame_count == 0 || pcm_s16le.len() != expected_bytes {
            return Err(IpcError::new(
                "audio_packet_invalid",
                "The audio packet does not match the expected 48 kHz stereo PCM layout.",
                true,
            ));
        }
        let byte_length = u32::try_from(pcm_s16le.len()).map_err(|_| {
            IpcError::new(
                "audio_packet_too_large",
                "The audio packet is too large to encode.",
                true,
            )
        })?;
        let buffer = unsafe {
            MFCreateMemoryBuffer(byte_length)
                .map_err(|error| mf_error("Could not allocate an audio sample buffer", error))?
        };
        let mut destination = std::ptr::null_mut();
        let mut maximum_length = 0;
        unsafe {
            buffer
                .Lock(&mut destination, Some(&mut maximum_length), None)
                .map_err(|error| mf_error("Could not lock an audio sample buffer", error))?;
        }
        if maximum_length < byte_length || destination.is_null() {
            let _ = unsafe { buffer.Unlock() };
            return Err(IpcError::new(
                "audio_buffer_invalid",
                "Windows returned an audio buffer smaller than the audio packet.",
                true,
            ));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(pcm_s16le.as_ptr(), destination, pcm_s16le.len());
            buffer
                .Unlock()
                .map_err(|error| mf_error("Could not unlock an audio sample buffer", error))?;
            buffer
                .SetCurrentLength(byte_length)
                .map_err(|error| mf_error("Could not set the audio sample size", error))?;
        }
        let sample = unsafe {
            MFCreateSample().map_err(|error| mf_error("Could not create an audio sample", error))?
        };
        let duration_100ns = (u64::from(frame_count) * 10_000_000 / 48_000) as i64;
        unsafe {
            sample
                .AddBuffer(&buffer)
                .map_err(|error| mf_error("Could not attach the audio packet", error))?;
            sample
                .SetSampleTime(sample_time_100ns)
                .map_err(|error| mf_error("Could not timestamp the audio packet", error))?;
            sample
                .SetSampleDuration(duration_100ns)
                .map_err(|error| mf_error("Could not set the audio packet duration", error))?;
            self.sink_writer
                .WriteSample(stream_index, &sample)
                .map_err(|error| mf_error("Windows could not encode the AAC packet", error))?;
        }
        match source {
            AudioSource::Microphone => {
                self.microphone_audio_last_time_100ns = Some(sample_time_100ns)
            }
            AudioSource::System => self.system_audio_last_time_100ns = Some(sample_time_100ns),
        }
        Ok(())
    }
}

fn add_audio_stream(
    writer: &windows::Win32::Media::MediaFoundation::IMFSinkWriter,
    enabled: bool,
) -> Result<Option<u32>, IpcError> {
    if !enabled {
        return Ok(None);
    }
    let output_type = create_audio_type(true)?;
    let stream_index = unsafe {
        writer
            .AddStream(&output_type)
            .map_err(|error| mf_error("Could not configure the AAC output stream", error))?
    };
    let input_type = create_audio_type(false)?;
    unsafe {
        writer
            .SetInputMediaType(stream_index, &input_type, None::<&IMFAttributes>)
            .map_err(|error| mf_error("Windows could not configure the PCM audio input", error))?;
    }
    Ok(Some(stream_index))
}

pub(crate) fn create_audio_type(
    encoded: bool,
) -> Result<windows::Win32::Media::MediaFoundation::IMFMediaType, IpcError> {
    use windows::Win32::Media::MediaFoundation::MFCreateMediaType;

    let media_type = unsafe {
        MFCreateMediaType()
            .map_err(|error| mf_error("Could not create an audio media type", error))?
    };
    unsafe {
        media_type
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)
            .map_err(|error| mf_error("Could not set the audio media type", error))?;
        media_type
            .SetGUID(
                &MF_MT_SUBTYPE,
                if encoded {
                    &MFAudioFormat_AAC
                } else {
                    &MFAudioFormat_PCM
                },
            )
            .map_err(|error| mf_error("Could not set the audio format", error))?;
        media_type
            .SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)
            .map_err(|error| mf_error("Could not set the audio channel count", error))?;
        media_type
            .SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, 48_000)
            .map_err(|error| mf_error("Could not set the audio sample rate", error))?;
        media_type
            .SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)
            .map_err(|error| mf_error("Could not set the audio sample depth", error))?;
        media_type
            .SetUINT32(&MF_MT_AUDIO_CHANNEL_MASK, 3)
            .map_err(|error| mf_error("Could not set the audio channel layout", error))?;
        if encoded {
            media_type
                .SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 24_000)
                .map_err(|error| mf_error("Could not set the AAC bitrate", error))?;
            media_type
                .SetUINT32(&MF_MT_AAC_PAYLOAD_TYPE, 0)
                .map_err(|error| mf_error("Could not set the AAC payload type", error))?;
        } else {
            media_type
                .SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4)
                .map_err(|error| mf_error("Could not set the PCM block alignment", error))?;
            media_type
                .SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 192_000)
                .map_err(|error| mf_error("Could not set the PCM byte rate", error))?;
        }
    }
    Ok(media_type)
}

fn validate_sample_timestamp(previous: Option<i64>, next: i64) -> Result<(), IpcError> {
    if next < 0 || previous.is_some_and(|previous| next <= previous) {
        return Err(IpcError::new(
            "frame_timestamp_invalid",
            "Video sample timestamps must be non-negative and strictly increasing.",
            true,
        ));
    }
    Ok(())
}

pub(crate) struct MediaFoundationRuntime {
    com_initialized: bool,
}

impl MediaFoundationRuntime {
    pub(crate) fn initialize() -> Result<Self, IpcError> {
        let com_result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if com_result.is_err() {
            return Err(IpcError::new(
                "media_runtime_unavailable",
                format!("Could not initialize the Windows media apartment: {com_result}"),
                true,
            ));
        }

        if let Err(error) = unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) } {
            unsafe { CoUninitialize() };
            return Err(mf_error(
                "Could not initialize Windows Media Foundation",
                error,
            ));
        }

        Ok(Self {
            com_initialized: true,
        })
    }
}

impl Drop for MediaFoundationRuntime {
    fn drop(&mut self) {
        let _ = unsafe { MFShutdown() };
        if self.com_initialized {
            unsafe { CoUninitialize() };
        }
    }
}

fn create_sink_writer_attributes() -> Result<IMFAttributes, IpcError> {
    let mut attributes = None;
    unsafe {
        MFCreateAttributes(&mut attributes, 1)
            .map_err(|error| mf_error("Could not configure the MP4 sink writer", error))?;
    }
    let attributes = attributes
        .ok_or_else(|| IpcError::internal("Windows did not create MP4 sink-writer attributes."))?;
    unsafe {
        attributes
            .SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)
            .map_err(|error| mf_error("Could not enable hardware video encoders", error))?;
    }
    Ok(attributes)
}

pub(crate) fn create_video_type(
    width: u32,
    height: u32,
    fps: u16,
    encoded: bool,
) -> Result<windows::Win32::Media::MediaFoundation::IMFMediaType, IpcError> {
    use windows::Win32::Media::MediaFoundation::MFCreateMediaType;

    let media_type = unsafe {
        MFCreateMediaType()
            .map_err(|error| mf_error("Could not create a Media Foundation media type", error))?
    };
    let frame_size = (u64::from(width) << 32) | u64::from(height);
    let frame_rate = (u64::from(fps) << 32) | 1;
    let pixel_aspect_ratio = (1_u64 << 32) | 1;
    unsafe {
        media_type
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(|error| mf_error("Could not set the video media type", error))?;
        media_type
            .SetGUID(
                &MF_MT_SUBTYPE,
                if encoded {
                    &MFVideoFormat_H264
                } else {
                    &MFVideoFormat_RGB32
                },
            )
            .map_err(|error| mf_error("Could not set the video pixel format", error))?;
        media_type
            .SetUINT64(&MF_MT_FRAME_SIZE, frame_size)
            .map_err(|error| mf_error("Could not set the video frame dimensions", error))?;
        media_type
            .SetUINT64(&MF_MT_FRAME_RATE, frame_rate)
            .map_err(|error| mf_error("Could not set the video frame rate", error))?;
        media_type
            .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pixel_aspect_ratio)
            .map_err(|error| mf_error("Could not set the video pixel aspect ratio", error))?;
        media_type
            .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .map_err(|error| mf_error("Could not configure progressive video", error))?;
        if encoded {
            media_type
                .SetUINT32(&MF_MT_AVG_BITRATE, bitrate_for(width, height, fps))
                .map_err(|error| mf_error("Could not configure the H.264 bitrate", error))?;
        } else {
            let stride = width
                .checked_mul(4)
                .and_then(|value| i32::try_from(value).ok())
                .ok_or_else(|| {
                    IpcError::new(
                        "frame_too_large",
                        "The selected frame width exceeds Media Foundation limits.",
                        true,
                    )
                })?;
            media_type
                .SetUINT32(&MF_MT_DEFAULT_STRIDE, stride as u32)
                .map_err(|error| mf_error("Could not configure top-down BGRA video rows", error))?;
        }
    }
    Ok(media_type)
}

pub fn output_dimensions(width: u32, height: u32, max_height: u16) -> Result<(u32, u32), IpcError> {
    if width < 2 || height < 2 || max_height == 0 {
        return Err(IpcError::new(
            "capture_dimensions_invalid",
            "Capture dimensions and maximum output height must be at least two pixels.",
            true,
        ));
    }
    let scale = (f64::from(max_height) / f64::from(height)).min(1.0);
    let output_width = ((f64::from(width) * scale).round() as u32) & !1;
    let output_height = ((f64::from(height) * scale).round() as u32) & !1;
    if output_width < 2 || output_height < 2 {
        return Err(IpcError::new(
            "capture_dimensions_invalid",
            "The selected capture is too small to encode as H.264.",
            true,
        ));
    }
    Ok((output_width, output_height))
}

fn validate_dimensions(width: u32, height: u32, fps: u16) -> Result<(), IpcError> {
    if width < 2 || height < 2 || !matches!(fps, 30 | 60) {
        return Err(IpcError::new(
            "capture_options_invalid",
            "Windows MP4 recording requires a capture at least 2 × 2 pixels and a frame rate of 30 or 60 FPS.",
            true,
        ));
    }
    frame_byte_length(width, height).map(|_| ())
}

pub(crate) fn frame_byte_length(width: u32, height: u32) -> Result<usize, IpcError> {
    let length = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| {
            IpcError::new(
                "frame_too_large",
                "The selected frame exceeds supported dimensions.",
                true,
            )
        })?;
    u32::try_from(length).map_err(|_| {
        IpcError::new(
            "frame_too_large",
            "The selected frame exceeds Media Foundation limits.",
            true,
        )
    })?;
    Ok(length)
}

fn bitrate_for(width: u32, height: u32, fps: u16) -> u32 {
    width
        .saturating_mul(height)
        .saturating_mul(u32::from(fps))
        .saturating_div(8)
        .clamp(2_000_000, 60_000_000)
}

pub(crate) fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

pub(crate) fn mf_error(context: &str, error: windows::core::Error) -> IpcError {
    if is_disk_full_hresult(error.code().0 as u32) {
        return IpcError::new(
            "output_disk_full",
            format!("{context}: {error}. Free space on the selected recording disk and try again."),
            true,
        );
    }
    IpcError::new(
        "media_foundation_error",
        format!("{context}: {error}"),
        true,
    )
}

fn is_disk_full_hresult(code: u32) -> bool {
    matches!(code, 0x8007_0070 | 0x8007_0027)
}

#[cfg(test)]
#[path = "windows_media_tests.rs"]
mod tests;
