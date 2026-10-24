<script lang="ts">
  import { onMount } from "svelte";
  import "./picker-tokens.css";
  import Check from "@lucide/svelte/icons/check";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Search from "@lucide/svelte/icons/search";
  import WindowPickerCard from "./WindowPickerCard.svelte";
  import WindowPickerHeader from "./WindowPickerHeader.svelte";
  import { closeWindowPicker, formatCaptureError, getSelectedWindow, listWindows, selectCaptureWindow } from "../../services/ipc/capture.js";
  import type { WindowSource } from "../../types/capture.js";
  import {
    Root as SelectRoot,
    Trigger as SelectTrigger,
    Value as SelectValue,
    Content as SelectContent,
    Item as SelectItem
  } from "../ui/select/index.js";

  type SortMode = "lastUsed" | "application" | "title";
  const sortItems = [
    { value: "lastUsed" as const, label: "Last used" },
    { value: "application" as const, label: "Application name" },
    { value: "title" as const, label: "Window title" }
  ];

  let windows = $state<WindowSource[]>([]);
  let selectedWindowId = $state("");
  let query = $state("");
  let sortMode = $state<SortMode>("lastUsed");
  let visibleOnly = $state(true);
  let loading = $state(true);
  let confirming = $state(false);
  let error = $state("");
  let refreshVersion = $state(0);

  const filteredWindows = $derived.by(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase();
    return windows
      .filter((source) => !visibleOnly || (source.isVisible && !source.isMinimized))
      .filter((source) => !normalizedQuery || `${source.appName} ${source.title}`.toLocaleLowerCase().includes(normalizedQuery))
      .toSorted((left, right) => {
        if (sortMode === "lastUsed") {
          const recent = (right.lastUsedAt ?? 0) - (left.lastUsedAt ?? 0);
          if (recent !== 0) return recent;
        }
        if (sortMode === "application") {
          const appOrder = left.appName.localeCompare(right.appName, undefined, { sensitivity: "base" });
          if (appOrder !== 0) return appOrder;
        }
        return left.title.localeCompare(right.title, undefined, { sensitivity: "base" });
      });
  });

  onMount(() => { void refresh(); });

  async function refresh() {
    refreshVersion += 1;
    loading = true;
    error = "";
    try {
      const sources = await listWindows();
      windows = sources;
      if (!sources.some((item) => item.windowId === selectedWindowId && item.isRecordable)) {
        selectedWindowId = "";
        try {
          const current = await getSelectedWindow();
          if (current && sources.some((item) => item.windowId === current.windowId && item.isRecordable)) {
            selectedWindowId = current.windowId;
          }
        } catch {
          selectedWindowId = "";
        }
      }
    } catch (reason) {
      error = `Could not load windows: ${formatCaptureError(reason)}`;
    } finally {
      loading = false;
    }
  }

  async function confirmSelection() {
    if (!selectedWindowId || confirming) return;
    confirming = true;
    error = "";
    try {
      await selectCaptureWindow(selectedWindowId);
    } catch (reason) {
      error = `Could not select this window: ${formatCaptureError(reason)}`;
    } finally {
      confirming = false;
    }
  }

  async function cancel() {
    try {
      await closeWindowPicker();
    } catch (reason) {
      error = formatCaptureError(reason);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      void cancel();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />
<svelte:head><title>Choose a window — RecoDex</title></svelte:head>

<main class="picker-shell">
  <WindowPickerHeader onClose={cancel} />

  <section class="controls" aria-label="Window search and sorting">
    <label class="search-box">
      <Search aria-hidden="true" class="size-4" />
      <input bind:value={query} type="search" placeholder="Search windows or applications…" aria-label="Search windows or applications" />
    </label>
    <div class="sort-control" role="group" aria-label="Sort windows">
      <span>Sort by</span>
      <SelectRoot type="single" items={sortItems} bind:value={sortMode}>
        <SelectTrigger class="sort-trigger"><SelectValue placeholder="Last used" /></SelectTrigger>
        <SelectContent>
          <SelectItem value="lastUsed" label="Last used" />
          <SelectItem value="application" label="Application name" />
          <SelectItem value="title" label="Window title" />
        </SelectContent>
      </SelectRoot>
    </div>
    <button class="refresh-button" type="button" disabled={loading} onclick={refresh} aria-label="Refresh open windows">
      <RefreshCw aria-hidden="true" class={loading ? "size-4 spin" : "size-4"} />
      <span>Refresh</span>
    </button>
  </section>

  {#if error}
    <div class="error-banner" role="alert">
      <span>{error}</span>
      <button type="button" class="retry-button" onclick={refresh}><RefreshCw aria-hidden="true" class="size-4" /> Retry</button>
    </div>
  {/if}

  <section class="window-list" aria-label="Available windows" aria-busy={loading}>
    {#if loading}
      <div class="loading-state" role="status"><LoaderCircle aria-hidden="true" class="size-6 spin" /><span>Finding open windows…</span></div>
    {:else if !error && windows.length === 0}
      <div class="empty-state"><span class="empty-icon">⌕</span><h2>No open windows found</h2><p>Open a window and refresh the list to capture it.</p><button class="secondary-button" type="button" onclick={refresh}><RefreshCw aria-hidden="true" class="size-4" /> Refresh windows</button></div>
    {:else if error}
      <div class="empty-state"><h2>Window list unavailable</h2><p>Refresh to try the native window list again.</p><button class="secondary-button" type="button" onclick={refresh}><RefreshCw aria-hidden="true" class="size-4" /> Retry</button></div>
    {:else if filteredWindows.length === 0}
      <div class="empty-state"><span class="empty-icon">⌕</span><h2>No matching windows</h2><p>{visibleOnly ? "Try another search or include hidden and minimized windows." : "Try another search term."}</p></div>
    {:else}
      <div class="window-grid">
        {#each filteredWindows as source (`${source.windowId}:${refreshVersion}`)}
          <WindowPickerCard
            {source}
            selected={selectedWindowId === source.windowId}
            onSelect={() => { selectedWindowId = source.windowId; }}
            onConfirm={confirmSelection}
          />
        {/each}
      </div>
    {/if}
  </section>

  <footer class="picker-footer">
    <label class="visible-toggle">
      <input type="checkbox" bind:checked={visibleOnly} />
      <span class="checkmark"><Check aria-hidden="true" class="size-3" /></span>
      <span>Show only visible windows</span>
    </label>
    <div class="footer-actions">
      <button class="cancel-button" type="button" onclick={cancel}>Cancel</button>
      <button class="select-button" type="button" disabled={!selectedWindowId || confirming} onclick={confirmSelection}>
        {#if confirming}<LoaderCircle aria-hidden="true" class="size-4 spin" />{/if}
        Select Window
      </button>
    </div>
  </footer>
</main>

<style>
  :global(html), :global(body) { width: 100%; height: 100%; min-width: 320px; min-height: 0; overflow: hidden; background: var(--wp-bg); }
  .picker-shell { display: flex; width: 100%; height: 100dvh; flex-direction: column; overflow: hidden; border: 1px solid var(--wp-border); border-radius: 13px; background: var(--wp-bg); color: var(--wp-text); }
  .controls { display: flex; min-height: 76px; align-items: center; gap: 14px; padding: 14px 26px; border-bottom: 1px solid var(--wp-border-muted); }
  .search-box { display: flex; height: 40px; min-width: 160px; flex: 1; align-items: center; gap: 10px; padding: 0 12px; border: 1px solid var(--wp-border); border-radius: 9px; background: var(--wp-surface); color: var(--wp-muted); }
  .search-box:focus-within { border-color: var(--wp-accent-bright); box-shadow: 0 0 0 2px color-mix(in srgb, var(--wp-accent) 15%, transparent); }
  input[type="search"] { width: 100%; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--wp-text); font: inherit; font-size: 12px; }
  input[type="search"]::placeholder { color: var(--wp-muted-strong); }
  .sort-control { display: flex; align-items: center; gap: 9px; color: var(--wp-muted); font-size: 11px; white-space: nowrap; }
  :global(.sort-trigger) { width: 148px; height: 38px; border-color: var(--wp-border); border-radius: 9px; background: var(--wp-surface); color: var(--wp-text-secondary); font-size: 12px; }
  .refresh-button { display: inline-flex; height: 38px; flex: none; align-items: center; justify-content: center; gap: 7px; padding: 0 11px; border: 1px solid var(--wp-border); border-radius: 9px; background: var(--wp-surface); color: var(--wp-text-secondary); font: inherit; font-size: 12px; }
  .refresh-button:hover:not(:disabled) { border-color: var(--wp-accent-bright); color: var(--wp-text); }
  .refresh-button:disabled { opacity: .65; cursor: wait; }
  .window-list { flex: 1; min-height: 0; overflow: auto; padding: 18px 26px 22px; scrollbar-color: color-mix(in srgb, var(--wp-accent) 48%, var(--wp-border)) var(--wp-bg); scrollbar-width: thin; }
  .window-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); align-content: start; gap: 14px; }
  .loading-state { display: flex; min-height: 220px; align-items: center; justify-content: center; gap: 11px; color: var(--wp-muted); font-size: 13px; }
  :global(.spin) { animation: spin 1.1s linear infinite; }
  .empty-state { display: flex; min-height: 270px; flex-direction: column; align-items: center; justify-content: center; text-align: center; }
  .empty-icon { color: var(--wp-muted-strong); font-size: 42px; line-height: 1; }
  h2 { margin: 14px 0 0; font-size: 15px; font-weight: 600; }
  .empty-state p { max-width: 340px; margin: 7px 0 18px; color: var(--wp-muted); font-size: 12px; line-height: 1.5; }
  .secondary-button, .retry-button { display: inline-flex; align-items: center; justify-content: center; gap: 8px; border: 1px solid var(--wp-border); border-radius: 8px; background: var(--wp-surface); color: var(--wp-text-secondary); font: inherit; font-size: 12px; }
  .secondary-button { min-height: 36px; padding: 0 12px; }
  .retry-button { flex: none; min-height: 30px; padding: 0 9px; }
  .secondary-button:hover, .retry-button:hover { border-color: var(--wp-accent-bright); }
  .error-banner { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin: 12px 26px 0; padding: 10px 12px; border: 1px solid color-mix(in srgb, var(--wp-accent-bright) 35%, transparent); border-radius: 9px; background: color-mix(in srgb, var(--wp-accent) 8%, transparent); color: var(--wp-accent-soft); font-size: 12px; }
  .picker-footer { display: flex; min-height: 76px; align-items: center; justify-content: space-between; gap: 20px; padding: 14px 26px; border-top: 1px solid var(--wp-border-muted); background: var(--wp-footer); }
  .visible-toggle { display: inline-flex; min-height: 32px; align-items: center; gap: 9px; color: var(--wp-text-secondary); font-size: 12px; cursor: pointer; }
  .visible-toggle input { position: absolute; width: 1px; height: 1px; opacity: 0; }
  .checkmark { display: grid; width: 16px; height: 16px; place-items: center; border: 1px solid var(--wp-border-strong); border-radius: 4px; background: var(--wp-surface); color: transparent; }
  .visible-toggle input:checked + .checkmark { border-color: var(--wp-accent); background: var(--wp-accent); color: var(--wp-white); }
  .visible-toggle input:focus-visible + .checkmark { outline: 2px solid var(--wp-accent-bright); outline-offset: 2px; }
  .footer-actions { display: flex; align-items: center; gap: 10px; }
  .cancel-button, .select-button { display: inline-flex; min-height: 38px; align-items: center; justify-content: center; gap: 8px; padding: 0 15px; border: 1px solid var(--wp-border); border-radius: 9px; font: inherit; font-size: 12px; font-weight: 550; }
  .cancel-button { background: var(--wp-surface-raised); color: var(--wp-text-secondary); }
  .cancel-button:hover { border-color: var(--wp-muted-strong); background: var(--wp-surface-hover); }
  .select-button { border-color: var(--wp-accent); background: var(--wp-accent); color: var(--wp-white); box-shadow: 0 5px 18px color-mix(in srgb, var(--wp-accent) 16%, transparent); }
  .select-button:hover:not(:disabled) { border-color: var(--wp-accent-bright); background: var(--wp-accent-bright); }
  .select-button:disabled { border-color: var(--wp-disabled-border); background: var(--wp-disabled-surface); color: var(--wp-disabled-text); cursor: not-allowed; box-shadow: none; }
  button:focus-visible { outline: 2px solid var(--wp-accent-bright); outline-offset: 2px; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 760px) { .window-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } .controls { align-items: stretch; flex-direction: column; } .sort-control { justify-content: space-between; } :global(.sort-trigger) { flex: 1; } }
  @media (max-width: 480px) { :global(.picker-header), .controls, .picker-footer { padding-right: 16px; padding-left: 16px; } .window-list { padding-right: 16px; padding-left: 16px; } .window-grid { grid-template-columns: 1fr; } .picker-footer { align-items: flex-start; flex-direction: column; } .footer-actions { width: 100%; } .footer-actions button { flex: 1; } }
  @media (prefers-reduced-motion: reduce) { :global(.spin) { animation: none; } }
</style>
