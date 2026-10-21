<script lang="ts">
  import AppWindow from "@lucide/svelte/icons/app-window";
  import Monitor from "@lucide/svelte/icons/monitor";
  import Scan from "@lucide/svelte/icons/scan";

  type CaptureTargetChoice = "screen" | "window" | "region";

  let {
    value,
    sourceMessage,
    sourceError = false,
    onChooseScreen,
    onChooseWindow,
    onChooseRegion
  }: {
    value: CaptureTargetChoice;
    sourceMessage: string;
    sourceError?: boolean;
    onChooseScreen: () => void;
    onChooseWindow: () => void;
    onChooseRegion: () => void;
  } = $props();
</script>

<section aria-labelledby="capture-area-title" class="source-section">
  <h2 id="capture-area-title">Capture Area</h2>
  <div class="source-grid" role="group" aria-label="Capture area">
    <button
      type="button"
      class="source-card"
      class:selected={value === "screen"}
      aria-pressed={value === "screen"}
      title="Select a display for full-screen capture."
      onclick={onChooseScreen}
    >
      <Monitor aria-hidden="true" class="size-6" />
      <span>Full Screen</span>
    </button>
    <button
      type="button"
      class="source-card"
      class:selected={value === "window"}
      aria-pressed={value === "window"}
      title="Choose a visible window from the native window list."
      onclick={onChooseWindow}
    >
      <AppWindow aria-hidden="true" class="size-6" />
      <span>Window</span>
    </button>
    <button
      type="button"
      class="source-card"
      class:selected={value === "region"}
      aria-pressed={value === "region"}
      title="Drag to select a physical-pixel region on the chosen display."
      onclick={onChooseRegion}
    >
      <Scan aria-hidden="true" class="size-6" />
      <span>Region</span>
    </button>
  </div>
  <p class="source-status" class:error={sourceError} role={sourceError ? "alert" : "status"}>
    {sourceMessage}
  </p>
</section>

<style>
  .source-section {
    display: grid;
    gap: 8px;
  }

  h2 {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
    font-weight: 500;
    line-height: 18px;
  }

  .source-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .source-card {
    display: flex;
    min-width: 0;
    height: 76px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border: 1px solid var(--border);
    border-radius: 11px;
    background: var(--surface);
    color: var(--muted);
    font: inherit;
    font-size: 13px;
    transition:
      background-color 150ms ease,
      border-color 150ms ease,
      color 150ms ease;
  }

  .source-card:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .source-card.selected {
    border: 1.5px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, var(--surface));
    color: var(--text);
  }

  .source-card.selected :global(svg) {
    color: var(--accent);
  }

  .source-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .source-status {
    min-height: 15px;
    margin: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    line-height: 15px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source-status.error {
    min-height: 30px;
    color: var(--warning);
    line-height: 15px;
    overflow-wrap: anywhere;
    text-overflow: clip;
    white-space: normal;
  }

  @media (prefers-reduced-motion: reduce) {
    .source-card {
      transition: none;
    }
  }
</style>
