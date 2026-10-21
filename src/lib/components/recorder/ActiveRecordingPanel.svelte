<script lang="ts">
  import CircleDot from "@lucide/svelte/icons/circle-dot";
  import Eye from "@lucide/svelte/icons/eye";
  import { Button } from "../ui/button/index.js";
  import { errorMessage } from "../../services/ipc/settings.js";
  import { focusRecordingReview, showRecordingToolbar } from "../../services/ipc/recording.js";
  import { recordingSession } from "../../stores/recording-session.svelte.js";
  import { preferences } from "../../stores/preferences.svelte.js";
  import { notifyApp } from "../../services/notifications.js";

  let pending = $state(false);

  const snapshot = $derived(recordingSession.snapshot);
  const status = $derived(snapshot?.status ?? "preparing");
  const elapsed = $derived(formatElapsed(snapshot?.recordedDurationMs ?? 0));

  async function openToolbar() {
    if (pending) return;
    pending = true;
    try {
      await showRecordingToolbar();
    } catch (cause) {
      void notifyApp("Recording toolbar unavailable", errorMessage(cause), preferences.settings?.showNotifications ?? true);
    } finally {
      pending = false;
    }
  }

  async function openReview() {
    if (pending) return;
    pending = true;
    try {
      await focusRecordingReview();
    } catch (cause) {
      void notifyApp("Recording preview unavailable", errorMessage(cause), preferences.settings?.showNotifications ?? true);
    } finally {
      pending = false;
    }
  }

  function formatElapsed(durationMs: number): string {
    const totalSeconds = Math.floor(durationMs / 1_000);
    const hours = Math.floor(totalSeconds / 3_600);
    const minutes = Math.floor((totalSeconds % 3_600) / 60);
    const seconds = totalSeconds % 60;
    return hours > 0
      ? [hours, String(minutes).padStart(2, "0"), String(seconds).padStart(2, "0")].join(":")
      : [String(minutes).padStart(2, "0"), String(seconds).padStart(2, "0")].join(":");
  }
</script>

<section class="active-session-panel" aria-labelledby="active-session-title">
  <div class="active-session-heading">
    <CircleDot aria-hidden="true" class="size-5" />
    <div>
      <h2 id="active-session-title">{status === "reviewing" ? "Recording ready" : "Recording session"}</h2>
      <p>{status === "reviewing" ? "Review before saving" : status === "paused" ? "Paused" : status === "finalizing" ? "Saving recording…" : status === "stopping" ? "Stopping…" : "Recording"} · {elapsed}</p>
    </div>
  </div>

  <Button type="button" disabled={pending} onclick={status === "reviewing" ? openReview : openToolbar}>
    {#if status === "reviewing"}
      <Eye aria-hidden="true" class="mr-2 size-4" />
      {pending ? "Opening preview…" : "Open Recording Preview"}
    {:else}
      {pending ? "Opening toolbar…" : "Show Recording Toolbar"}
    {/if}
  </Button>

</section>

<style>
  .active-session-panel {
    display: grid;
    align-content: start;
    gap: 16px;
    padding-top: 10px;
  }

  .active-session-heading {
    display: flex;
    align-items: center;
    gap: 11px;
    color: var(--accent);
  }

  h2 {
    margin: 0;
    color: var(--text);
    font-size: 15px;
    font-weight: 600;
  }

  .active-session-heading p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
  }

</style>
