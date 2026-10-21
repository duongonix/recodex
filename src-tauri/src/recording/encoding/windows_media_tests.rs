use std::io::Read;

use super::*;

#[test]
fn sink_writer_attributes_enable_hardware_transforms_when_available() {
    let _runtime = MediaFoundationRuntime::initialize().unwrap();
    let attributes = create_sink_writer_attributes().unwrap();
    let enabled = unsafe {
        attributes
            .GetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS)
            .unwrap()
    };

    assert_eq!(enabled, 1);
}

#[test]
fn top_down_bgra_input_uses_positive_media_foundation_stride() {
    let _runtime = MediaFoundationRuntime::initialize().unwrap();
    let media_type = create_video_type(320, 180, 30, false).unwrap();
    let stride = unsafe { media_type.GetUINT32(&MF_MT_DEFAULT_STRIDE).unwrap() as i32 };

    assert_eq!(stride, 320 * 4);
}

#[test]
fn aac_output_uses_a_192_kbps_stereo_bitrate() {
    let _runtime = MediaFoundationRuntime::initialize().unwrap();
    let media_type = create_audio_type(true).unwrap();
    let bitrate = unsafe {
        media_type
            .GetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND)
            .unwrap()
    };

    assert_eq!(bitrate, 24_000);
}

#[test]
fn media_foundation_disk_full_codes_have_a_specific_recoverable_error() {
    assert!(is_disk_full_hresult(0x8007_0070));
    assert!(is_disk_full_hresult(0x8007_0027));
    assert!(!is_disk_full_hresult(0x8007_0005));
}

#[test]
fn output_size_caps_height_without_distorting_aspect_ratio() {
    assert_eq!(output_dimensions(3840, 2160, 1080).unwrap(), (1920, 1080));
    assert_eq!(output_dimensions(1366, 768, 1080).unwrap(), (1366, 768));
}

#[test]
fn media_sample_timestamps_must_increase_strictly() {
    assert!(validate_sample_timestamp(None, 0).is_ok());
    assert!(validate_sample_timestamp(Some(100), 101).is_ok());
    assert_eq!(
        validate_sample_timestamp(None, -1).unwrap_err().code,
        "frame_timestamp_invalid"
    );
    assert_eq!(
        validate_sample_timestamp(Some(100), 100).unwrap_err().code,
        "frame_timestamp_invalid"
    );
    assert_eq!(
        validate_sample_timestamp(Some(100), 99).unwrap_err().code,
        "frame_timestamp_invalid"
    );
}

