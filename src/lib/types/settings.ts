export type CaptureMode = "record" | "screenshot";
export type CaptureTargetKind = "screen" | "window" | "region";
export type ScreenshotFormat = "png" | "jpg";
export type WebcamPosition = "top-left" | "top-center" | "top-right" | "bottom-left" | "bottom-center" | "bottom-right" | "custom";
export type WebcamShape = "rectangle" | "circle";
export type WebcamBackgroundMode = "none" | "blur" | "color" | "image";

export type WebcamSettings = {
  enabled: boolean;
  cameraId: string | null;
  position: WebcamPosition;
  x: number;
  y: number;
  width: number;
  shape: WebcamShape;
  cornerRadius: number;
  borderEnabled: boolean;
  borderColor: string;
  borderWidth: number;
  backgroundMode: WebcamBackgroundMode;
  backgroundColor: string;
  backgroundImagePath: string | null;
  opacity: number;
  mirror: boolean;
};

export const DEFAULT_WEBCAM_SETTINGS: WebcamSettings = {
  enabled: false,
  cameraId: null,
  position: "bottom-right",
  x: 1,
  y: 1,
  width: 180,
  shape: "rectangle",
  cornerRadius: 12,
  borderEnabled: true,
  borderColor: "#FF3038",
  borderWidth: 2,
  backgroundMode: "none",
  backgroundColor: "#17191F",
  backgroundImagePath: null,
  opacity: 100,
  mirror: false
};

export type AppSettings = {
  schemaVersion: number;
  mode: CaptureMode;
  captureTarget: CaptureTargetKind;
  fps: 30 | 60;
  maxHeight: 720 | 1080 | 2160;
  microphoneEnabled: boolean;
  systemAudioEnabled: boolean;
  microphoneDeviceId: string;
  startWithSystem: boolean;
  minimizeToTray: boolean;
  showNotifications: boolean;
  showMouseCursor: boolean;
  zoomOnHoldEnabled: boolean;
  zoomOnHoldShortcut: string;
  showRecordingPreview: boolean;
  videoOutputDirectory: string;
  screenshotFormat: ScreenshotFormat;
  screenshotCopyToClipboard: boolean;
  screenshotSaveToFile: boolean;
  screenshotOutputDirectory: string;
  showScreenshotPreview: boolean;
  webcam: WebcamSettings;
};

export type SettingsPatch = Partial<Omit<AppSettings, "schemaVersion">>;
