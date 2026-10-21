export type SessionStatus =
  | "idle"
  | "preparing"
  | "recording"
  | "paused"
  | "stopping"
  | "finalizing"
  | "reviewing"
  | "completed"
  | "failed";

export type CaptureTarget =
  | { kind: "display"; displayId: string }
  | { kind: "window"; windowId: string }
  | {
      kind: "region";
      displayId: string;
      x: number;
      y: number;
      width: number;
      height: number;
    };

export type RecordingOptions = {
  target: CaptureTarget;
  fps: 30 | 60;
  maxHeight: 720 | 1080 | 2160;
  microphoneEnabled: boolean;
  microphoneDeviceId?: string;
  systemAudio: boolean;
  showMouseCursor: boolean;
  zoomOnHoldEnabled: boolean;
  showPreviewAfterRecording: boolean;
  outputDirectory: string;
};

export type AudioSource = "microphone" | "system";

export type RecordingError = {
  code: string;
  message: string;
  recoverable: boolean;
};

export type RecordingQuality = {
  captureFps: number | null;
  droppedVideoFrames: number;
  audioGaps: number;
  microphoneLevelPercent: number;
  systemAudioLevelPercent: number;
  availableDiskBytes: number | null;
};

export type PreflightStatus = "ready" | "warning" | "error" | "skipped";

export type PreflightCheck = {
  key: string;
  label: string;
  status: PreflightStatus;
  detail: string;
};

export type RecordingPreflightReport = {
  canStart: boolean;
  fps: number;
  maxHeight: number;
  outputWidth: number | null;
  outputHeight: number | null;
  workloadPixelsPerSecond: number | null;
  freeDiskBytes: number | null;
  estimatedMinutesRemaining: number | null;
  checks: PreflightCheck[];
};

export type AudioLevel = {
  peakPercent: number;
  rmsPercent: number;
};

export type AudioInputTestResult = {
  durationMs: number;
  microphone: AudioLevel | null;
  system: AudioLevel | null;
  microphonePackets: number;
  systemPackets: number;
};

export type RecordingSnapshot = {
  sessionId: string | null;
  status: SessionStatus;
  recordedDurationMs: number;
  outputPath: string | null;
  error: RecordingError | null;
  audioWarning: string | null;
  microphoneEnabled: boolean;
  systemAudioEnabled: boolean;
  microphoneAvailable: boolean;
  systemAudioAvailable: boolean;
  webcamEnabled: boolean;
  webcamAvailable: boolean;
  quality: RecordingQuality;
  revision: number;
};

export type RecordingEvent = Omit<RecordingSnapshot, "sessionId"> & {
  sessionId: string;
};

export type IpcError = {
  code: string;
  message: string;
  recoverable: boolean;
};