#[test]
fn encodes_and_decodes_h264_mp4_with_media_foundation() {
    use crate::{capture::platform::windows::CapturedFrame, recording::output::VideoOutput};
    use windows::Win32::Media::MediaFoundation::{
        MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE,
        MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE, MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING,
        MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_SOURCE_READERF_ENDOFSTREAM, MFCreateAttributes,
        MFCreateMediaType, MFCreateSourceReaderFromURL, MFMediaType_Video, MFVideoFormat_RGB32,
        MFVideoInterlace_Progressive,
    };

    let runtime = MediaFoundationRuntime::initialize().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let width = 320;
    let height = 180;
    let mut output = VideoOutput::create(directory.path(), width, height, 720, 30).unwrap();
    let mut frame = vec![0u8; frame_byte_length(width, height).unwrap()];
    for frame_index in 0..30_i64 {
        let (pixels, _) = frame.as_chunks_mut::<4>();
        for pixel in pixels {
            *pixel = [(frame_index * 7) as u8, 32, 192, 255];
        }
        output
            .write_frame(&CapturedFrame {
                width,
                height,
                timestamp_100ns: frame_index * 333_333,
                pixels_bgra: frame.clone(),
            })
            .unwrap();
    }
    let output_path = output.finalize().unwrap();
    let verbatim_path = std::path::PathBuf::from(format!(r"\\?\{}", output_path.display()));
    drop(
        crate::recording::encoding::windows_media_source::open_reader(&verbatim_path)
            .expect("native reader opens a recording from a verbatim Windows path"),
    );
    assert_eq!(
        output_path
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("mp4")
    );

    let mut file = std::fs::File::open(&output_path).unwrap();
    let mut header = [0u8; 12];
    file.read_exact(&mut header).unwrap();
    assert_eq!(&header[4..8], b"ftyp");
    assert!(file.metadata().unwrap().len() > 1_000);

    let wide_path = wide_null(output_path.as_os_str());
    let mut reader_attributes = None;
    unsafe { MFCreateAttributes(&mut reader_attributes, 1).unwrap() };
    let reader_attributes =
        reader_attributes.expect("Media Foundation creates source-reader attributes");
    unsafe {
        reader_attributes
            .SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1)
            .unwrap();
    }
    let reader = unsafe {
        MFCreateSourceReaderFromURL(PCWSTR(wide_path.as_ptr()), Some(&reader_attributes)).unwrap()
    };
    let decoded_type = unsafe { MFCreateMediaType().unwrap() };
    unsafe {
        decoded_type
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .unwrap();
        decoded_type
            .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)
            .unwrap();
        decoded_type
            .SetUINT64(
                &MF_MT_FRAME_SIZE,
                (u64::from(width) << 32) | u64::from(height),
            )
            .unwrap();
        decoded_type
            .SetUINT64(&MF_MT_FRAME_RATE, (30_u64 << 32) | 1)
            .unwrap();
        decoded_type
            .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1_u64 << 32) | 1)
            .unwrap();
        decoded_type
            .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .unwrap();
        reader
            .SetCurrentMediaType(
                MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                None::<*const u32>,
                &decoded_type,
            )
            .unwrap();
    }

    let mut decoded_sample_count = 0;
    for _ in 0..5 {
        let mut stream_index = 0;
        let mut flags = 0;
        let mut timestamp = 0;
        let mut sample = None;
        unsafe {
            reader
                .ReadSample(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    0,
                    Some(&mut stream_index),
                    Some(&mut flags),
                    Some(&mut timestamp),
                    Some(&mut sample),
                )
                .unwrap();
        }
        if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
            break;
        }
        if sample.is_some() {
            decoded_sample_count += 1;
        }
    }
    drop(reader);
    drop(runtime);
    assert!(
        decoded_sample_count > 0,
        "Media Foundation should decode at least one H.264 sample"
    );
}

