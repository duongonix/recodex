import { invoke } from "@tauri-apps/api/core";
import type { HoldZoomShortcutStatus, QuickAccessShortcutStatus } from "../../types/shortcuts.js";

export function getQuickAccessShortcutStatus(): Promise<QuickAccessShortcutStatus> {
  return invoke<QuickAccessShortcutStatus>("get_quick_access_shortcut_status");
}

export function getHoldZoomShortcutStatus(): Promise<HoldZoomShortcutStatus> {
  return invoke<HoldZoomShortcutStatus>("get_hold_zoom_shortcut_status");
}
