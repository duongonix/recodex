<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import type { PointerEventHandler } from "svelte/elements";
  import {
    cancelRegionPicker,
    completeRegionSelection,
    formatCaptureError,
    getRegionPickerState
  } from "../lib/services/ipc/capture.js";
  import type { RegionPickerState } from "../lib/types/capture.js";

  type DragSelection = { startX: number; startY: number; endX: number; endY: number };

  const REGION_PICKER_OPENED_EVENT = "capture://region-picker-opened";

  let picker = $state<RegionPickerState | null>(null);
  let selection = $state<DragSelection | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let completing = $state(false);

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let revision = 0;

    const applyPicker = (value: RegionPickerState) => {
      revision += 1;
      picker = value;
      selection = null;
      error = null;
      loading = false;
    };

    const loadActivePicker = () => {
      const requestRevision = revision;
      void getRegionPickerState().then((value) => {
        if (!disposed && revision === requestRevision) applyPicker(value);
      }).catch((reason: unknown) => {
        if (disposed || revision !== requestRevision || isNoActivePickerError(reason)) return;
        error = `Could not load the selected display: ${formatCaptureError(reason)}`;
      }).finally(() => {
        if (!disposed && revision === requestRevision) loading = false;
      });
    };

    void listen<RegionPickerState>(REGION_PICKER_OPENED_EVENT, ({ payload }) => {
      applyPicker(payload);
    }).then((stop) => {
      if (disposed) stop();
      else {
        unlisten = stop;
        loadActivePicker();
      }
    }).catch((reason: unknown) => {
      if (disposed) return;
      error = `Could not listen for the region picker: ${formatCaptureError(reason)}`;
      loading = false;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function isNoActivePickerError(reason: unknown): boolean {
    if (typeof reason === "object" && reason !== null && "code" in reason) {
      if (reason.code === "invalid_state") return true;
    }
    return formatCaptureError(reason).includes("No region selection is currently active.");
  }

  const selectionStyle = $derived.by(() => {
    if (!selection) return "display: none";
    const left = Math.min(selection.startX, selection.endX);
    const top = Math.min(selection.startY, selection.endY);
    const width = Math.abs(selection.endX - selection.startX);
    const height = Math.abs(selection.endY - selection.startY);
    return `left:${left}px;top:${top}px;width:${width}px;height:${height}px`;
  });

  const handlePointerDown: PointerEventHandler<HTMLElement> = (event) => {
    if (!picker || loading || completing || isToolbarTarget(event.target)) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    selection = {
      startX: event.clientX,
      startY: event.clientY,
      endX: event.clientX,
      endY: event.clientY
    };
    error = null;
  };

  const handlePointerMove: PointerEventHandler<HTMLElement> = (event) => {
    if (!selection) return;
    selection = { ...selection, endX: event.clientX, endY: event.clientY };
  };

  const handlePointerUp: PointerEventHandler<HTMLElement> = async (event) => {
    if (!selection) return;
    const finalSelection = { ...selection, endX: event.clientX, endY: event.clientY };
    selection = finalSelection;
    completing = true;
    try {
      await completeRegionSelection(
        finalSelection.startX,
        finalSelection.startY,
        finalSelection.endX,
        finalSelection.endY
      );
    } catch (reason) {
      error = formatCaptureError(reason);
      selection = null;
    } finally {
      completing = false;
    }
  };

  function isToolbarTarget(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest(".overlay-toolbar") !== null;
  }

  async function cancel() {
    completing = true;
    error = null;
    try {
      await cancelRegionPicker();
    } catch (reason) {
      error = formatCaptureError(reason);
    } finally {
      completing = false;
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape" && !completing) {
      event.preventDefault();
      void cancel();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<main
  class="selection-surface"
  aria-label="Select a screen region"
  aria-describedby="selection-instructions"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={() => (selection = null)}
  oncontextmenu={(event) => event.preventDefault()}
>
  {#if selection}
    <div class="selection-box" style={selectionStyle} aria-hidden="true"></div>
  {/if}

  <section class="overlay-toolbar" aria-live="polite">
    <div>
      <h1>{loading ? "Preparing region picker…" : picker?.display.name ?? "Region selection"}</h1>
      <p id="selection-instructions">
        {#if picker}
          {picker.display.width} × {picker.display.height} px · {Math.round(picker.display.scaleFactor * 100)}% scale · Drag to select; Esc cancels
        {:else}
          {error ?? "Choose a display from the main window to continue."}
        {/if}
      </p>
      {#if error && picker}<p class="error" role="alert">{error}</p>{/if}
    </div>
    <button type="button" disabled={completing} onclick={cancel}>Cancel</button>
  </section>
</main>

<style>
  :global(html),
  :global(body),
  :global(#svelte) {
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: hidden;
    background: transparent;
  }

  .selection-surface {
    position: fixed;
    inset: 0;
    overflow: hidden;
    background: rgb(7 9 13 / 48%);
    color: var(--text);
    cursor: crosshair;
    touch-action: none;
    user-select: none;
  }

  .overlay-toolbar {
    position: fixed;
    z-index: 2;
    top: 18px;
    left: 50%;
    display: flex;
    width: min(500px, calc(100vw - 32px));
    min-height: 54px;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 10px 12px 10px 16px;
    transform: translateX(-50%);
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg) 94%, transparent);
    box-shadow: 0 10px 40px rgb(0 0 0 / 38%);
    cursor: default;
    user-select: text;
  }

  h1 {
    margin: 0 0 3px;
    font-size: 13px;
    font-weight: 600;
  }

  p {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.4;
  }

  .error {
    margin-top: 3px;
    color: var(--warning);
  }

  button {
    min-width: 66px;
    min-height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  button:hover:not(:disabled),
  button:focus-visible {
    border-color: var(--accent);
    outline: none;
  }

  button:disabled {
    color: var(--disabled);
    cursor: wait;
  }

  .selection-box {
    position: fixed;
    border: 2px solid var(--accent);
    background: rgb(240 45 54 / 15%);
    box-shadow: 0 0 0 9999px rgb(7 9 13 / 24%);
    pointer-events: none;
  }
</style>
