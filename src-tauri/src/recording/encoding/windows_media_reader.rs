use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, codecs::jpeg::JpegEncoder};
use windows::Win32::Media::MediaFoundation::{
    IMFSourceReader, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_MAJOR_TYPE,
    MF_SOURCE_READER_FIRST_AUDIO_STREAM, MF_SOURCE_READER_FIRST_VIDEO_STREAM, MFMediaType_Audio,
};

use crate::{
    errors::IpcError,
    recording::{
        AudioSource,
        editor::ExportSegment,
        encoding::editor_render::{EditorRenderSettings, retime_audio_pcm, transform_frame_bgra},
        encoding::windows_media::{
            MediaFoundationRuntime, create_audio_type, create_video_type, frame_byte_length,
            mf_error,
        },
        encoding::windows_media_source::{SampleReader, TimedBytes, open_reader},
        output::{PendingVideoOutput, VideoOutput},
    },
};

const AUDIO_SAMPLE_RATE: i64 = 48_000;
const HNS_PER_SECOND: i64 = 10_000_000;
const MF_E_INVALIDSTREAMNUMBER: u32 = 0xC00D_36B3;

fn enumerate_audio_streams(reader: &IMFSourceReader) -> Result<Vec<u32>, IpcError> {
    let mut streams = Vec::new();
    for stream in 0..32_u32 {
        let media_type = match unsafe { reader.GetNativeMediaType(stream, 0) } {
            Ok(media_type) => media_type,
            Err(error) if error.code().0 as u32 == MF_E_INVALIDSTREAMNUMBER => break,
            Err(error) => {
                return Err(mf_error(
                    "Could not inspect the recording's media streams",
                    error,
                ));
            }
        };
        let major_type = unsafe {
            media_type
                .GetGUID(&MF_MT_MAJOR_TYPE)
                .map_err(|error| mf_error("Could not inspect a recording media type", error))?
        };
        if major_type == MFMediaType_Audio {
            streams.push(stream);
        }
    }
    Ok(streams)
}

#[derive(Clone, Copy)]
struct ExportBound {
    start: i64,
    end: i64,
    audio_start: i64,
    audio_end: i64,
    output_offset_100ns: i64,
    output_audio_offset: i64,
}

/// Decodes and re-encodes a selected segment using Media Foundation. Only one decoded frame and
/// one audio packet are held at once, so memory use stays bounded for long recordings.
pub fn transcode_segments(
    input_path: &Path,
    output_directory: &Path,
    segments: &[ExportSegment],
) -> Result<Vec<PendingVideoOutput>, IpcError> {
    transcode_ranges(
        input_path,
        output_directory,
        segments,
        false,
        &EditorRenderSettings::default(),
        None,
        &mut |_| {},
    )
}

pub fn transcode_timeline(
    input_path: &Path,
    output_path: &Path,
    segments: &[ExportSegment],
    settings: &EditorRenderSettings,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(f32),
) -> Result<std::path::PathBuf, IpcError> {
    let output_directory = output_path.parent().ok_or_else(|| {
        IpcError::new(
            "output_path_invalid",
            "Choose an MP4 file inside an existing folder.",
            true,
        )
    })?;
    let mut outputs = transcode_ranges(
        input_path,
        output_directory,
        segments,
        true,
        settings,
        Some(cancelled),
        progress,
    )?;
    let mut output = outputs
        .pop()
        .ok_or_else(|| IpcError::internal("The edited timeline export was not created."))?;
    progress(0.95);
    let final_path = output.save_as(output_path, cancelled, progress)?;
    progress(1.0);
    Ok(final_path)
}

