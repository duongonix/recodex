import { isTauri } from "@tauri-apps/api/core";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { open } from "@tauri-apps/plugin-dialog";
import { errorMessage, getSettings, updateSettings } from "../services/ipc/settings.js";
import type { AppSettings, SettingsPatch } from "../types/settings.js";

class PreferencesStore {
  settings = $state<AppSettings | null>(null);
  error = $state<string | null>(null);
  pendingWrites = $state(0);
  desktopApp = $state(false);
  autoStartAvailable = $state(false);
  autoStartError = $state<string | null>(null);
  ready = $state(false);

  private writeQueue: Promise<void> = Promise.resolve();
  private initialization: Promise<void> | null = null;

  initialize(): Promise<void> {
    if (this.initialization) return this.initialization;
    this.desktopApp = isTauri();
    if (!this.desktopApp) return Promise.resolve();

    this.initialization = this.loadSettings();
    return this.initialization;
  }

  private async loadSettings(): Promise<void> {
    try {
      const settings = await getSettings();
      this.settings = settings;
      try {
        const startupEnabled = await isEnabled();
        this.autoStartAvailable = true;
        this.autoStartError = null;
        this.settings = { ...settings, startWithSystem: startupEnabled };
        if (startupEnabled !== settings.startWithSystem) {
          await this.update({ startWithSystem: startupEnabled });
        }
      } catch (error) {
        this.autoStartAvailable = false;
        this.autoStartError = errorMessage(error);
      }
      this.ready = true;
    } catch (error) {
      this.error = errorMessage(error);
    }
  }

  update(patch: SettingsPatch): Promise<boolean> {
    if (!this.desktopApp) return Promise.resolve(false);
    this.pendingWrites += 1;

    const write = this.writeQueue.then(async () => {
      try {
        this.settings = await updateSettings(patch);
        this.error = null;
        return true;
      } catch (error) {
        this.error = errorMessage(error);
        return false;
      } finally {
        this.pendingWrites -= 1;
      }
    });
    this.writeQueue = write.then(() => undefined);
    return write;
  }

  async updateStartWithSystem(enabled: boolean): Promise<boolean> {
    const previous = this.settings?.startWithSystem ?? false;
    try {
      if (enabled) await enable();
      else await disable();
      this.autoStartAvailable = true;
      this.autoStartError = null;
    } catch (error) {
      this.autoStartAvailable = false;
      this.autoStartError = errorMessage(error);
      this.error = this.autoStartError;
      return false;
    }

    const saved = await this.update({ startWithSystem: enabled });
    if (saved) return true;

    try {
      if (previous) await enable();
      else await disable();
    } catch (error) {
      this.autoStartError = `Could not restore the previous startup setting: ${errorMessage(error)}`;
      this.error = this.autoStartError;
    }
    return false;
  }

  async chooseOutputFolder(kind: "video" | "screenshot"): Promise<void> {
    if (!this.desktopApp) return;
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) return;
      const patch = kind === "video"
        ? { videoOutputDirectory: selected }
        : { screenshotOutputDirectory: selected };
      await this.update(patch);
    } catch (error) {
      this.error = errorMessage(error);
    }
  }
}

export const preferences = new PreferencesStore();
