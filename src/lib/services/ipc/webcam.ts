import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { WebcamDevice, WebcamPreviewFrame } from "../../types/webcam.js";
import type { WebcamSettings } from "../../types/settings.js";

const PREVIEW_FRAME_EVENT = "webcam://preview-frame";
const PREVIEW_ERROR_EVENT = "webcam://preview-error";
const OVERLAY_SETTINGS_EVENT = "webcam://overlay-settings-changed";

export function listWebcamDevices(): Promise<WebcamDevice[]> {
  return invoke<WebcamDevice[]>("list_webcam_devices");
}

export function getWebcamOverlaySettings(): Promise<WebcamSettings> {
  return invoke<WebcamSettings>("get_webcam_overlay_settings");
}

export function startWebcamPreview(cameraId: string): Promise<void> {
  return invoke<void>("start_webcam_preview", { cameraId });
}

export function stopWebcamPreview(): Promise<void> {
  return invoke<void>("stop_webcam_preview");
}

export function getWebcamBackgroundPreview(path: string): Promise<string> {
  return invoke<string>("get_webcam_background_preview", { path });
}

export async function watchWebcamPreview(
  onFrame: (frame: WebcamPreviewFrame) => void,
  onError: (message: string) => void
): Promise<() => void> {
  const stopFrame = await listen<WebcamPreviewFrame>(PREVIEW_FRAME_EVENT, ({ payload }) => onFrame(payload));
  try {
    const stopError = await listen<string>(PREVIEW_ERROR_EVENT, ({ payload }) => onError(payload));
    return () => {
      stopFrame();
      stopError();
    };
  } catch (error) {
    stopFrame();
    throw error;
  }
}

export function watchWebcamOverlaySettings(
  onSettings: (settings: WebcamSettings) => void
): Promise<() => void> {
  return listen<WebcamSettings>(OVERLAY_SETTINGS_EVENT, ({ payload }) => onSettings(payload));
}