pub fn extract_audio_waveform(
    input_path: &Path,
    duration_ms: u64,
    requested_bins: usize,
) -> Result<Vec<f32>, IpcError> {
    let bin_count = requested_bins.clamp(16, 1024);
    let total_frames = milliseconds_to_audio_frames(duration_ms)?.max(1);
    let _runtime = MediaFoundationRuntime::initialize()?;
    let reader = open_reader(input_path)?;
    let audio_stream = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;
    match unsafe { reader.GetNativeMediaType(audio_stream, 0) } {
        Ok(_) => {}
        Err(error) if error.code().0 as u32 == MF_E_INVALIDSTREAMNUMBER => return Ok(Vec::new()),
        Err(error) => {
            return Err(mf_error(
                "Could not inspect the recording's audio stream",
                error,
            ));
        }
    }
    let decoded_type = create_audio_type(false)?;
    unsafe {
        reader
            .SetCurrentMediaType(audio_stream, None::<*const u32>, &decoded_type)
            .map_err(|error| {
                mf_error(
                    "Windows could not decode the recording's audio stream",
                    error,
                )
            })?;
    }
    let audio_reader = SampleReader::new(reader, audio_stream);
    let mut peaks = vec![0.0_f32; bin_count];
    while let Some(sample) = audio_reader.next()? {
        if !sample.bytes.len().is_multiple_of(4) {
            return Err(IpcError::new(
                "recording_audio_layout_invalid",
                "Windows decoded audio that does not match 48 kHz stereo PCM.",
                true,
            ));
        }
        let first_frame = timestamp_to_audio_frames(sample.timestamp_100ns).max(0);
        let frame_count = sample.bytes.len() / 4;
        for frame in 0..frame_count {
            let absolute_frame = first_frame.saturating_add(frame as i64);
            if absolute_frame >= total_frames {
                break;
            }
            let left_offset = frame * 4;
            let left =
                i16::from_le_bytes([sample.bytes[left_offset], sample.bytes[left_offset + 1]]);
            let right =
                i16::from_le_bytes([sample.bytes[left_offset + 2], sample.bytes[left_offset + 3]]);
            let amplitude = f32::from(left.unsigned_abs().max(right.unsigned_abs()))
                / f32::from(i16::MAX as u16);
            let bin = ((absolute_frame as i128 * bin_count as i128) / i128::from(total_frames))
                .clamp(0, (bin_count - 1) as i128) as usize;
            peaks[bin] = peaks[bin].max(amplitude);
        }
    }
    Ok(peaks)
}

pub fn extract_video_thumbnails(
    input_path: &Path,
    duration_ms: u64,
    requested_count: usize,
) -> Result<Vec<String>, IpcError> {
    let count = requested_count.clamp(4, 48);
    let _runtime = MediaFoundationRuntime::initialize()?;
    let reader = open_reader(input_path)?;
    let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
    let source_type = unsafe {
        reader
            .GetNativeMediaType(stream, 0)
            .map_err(|error| mf_error("Could not inspect the recording's video stream", error))?
    };
    let frame_size = unsafe {
        source_type
            .GetUINT64(&MF_MT_FRAME_SIZE)
            .map_err(|error| mf_error("Could not read the recording's video dimensions", error))?
    };
    let source_width = (frame_size >> 32) as u32;
    let source_height = frame_size as u32;
    if source_width < 2 || source_height < 2 {
        return Err(IpcError::new(
            "recording_video_format_invalid",
            "The recording does not contain valid video dimensions.",
            true,
        ));
    }
    let frame_rate = unsafe {
        source_type
            .GetUINT64(&MF_MT_FRAME_RATE)
            .map_err(|error| mf_error("Could not read the recording's frame rate", error))?
    };
    let numerator = (frame_rate >> 32) as u32;
    let denominator = frame_rate as u32;
    let fps = if numerator == 0 || denominator == 0 {
        30
    } else {
        ((numerator + denominator / 2) / denominator).clamp(1, 120) as u16
    };
    let aspect = source_width as f64 / source_height as f64;
    let (thumbnail_width, thumbnail_height) = if aspect >= 16.0 / 9.0 {
        (160, ((160.0 / aspect).round() as u32).max(2) & !1)
    } else {
        (((90.0 * aspect).round() as u32).max(2) & !1, 90)
    };
    let decoded_type = create_video_type(thumbnail_width, thumbnail_height, fps, false)?;
    unsafe {
        reader
            .SetCurrentMediaType(stream, None::<*const u32>, &decoded_type)
            .map_err(|error| mf_error("Windows could not prepare video thumbnails", error))?;
    }
    let video_reader = SampleReader::new(reader, stream);
    let mut thumbnails = Vec::with_capacity(count);
    for index in 0..count {
        let position_ms = (u128::from(duration_ms) * index as u128 / (count - 1) as u128) as u64;
        video_reader.seek_to(milliseconds_to_100ns(position_ms)?)?;
        let Some(frame) = video_reader.next()? else {
            break;
        };
        let frame_bytes =
            normalize_decoded_bgra_frame(frame.bytes, thumbnail_width, thumbnail_height)?;
        let mut rgb = Vec::with_capacity((thumbnail_width * thumbnail_height * 3) as usize);
        let (pixels, _) = frame_bytes.as_chunks::<4>();
        for [blue, green, red, _alpha] in pixels {
            rgb.extend_from_slice(&[*red, *green, *blue]);
        }
        let image = image::RgbImage::from_raw(thumbnail_width, thumbnail_height, rgb)
            .ok_or_else(|| IpcError::internal("The decoded thumbnail pixels were invalid."))?;
        let mut encoded = Vec::new();
        JpegEncoder::new_with_quality(&mut encoded, 68)
            .encode_image(&DynamicImage::ImageRgb8(image))
            .map_err(|error| {
                IpcError::internal(format!("Could not encode a video thumbnail: {error}"))
            })?;
        thumbnails.push(STANDARD.encode(encoded));
    }
    Ok(thumbnails)
}

