<script lang="ts">
  import GripVertical from "@lucide/svelte/icons/grip-vertical";
  import Maximize from "@lucide/svelte/icons/maximize";
  import Minus from "@lucide/svelte/icons/minus";
  import X from "@lucide/svelte/icons/x";
  import { Button } from "../ui/button/index.js";

  let {
    compact,
    onStartDragging,
    onCompactToggle,
    onHide
  }: {
    compact: boolean;
    onStartDragging?: () => void;
    onCompactToggle?: () => void;
    onHide?: () => void;
  } = $props();
</script>

<div class="window-actions" class:compact>
  <Button
    variant="ghost"
    size="icon"
    disabled={!onStartDragging}
    aria-label="Drag recording toolbar to move it"
    title={onStartDragging ? "Drag toolbar to move" : "Toolbar movement is unavailable."}
    onpointerdown={(event) => {
      if (event.button === 0) onStartDragging?.();
    }}
    onclick={(event) => {
      if (event.detail === 0) onStartDragging?.();
    }}
    class="window-action drag-action"
  >
    <GripVertical aria-hidden="true" class="size-[18px]" />
  </Button>
  <Button
    variant="ghost"
    size="icon"
    disabled={!onCompactToggle}
    aria-label={compact ? "Expand recording toolbar" : "Compact recording toolbar"}
    title={onCompactToggle
      ? compact ? "Expand toolbar" : "Compact toolbar"
      : "Toolbar sizing is unavailable without window-control integration."}
    onclick={onCompactToggle}
    class="window-action"
  >
    {#if compact}
      <Maximize aria-hidden="true" class="size-[18px]" />
    {:else}
      <Minus aria-hidden="true" class="size-[18px]" />
    {/if}
  </Button>
  {#if !compact}
    <Button
      variant="ghost"
      size="icon"
      disabled={!onHide}
      aria-label="Hide toolbar without stopping recording"
      title={onHide ? "Hide toolbar; recording continues" : "Hide control is unavailable without window integration."}
      onclick={onHide}
      class="window-action close-action"
    >
      <X aria-hidden="true" class="size-[19px]" />
    </Button>
  {/if}
</div>

<style>
  .window-actions {
    display: flex;
    flex: none;
    align-items: center;
    gap: 4px;
  }

  :global(.window-action) {
    width: 34px;
    height: 36px;
    color: var(--muted);
  }

  :global(.window-action:hover:not(:disabled)) {
    background: var(--toolbar-surface-hover);
    color: var(--text);
  }

  :global(button.drag-action:not(:disabled)) {
    cursor: grab;
  }

  :global(button.drag-action:active:not(:disabled)) {
    cursor: grabbing;
  }

  .window-actions.compact {
    gap: 2px;
  }

  :global(.close-action:hover:not(:disabled)) {
    color: var(--text);
  }
</style>
