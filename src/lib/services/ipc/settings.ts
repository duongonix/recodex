import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, SettingsPatch } from "../../types/settings.js";

export function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_settings");
}

export function updateSettings(patch: SettingsPatch): Promise<AppSettings> {
  return invoke<AppSettings>("update_settings", { patch });
}

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) {
    const message = error.message;
    if (typeof message === "string") return message;
  }
  return "The request could not be completed.";
}
