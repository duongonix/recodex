<script lang="ts">
  import Camera from "@lucide/svelte/icons/camera";
  import Video from "@lucide/svelte/icons/video";
  import {
    Root as TabsRoot,
    List as TabsList,
    Trigger as TabsTrigger
  } from "../ui/tabs/index.js";

  type CaptureMode = "record" | "screenshot";

  let { value = $bindable("record") }: { value?: CaptureMode } = $props();
</script>

<TabsRoot bind:value>
  <TabsList class="mode-list">
    <TabsTrigger value="record" class="mode-tab">
      <Video aria-hidden="true" class="size-5" />
      <span>Record</span>
    </TabsTrigger>
    <TabsTrigger value="screenshot" class="mode-tab">
      <Camera aria-hidden="true" class="size-5" />
      <span>Screenshot</span>
    </TabsTrigger>
  </TabsList>
</TabsRoot>

<style>
  :global(.mode-list) {
    display: flex;
    width: 100%;
    height: 52px;
    gap: 8px;
    padding: 0;
    border-radius: 12px;
    background: transparent;
  }

  :global(.mode-tab) {
    position: relative;
    flex: 1;
    height: 52px;
    gap: 10px;
    border: 1.5px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
    font-size: 14px;
    transition:
      background-color 150ms ease,
      border-color 150ms ease,
      color 150ms ease,
      box-shadow 150ms ease;
  }

  :global(.mode-tab:hover) {
    background: var(--surface-hover);
    color: var(--text);
  }

  :global(.mode-tab[data-active]),
  :global(.mode-tab[aria-selected="true"]) {
    border-width: 2px;
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 36%, var(--surface));
    color: var(--text);
    font-weight: 700;
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--accent) 48%, transparent),
      0 0 0 1px color-mix(in srgb, var(--accent) 24%, transparent);
  }

  :global(.mode-tab[data-active]::after),
  :global(.mode-tab[aria-selected="true"]::after) {
    position: absolute;
    right: 25%;
    bottom: -1px;
    left: 25%;
    height: 3px;
    border-radius: 3px 3px 0 0;
    background: var(--accent);
    content: "";
  }

  :global(.mode-tab[data-active] svg),
  :global(.mode-tab[aria-selected="true"] svg) {
    color: var(--accent);
  }

  @media (prefers-reduced-motion: reduce) {
    :global(.mode-tab) {
      transition: none;
    }
  }
</style>
