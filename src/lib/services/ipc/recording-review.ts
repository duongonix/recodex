import { invoke } from "@tauri-apps/api/core";
import type { EditorSettings } from "../../../features/video-editor/types.js";

export type RecordingReviewInfo = {
  reviewId: string;
  videoPath: string;
  durationMs: number;
  fps: number;
};

export type RecordingTimelineAssets = {
  audioWaveform: number[];
  videoThumbnails: string[];
};

export function getPendingRecordingReview(): Promise<RecordingReviewInfo | null> {
  return invoke<RecordingReviewInfo | null>("get_pending_recording_review");
}

export function getRecordingReviewAssets(reviewId: string, binCount = 240, thumbnailCount = 24): Promise<RecordingTimelineAssets> {
  return invoke<RecordingTimelineAssets>("get_recording_review_assets", { reviewId, binCount, thumbnailCount });
}

export function saveRecordingReview(reviewId: string): Promise<string> {
  return invoke<string>("save_recording_review", { reviewId });
}

export function trimRecordingReview(reviewId: string, startMs: number, endMs: number): Promise<string> {
  return invoke<string>("trim_recording_review", { reviewId, startMs, endMs });
}

export function splitRecordingReview(
  reviewId: string,
  startMs: number,
  splitMs: number,
  endMs: number
): Promise<string[]> {
  return invoke<string[]>("split_recording_review", { reviewId, startMs, splitMs, endMs });
}

export function exportRecordingTimeline(
  reviewId: string,
  segments: Array<{ startMs: number; endMs: number }>,
  settings: EditorSettings,
  outputPath: string
): Promise<string> {
  return invoke<string>("export_recording_timeline", { reviewId, segments, settings, outputPath });
}

export function cancelRecordingTimelineExport(): Promise<void> {
  return invoke<void>("cancel_recording_timeline_export");
}

export function discardRecordingReview(reviewId: string): Promise<void> {
  return invoke<void>("discard_recording_review", { reviewId });
}
