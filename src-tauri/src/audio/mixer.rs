use std::collections::VecDeque;

use crate::{audio::AudioSample, errors::IpcError, recording::AudioSource};

const SAMPLE_RATE: i64 = 48_000;
const HNS_PER_SECOND: i64 = 10_000_000;
const CHANNELS: usize = 2;
const BYTES_PER_FRAME: usize = CHANNELS * 2;
const MIX_BLOCK_FRAMES: usize = 480;
const MAX_BUFFER_LEAD_FRAMES: i64 = SAMPLE_RATE * 2;
const MIXER_LATENCY_FRAMES: i64 = SAMPLE_RATE * 15 / 100; // 150 ms

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioMixSample {
    pub timestamp_100ns: i64,
    pub frame_count: u32,
    pub pcm_s16le: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Default)]
struct SourceFrame {
    samples: [i16; CHANNELS],
}

#[derive(Default)]
struct SourceLane {
    first_frame: Option<i64>,
    frames: VecDeque<SourceFrame>,
}

impl SourceLane {
    fn insert(&mut self, start_frame: i64, frames: impl IntoIterator<Item = SourceFrame>) {
        let mut incoming: Vec<SourceFrame> = frames.into_iter().collect();
        if incoming.is_empty() {
            return;
        }

        let mut start_frame = start_frame;
        if let Some(first_frame) = self.first_frame {
            if start_frame < first_frame {
                let skipped = usize::try_from(first_frame - start_frame)
                    .unwrap_or(usize::MAX)
                    .min(incoming.len());
                incoming.drain(..skipped);
                start_frame = first_frame;
                if incoming.is_empty() {
                    return;
                }
            }
        } else {
            self.first_frame = Some(start_frame);
        }

        let first_frame = self.first_frame.unwrap_or(start_frame);
        let end_frame = first_frame.saturating_add(self.frames.len() as i64);
        if start_frame > end_frame {
            let gap = usize::try_from(start_frame - end_frame).unwrap_or(usize::MAX);
            if gap > MAX_BUFFER_LEAD_FRAMES as usize {
                self.frames.clear();
                self.first_frame = Some(start_frame);
            } else {
                self.frames
                    .resize(self.frames.len() + gap, SourceFrame::default());
            }
        }

        let first_frame = self.first_frame.unwrap_or(start_frame);
        let offset = usize::try_from(start_frame.saturating_sub(first_frame)).unwrap_or(usize::MAX);
        for (index, frame) in incoming.into_iter().enumerate() {
            let position = offset.saturating_add(index);
            if position < self.frames.len() {
                self.frames[position] = frame;
            } else if position == self.frames.len() {
                self.frames.push_back(frame);
            } else {
                break;
            }
        }
    }

    fn clear(&mut self) {
        self.first_frame = None;
        self.frames.clear();
    }

    fn sample_at(&self, frame: i64) -> SourceFrame {
        let Some(first_frame) = self.first_frame else {
            return SourceFrame::default();
        };
        let offset = frame.saturating_sub(first_frame);
        usize::try_from(offset)
            .ok()
            .and_then(|offset| self.frames.get(offset).copied())
            .unwrap_or_default()
    }

    fn end_frame(&self) -> Option<i64> {
        self.first_frame
            .map(|first| first.saturating_add(self.frames.len() as i64))
    }

