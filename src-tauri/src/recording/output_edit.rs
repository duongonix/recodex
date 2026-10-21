use std::{path::PathBuf, sync::atomic::AtomicBool};

use crate::{
    errors::IpcError,
    recording::{
        editor::ExportSegment,
        encoding::{
            editor_render::EditorRenderSettings,
            windows_media_reader::{transcode_segments, transcode_timeline},
        },
        output::PendingVideoOutput,
    },
};

impl PendingVideoOutput {
    pub fn export_timeline(
        &mut self,
        segments: &[ExportSegment],
        settings: &EditorRenderSettings,
        target_path: &std::path::Path,
        cancelled: &AtomicBool,
        progress: &mut dyn FnMut(f32),
    ) -> Result<PathBuf, IpcError> {
        let output_path = transcode_timeline(
            self.preview_path(),
            target_path,
            segments,
            settings,
            cancelled,
            progress,
        )?;
        self.discard();
        Ok(output_path)
    }

    pub fn export_trimmed(&mut self, start_ms: u64, end_ms: u64) -> Result<PathBuf, IpcError> {
        let mut exports = transcode_segments(
            self.preview_path(),
            self.output_directory(),
            &[ExportSegment { start_ms, end_ms }],
        )?;
        let mut output = exports
            .pop()
            .ok_or_else(|| IpcError::internal("The trimmed video export was not created."))?;
        let final_path = output.save()?;
        self.discard();
        Ok(final_path)
    }

    pub fn export_split(
        &mut self,
        start_ms: u64,
        split_ms: u64,
        end_ms: u64,
    ) -> Result<Vec<PathBuf>, IpcError> {
        if start_ms >= split_ms || split_ms >= end_ms {
            return Err(IpcError::new(
                "recording_edit_range_invalid",
                "The split point must be inside the selected recording range.",
                true,
            ));
        }
        let mut exports = transcode_segments(
            self.preview_path(),
            self.output_directory(),
            &[
                ExportSegment {
                    start_ms,
                    end_ms: split_ms,
                },
                ExportSegment {
                    start_ms: split_ms,
                    end_ms,
                },
            ],
        )?;
        if exports.len() != 2 {
            return Err(IpcError::internal(
                "The split export did not produce two video files.",
            ));
        }
        let mut second = exports
            .pop()
            .ok_or_else(|| IpcError::internal("The second split segment was not created."))?;
        let mut first = exports
            .pop()
            .ok_or_else(|| IpcError::internal("The first split segment was not created."))?;
        let first_path = first.save()?;
        let second_path = second.save().map_err(|mut error| {
            error.message.push_str(&format!(
                " The first split segment was saved to {}.",
                first_path.display()
            ));
            error
        })?;
        self.discard();
        Ok(vec![first_path, second_path])
    }
}
