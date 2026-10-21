import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AudioSource,
  RecordingEvent,
  RecordingOptions,
  RecordingPreflightReport,
  RecordingSnapshot
} from "../../types/recording.js";

const STATE_CHANGED_EVENT = "recording://state-changed";
const PROGRESS_EVENT = "recording://progress";

export function getRecordingStatus(): Promise<RecordingSnapshot> {
  return invoke<RecordingSnapshot>("get_recording_status");
}

export async function watchRecordingStatus(
  onSnapshot: (snapshot: RecordingSnapshot) => void
): Promise<() => void> {
  let revision = -1;
  const applyIfCurrent = (snapshot: RecordingSnapshot) => {
    if (snapshot.revision < revision) return;
    revision = snapshot.revision;
    onSnapshot(snapshot);
  };

  const unlistenState = await listen<RecordingEvent>(STATE_CHANGED_EVENT, ({ payload }) => applyIfCurrent(payload));
  const unlistenProgress = await listen<RecordingEvent>(PROGRESS_EVENT, ({ payload }) => applyIfCurrent(payload));

  try {
    applyIfCurrent(await getRecordingStatus());
  } catch (error) {
    unlistenState();
    unlistenProgress();
    throw error;
  }

  return () => {
    unlistenState();
    unlistenProgress();
  };
}

export function startRecording(options: RecordingOptions): Promise<string> {
  return invoke<string>("start_recording", { options });
}

export function getRecordingPreflight(
  options: RecordingOptions
): Promise<RecordingPreflightReport> {
  return invoke<RecordingPreflightReport>("get_recording_preflight", { options });
}

export function showRecordingToolbar(): Promise<void> {
  return invoke<void>("show_recording_toolbar");
}

export function hideRecordingToolbar(): Promise<void> {
  return invoke<void>("hide_recording_toolbar");
}

export function pauseRecording(sessionId: string): Promise<RecordingSnapshot> {
  return invoke<RecordingSnapshot>("pause_recording", { sessionId });
}

export function resumeRecording(sessionId: string): Promise<RecordingSnapshot> {
  return invoke<RecordingSnapshot>("resume_recording", { sessionId });
}

export function stopRecording(sessionId: string): Promise<RecordingSnapshot> {
  return invoke<RecordingSnapshot>("stop_recording", { sessionId });
}

export function focusRecordingReview(): Promise<void> {
  return invoke<void>("focus_recording_review");
}

export function setRecordingAudio(
  sessionId: string,
  source: AudioSource,
  enabled: boolean
): Promise<RecordingSnapshot> {
  return invoke<RecordingSnapshot>("set_recording_audio", { sessionId, source, enabled });
}

export function setRecordingWebcam(sessionId: string, enabled: boolean): Promise<RecordingSnapshot> {
  return invoke<RecordingSnapshot>("set_recording_webcam", { sessionId, enabled });
}