    fn discard_before(&mut self, frame: i64) {
        let Some(first_frame) = self.first_frame else {
            return;
        };
        // Keep future audio queued while the mixer emits initial timeline silence.
        if frame <= first_frame {
            return;
        }
        let count = usize::try_from(frame.saturating_sub(first_frame))
            .unwrap_or(usize::MAX)
            .min(self.frames.len());
        self.frames.drain(..count);
        if count > 0 {
            self.first_frame = Some(first_frame.saturating_add(count as i64));
        }
        if self.frames.is_empty() {
            self.first_frame = None;
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AudioMixerDiagnostics {
    pub microphone_enabled: bool,
    pub system_enabled: bool,
    pub microphone_received_packets: u64,
    pub system_received_packets: u64,
    pub microphone_accepted_packets: u64,
    pub system_accepted_packets: u64,
    pub negative_timestamp_packets: u64,
    pub stale_packets: u64,
    pub disabled_packets: u64,
    pub microphone_inserted_signal_frames: u64,
    pub system_inserted_signal_frames: u64,
    pub mixed_output_signal_frames: u64,
    pub packet_boundary_adjustments: u64,
}

/// Aligns independent 48 kHz stereo inputs and combines them into one clipped-safe PCM stream.
pub struct AudioMixer {
    microphone: SourceLane,
    system: SourceLane,
    next_frame: i64,
    audio_track: bool,
    microphone_enabled: bool,
    system_enabled: bool,
    diagnostics: AudioMixerDiagnostics,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new(true, true)
    }
}

impl AudioMixer {
    pub fn new(microphone_enabled: bool, system_enabled: bool) -> Self {
        Self {
            microphone: SourceLane::default(),
            system: SourceLane::default(),
            next_frame: 0,
            audio_track: microphone_enabled || system_enabled,
            microphone_enabled,
            system_enabled,
            diagnostics: AudioMixerDiagnostics {
                microphone_enabled,
                system_enabled,
                ..AudioMixerDiagnostics::default()
            },
        }
    }

    pub fn diagnostics(&self) -> &AudioMixerDiagnostics {
        &self.diagnostics
    }

    pub fn set_source_enabled(&mut self, source: AudioSource, enabled: bool) {
        let was_enabled = self.source_enabled(source);
        match source {
            AudioSource::Microphone => self.microphone_enabled = enabled,
            AudioSource::System => self.system_enabled = enabled,
        }
        match source {
            AudioSource::Microphone => self.diagnostics.microphone_enabled = enabled,
            AudioSource::System => self.diagnostics.system_enabled = enabled,
        }
        if was_enabled && !enabled {
            self.lane_mut(source).clear();
        }
    }

    fn source_enabled(&self, source: AudioSource) -> bool {
        match source {
            AudioSource::Microphone => self.microphone_enabled,
            AudioSource::System => self.system_enabled,
        }
    }
    pub fn push(&mut self, sample: AudioSample, timestamp_100ns: i64) -> Result<(), IpcError> {
        if timestamp_100ns < 0 {
            self.diagnostics.negative_timestamp_packets = self
                .diagnostics
                .negative_timestamp_packets
                .saturating_add(1);
        }
        let expected_bytes = usize::try_from(sample.frame_count)
            .ok()
            .and_then(|frames| frames.checked_mul(BYTES_PER_FRAME))
            .ok_or_else(invalid_audio_packet)?;
        if sample.frame_count == 0 || sample.pcm_s16le.len() != expected_bytes {
            return Err(invalid_audio_packet());
        }

        match sample.source {
            AudioSource::Microphone => {
                self.diagnostics.microphone_received_packets = self
                    .diagnostics
                    .microphone_received_packets
                    .saturating_add(1);
            }
            AudioSource::System => {
                self.diagnostics.system_received_packets =
                    self.diagnostics.system_received_packets.saturating_add(1);
            }
        }
        if !self.source_enabled(sample.source) {
            self.diagnostics.disabled_packets = self.diagnostics.disabled_packets.saturating_add(1);
            return Ok(());
        }
        let timestamp_frame = timestamp_to_frame(timestamp_100ns)?;
        if timestamp_frame > self.next_frame.saturating_add(MAX_BUFFER_LEAD_FRAMES) {
            return Err(IpcError::new(
                "audio_timestamp_invalid",
                "An audio packet timestamp is more than two seconds ahead of the recording timeline.",
                true,
            ));
        }

        // QPC timestamps have 100 ns precision while 48 kHz sample boundaries are 20.83 µs.
        // Quantizing each packet independently can shift an otherwise contiguous boundary by one
        // sample; snapping only that tiny jitter avoids inserting/removing isolated samples.
        let lane_end = match sample.source {
            AudioSource::Microphone => self.microphone.end_frame(),
            AudioSource::System => self.system.end_frame(),
        };
        let expected_frame = lane_end.unwrap_or(self.next_frame);
        let boundary_delta = timestamp_frame.saturating_sub(expected_frame);
        let start_frame = if !sample.discontinuity && boundary_delta.unsigned_abs() <= 1 {
            if boundary_delta != 0 {
                self.diagnostics.packet_boundary_adjustments = self
                    .diagnostics
                    .packet_boundary_adjustments
                    .saturating_add(1);
            }
            expected_frame
        } else {
            timestamp_frame
        };
        let packet_end = start_frame.saturating_add(i64::from(sample.frame_count));
        if packet_end <= self.next_frame {
            self.diagnostics.stale_packets = self.diagnostics.stale_packets.saturating_add(1);
            return Ok(());
        }

        let skipped = usize::try_from(self.next_frame.saturating_sub(start_frame))
            .unwrap_or(0)
            .min(sample.frame_count as usize);
        let start_frame = start_frame.max(self.next_frame);
        let (pcm_frames, _) = sample.pcm_s16le.as_chunks::<BYTES_PER_FRAME>();
        let frames: Vec<SourceFrame> = pcm_frames
            .iter()
            .skip(skipped)
            .map(|bytes| {
                let samples = [
                    i16::from_le_bytes([bytes[0], bytes[1]]),
                    i16::from_le_bytes([bytes[2], bytes[3]]),
                ];
                SourceFrame { samples }
            })
            .collect();
        let signal_frames = frames
            .iter()
            .filter(|frame| frame.samples != [0; CHANNELS])
            .count();
        match sample.source {
            AudioSource::Microphone => {
                self.diagnostics.microphone_accepted_packets = self
                    .diagnostics
                    .microphone_accepted_packets
                    .saturating_add(1);
                self.diagnostics.microphone_inserted_signal_frames = self
                    .diagnostics
                    .microphone_inserted_signal_frames
                    .saturating_add(signal_frames as u64);
            }
            AudioSource::System => {
                self.diagnostics.system_accepted_packets =
                    self.diagnostics.system_accepted_packets.saturating_add(1);
                self.diagnostics.system_inserted_signal_frames = self
                    .diagnostics
                    .system_inserted_signal_frames
                    .saturating_add(signal_frames as u64);
            }
        }
        self.lane_mut(sample.source).insert(start_frame, frames);
        Ok(())
    }

    /// Flushes only behind the latest received source watermark, leaving time for a slower
    /// enabled source to deliver its overlapping packets without blocking a source that is active.
    pub fn flush_available(&mut self) -> Vec<AudioMixSample> {
        let watermark = [
            (self.microphone_enabled, &self.microphone),
            (self.system_enabled, &self.system),
        ]
        .into_iter()
        .filter_map(|(enabled, lane)| enabled.then(|| lane.end_frame()).flatten())
        .max();
        let Some(watermark) = watermark else {
            return Vec::new();
        };
        let safe_end = watermark.saturating_sub(MIXER_LATENCY_FRAMES);
        let complete_blocks = safe_end / MIX_BLOCK_FRAMES as i64 * MIX_BLOCK_FRAMES as i64;
        self.flush_to(complete_blocks, false)
    }

    pub fn flush_all(&mut self) -> Vec<AudioMixSample> {
        let end_frame = [
            (self.microphone_enabled, self.microphone.end_frame()),
            (self.system_enabled, self.system.end_frame()),
        ]
        .into_iter()
        .filter_map(|(enabled, end)| enabled.then_some(end).flatten())
        .max()
        .unwrap_or(self.next_frame);
        self.flush_to(end_frame, true)
    }

    fn lane_mut(&mut self, source: AudioSource) -> &mut SourceLane {
        match source {
            AudioSource::Microphone => &mut self.microphone,
            AudioSource::System => &mut self.system,
        }
    }

    fn flush_to(&mut self, end_frame: i64, allow_partial: bool) -> Vec<AudioMixSample> {
        let mut output = Vec::new();
        if !self.audio_track {
            return output;
        }
        while self.next_frame < end_frame {
            let remaining = usize::try_from(end_frame - self.next_frame).unwrap_or(usize::MAX);
            if !allow_partial && remaining < MIX_BLOCK_FRAMES {
                break;
            }
            let frame_count = remaining.min(MIX_BLOCK_FRAMES);
            let start_frame = self.next_frame;
            let mixed_frames: Vec<[i32; CHANNELS]> = (start_frame
                ..start_frame + frame_count as i64)
                .map(|frame_index| {
                    let microphone = if self.microphone_enabled {
                        self.microphone.sample_at(frame_index)
                    } else {
                        SourceFrame::default()
                    };
                    let system = if self.system_enabled {
                        self.system.sample_at(frame_index)
                    } else {
                        SourceFrame::default()
                    };
                    [
                        i32::from(microphone.samples[0]) + i32::from(system.samples[0]),
                        i32::from(microphone.samples[1]) + i32::from(system.samples[1]),
                    ]
                })
                .collect();
            let peak = mixed_frames
                .iter()
                .flat_map(|frame| frame.iter())
                .map(|sample| sample.abs())
                .max()
                .unwrap_or(0);
            let scale = if peak > i32::from(i16::MAX) {
                f64::from(i16::MAX) / f64::from(peak)
            } else {
                1.0
            };
            let mut pcm_s16le = Vec::with_capacity(frame_count * BYTES_PER_FRAME);
            for mut frame in mixed_frames {
                if scale < 1.0 {
                    for sample in &mut frame {
                        *sample = (f64::from(*sample) * scale).round() as i32;
                    }
                }
                let mixed = SourceFrame {
                    samples: [
                        frame[0].clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
                        frame[1].clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
                    ],
                };
                if mixed.samples != [0; CHANNELS] {
                    self.diagnostics.mixed_output_signal_frames = self
                        .diagnostics
                        .mixed_output_signal_frames
                        .saturating_add(1);
                }
                append_frame(&mut pcm_s16le, mixed);
            }
            output.push(AudioMixSample {
                timestamp_100ns: frame_to_timestamp(start_frame),
                frame_count: frame_count as u32,
                pcm_s16le,
            });
            self.next_frame = self.next_frame.saturating_add(frame_count as i64);
            self.microphone.discard_before(self.next_frame);
            self.system.discard_before(self.next_frame);
        }
        output
    }
}

fn append_frame(pcm: &mut Vec<u8>, frame: SourceFrame) {
    for sample in frame.samples {
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
}

fn timestamp_to_frame(timestamp_100ns: i64) -> Result<i64, IpcError> {
    let frames = i128::from(timestamp_100ns) * i128::from(SAMPLE_RATE) / i128::from(HNS_PER_SECOND);
    i64::try_from(frames).map_err(|_| {
        IpcError::new(
            "audio_timestamp_invalid",
            "The audio packet timestamp exceeds the supported recording range.",
            true,
        )
    })
}

fn frame_to_timestamp(frame: i64) -> i64 {
    (i128::from(frame) * i128::from(HNS_PER_SECOND) / i128::from(SAMPLE_RATE))
        .min(i128::from(i64::MAX)) as i64
}

fn invalid_audio_packet() -> IpcError {
    IpcError::new(
        "audio_packet_invalid",
        "The audio packet does not match the expected 48 kHz stereo PCM layout.",
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::AudioMixer;
    use crate::{audio::AudioSample, recording::AudioSource};

    fn sample(source: AudioSource, timestamp_100ns: i64, frames: u32, value: i16) -> AudioSample {
        let mut pcm_s16le = Vec::with_capacity(frames as usize * 4);
        for _ in 0..frames {
            pcm_s16le.extend_from_slice(&value.to_le_bytes());
            pcm_s16le.extend_from_slice(&value.to_le_bytes());
        }
        AudioSample {
            source,
            timestamp_100ns,
            discontinuity: false,
            frame_count: frames,
            pcm_s16le,
        }
    }

    fn first_stereo_frame(pcm: &[u8]) -> [i16; 2] {
        [
            i16::from_le_bytes([pcm[0], pcm[1]]),
            i16::from_le_bytes([pcm[2], pcm[3]]),
        ]
    }

    #[test]
    fn joins_contiguous_packets_when_qpc_rounding_shifts_one_sample() {
        let mut mixer = AudioMixer::new(false, true);
        mixer
            .push(sample(AudioSource::System, 0, 480, 10_000), 0)
            .unwrap();
        let first = mixer.flush_all();
        assert_eq!(first[0].frame_count, 480);

        // 100,209 × 48,000 / 10,000,000 rounds to frame 481.
        mixer
            .push(sample(AudioSource::System, 100_209, 480, 12_000), 100_209)
            .unwrap();
        let output = mixer.flush_all();

        assert_eq!(output.len(), 1);
        assert_eq!(output[0].frame_count, 480);
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [12_000, 12_000]);
        assert_eq!(mixer.diagnostics().packet_boundary_adjustments, 1);
    }

    #[test]
    fn preserves_gaps_larger_than_one_sample() {
        let mut mixer = AudioMixer::new(false, true);
        mixer
            .push(sample(AudioSource::System, 0, 480, 10_000), 0)
            .unwrap();
        let _ = mixer.flush_all();

        // 100,418 × 48,000 / 10,000,000 rounds to frame 482: a real two-sample gap.
        mixer
            .push(sample(AudioSource::System, 100_418, 480, 12_000), 100_418)
            .unwrap();
        let output = mixer.flush_all();

        assert_eq!(
            output.iter().map(|sample| sample.frame_count).sum::<u32>(),
            482
        );
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [0, 0]);
        assert_eq!(&output[0].pcm_s16le[8..12], &[0xE0, 0x2E, 0xE0, 0x2E]);
        assert_eq!(mixer.diagnostics().packet_boundary_adjustments, 0);
    }

    #[test]
    fn mixes_microphone_and_system_audio_into_one_track() {
        let mut mixer = AudioMixer::default();
        mixer
            .push(sample(AudioSource::Microphone, 0, 480, 12_000), 0)
            .unwrap();
        mixer
            .push(sample(AudioSource::System, 0, 480, 8_000), 0)
            .unwrap();

        let output = mixer.flush_all();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].timestamp_100ns, 0);
        assert_eq!(output[0].frame_count, 480);
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [20_000, 20_000]);
    }

    #[test]
    fn aligns_independent_inputs_before_mixing() {
        let mut mixer = AudioMixer::default();
        mixer
            .push(sample(AudioSource::Microphone, 0, 960, 10_000), 0)
            .unwrap();
        mixer
            .push(sample(AudioSource::System, 100_000, 480, 6_000), 100_000)
            .unwrap();

        let output = mixer.flush_all();
        assert_eq!(output.len(), 2);
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [10_000, 10_000]);
        assert_eq!(output[1].timestamp_100ns, 100_000);
        assert_eq!(first_stereo_frame(&output[1].pcm_s16le), [16_000, 16_000]);
    }

    #[test]
    fn peak_scales_overlapping_audio_to_prevent_clipping() {
        let mut mixer = AudioMixer::default();
        mixer
            .push(sample(AudioSource::Microphone, 0, 480, 30_000), 0)
            .unwrap();
        mixer
            .push(sample(AudioSource::System, 0, 480, 10_000), 0)
            .unwrap();

        let output = mixer.flush_all();
        assert_eq!(
            first_stereo_frame(&output[0].pcm_s16le),
            [i16::MAX, i16::MAX]
        );
    }

    #[test]
    fn clips_audio_packet_preroll_at_the_recording_origin() {
        let mut mixer = AudioMixer::new(true, false);
        mixer
            .push(
                sample(AudioSource::Microphone, -50_000, 480, 7_000),
                -50_000,
            )
            .unwrap();

        let output = mixer.flush_all();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].timestamp_100ns, 0);
        assert_eq!(output[0].frame_count, 240);
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [7_000, 7_000]);
        assert_eq!(mixer.diagnostics().negative_timestamp_packets, 1);
    }

    #[test]
    fn silent_system_packets_do_not_change_the_microphone_signal() {
        let mut mixer = AudioMixer::default();
        mixer
            .push(sample(AudioSource::Microphone, 0, 480, 12_000), 0)
            .unwrap();
        mixer
            .push(sample(AudioSource::System, 0, 480, 0), 0)
            .unwrap();

        let output = mixer.flush_all();
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [12_000, 12_000]);
    }

    #[test]
    fn waits_for_native_audio_packets_before_flushing_timeline_silence() {
        let mut mixer = AudioMixer::new(true, false);
        assert!(mixer.flush_available().is_empty());

        for packet in 0..16_i64 {
            let timestamp = packet * 100_000;
            mixer
                .push(
                    sample(AudioSource::Microphone, timestamp, 480, 9_000),
                    timestamp,
                )
                .unwrap();
        }

        let output = mixer.flush_available();
        assert_eq!(output.len(), 1);
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [9_000, 9_000]);
    }

    #[test]
    fn microphone_is_not_blocked_by_silent_loopback() {
        let mut mixer = AudioMixer::default();
        for packet in 0..16_i64 {
            let timestamp = packet * 100_000;
            mixer
                .push(
                    sample(AudioSource::Microphone, timestamp, 480, 9_000),
                    timestamp,
                )
                .unwrap();
        }

        let output = mixer.flush_available();
        assert_eq!(output.len(), 1);
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [9_000, 9_000]);
    }

    #[test]
    fn disabling_a_source_discards_its_buffered_samples() {
        let mut mixer = AudioMixer::default();
        mixer
            .push(sample(AudioSource::Microphone, 0, 480, 12_000), 0)
            .unwrap();
        mixer
            .push(sample(AudioSource::System, 0, 480, 8_000), 0)
            .unwrap();
        mixer.set_source_enabled(AudioSource::Microphone, false);

        for packet in 1..16_i64 {
            let timestamp = packet * 100_000;
            mixer
                .push(
                    sample(AudioSource::System, timestamp, 480, 8_000),
                    timestamp,
                )
                .unwrap();
        }

        let output = mixer.flush_available();
        assert_eq!(output.len(), 1);
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [8_000, 8_000]);
    }

    #[test]
    fn microphone_only_selection_discards_system_audio_packets() {
        let mut mixer = AudioMixer::new(true, false);
        mixer
            .push(sample(AudioSource::Microphone, 0, 480, 7_000), 0)
            .unwrap();
        mixer
            .push(sample(AudioSource::System, 0, 480, 12_000), 0)
            .unwrap();

        let output = mixer.flush_all();
        assert_eq!(output.len(), 1);
        assert_eq!(first_stereo_frame(&output[0].pcm_s16le), [7_000, 7_000]);
        assert_eq!(mixer.diagnostics().system_accepted_packets, 0);
        assert_eq!(mixer.diagnostics().disabled_packets, 1);
    }

    #[test]
    fn disabled_mixer_does_not_emit_an_audio_sample() {
        let mut mixer = AudioMixer::new(false, false);
        assert!(mixer.flush_available().is_empty());
        assert!(mixer.flush_all().is_empty());
    }
}