fn transcode_ranges(
    input_path: &Path,
    output_directory: &Path,
    segments: &[ExportSegment],
    combine_segments: bool,
    settings: &EditorRenderSettings,
    cancelled: Option<&AtomicBool>,
    progress: &mut dyn FnMut(f32),
) -> Result<Vec<PendingVideoOutput>, IpcError> {
    settings.validate()?;
    if segments.is_empty() || segments.len() > if combine_segments { 256 } else { 2 } {
        return Err(IpcError::new(
            "recording_edit_invalid",
            "The recording edit contains an unsupported number of timeline segments.",
            true,
        ));
    }
    let mut previous_end = 0;
    for segment in segments {
        if segment.start_ms >= segment.end_ms {
            return Err(IpcError::new(
                "recording_edit_range_invalid",
                "The selected recording range must have a positive duration.",
                true,
            ));
        }
        if segment.start_ms < previous_end {
            return Err(IpcError::new(
                "recording_edit_range_invalid",
                "Timeline source ranges must be ordered and must not overlap.",
                true,
            ));
        }
        previous_end = segment.end_ms;
    }

    let _runtime = MediaFoundationRuntime::initialize()?;
    let video_reader = open_reader(input_path)?;
    let video_stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
    let source_video_type = unsafe {
        video_reader
            .GetNativeMediaType(video_stream, 0)
            .map_err(|error| mf_error("Could not inspect the recording's video stream", error))?
    };
    let frame_size = unsafe {
        source_video_type
            .GetUINT64(&MF_MT_FRAME_SIZE)
            .map_err(|error| mf_error("Could not read the recording's video dimensions", error))?
    };
    let input_width = (frame_size >> 32) as u32;
    let input_height = frame_size as u32;
    if input_width == 0 || input_height == 0 {
        return Err(IpcError::new(
            "recording_video_format_invalid",
            "The recording does not contain valid video dimensions.",
            true,
        ));
    }
    let frame_rate = unsafe {
        source_video_type
            .GetUINT64(&MF_MT_FRAME_RATE)
            .map_err(|error| mf_error("Could not read the recording's frame rate", error))?
    };
    let numerator = (frame_rate >> 32) as u32;
    let denominator = frame_rate as u32;
    let fps = if numerator == 0 || denominator == 0 {
        30
    } else {
        ((numerator + denominator / 2) / denominator).clamp(1, 120) as u16
    };

    let audio_streams = enumerate_audio_streams(&video_reader)?;
    if audio_streams.len() > 2 {
        return Err(IpcError::new(
            "recording_audio_tracks_unsupported",
            "The editor can preserve up to two audio tracks in a recording.",
            true,
        ));
    }
    let decoded_video_type = create_video_type(input_width, input_height, fps, false)?;
    unsafe {
        video_reader
            .SetCurrentMediaType(video_stream, None::<*const u32>, &decoded_video_type)
            .map_err(|error| {
                mf_error(
                    "Windows could not decode the recording's video format",
                    error,
                )
            })?;
    }
    let video_reader = SampleReader::new(video_reader, video_stream);

    let audio_readers = audio_streams
        .iter()
        .map(|stream| {
            let reader = open_reader(input_path)?;
            unsafe {
                reader.SetStreamSelection(*stream, true).map_err(|error| {
                    mf_error("Windows could not select an audio track for export", error)
                })?;
            }
            let decoded_audio_type = create_audio_type(false)?;
            unsafe {
                reader
                    .SetCurrentMediaType(*stream, None::<*const u32>, &decoded_audio_type)
                    .map_err(|error| {
                        mf_error(
                            "Windows could not decode the recording's audio format",
                            error,
                        )
                    })?;
            }
            Ok(SampleReader::new(reader, *stream))
        })
        .collect::<Result<Vec<_>, IpcError>>()?;

    let (output_width, output_height) = settings.output_dimensions(input_width, input_height)?;
    let max_height = u16::try_from(output_height).map_err(|_| {
        IpcError::new(
            "recording_video_format_invalid",
            "The recording's video height is outside the supported range.",
            true,
        )
    })?;
    let output_count = if combine_segments { 1 } else { segments.len() };
    let mut outputs = (0..output_count)
        .map(|_| {
            VideoOutput::create_with_audio(
                output_directory,
                output_width,
                output_height,
                max_height,
                fps,
                !audio_streams.is_empty(),
                audio_streams.len() > 1,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut output_offset_100ns = 0_i64;
    let segment_bounds = segments
        .iter()
        .map(|segment| {
            let start = milliseconds_to_100ns(segment.start_ms)?;
            let end = milliseconds_to_100ns(segment.end_ms)?;
            let audio_start = milliseconds_to_audio_frames(segment.start_ms)?;
            let audio_end = milliseconds_to_audio_frames(segment.end_ms)?;
            let bound = ExportBound {
                start,
                end,
                audio_start,
                audio_end,
                output_offset_100ns,
                output_audio_offset: timestamp_to_audio_frames(output_offset_100ns),
            };
            let duration = end.saturating_sub(start);
            output_offset_100ns = output_offset_100ns
                .saturating_add((duration as f64 / f64::from(settings.speed)).round() as i64);
            Ok(bound)
        })
        .collect::<Result<Vec<_>, IpcError>>()?;

    let mut next_video = video_reader.next()?;
    let mut next_audio = audio_readers
        .iter()
        .map(SampleReader::next)
        .collect::<Result<Vec<_>, _>>()?;
    let export_end = segment_bounds
        .iter()
        .map(|bound| bound.end)
        .max()
        .ok_or_else(|| IpcError::internal("The recording export has no time range."))?;
    let mut last_progress = Instant::now();
    while next_video.is_some() || next_audio.iter().any(Option::is_some) {
        let next_audio_index = next_audio
            .iter()
            .enumerate()
            .filter_map(|(index, sample)| {
                sample
                    .as_ref()
                    .map(|sample| (index, sample.timestamp_100ns))
            })
            .min_by_key(|(_, timestamp)| *timestamp)
            .map(|(index, _)| index);
        let next_audio_timestamp = next_audio_index.and_then(|index| {
            next_audio[index]
                .as_ref()
                .map(|sample| sample.timestamp_100ns)
        });
        let next_timestamp = match (next_video.as_ref(), next_audio_timestamp) {
            (Some(video), Some(audio)) => video.timestamp_100ns.min(audio),
            (Some(video), None) => video.timestamp_100ns,
            (None, Some(audio)) => audio,
            (None, None) => break,
        };
        if next_timestamp >= export_end {
            break;
        }
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            for output in &mut outputs {
                output.discard_incomplete();
            }
            return Err(IpcError::new(
                "video_export_cancelled",
                "The video export was canceled. The original recording is still available.",
                true,
            ));
        }
        if last_progress.elapsed().as_millis() >= 250 {
            progress(0.95 * (next_timestamp.max(0) as f32 / export_end as f32).clamp(0.0, 1.0));
            last_progress = Instant::now();
        }
        let write_video = match (next_video.as_ref(), next_audio_timestamp) {
            (Some(video), Some(audio)) => video.timestamp_100ns <= audio,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if write_video {
            let video = next_video.take().ok_or_else(|| {
                IpcError::internal("A decoded video sample disappeared during export.")
            })?;
            let frame_bytes = normalize_decoded_bgra_frame(video.bytes, input_width, input_height)?;
            for (index, bound) in segment_bounds.iter().enumerate() {
                if video.timestamp_100ns < bound.start || video.timestamp_100ns >= bound.end {
                    continue;
                }
                let output_index = if combine_segments { 0 } else { index };
                let output_offset = if combine_segments {
                    bound.output_offset_100ns
                } else {
                    0
                };
                let timestamp = output_offset
                    + ((video.timestamp_100ns - bound.start) as f64 / f64::from(settings.speed))
                        .round() as i64;
                let pixels = transform_frame_bgra(
                    &frame_bytes,
                    input_width,
                    input_height,
                    output_width,
                    output_height,
                    settings,
                )?;
                outputs[output_index].write_frame_pixels(
                    output_width,
                    output_height,
                    &pixels,
                    timestamp,
                )?;
            }
            next_video = video_reader.next()?;
        } else {
            let audio_index = next_audio_index.ok_or_else(|| {
                IpcError::internal("A decoded audio stream disappeared during export.")
            })?;
            let audio = next_audio[audio_index].take().ok_or_else(|| {
                IpcError::internal("A decoded audio sample disappeared during export.")
            })?;
            write_audio_sample(
                &mut outputs,
                &segment_bounds,
                audio,
                if audio_index == 0 {
                    AudioSource::Microphone
                } else {
                    AudioSource::System
                },
                combine_segments,
                settings,
            )?;
            next_audio[audio_index] = audio_readers[audio_index].next()?;
        }
    }

    if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        for output in &mut outputs {
            output.discard_incomplete();
        }
        return Err(IpcError::new(
            "video_export_cancelled",
            "The video export was canceled. The original recording is still available.",
            true,
        ));
    }

    drop(audio_readers);
    drop(video_reader);
    outputs
        .into_iter()
        .map(VideoOutput::finalize_for_review)
        .collect()
}

fn write_audio_sample(
    outputs: &mut [VideoOutput],
    bounds: &[ExportBound],
    sample: TimedBytes,
    source: AudioSource,
    combine_segments: bool,
    settings: &EditorRenderSettings,
) -> Result<(), IpcError> {
    if !sample.bytes.len().is_multiple_of(4) {
        return Err(IpcError::new(
            "recording_audio_layout_invalid",
            "Windows decoded audio that does not match 48 kHz stereo PCM.",
            true,
        ));
    }
    let sample_frame_count = i64::try_from(sample.bytes.len() / 4).map_err(|_| {
        IpcError::new(
            "recording_audio_sample_invalid",
            "The decoded audio sample exceeds supported size limits.",
            true,
        )
    })?;
    let first_frame = timestamp_to_audio_frames(sample.timestamp_100ns);
    let sample_end_frame = first_frame.saturating_add(sample_frame_count);
    for (index, bound) in bounds.iter().enumerate() {
        let selected_start = first_frame.max(bound.audio_start);
        let selected_end = sample_end_frame.min(bound.audio_end);
        if selected_start >= selected_end {
            continue;
        }
        let source_start = usize::try_from(selected_start - first_frame)
            .map_err(|_| IpcError::internal("The decoded audio sample start offset is invalid."))?;
        let source_end = usize::try_from(selected_end - first_frame)
            .map_err(|_| IpcError::internal("The decoded audio sample end offset is invalid."))?;
        let relative_output_start = (source_frames_before(selected_start, bound.audio_start) as f64
            / f64::from(settings.speed))
        .round() as i64;
        let output_offset = if combine_segments {
            bound.output_audio_offset
        } else {
            0
        };
        let output_start = output_offset + relative_output_start;
        let timestamp_100ns = audio_frames_to_100ns(output_start);
        let bytes = retime_audio_pcm(
            &sample.bytes[source_start * 4..source_end * 4],
            settings.speed,
            settings.volume,
            settings.muted,
        )?;
        let frame_count = u32::try_from(bytes.len() / 4).map_err(|_| {
            IpcError::new(
                "recording_audio_sample_invalid",
                "The edited audio packet exceeds supported size limits.",
                true,
            )
        })?;
        let output_index = if combine_segments { 0 } else { index };
        outputs[output_index].write_audio_track(source, &bytes, frame_count, timestamp_100ns)?;
    }
    Ok(())
}

fn source_frames_before(selected_start: i64, segment_start: i64) -> i64 {
    selected_start.saturating_sub(segment_start)
}

fn milliseconds_to_100ns(milliseconds: u64) -> Result<i64, IpcError> {
    i64::try_from(milliseconds)
        .ok()
        .and_then(|value| value.checked_mul(10_000))
        .ok_or_else(invalid_range)
}

fn milliseconds_to_audio_frames(milliseconds: u64) -> Result<i64, IpcError> {
    i64::try_from(milliseconds)
        .ok()
        .and_then(|value| value.checked_mul(AUDIO_SAMPLE_RATE))
        .map(|frames| frames / 1_000)
        .ok_or_else(invalid_range)
}

fn timestamp_to_audio_frames(timestamp_100ns: i64) -> i64 {
    let scaled = i128::from(timestamp_100ns) * i128::from(AUDIO_SAMPLE_RATE);
    let rounded = if scaled >= 0 {
        (scaled + i128::from(HNS_PER_SECOND) / 2) / i128::from(HNS_PER_SECOND)
    } else {
        (scaled - i128::from(HNS_PER_SECOND) / 2) / i128::from(HNS_PER_SECOND)
    };
    rounded.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

fn audio_frames_to_100ns(frames: i64) -> i64 {
    (i128::from(frames) * i128::from(HNS_PER_SECOND) / i128::from(AUDIO_SAMPLE_RATE))
        .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

/// Media Foundation may return RGB32 samples whose allocation height is aligned beyond the
/// visible frame height (for example 1920 × 1088 storage for a 1920 × 1080 video). The requested
/// positive BGRA stride is width × 4, so complete trailing alignment rows can be discarded.
fn normalize_decoded_bgra_frame(
    mut bytes: Vec<u8>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, IpcError> {
    let expected = frame_byte_length(width, height)?;
    if bytes.len() == expected {
        return Ok(bytes);
    }

    let row_bytes = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .filter(|row_bytes| *row_bytes > 0)
        .ok_or_else(invalid_decoded_video_frame)?;
    let stored_rows = bytes.len() / row_bytes;
    let row_aligned = bytes.len().is_multiple_of(row_bytes);
    let aligned_height_limit = height.checked_add(63).map(|value| value & !63);
    let has_only_trailing_alignment_rows = row_aligned
        && stored_rows > height as usize
        && aligned_height_limit.is_some_and(|limit| stored_rows <= limit as usize)
        && stored_rows.is_multiple_of(16);
    if has_only_trailing_alignment_rows {
        bytes.truncate(expected);
        return Ok(bytes);
    }

    Err(IpcError::new(
        "recording_frame_layout_invalid",
        format!(
            "Windows decoded {} bytes for a {width} × {height} BGRA frame; expected {expected} bytes or complete trailing alignment rows.",
            bytes.len()
        ),
        true,
    ))
}

fn invalid_decoded_video_frame() -> IpcError {
    IpcError::new(
        "recording_frame_layout_invalid",
        "Windows returned a decoded video frame with invalid dimensions.",
        true,
    )
}

#[cfg(test)]
mod decoded_frame_tests {
    use super::normalize_decoded_bgra_frame;

    #[test]
    fn strips_media_foundation_trailing_rows_from_1080p_frame() {
        let width = 1920_u32;
        let visible_height = 1080_u32;
        let stored_height = 1088_u32;
        let mut bytes = vec![0_u8; (width * stored_height * 4) as usize];
        for row in 0..stored_height {
            let start = (row * width * 4) as usize;
            let end = start + (width * 4) as usize;
            bytes[start..end].fill(row as u8);
        }

        let normalized = normalize_decoded_bgra_frame(bytes, width, visible_height).unwrap();

        assert_eq!(normalized.len(), (width * visible_height * 4) as usize);
        assert_eq!(normalized[0], 0);
        assert_eq!(
            normalized[((visible_height - 1) * width * 4) as usize],
            1079_u32 as u8
        );
    }

    #[test]
    fn rejects_partial_rows_and_unbounded_extra_rows() {
        let width = 4_u32;
        let height = 16_u32;
        let expected = (width * height * 4) as usize;

        assert!(normalize_decoded_bgra_frame(vec![0; expected + 1], width, height).is_err());
        assert!(
            normalize_decoded_bgra_frame(vec![0; width as usize * 4 * 96], width, height).is_err()
        );
    }
}

fn invalid_range() -> IpcError {
    IpcError::new(
        "recording_edit_range_invalid",
        "The selected recording time range is outside the supported duration.",
        true,
    )
}
