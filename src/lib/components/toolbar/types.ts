import type { RecordingQuality } from "../../types/recording.js";

export type ToolbarStatus =
  | "recording"
  | "paused"
  | "finalizing"
  | "error"
  | "unavailable";

export type RecordingToolbarProps = {
  status: ToolbarStatus;
  elapsedMs: number | null;
  compact?: boolean;
  microphoneEnabled?: boolean | null;
  systemAudioEnabled?: boolean | null;
  microphoneAvailable?: boolean;
  systemAudioAvailable?: boolean;
  webcamEnabled?: boolean;
  webcamAvailable?: boolean;
  audioWarning?: string | null;
  quality?: RecordingQuality;
  message?: string;
  onPauseResume?: () => void;
  onStop?: () => void;
  onToggleMicrophone?: () => void;
  onToggleSystemAudio?: () => void;
  onToggleWebcam?: () => void;
  onStartDragging?: () => void;
  onCompactToggle?: () => void;
  onHide?: () => void;
};
