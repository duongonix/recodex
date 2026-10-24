<script lang="ts">
  import { onMount } from "svelte";
  import Search from "@lucide/svelte/icons/search";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import LibraryHeader from "./LibraryHeader.svelte";
  import LibraryFilters from "./LibraryFilters.svelte";
  import LibraryEmptyState from "./LibraryEmptyState.svelte";
  import LibraryCaptureRow from "./LibraryCaptureRow.svelte";
  import { errorMessage } from "../../../lib/services/ipc/settings.js";
  import {
    listLibraryItems,
    openLibraryFolder,
    openLibraryItem,
    revealLibraryItem
  } from "../../../lib/services/ipc/library.js";
  import type { LibraryFilter, LibraryItem } from "../../../lib/types/library.js";

  let { onBack }: { onBack: () => void } = $props();
  let items = $state<LibraryItem[]>([]);
  let filter = $state<LibraryFilter>("all");
  let query = $state("");
  let loading = $state(true);
  let openingFolder = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);

  const filteredItems = $derived.by(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase();
    return items.filter((item) => {
      const matchesFilter = filter === "all" || item.kind === filter;
      const matchesQuery = !normalizedQuery || item.name.toLocaleLowerCase().includes(normalizedQuery);
      return matchesFilter && matchesQuery;
    });
  });

  onMount(() => {
    void refresh();
  });

  async function refresh() {
    loading = true;
    error = null;
    try {
      items = await listLibraryItems();
    } catch (cause) {
      error = errorMessage(cause);
      console.error("[RecoDex][library] Could not load captures: " + error);
    } finally {
      loading = false;
    }
  }

  async function openItem(item: LibraryItem) {
    notice = null;
    try {
      await openLibraryItem(item.path);
    } catch (cause) {
      notice = "Could not open this capture: " + errorMessage(cause);
    }
  }

  async function revealItem(item: LibraryItem) {
    notice = null;
    try {
      await revealLibraryItem(item.path);
    } catch (cause) {
      notice = "Could not show this capture: " + errorMessage(cause);
    }
  }

  async function copyPath(item: LibraryItem) {
    notice = null;
    try {
      await navigator.clipboard.writeText(item.path);
      notice = "File path copied.";
    } catch (cause) {
      notice = "Could not copy the file path: " + errorMessage(cause);
    }
  }

  async function showOutputFolder() {
    if (openingFolder) return;
    openingFolder = true;
    notice = null;
    const folderKind = filter === "screenshot" || (filter === "all" && items[0]?.kind === "screenshot")
      ? "screenshot"
      : "video";
    try {
      await openLibraryFolder(folderKind);
    } catch (cause) {
      notice = "Could not open the output folder: " + errorMessage(cause);
    } finally {
      openingFolder = false;
    }
  }

</script>

<section class="library-page" aria-labelledby="library-title">
  <LibraryHeader {loading} onBack={onBack} onRefresh={refresh} />

  <label class="search-box">
    <Search aria-hidden="true" class="size-4" />
    <input bind:value={query} type="search" placeholder="Search captures…" aria-label="Search captures" />
  </label>

  <LibraryFilters bind:value={filter} count={filteredItems.length} />

  {#if notice}
    <p class="notice" role="status">{notice}</p>
  {/if}

  <div class="capture-list" aria-live="polite" aria-busy={loading}>
    {#if loading || error || filteredItems.length === 0}
      <LibraryEmptyState {loading} {error} {query} onRetry={refresh} />
    {:else}
      {#each filteredItems as item (item.path)}
        <LibraryCaptureRow {item} onOpen={openItem} onReveal={revealItem} onCopy={copyPath} />
      {/each}
    {/if}
  </div>

  <footer class="library-footer">
    <span>{items.length} saved {items.length === 1 ? "capture" : "captures"}</span>
    <button type="button" class="folder-button" disabled={openingFolder} onclick={showOutputFolder}>
      <FolderOpen aria-hidden="true" class="size-4" />
      <span>{openingFolder ? "Opening…" : "Open folder"}</span>
    </button>
  </footer>
</section>

<style>
  .library-page {
    display: flex;
    min-height: 0;
    flex-direction: column;
    gap: 10px;
    overflow: hidden;
  }

  .search-box {
    display: flex;
    height: 34px;
    flex: none;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    color: var(--muted);
  }

  .search-box:focus-within { border-color: color-mix(in srgb, var(--accent) 65%, var(--border)); }
  .search-box input {
    width: 100%;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 11px;
  }
  .search-box input::placeholder { color: var(--muted); }

  .notice {
    flex: none;
    margin: -2px 0;
    overflow-wrap: anywhere;
    color: var(--accent);
    font-size: 10px;
    line-height: 1.4;
  }

  .capture-list {
    display: grid;
    min-height: 0;
    flex: 1;
    align-content: start;
    gap: 7px;
    overflow: auto;
    padding-right: 3px;
    scrollbar-color: #454955 transparent;
    scrollbar-width: thin;
  }

  .capture-list::-webkit-scrollbar { width: 6px; }
  .capture-list::-webkit-scrollbar-track { background: transparent; }
  .capture-list::-webkit-scrollbar-thumb { border-radius: 6px; background: #454955; }
  .capture-list::-webkit-scrollbar-thumb:hover { background: #616572; }

  .library-footer {
    display: flex;
    min-height: 34px;
    flex: none;
    align-items: center;
    justify-content: space-between;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 9px;
  }

  .folder-button {
    display: inline-flex;
    height: 29px;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 10px;
    cursor: pointer;
  }
  .folder-button:hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 54%, var(--border)); color: var(--accent); }
  .folder-button:disabled { cursor: default; opacity: 0.7; }

  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
</style>