#[test]
fn encodes_h264_with_mixed_microphone_and_system_aac() {
    use crate::{
        audio::{AudioMixer, AudioSample},
        capture::platform::windows::CapturedFrame,
        recording::{
            AudioSource,
            editor::ExportSegment,
            encoding::{
                editor_render::EditorRenderSettings, windows_media_reader::transcode_timeline,
            },
            output::VideoOutput,
        },
    };
    use std::sync::atomic::AtomicBool;
    use windows::Win32::Media::MediaFoundation::{
        MF_MT_MAJOR_TYPE, MF_SOURCE_READERF_ENDOFSTREAM, MFCreateSourceReaderFromURL,
        MFMediaType_Audio,
    };

    let directory = tempfile::tempdir().unwrap();
    let width = 320;
    let height = 192;
    let mut output =
        VideoOutput::create_mixed_audio(directory.path(), width, height, 720, 30, true).unwrap();
    let frame = vec![0_u8; frame_byte_length(width, height).unwrap()];
    for frame_index in 0..30_i64 {
        output
            .write_frame(&CapturedFrame {
                width,
                height,
                timestamp_100ns: frame_index * 333_333,
                pixels_bgra: frame.clone(),
            })
            .unwrap();
    }

    let mut mixer = AudioMixer::default();
    for sample_index in 0..10_i64 {
        let sample_time_100ns = sample_index * 100_000;
        for (source, value) in [
            (AudioSource::Microphone, 12_000_i16),
            (AudioSource::System, 8_000_i16),
        ] {
            let mut pcm_s16le = Vec::with_capacity(480 * 4);
            for _ in 0..480 {
                pcm_s16le.extend_from_slice(&value.to_le_bytes());
                pcm_s16le.extend_from_slice(&value.to_le_bytes());
            }
            mixer
                .push(
                    AudioSample {
                        source,
                        timestamp_100ns: sample_time_100ns,
                        discontinuity: false,
                        frame_count: 480,
                        pcm_s16le,
                    },
                    sample_time_100ns,
                )
                .unwrap();
        }
    }
    for sample in mixer.flush_all() {
        output.write_mixed_audio(&sample).unwrap();
    }
    let output_path = output.finalize().unwrap();

    let runtime = MediaFoundationRuntime::initialize().unwrap();
    let wide_path = wide_null(output_path.as_os_str());
    let discovery_reader =
        unsafe { MFCreateSourceReaderFromURL(PCWSTR(wide_path.as_ptr()), None).unwrap() };
    let audio_streams = (0..32_u32)
        .filter_map(|stream| {
            let media_type = unsafe { discovery_reader.GetNativeMediaType(stream, 0).ok()? };
            let major_type = unsafe { media_type.GetGUID(&MF_MT_MAJOR_TYPE).ok()? };
            (major_type == MFMediaType_Audio).then_some(stream)
        })
        .collect::<Vec<_>>();
    drop(discovery_reader);
    assert_eq!(
        audio_streams.len(),
        1,
        "the MP4 should expose one mixed AAC stream"
    );
    let mut decoded_audio_samples = vec![0_u32; audio_streams.len()];
    for (track_number, stream) in audio_streams.into_iter().enumerate() {
        let reader =
            unsafe { MFCreateSourceReaderFromURL(PCWSTR(wide_path.as_ptr()), None).unwrap() };
        unsafe { reader.SetStreamSelection(stream, true).unwrap() };
        let media_type = unsafe { reader.GetNativeMediaType(stream, 0).unwrap() };
        assert_eq!(
            unsafe { media_type.GetGUID(&MF_MT_MAJOR_TYPE).unwrap() },
            MFMediaType_Audio
        );
        for _ in 0..20 {
            let mut actual_stream = 0;
            let mut flags = 0;
            let mut timestamp = 0;
            let mut sample = None;
            unsafe {
                reader
                    .ReadSample(
                        stream,
                        0,
                        Some(&mut actual_stream),
                        Some(&mut flags),
                        Some(&mut timestamp),
                        Some(&mut sample),
                    )
                    .unwrap();
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                break;
            }
            if sample.is_some() {
                decoded_audio_samples[track_number] += 1;
            }
        }
        drop(reader);
    }
    assert!(
        decoded_audio_samples.iter().all(|count| *count > 0),
        "The mixed AAC stream should be readable from MP4"
    );

    let edited_path = directory.path().join("edited.mp4");
    let cancelled = AtomicBool::new(false);
    transcode_timeline(
        &output_path,
        &edited_path,
        &[ExportSegment {
            start_ms: 0,
            end_ms: 100,
        }],
        &EditorRenderSettings::default(),
        &cancelled,
        &mut |_| {},
    )
    .unwrap();
    let edited_wide_path = wide_null(edited_path.as_os_str());
    let edited_reader =
        unsafe { MFCreateSourceReaderFromURL(PCWSTR(edited_wide_path.as_ptr()), None).unwrap() };
    let edited_audio_streams = (0..32_u32)
        .filter_map(|stream| {
            let media_type = unsafe { edited_reader.GetNativeMediaType(stream, 0).ok()? };
            let major_type = unsafe { media_type.GetGUID(&MF_MT_MAJOR_TYPE).ok()? };
            (major_type == MFMediaType_Audio).then_some(stream)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        edited_audio_streams.len(),
        1,
        "timeline export should preserve the mixed audio track"
    );
    drop(edited_reader);
    for stream in edited_audio_streams {
        let reader = unsafe {
            MFCreateSourceReaderFromURL(PCWSTR(edited_wide_path.as_ptr()), None).unwrap()
        };
        unsafe { reader.SetStreamSelection(stream, true).unwrap() };
        let mut decoded = false;
        for _ in 0..10 {
            let mut actual_stream = 0;
            let mut flags = 0;
            let mut timestamp = 0;
            let mut sample = None;
            unsafe {
                reader
                    .ReadSample(
                        stream,
                        0,
                        Some(&mut actual_stream),
                        Some(&mut flags),
                        Some(&mut timestamp),
                        Some(&mut sample),
                    )
                    .unwrap();
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                break;
            }
            if sample.is_some() {
                decoded = true;
                break;
            }
        }
        assert!(
            decoded,
            "timeline export should retain audio samples on every track"
        );
    }
    drop(runtime);
}
