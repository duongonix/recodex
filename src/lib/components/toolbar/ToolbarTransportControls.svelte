<script lang="ts">
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import Square from "@lucide/svelte/icons/square";
  import { Button } from "../ui/button/index.js";
  import type { ToolbarStatus } from "./types.js";

  let {
    status,
    compact,
    onPauseResume,
    onStop
  }: {
    status: ToolbarStatus;
    compact: boolean;
    onPauseResume?: () => void;
    onStop?: () => void;
  } = $props();

  const active = $derived(status === "recording" || status === "paused");
  const pauseLabel = $derived(status === "paused" ? "Resume" : "Pause");
</script>

<div class="transport-controls">
  {#if !compact}
    <Button
      variant="secondary"
      size="default"
      disabled={!active || !onPauseResume}
      title={onPauseResume ? pauseLabel : "Pause/resume is unavailable without the capture service."}
      aria-label={pauseLabel}
      onclick={onPauseResume}
      class="transport-button pause-button"
    >
      {#if status === "paused"}
        <Play aria-hidden="true" class="size-[18px]" />
        <span>Resume</span>
      {:else}
        <Pause aria-hidden="true" class="size-[18px]" />
        <span>Pause</span>
      {/if}
    </Button>
  {/if}

  <Button
    variant="secondary"
    size={compact ? "icon" : "default"}
    disabled={!active || !onStop}
    title={onStop ? "Stop and save recording" : "Stop is unavailable without the capture service."}
    aria-label="Stop recording"
    onclick={onStop}
    class={compact
      ? "transport-button stop-button compact-stop"
      : "transport-button stop-button"}
  >
    <Square aria-hidden="true" class="size-[17px] fill-current" />
    {#if !compact}<span>Stop</span>{/if}
  </Button>
</div>

<style>
  .transport-controls {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
  }

  :global(.transport-button) {
    height: 38px;
    border-color: var(--toolbar-border);
    border-radius: 10px;
    background: var(--toolbar-surface);
    color: var(--text);
    font-size: 13px;
  }

  :global(.transport-button:hover:not(:disabled)) {
    background: var(--toolbar-surface-hover);
  }

  :global(.pause-button) {
    min-width: 92px;
    gap: 8px;
  }

  :global(.stop-button:not(.compact-stop)) {
    min-width: 82px;
    gap: 8px;
  }

  :global(.stop-button.compact-stop) {
    width: 38px;
    padding: 0;
  }
</style>
