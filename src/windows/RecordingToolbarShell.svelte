<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import RecordingToolbar from "../lib/components/toolbar/RecordingToolbar.svelte";
  import { errorMessage } from "../lib/services/ipc/settings.js";
  import { hideRecordingToolbar } from "../lib/services/ipc/recording.js";
  import {
    pauseRecording,
    resumeRecording,
    setRecordingAudio,
    setRecordingWebcam,
    stopRecording,
    watchRecordingStatus
  } from "../lib/services/ipc/recording.js";
  import type { RecordingQuality, RecordingSnapshot, SessionStatus } from "../lib/types/recording.js";
  import type { ToolbarStatus } from "../lib/components/toolbar/types.js";

  let status = $state<ToolbarStatus>("unavailable");
  let elapsedMs = $state<number | null>(null);
  let message = $state("No recording session is active.");
  let audioWarning = $state<string | null>(null);
  let quality = $state<RecordingQuality | undefined>(undefined);
  let windowError = $state<string | null>(null);
  let sessionId = $state<string | null>(null);
  let actionPending = $state(false);
  let windowActionPending = $state(false);
  let compact = $state(false);
  let microphoneEnabled = $state<boolean | null>(null);
  let systemAudioEnabled = $state<boolean | null>(null);
  let microphoneAvailable = $state(false);
  let systemAudioAvailable = $state(false);
  let webcamEnabled = $state(false);
  let webcamAvailable = $state(false);

  async function toggleCompact() {
    if (windowActionPending) return;
    windowActionPending = true;
    const nextCompact = !compact;
    windowError = null;
    try {
      await getCurrentWindow().setSize(
        new LogicalSize(nextCompact ? 270 : 620, nextCompact ? 50 : 58)
      );
      compact = nextCompact;
    } catch (error) {
      windowError = `Could not resize the recording toolbar: ${errorMessage(error)}`;
    } finally {
      windowActionPending = false;
    }
  }

  async function startToolbarDragging() {
    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      windowError = `Could not move the recording toolbar: ${errorMessage(error)}`;
      console.error("[RecoDex][toolbar] Could not start native window dragging:", error);
    }
  }

  async function hideToolbar() {
    if (windowActionPending) return;
    windowActionPending = true;
    windowError = null;
    try {
      await hideRecordingToolbar();
    } catch (error) {
      windowError = errorMessage(error);
    } finally {
      windowActionPending = false;
    }
  }

  async function togglePauseResume() {
    if (!sessionId || actionPending) return;
    actionPending = true;
    try {
      if (status === "paused") await resumeRecording(sessionId);
      else await pauseRecording(sessionId);
    } catch (error) {
      message = errorMessage(error);
    } finally {
      actionPending = false;
    }
  }

  async function stopCapture() {
    if (!sessionId || actionPending) return;
    actionPending = true;
    message = "Saving recording…";
    try {
      await stopRecording(sessionId);
    } catch (error) {
      message = errorMessage(error);
    } finally {
      actionPending = false;
    }
  }

  async function toggleAudio(source: "microphone" | "system") {
    if (!sessionId || actionPending) return;
    const current = source === "microphone" ? microphoneEnabled : systemAudioEnabled;
    if (current == null) return;
    actionPending = true;
    try {
      applySnapshot(await setRecordingAudio(sessionId, source, !current));
    } catch (error) {
      message = errorMessage(error);
    } finally {
      actionPending = false;
    }
  }

  async function toggleWebcam() {
    if (!sessionId || actionPending || !webcamAvailable) return;
    actionPending = true;
    try {
      applySnapshot(await setRecordingWebcam(sessionId, !webcamEnabled));
    } catch (error) {
      message = errorMessage(error);
    } finally {
      actionPending = false;
    }
  }

  function toolbarStatus(sessionStatus: SessionStatus): ToolbarStatus {
    if (sessionStatus === "recording" || sessionStatus === "paused") return sessionStatus;
    if (sessionStatus === "stopping" || sessionStatus === "finalizing") return "finalizing";
    if (sessionStatus === "failed") return "error";
    return "unavailable";
  }

  function applySnapshot(snapshot: RecordingSnapshot) {
    sessionId = snapshot.sessionId;
    status = toolbarStatus(snapshot.status);
    elapsedMs = snapshot.status === "idle" ? null : snapshot.recordedDurationMs;
    microphoneAvailable = snapshot.microphoneAvailable;
    systemAudioAvailable = snapshot.systemAudioAvailable;
    webcamEnabled = snapshot.webcamEnabled;
    webcamAvailable = snapshot.webcamAvailable;
    microphoneEnabled = snapshot.microphoneAvailable ? snapshot.microphoneEnabled : null;
    systemAudioEnabled = snapshot.systemAudioAvailable ? snapshot.systemAudioEnabled : null;
    audioWarning = snapshot.audioWarning;
    quality = snapshot.quality;
    message = snapshot.error?.message
      ?? snapshot.audioWarning
      ?? (snapshot.status === "idle" ? "No recording session is active." : `Recorder state: ${snapshot.status}.`);
  }

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void watchRecordingStatus(applySnapshot)
      .then((stopWatching) => {
        if (disposed) stopWatching();
        else unlisten = stopWatching;
      })
      .catch((error: unknown) => {
        if (!disposed) {
          status = "unavailable";
          message = `Could not read recorder state: ${errorMessage(error)}`;
        }
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<main class="toolbar-shell">
  <RecordingToolbar
    {status}
    {elapsedMs}
    message={windowError ?? message}
    {audioWarning}
    {quality}
    {compact}
    {microphoneEnabled}
    {systemAudioEnabled}
    {microphoneAvailable}
    {systemAudioAvailable}
    {webcamEnabled}
    {webcamAvailable}
    onStartDragging={startToolbarDragging}
    onPauseResume={actionPending ? undefined : togglePauseResume}
    onStop={actionPending ? undefined : stopCapture}
    onToggleMicrophone={actionPending ? undefined : () => toggleAudio("microphone")}
    onToggleSystemAudio={actionPending ? undefined : () => toggleAudio("system")}
    onToggleWebcam={actionPending ? undefined : toggleWebcam}
    onCompactToggle={actionPending || windowActionPending ? undefined : toggleCompact}
    onHide={actionPending || windowActionPending ? undefined : hideToolbar}
  />
</main>

<style>
  :global(html),
  :global(body) {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .toolbar-shell {
    display: grid;
    width: 100vw;
    height: 100vh;
    min-width: 0;
    min-height: 0;
    place-items: center;
    overflow: hidden;
    background: transparent;
  }
</style>
