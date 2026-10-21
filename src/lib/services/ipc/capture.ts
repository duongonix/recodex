import { invoke } from "@tauri-apps/api/core";
import type {
  CaptureCapabilities,
  AudioDevice,
  DisplaySource,
  PermissionInfo,
  PermissionKind,
  RegionPickerState,
  WindowSource
} from "../../types/capture.js";

export function getCaptureCapabilities(): Promise<CaptureCapabilities> {
  return invoke<CaptureCapabilities>("get_capture_capabilities");
}

export function getCapturePermissions(): Promise<PermissionInfo[]> {
  return invoke<PermissionInfo[]>("get_capture_permissions");
}

export function listAudioDevices(): Promise<AudioDevice[]> {
  return invoke<AudioDevice[]>("list_audio_devices");
}

export function requestCapturePermission(kind: PermissionKind): Promise<PermissionInfo> {
  return invoke<PermissionInfo>("request_capture_permission", { kind });
}

export function listDisplays(): Promise<DisplaySource[]> {
  return invoke<DisplaySource[]>("list_displays");
}

export function listWindows(): Promise<WindowSource[]> {
  return invoke<WindowSource[]>("list_windows");
}

export function getWindowThumbnail(windowId: string): Promise<string | null> {
  return invoke<string | null>("get_window_thumbnail", { windowId });
}

export function getSelectedWindow(): Promise<WindowSource | null> {
  return invoke<WindowSource | null>("get_selected_window");
}

export function openWindowPicker(): Promise<void> {
  return invoke<void>("open_window_picker");
}

export function closeWindowPicker(): Promise<void> {
  return invoke<void>("close_window_picker");
}

export function selectCaptureWindow(windowId: string): Promise<WindowSource> {
  return invoke<WindowSource>("select_capture_window", { windowId });
}

export function openRegionPicker(displayId: string): Promise<void> {
  return invoke<void>("open_region_picker", { displayId });
}

export function getRegionPickerState(): Promise<RegionPickerState> {
  return invoke<RegionPickerState>("get_region_picker_state");
}

export function completeRegionSelection(
  startX: number,
  startY: number,
  endX: number,
  endY: number
): Promise<void> {
  return invoke<void>("complete_region_selection", { startX, startY, endX, endY });
}

export function cancelRegionPicker(): Promise<void> {
  return invoke<void>("cancel_region_picker");
}

export function formatCaptureError(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = error.message;
    if (typeof message === "string") return message;
  }
  return String(error);
}
