use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingReviewInfo {
    pub review_id: String,
    pub video_path: String,
    pub duration_ms: u64,
    pub fps: u16,
}

#[cfg(windows)]
mod store {
    use std::{
        path::PathBuf,
        sync::{Mutex, atomic::AtomicBool},
    };

    use crate::{
        errors::IpcError,
        recording::{
            editor::ExportSegment, encoding::editor_render::EditorRenderSettings,
            output::PendingVideoOutput,
        },
    };

    pub struct PendingReview {
        pub review_id: String,
        pub duration_ms: u64,
        pub fps: u16,
        pub output: PendingVideoOutput,
    }

    #[derive(Default)]
    pub struct RecordingReviewStore {
        pending: Mutex<Option<PendingReview>>,
    }

    pub struct ReviewOutcome {
        pub review_id: String,
        pub duration_ms: u64,
        pub video_path: PathBuf,
        pub output_path: Option<PathBuf>,
    }

    pub struct ReviewExportOutcome {
        pub review_id: String,
        pub duration_ms: u64,
        pub video_path: PathBuf,
        pub output_paths: Vec<PathBuf>,
    }

    impl RecordingReviewStore {
        pub fn begin(
            &self,
            review_id: String,
            duration_ms: u64,
            fps: u16,
            output: PendingVideoOutput,
        ) -> Result<super::RecordingReviewInfo, IpcError> {
            let mut pending = self.pending.lock().map_err(|_| {
                IpcError::internal("Recording review state is unavailable because its lock failed.")
            })?;
            if pending.is_some() {
                return Err(IpcError::invalid_state(
                    "A recording is already waiting for review.",
                ));
            }
            let info = super::RecordingReviewInfo {
                review_id: review_id.clone(),
                video_path: output.preview_path().to_string_lossy().into_owned(),
                duration_ms,
                fps,
            };
            *pending = Some(PendingReview {
                review_id,
                duration_ms,
                fps,
                output,
            });
            Ok(info)
        }

        pub fn current(&self) -> Result<Option<super::RecordingReviewInfo>, IpcError> {
            let pending = self.pending.lock().map_err(|_| {
                IpcError::internal("Recording review state is unavailable because its lock failed.")
            })?;
            Ok(pending.as_ref().map(|review| super::RecordingReviewInfo {
                review_id: review.review_id.clone(),
                video_path: review.output.preview_path().to_string_lossy().into_owned(),
                duration_ms: review.duration_ms,
                fps: review.fps,
            }))
        }

        pub fn save(&self, review_id: &str) -> Result<ReviewOutcome, IpcError> {
            let mut pending = self.pending.lock().map_err(|_| {
                IpcError::internal("Recording review state is unavailable because its lock failed.")
            })?;
            let review = pending.as_mut().ok_or_else(|| {
                IpcError::invalid_state("There is no recording waiting for review.")
            })?;
            if review.review_id != review_id {
                return Err(IpcError::invalid_state(
                    "The review ID does not match the pending recording.",
                ));
            }
            let video_path = review.output.preview_path().to_owned();
            let output_path = review.output.save()?;
            let review = pending.take().ok_or_else(|| {
                IpcError::internal("The pending recording disappeared during save.")
            })?;
            Ok(ReviewOutcome {
                review_id: review.review_id,
                duration_ms: review.duration_ms,
                video_path,
                output_path: Some(output_path),
            })
        }

        pub fn discard(&self, review_id: &str) -> Result<ReviewOutcome, IpcError> {
            let mut pending = self.pending.lock().map_err(|_| {
                IpcError::internal("Recording review state is unavailable because its lock failed.")
            })?;
            let review = pending.as_ref().ok_or_else(|| {
                IpcError::invalid_state("There is no recording waiting for review.")
            })?;
            if review.review_id != review_id {
                return Err(IpcError::invalid_state(
                    "The review ID does not match the pending recording.",
                ));
            }
            let mut review = pending.take().ok_or_else(|| {
                IpcError::internal("The pending recording disappeared during discard.")
            })?;
            let video_path = review.output.preview_path().to_owned();
            review.output.discard();
            Ok(ReviewOutcome {
                review_id: review.review_id,
                duration_ms: review.duration_ms,
                video_path,
                output_path: None,
            })
        }

