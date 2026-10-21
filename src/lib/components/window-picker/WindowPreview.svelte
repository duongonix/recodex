<script lang="ts">
  import { onDestroy } from "svelte";
  import Image from "@lucide/svelte/icons/image";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import { loadWindowPreview } from "../../services/window-preview-queue.js";

  let { windowId }: { windowId: string } = $props();
  let container = $state<HTMLDivElement | null>(null);
  let preview = $state<string | null>(null);
  let loading = $state(false);
  let unavailable = $state(false);
  let disposed = false;
  let observer: IntersectionObserver | undefined;

  $effect(() => {
    const element = container;
    if (!element) return;
    observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) void loadPreview();
    }, { rootMargin: "100px" });
    observer.observe(element);
    return () => observer?.disconnect();
  });

  async function loadPreview() {
    if (loading || preview || unavailable || disposed) return;
    loading = true;
    try {
      preview = await loadWindowPreview(windowId);
      unavailable = preview === null;
    } catch {
      unavailable = true;
    } finally {
      loading = false;
    }
  }

  onDestroy(() => { disposed = true; observer?.disconnect(); });
</script>

<div bind:this={container} class="preview" aria-label={preview ? "Window preview" : "Window preview loading"}>
  {#if preview}
    <img src={preview} alt="" draggable="false" />
  {:else if loading}
    <LoaderCircle aria-hidden="true" class="size-6 spin" />
    <span>Loading preview</span>
  {:else if unavailable}
    <Image aria-hidden="true" class="size-6" />
    <span>Preview unavailable</span>
  {:else}
    <span class="skeleton"></span>
  {/if}
</div>

<style>
  .preview {
    position: relative;
    display: flex;
    width: 100%;
    aspect-ratio: 16 / 9;
    align-items: center;
    justify-content: center;
    gap: 8px;
    overflow: hidden;
    border-bottom: 1px solid var(--wp-border);
    background: var(--wp-bg);
    color: var(--wp-muted);
    font-size: 11px;
  }
  img { width: 100%; height: 100%; object-fit: cover; }
  .skeleton { position: absolute; inset: 0; background: linear-gradient(100deg, var(--wp-surface) 25%, var(--wp-surface-hover) 40%, var(--wp-surface) 55%); background-size: 220% 100%; animation: shimmer 1.4s ease infinite; }
  :global(.spin) { animation: spin 1.2s linear infinite; }
  @keyframes shimmer { to { background-position-x: -220%; } }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { :global(.spin), .skeleton { animation: none; } }
</style>
