use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevel {
    pub peak_percent: u8,
    pub rms_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioInputTestResult {
    pub duration_ms: u64,
    pub microphone: Option<AudioLevel>,
    pub system: Option<AudioLevel>,
    pub microphone_packets: u64,
    pub system_packets: u64,
}

impl AudioLevel {
    pub fn max(self, other: Self) -> Self {
        Self {
            peak_percent: self.peak_percent.max(other.peak_percent),
            rms_percent: self.rms_percent.max(other.rms_percent),
        }
    }
}

pub fn measure_s16le(pcm: &[u8]) -> AudioLevel {
    let samples = pcm.as_chunks::<2>().0;
    if samples.is_empty() {
        return AudioLevel::default();
    }

    let mut peak = 0_u16;
    let mut squares = 0_u128;
    for bytes in samples {
        let value = i16::from_le_bytes(*bytes);
        let magnitude = value.unsigned_abs();
        peak = peak.max(magnitude);
        squares += u128::from(magnitude) * u128::from(magnitude);
    }

    let rms = ((squares as f64 / samples.len() as f64).sqrt()).round() as u32;
    AudioLevel {
        peak_percent: amplitude_percent(u32::from(peak)),
        rms_percent: amplitude_percent(rms),
    }
}

fn amplitude_percent(value: u32) -> u8 {
    ((value.min(32_768) * 100 + 16_384) / 32_768) as u8
}
