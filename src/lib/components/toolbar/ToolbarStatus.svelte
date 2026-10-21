<script lang="ts">
  import CircleHelp from "@lucide/svelte/icons/circle-help";
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import Pause from "@lucide/svelte/icons/pause";
  import { type ToolbarStatus } from "./types.js";
  import type { RecordingQuality } from "../../types/recording.js";

  let {
    status,
    elapsedMs,
    message,
    audioWarning = null,
    quality,
    compact = false
  }: {
    status: ToolbarStatus;
    elapsedMs: number | null;
    message?: string;
    audioWarning?: string | null;
    quality?: RecordingQuality;
    compact?: boolean;
  } = $props();

  const formattedTime = $derived(formatElapsed(elapsedMs));
  const qualityWarning = $derived(Boolean(
    quality && (
      quality.droppedVideoFrames > 0
      || quality.audioGaps > 0
      || (quality.availableDiskBytes !== null && quality.availableDiskBytes < 1_073_741_824)
    )
  ));
  const qualitySummary = $derived.by(() => {
    if (!quality) return "";
    const parts = [
      quality.captureFps === null ? "Capture FPS: checking" : `Capture FPS: ${quality.captureFps}`,
      `Dropped frames: ${quality.droppedVideoFrames}`,
      `Audio gaps: ${quality.audioGaps}`,
      `Mic level: ${quality.microphoneLevelPercent}%`,
      `System level: ${quality.systemAudioLevelPercent}%`
    ];
    if (quality.availableDiskBytes !== null) {
      parts.push(`Free disk: ${formatBytes(quality.availableDiskBytes)}`);
    }
    return parts.join(" · ");
  });
  const statusHelp = $derived([
    audioWarning ?? message ?? statusLabel(status),
    qualitySummary
  ].filter(Boolean).join(" · "));

  function formatElapsed(value: number | null): string {
    if (value === null || !Number.isFinite(value) || value < 0) {
      return "--:--:--";
    }

    const totalSeconds = Math.floor(value / 1000);
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    return [hours, minutes, seconds]
      .map((part) => String(part).padStart(2, "0"))
      .join(":");
  }

  function statusLabel(value: ToolbarStatus): string {
    switch (value) {
      case "recording":
        return "Recording status from native capture service";
      case "paused":
        return "Recording paused";
      case "finalizing":
        return "Saving recording";
      case "error":
        return "Recording failed";
      case "unavailable":
        return "No active recording session";
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
    return `${Math.floor(bytes / 1_048_576)} MB`;
  }

  const qualityLabel = $derived.by(() => {
    if (!quality) return "";
    if (quality.droppedVideoFrames > 0 || quality.audioGaps > 0) {
      return `${quality.droppedVideoFrames} drops · ${quality.audioGaps} gaps`;
    }
    if (quality.availableDiskBytes !== null && quality.availableDiskBytes < 1_073_741_824) {
      return "Low disk space";
    }
    return quality.captureFps === null ? "Checking FPS" : `${quality.captureFps} FPS`;
  });
</script>

<div
  class="status-group"
  class:paused={status === "paused"}
  class:failed={status === "error"}
  role="status"
  aria-label={statusHelp}
  title={statusHelp}
>
  {#if status === "recording"}
    {#if audioWarning || qualityWarning}
      <CircleAlert aria-hidden="true" class="size-4 warning-indicator" />
    {:else}
      <span class="record-dot" aria-hidden="true"></span>
    {/if}
    <span class="recording-meta">
      <time class="timer" aria-label={elapsedMs === null ? "Waiting for backend timer" : `Elapsed ${formattedTime}`}>
        {formattedTime}
      </time>
      {#if !compact && quality}
        <span class="quality-label" class:warning={qualityWarning}>{qualityLabel}</span>
      {/if}
    </span>
  {:else if status === "paused"}
    <Pause aria-hidden="true" class="size-5" />
    <time class="timer" aria-label={elapsedMs === null ? "Waiting for backend timer" : `Elapsed ${formattedTime}`}>
      {formattedTime}
    </time>
    <span class="state-label">Paused</span>
  {:else if status === "finalizing"}
    <LoaderCircle aria-hidden="true" class="size-5" />
    <span class="state-label">Saving…</span>
  {:else if status === "error"}
    <CircleAlert aria-hidden="true" class="size-5" />
    <span class="state-label">Capture error</span>
  {:else}
    <CircleHelp aria-hidden="true" class="size-5" />
    <span class="state-label">No session</span>
  {/if}
</div>

<style>
  .status-group {
    display: flex;
    width: 150px;
    min-width: 0;
    height: 40px;
    flex: none;
    align-items: center;
    gap: 9px;
    padding: 0 6px;
    overflow: hidden;
    color: var(--text);
  }

  .record-dot {
    width: 16px;
    height: 16px;
    flex: none;
    border: 2px solid var(--accent);
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent) 38%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
  }

  .timer {
    color: var(--text);
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .recording-meta {
    display: grid;
    min-width: 0;
    gap: 1px;
    line-height: 1.1;
  }

  .quality-label {
    overflow: hidden;
    color: var(--muted);
    font-size: 9px;
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quality-label.warning {
    color: var(--warning);
  }

  .state-label {
    overflow: hidden;
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .paused {
    color: var(--warning);
  }

  .paused .timer,
  .paused .state-label {
    color: var(--warning);
  }

  .failed,
  :global(.warning-indicator) {
    color: var(--warning);
  }

  :global(.warning-indicator) {
    flex: none;
  }
</style>
