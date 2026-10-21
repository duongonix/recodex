export type FeatureStatus =
  | "available"
  | "limited"
  | "unavailable"
  | "notImplemented"
  | "unknown";

export type FeatureSupport = {
  status: FeatureStatus;
  message: string;
};

export type CaptureCapabilities = {
  platform: string;
  supportedCaptureModes: string[];
  videoCodecs: string[];
  availableFps: number[];
  maxResolutionHeight: number | null;
  videoCapture: FeatureSupport;
  screenshotCapture: FeatureSupport;
  displaySources: FeatureSupport;
  windowSources: FeatureSupport;
  regionSelection: FeatureSupport;
  microphoneCapture: FeatureSupport;
  systemAudioCapture: FeatureSupport;
  captureExclusion: FeatureSupport;
  zoomFollowCursor: FeatureSupport;
};

export type PermissionKind = "screenRecording" | "microphone";
export type PermissionStatus =
  | "granted"
  | "denied"
  | "notRequired"
  | "unknown"
  | "unsupported";

export type PermissionInfo = {
  kind: PermissionKind;
  status: PermissionStatus;
  message: string;
};

export type DisplaySource = {
  displayId: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactor: number;
  primary: boolean;
};

export type WindowSource = {
  windowId: string;
  title: string;
  appName: string;
  appIcon: string | null;
  isVisible: boolean;
  isMinimized: boolean;
  isRecordable: boolean;
  lastUsedAt: number | null;
};

export type AudioDevice = {
  deviceId: string;
  name: string;
  source: "microphone" | "system";
  isDefault: boolean;
};

export type RegionPickerState = {
  display: DisplaySource;
};
