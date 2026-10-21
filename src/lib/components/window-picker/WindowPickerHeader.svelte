<script lang="ts">
  import AppWindow from "@lucide/svelte/icons/app-window";
  import X from "@lucide/svelte/icons/x";

  let { onClose }: { onClose: () => void } = $props();
</script>

<header class="picker-header" data-tauri-drag-region>
  <div class="title-row" data-tauri-drag-region>
    <span class="title-icon"><AppWindow aria-hidden="true" class="size-5" /></span>
    <div data-tauri-drag-region>
      <h1>Choose a window</h1>
      <p>Select a window to capture</p>
    </div>
  </div>
  <button class="icon-button" type="button" aria-label="Close window picker" onclick={onClose}>
    <X aria-hidden="true" class="size-4" />
  </button>
</header>

<style>
  .picker-header {
    display: flex;
    min-height: 88px;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 20px 26px 18px;
    border-bottom: 1px solid var(--wp-border-muted);
    cursor: move;
  }
  .title-row { display: flex; align-items: center; gap: 14px; }
  .title-icon {
    display: grid;
    width: 42px;
    height: 42px;
    flex: none;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--wp-accent) 35%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--wp-accent) 10%, transparent);
    color: var(--wp-accent-bright);
  }
  h1 { margin: 0; color: var(--wp-text); font-size: 18px; font-weight: 650; letter-spacing: -0.02em; }
  p { margin: 4px 0 0; color: var(--wp-muted); font-size: 12px; }
  .icon-button {
    display: grid;
    width: 34px;
    height: 34px;
    flex: none;
    place-items: center;
    border: 1px solid var(--wp-border);
    border-radius: 9px;
    background: var(--wp-surface);
    color: var(--wp-muted);
  }
  .icon-button:hover { border-color: var(--wp-accent-bright); color: #fff; }
  .icon-button:focus-visible { outline: 2px solid var(--wp-accent-bright); outline-offset: 2px; }
</style>
