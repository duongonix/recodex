import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, SettingsPatch } from "../../types/settings.js";

const SETTINGS_INITIALIZATION_RETRIES = 20;
const SETTINGS_INITIALIZATION_DELAY_MS = 100;

export async function getSettings(): Promise<AppSettings> {
  for (let attempt = 0; ; attempt += 1) {
    try {
      return await invoke<AppSettings>("get_settings");
    } catch (error) {
      if (!isSettingsInitializing(error) || attempt >= SETTINGS_INITIALIZATION_RETRIES) {
        throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, SETTINGS_INITIALIZATION_DELAY_MS));
    }
  }
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

function isSettingsInitializing(error: unknown): boolean {
  return error !== null
    && typeof error === "object"
    && "code" in error
    && error.code === "settings_initializing";
}
