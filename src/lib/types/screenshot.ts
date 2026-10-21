import type { ScreenshotFormat } from "./settings.js";
import type { CaptureTarget } from "./recording.js";

export type ScreenshotRequest = {
  target: CaptureTarget;
  format: ScreenshotFormat;
  copyToClipboard: boolean;
  saveToFile: boolean;
};

export type ScreenshotResult = {
  width: number;
  height: number;
  outputPath: string | null;
  copiedToClipboard: boolean;
  warning: string | null;
  editorSessionId: string | null;
};
