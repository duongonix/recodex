import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { errorMessage } from "../services/ipc/settings.js";

const UPDATE_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
const FOCUS_CHECK_THROTTLE_MS = 5 * 60 * 1000;

class AppUpdateStore {
  updateAvailable = $state(false);
  version = $state<string | null>(null);
  currentVersion = $state<string | null>(null);
  releaseDate = $state<string | null>(null);
  releaseNotes = $state("");
  checking = $state(false);
  installing = $state(false);
  downloadProgress = $state<number | null>(null);
  error = $state<string | null>(null);

  private pendingUpdate: Update | null = null;
  private monitoring = false;
  private checkRequestId = 0;
  private lastCheckedAt = 0;
  private downloadedBytes = 0;
  private expectedBytes: number | null = null;

  start(desktopApp: boolean): () => void {
    if (!desktopApp || import.meta.env.DEV || this.monitoring) return () => undefined;

    this.monitoring = true;
    void this.checkForUpdate(true);
    const interval = window.setInterval(() => void this.checkForUpdate(true), UPDATE_CHECK_INTERVAL_MS);
    const handleFocus = () => void this.checkForUpdate();
    window.addEventListener("focus", handleFocus);

    return () => {
      this.monitoring = false;
      this.checkRequestId += 1;
      this.checking = false;
      window.clearInterval(interval);
      window.removeEventListener("focus", handleFocus);
      if (this.pendingUpdate && !this.installing) {
        void this.pendingUpdate.close().catch((error: unknown) => {
          console.warn("[RecoDex][update] Could not release update metadata: " + errorMessage(error));
        });
        this.pendingUpdate = null;
      }
    };
  }

  private async checkForUpdate(force = false): Promise<void> {
    const now = Date.now();
    if (
      !this.monitoring
      || this.checking
      || this.installing
      || (!force && now - this.lastCheckedAt < FOCUS_CHECK_THROTTLE_MS)
    ) return;

    this.checking = true;
    this.lastCheckedAt = now;
    const requestId = ++this.checkRequestId;

    try {
      const nextUpdate = await check({ timeout: 15_000 });
      if (!this.monitoring || requestId !== this.checkRequestId) {
        await nextUpdate?.close();
        return;
      }

      const previousUpdate = this.pendingUpdate;
      this.pendingUpdate = nextUpdate;
      this.updateAvailable = nextUpdate !== null;
      this.version = nextUpdate?.version ?? null;
      this.currentVersion = nextUpdate?.currentVersion ?? null;
      this.releaseDate = nextUpdate?.date ?? null;
      this.releaseNotes = nextUpdate?.body ?? "";
      this.error = null;

      if (previousUpdate && previousUpdate !== nextUpdate) await previousUpdate.close();
    } catch (error) {
      if (this.monitoring && requestId === this.checkRequestId) {
        console.warn("[RecoDex][update] Could not check for updates: " + errorMessage(error));
      }
    } finally {
      if (requestId === this.checkRequestId) this.checking = false;
    }
  }

  async install(): Promise<void> {
    const update = this.pendingUpdate;
    if (!update || this.installing) return;

    this.installing = true;
    this.error = null;
    this.downloadProgress = null;
    this.downloadedBytes = 0;
    this.expectedBytes = null;

    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          this.expectedBytes = event.data.contentLength ?? null;
          this.downloadedBytes = 0;
          this.downloadProgress = null;
        } else if (event.event === "Progress") {
          this.downloadedBytes += event.data.chunkLength;
          this.downloadProgress = this.expectedBytes && this.expectedBytes > 0
            ? Math.min(99, Math.floor((this.downloadedBytes / this.expectedBytes) * 100))
            : null;
        } else {
          this.downloadProgress = 100;
        }
      });

      this.pendingUpdate = null;
      this.updateAvailable = false;
      await relaunch();
    } catch (error) {
      this.error = errorMessage(error);
      console.error("[RecoDex][update] Update installation failed: " + this.error);
      this.installing = false;

      if (this.pendingUpdate === null) {
        this.version = null;
        void this.checkForUpdate(true);
      }
    }
  }
}

export const appUpdate = new AppUpdateStore();
