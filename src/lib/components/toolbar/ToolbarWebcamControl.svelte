<script lang="ts">
  import Camera from "@lucide/svelte/icons/camera";
  import CameraOff from "@lucide/svelte/icons/camera-off";
  import { Button } from "../ui/button/index.js";
  import type { ToolbarStatus } from "./types.js";

  let {
    status,
    enabled = false,
    available = false,
    onToggle
  }: {
    status: ToolbarStatus;
    enabled?: boolean;
    available?: boolean;
    onToggle?: () => void;
  } = $props();

  const active = $derived(status === "recording" || status === "paused");
</script>

<Button
  variant="secondary"
  size="icon"
  disabled={!active || !available || !onToggle}
  aria-label={!available ? "Webcam unavailable" : enabled ? "Turn webcam off" : "Turn webcam on"}
  aria-pressed={enabled}
  title={!available ? "Select a camera in Webcam Overlay settings before recording." : enabled ? "Turn webcam off" : "Turn webcam on"}
  onclick={onToggle}
  class={`toolbar-webcam-button ${enabled ? "toolbar-webcam-active" : ""}`}
>
  {#if enabled}
    <Camera aria-hidden="true" class="size-[19px]" />
  {:else}
    <CameraOff aria-hidden="true" class="size-[19px]" />
  {/if}
</Button>

<style>
  :global(.toolbar-webcam-button) {
    width: 38px;
    height: 38px;
    flex: none;
    border: 1px solid var(--toolbar-border);
    border-radius: 10px;
    background: var(--toolbar-surface);
    color: var(--muted);
    transition: background 160ms ease, border-color 160ms ease, color 160ms ease;
  }

  :global(.toolbar-webcam-button:hover:not(:disabled)) {
    background: var(--toolbar-surface-hover);
    color: var(--text);
  }

  :global(.toolbar-webcam-button.toolbar-webcam-active) {
    border-color: rgb(255 48 56 / 72%);
    background: rgb(255 48 56 / 18%);
    color: #ff6269;
  }
</style>