        pub fn trim(
            &self,
            review_id: &str,
            start_ms: u64,
            end_ms: u64,
        ) -> Result<ReviewExportOutcome, IpcError> {
            if start_ms >= end_ms {
                return Err(IpcError::new(
                    "recording_edit_range_invalid",
                    "The selected recording range must have a positive duration.",
                    true,
                ));
            }
            let mut pending = self.pending.lock().map_err(|_| {
                IpcError::internal("Recording review state is unavailable because its lock failed.")
            })?;
            let review = pending.as_mut().ok_or_else(|| {
                IpcError::invalid_state("There is no recording waiting for review.")
            })?;
            if review.review_id != review_id {
                return Err(IpcError::invalid_state(
                    "The review ID does not match the pending recording.",
                ));
            }
            let video_path = review.output.preview_path().to_owned();
            let output_path = review.output.export_trimmed(start_ms, end_ms)?;
            let review = pending.take().ok_or_else(|| {
                IpcError::internal("The pending recording disappeared during trim export.")
            })?;
            Ok(ReviewExportOutcome {
                review_id: review.review_id,
                duration_ms: review.duration_ms,
                video_path,
                output_paths: vec![output_path],
            })
        }

        pub fn split(
            &self,
            review_id: &str,
            start_ms: u64,
            split_ms: u64,
            end_ms: u64,
        ) -> Result<ReviewExportOutcome, IpcError> {
            if start_ms >= split_ms || split_ms >= end_ms {
                return Err(IpcError::new(
                    "recording_edit_range_invalid",
                    "The split point must be inside the selected recording range.",
                    true,
                ));
            }
            let mut pending = self.pending.lock().map_err(|_| {
                IpcError::internal("Recording review state is unavailable because its lock failed.")
            })?;
            let review = pending.as_mut().ok_or_else(|| {
                IpcError::invalid_state("There is no recording waiting for review.")
            })?;
            if review.review_id != review_id {
                return Err(IpcError::invalid_state(
                    "The review ID does not match the pending recording.",
                ));
            }
            let video_path = review.output.preview_path().to_owned();
            let output_paths = review.output.export_split(start_ms, split_ms, end_ms)?;
            let review = pending.take().ok_or_else(|| {
                IpcError::internal("The pending recording disappeared during split export.")
            })?;
            Ok(ReviewExportOutcome {
                review_id: review.review_id,
                duration_ms: review.duration_ms,
                video_path,
                output_paths,
            })
        }

        pub fn export_timeline(
            &self,
            review_id: &str,
            segments: &[ExportSegment],
            settings: &EditorRenderSettings,
            target_path: &std::path::Path,
            cancelled: &AtomicBool,
            progress: &mut dyn FnMut(f32),
        ) -> Result<ReviewExportOutcome, IpcError> {
            settings.validate()?;
            if segments.is_empty() || segments.len() > 256 {
                return Err(IpcError::new(
                    "recording_edit_invalid",
                    "The timeline must contain between one and 256 segments to export.",
                    true,
                ));
            }
            let mut pending = self.pending.lock().map_err(|_| {
                IpcError::internal("Recording review state is unavailable because its lock failed.")
            })?;
            let review = pending.as_mut().ok_or_else(|| {
                IpcError::invalid_state("There is no recording waiting for review.")
            })?;
            if review.review_id != review_id {
                return Err(IpcError::invalid_state(
                    "The review ID does not match the pending recording.",
                ));
            }
            let mut previous_end = 0;
            for segment in segments {
                if segment.start_ms >= segment.end_ms
                    || segment.end_ms > review.duration_ms
                    || segment.start_ms < previous_end
                {
                    return Err(IpcError::new(
                        "recording_edit_range_invalid",
                        "Timeline source ranges must be valid, ordered and inside the recording.",
                        true,
                    ));
                }
                previous_end = segment.end_ms;
            }
            let video_path = review.output.preview_path().to_owned();
            let output_path = review.output.export_timeline(
                segments,
                settings,
                target_path,
                cancelled,
                progress,
            )?;
            let review = pending.take().ok_or_else(|| {
                IpcError::internal("The pending recording disappeared during timeline export.")
            })?;
            Ok(ReviewExportOutcome {
                review_id: review.review_id,
                duration_ms: review.duration_ms,
                video_path,
                output_paths: vec![output_path],
            })
        }
    }
}

#[cfg(windows)]
pub use store::{RecordingReviewStore, ReviewExportOutcome, ReviewOutcome};
