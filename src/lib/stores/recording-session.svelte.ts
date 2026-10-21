import { isTauri } from "@tauri-apps/api/core";
import { errorMessage } from "../services/ipc/settings.js";
import { watchRecordingStatus } from "../services/ipc/recording.js";
import type { RecordingSnapshot } from "../types/recording.js";

class RecordingSessionStore {
  snapshot = $state<RecordingSnapshot | null>(null);
  error = $state<string | null>(null);

  async connect(): Promise<() => void> {
    if (!isTauri()) return () => undefined;
    try {
      const stopWatching = await watchRecordingStatus((snapshot) => {
        this.snapshot = snapshot;
        this.error = null;
      });
      return stopWatching;
    } catch (error) {
      this.error = errorMessage(error);
      return () => undefined;
    }
  }
}

export const recordingSession = new RecordingSessionStore();
