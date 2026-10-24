<script lang="ts">
  import { onMount } from "svelte";
  import Play from "@lucide/svelte/icons/play";
  import Camera from "@lucide/svelte/icons/camera";
  import Video from "@lucide/svelte/icons/video";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import Copy from "@lucide/svelte/icons/copy";
  import { errorMessage } from "../../../lib/services/ipc/settings.js";
  import { getLibraryThumbnail } from "../../../lib/services/ipc/library.js";
  import type { LibraryItem } from "../../../lib/types/library.js";

  let {
    item,
    onOpen,
    onReveal,
    onCopy
  }: {
    item: LibraryItem;
    onOpen: (item: LibraryItem) => void;
    onReveal: (item: LibraryItem) => void;
    onCopy: (item: LibraryItem) => void;
  } = $props();

  let thumbnailHost: HTMLSpanElement;
  let thumbnail = $state<string | null>(null);
  let thumbnailFailed = $state(false);

  onMount(() => {
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer.disconnect();
      void getLibraryThumbnail(item.path)
        .then((dataUrl) => (thumbnail = dataUrl))
        .catch((error: unknown) => {
          thumbnailFailed = true;
          console.warn("[RecoDex][library] Thumbnail unavailable: " + errorMessage(error));
        });
    }, { rootMargin: "80px" });
    observer.observe(thumbnailHost);
    return () => observer.disconnect();
  });

  const modifiedLabel = $derived(formatDate(item.modifiedAtMs));
  const detailLabel = $derived(item.kind === "video"
    ? (item.durationMs === null ? "Video" : formatDuration(item.durationMs)) + " · " + formatSize(item.sizeBytes) + " · " + modifiedLabel
    : formatDimensionsFromName(item.name) + " · " + formatSize(item.sizeBytes) + " · " + modifiedLabel);

  function formatDate(value: number): string {
    if (!Number.isFinite(value) || value <= 0) return "Date unavailable";
    return new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" }).format(value);
  }

  function formatDuration(value: number): string {
    const seconds = Math.floor(value / 1000);
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const remainder = seconds % 60;
    return hours > 0
      ? hours + ":" + String(minutes).padStart(2, "0") + ":" + String(remainder).padStart(2, "0")
      : minutes + ":" + String(remainder).padStart(2, "0");
  }

  function formatSize(value: number): string {
    if (value < 1024) return value + " B";
    const units = ["KB", "MB", "GB", "TB"];
    let size = value / 1024;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return size.toFixed(size >= 10 ? 0 : 1) + " " + units[unit];
  }

  function formatDimensionsFromName(name: string): string {
    const match = name.match(/(\d{3,5})[x×](\d{3,5})/i);
    return match ? match[1] + " × " + match[2] : "Screenshot";
  }
</script>

<article class="capture-row" class:unavailable={!item.isAvailable}>
  <button
    class="capture-main"
    type="button"
    disabled={!item.isAvailable}
    aria-label={"Open " + item.name}
    onclick={() => onOpen(item)}
  >
    <span class="capture-thumbnail" bind:this={thumbnailHost}>
      {#if thumbnail}
        <img src={thumbnail} alt="" loading="lazy" />
      {:else if item.kind === "video"}
        <Video aria-hidden="true" class="type-icon" />
      {:else}
        <Camera aria-hidden="true" class="type-icon" />
      {/if}
      {#if item.kind === "video" && thumbnail}
        <span class="play-badge"><Play aria-hidden="true" class="size-3" fill="currentColor" /></span>
      {/if}
      {#if thumbnailFailed}
        <span class="thumbnail-error" aria-hidden="true">Preview unavailable</span>
      {/if}
    </span>
    <span class="capture-copy">
      <span class="capture-name" title={item.name}>{item.name}</span>
      <span class="capture-details">{item.isAvailable ? detailLabel : "File no longer available"}</span>
    </span>
  </button>
  <div class="capture-actions">
    <button type="button" aria-label={"Show " + item.name + " in folder"} title="Show in folder" disabled={!item.isAvailable} onclick={() => onReveal(item)}>
      <FolderOpen aria-hidden="true" class="size-4" />
    </button>
    <button type="button" aria-label={"Copy path for " + item.name} title="Copy path" disabled={!item.isAvailable} onclick={() => onCopy(item)}>
      <Copy aria-hidden="true" class="size-4" />
    </button>
  </div>
</article>

<style>
  .capture-row {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 4px;
    border-radius: 10px;
    background: var(--surface);
    transition: background-color 150ms ease;
  }

  .capture-row:hover { background: var(--surface-hover); }
  .capture-row.unavailable { opacity: 0.58; }

  .capture-main {
    display: flex;
    min-width: 0;
    flex: 1;
    align-items: center;
    gap: 10px;
    padding: 9px;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .capture-main:disabled { cursor: default; }
  .capture-thumbnail {
    position: relative;
    display: grid;
    width: 72px;
    height: 48px;
    flex: none;
    place-items: center;
    overflow: hidden;
    border-radius: 7px;
    background: #0e1015;
    color: var(--muted);
  }

  .capture-thumbnail img { width: 100%; height: 100%; object-fit: cover; }
  .capture-thumbnail :global(.type-icon) { width: 19px; height: 19px; }
  .play-badge {
    position: absolute;
    display: grid;
    width: 24px;
    height: 24px;
    place-items: center;
    border-radius: 50%;
    background: color-mix(in srgb, #08090c 78%, transparent);
    color: white;
  }

  .thumbnail-error { position: absolute; right: 3px; bottom: 2px; left: 3px; font-size: 8px; text-align: center; }
  .capture-copy { display: grid; min-width: 0; flex: 1; gap: 5px; }
  .capture-name { overflow: hidden; color: var(--text); font-size: 11px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .capture-details { color: var(--muted); font-size: 9px; line-height: 1.4; }
  .capture-actions { display: flex; flex: none; align-items: center; gap: 2px; padding-right: 5px; }
  .capture-actions button {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .capture-actions button:hover:not(:disabled) { background: var(--surface-raised); color: var(--accent); }
  .capture-actions button:disabled { cursor: default; opacity: 0.5; }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
</style>
