<script lang="ts">
  import AppWindow from "@lucide/svelte/icons/app-window";
  import Check from "@lucide/svelte/icons/check";
  import WindowPreview from "./WindowPreview.svelte";
  import type { WindowSource } from "../../types/capture.js";

  let {
    source,
    selected,
    onSelect,
    onConfirm
  }: {
    source: WindowSource;
    selected: boolean;
    onSelect: () => void;
    onConfirm: () => void;
  } = $props();
</script>

<button
  type="button"
  class="window-card"
  class:selected
  disabled={!source.isRecordable}
  aria-pressed={selected}
  aria-label={`${source.appName}, ${source.title}${source.isMinimized ? ", minimized" : source.isVisible ? "" : ", hidden"}`}
  data-window-card
  onclick={onSelect}
  ondblclick={onConfirm}
>
  <WindowPreview windowId={source.windowId} />
  <div class="details">
    <span class="app-icon">
      {#if source.appIcon}
        <img src={source.appIcon} alt="" />
      {:else}
        <AppWindow aria-hidden="true" class="size-4" />
      {/if}
    </span>
    <span class="labels">
      <span class="app-name">{source.appName}</span>
      <span class="window-title" title={source.title}>{source.title}</span>
    </span>
    {#if selected}<span class="selected-mark"><Check aria-hidden="true" class="size-4" /></span>{/if}
    {#if !source.isRecordable}
      <span class="availability">{source.isMinimized ? "Minimized" : "Hidden"}</span>
    {/if}
  </div>
</button>

<style>
  .window-card {
    display: block;
    min-width: 0;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--wp-border);
    border-radius: 11px;
    background: var(--wp-surface);
    color: var(--wp-text);
    text-align: left;
    transition: border-color 140ms ease, background-color 140ms ease, box-shadow 140ms ease;
  }
  .window-card:not(:disabled):hover { border-color: var(--wp-border-hover); background: var(--wp-surface-raised); }
  .window-card.selected { border-color: var(--wp-accent); box-shadow: 0 0 0 1px var(--wp-accent), 0 0 18px color-mix(in srgb, var(--wp-accent) 15%, transparent); }
  .window-card:focus-visible { outline: 2px solid var(--wp-accent-bright); outline-offset: 2px; }
  .window-card:disabled { cursor: not-allowed; opacity: .66; }
  .details { display: flex; min-width: 0; min-height: 62px; align-items: center; gap: 10px; padding: 10px 11px; }
  .app-icon { display: grid; width: 30px; height: 30px; flex: none; place-items: center; overflow: hidden; border: 1px solid var(--wp-border); border-radius: 8px; background: var(--wp-surface-hover); color: var(--wp-muted); }
  .app-icon img { width: 22px; height: 22px; object-fit: contain; }
  .labels { display: grid; min-width: 0; flex: 1; gap: 3px; }
  .app-name, .window-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .app-name { font-size: 12px; font-weight: 600; }
  .window-title { color: var(--wp-muted); font-size: 11px; }
  .selected-mark { display: grid; width: 21px; height: 21px; flex: none; place-items: center; border-radius: 50%; background: var(--wp-accent); color: white; }
  .availability { flex: none; color: var(--wp-muted); font-size: 10px; }
  @media (prefers-reduced-motion: reduce) { .window-card { transition: none; } }
</style>
