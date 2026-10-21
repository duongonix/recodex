<script lang="ts">
  import Mic from "@lucide/svelte/icons/mic";
  import MicOff from "@lucide/svelte/icons/mic-off";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import { Button } from "../ui/button/index.js";
  import type { ToolbarStatus } from "./types.js";

  let {
    status,
    microphoneEnabled,
    systemAudioEnabled,
    microphoneAvailable = false,
    systemAudioAvailable = false,
    microphoneLevelPercent = 0,
    systemAudioLevelPercent = 0,
    onToggleMicrophone,
    onToggleSystemAudio
  }: {
    status: ToolbarStatus;
    microphoneEnabled?: boolean | null;
    systemAudioEnabled?: boolean | null;
    microphoneAvailable?: boolean;
    systemAudioAvailable?: boolean;
    microphoneLevelPercent?: number;
    systemAudioLevelPercent?: number;
    onToggleMicrophone?: () => void;
    onToggleSystemAudio?: () => void;
  } = $props();

  const active = $derived(status === "recording" || status === "paused");
</script>

<div class="audio-controls" aria-label="Recording audio controls">
  <Button
    variant="secondary"
    size="icon"
    disabled={!active || !microphoneAvailable || microphoneEnabled == null || !onToggleMicrophone}
    aria-label={!microphoneAvailable || microphoneEnabled == null
      ? "Microphone control unavailable"
      : microphoneEnabled ? "Mute microphone" : "Enable microphone"}
    aria-pressed={microphoneEnabled ?? undefined}
    title={microphoneAvailable && onToggleMicrophone ? `Toggle microphone · input ${microphoneLevelPercent}% RMS` : "Microphone was not selected when recording started."}
    onclick={onToggleMicrophone}
    class="audio-button"
  >
    {#if microphoneEnabled}
      <Mic aria-hidden="true" class="size-[19px]" />
    {:else}
      <MicOff aria-hidden="true" class="size-[19px]" />
    {/if}
    <span class="audio-level-track" aria-hidden="true">
      <span class="audio-level-fill" style:width={`${Math.min(100, microphoneLevelPercent)}%`}></span>
    </span>
  </Button>
  <Button
    variant="secondary"
    size="icon"
    disabled={!active || !systemAudioAvailable || systemAudioEnabled == null || !onToggleSystemAudio}
    aria-label={!systemAudioAvailable || systemAudioEnabled == null
      ? "System audio control unavailable"
      : systemAudioEnabled ? "Mute system audio" : "Enable system audio"}
    aria-pressed={systemAudioEnabled ?? undefined}
    title={systemAudioAvailable && onToggleSystemAudio ? `Toggle system audio · input ${systemAudioLevelPercent}% RMS` : "System audio was not selected when recording started."}
    onclick={onToggleSystemAudio}
    class="audio-button"
  >
    {#if systemAudioEnabled}
      <Volume2 aria-hidden="true" class="size-[19px]" />
    {:else}
      <VolumeX aria-hidden="true" class="size-[19px]" />
    {/if}
    <span class="audio-level-track" aria-hidden="true">
      <span class="audio-level-fill" style:width={`${Math.min(100, systemAudioLevelPercent)}%`}></span>
    </span>
  </Button>
</div>

<style>
  .audio-controls {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
  }

  :global(.audio-button) {
    position: relative;
    width: 38px;
    height: 38px;
    border: 1px solid transparent;
    border-radius: 10px;
    background: var(--toolbar-surface);
    color: var(--text);
  }

  :global(.audio-button:hover:not(:disabled)) {
    background: var(--toolbar-surface-hover);
  }

  .audio-level-track {
    position: absolute;
    right: 8px;
    bottom: 4px;
    left: 8px;
    height: 2px;
    overflow: hidden;
    border-radius: 99px;
    background: color-mix(in srgb, var(--text) 18%, transparent);
  }

  .audio-level-fill {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width 120ms linear;
  }

  @media (prefers-reduced-motion: reduce) {
    .audio-level-fill { transition: none; }
  }
</style>
