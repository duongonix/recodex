<script lang="ts">
  import Gauge from "@lucide/svelte/icons/gauge";
  import Monitor from "@lucide/svelte/icons/monitor";
  import {
    Root as SelectRoot,
    Trigger as SelectTrigger,
    Value as SelectValue,
    Content as SelectContent,
    Item as SelectItem
  } from "../ui/select/index.js";

  type Quality = "720p" | "1080p" | "2160p";
  type Fps = "30" | "60";

  const qualityItems: { value: Quality; label: string }[] = [
    { value: "720p", label: "720p (HD)" },
    { value: "1080p", label: "1080p (FHD)" },
    { value: "2160p", label: "2160p (UHD)" }
  ];
  const fpsItems: { value: Fps; label: string }[] = [
    { value: "30", label: "30 FPS" },
    { value: "60", label: "60 FPS" }
  ];

  let {
    quality = $bindable("1080p"),
    fps = $bindable("30"),
    maxResolutionHeight = null,
    availableFps = []
  }: {
    quality?: Quality;
    fps?: Fps;
    maxResolutionHeight?: number | null;
    availableFps?: number[];
  } = $props();

  const supports2160p = $derived((maxResolutionHeight ?? 0) >= 2160);
  const supports60fps = $derived(availableFps.includes(60));
</script>

<section aria-label="Recording options" class="options-section">
  <div class="option-row">
    <label for="quality-select" class="option-label">
      <Monitor aria-hidden="true" class="size-5" />
      <span>Quality</span>
    </label>
    <SelectRoot type="single" items={qualityItems} bind:value={quality}>
      <SelectTrigger id="quality-select" class="option-select">
        <SelectValue placeholder="1080p (FHD)" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="720p" label="720p (HD)" />
        <SelectItem value="1080p" label="1080p (FHD)" />
        <SelectItem
          value="2160p"
          label="2160p (UHD)"
          disabled={!supports2160p}
          title={supports2160p
            ? "Output is capped at 2160p and will not upscale a smaller source."
            : "2160p is unavailable until the capture service reports support."}
        />
      </SelectContent>
    </SelectRoot>
  </div>

  <div class="option-row">
    <label for="fps-select" class="option-label">
      <Gauge aria-hidden="true" class="size-5" />
      <span>FPS</span>
    </label>
    <SelectRoot type="single" items={fpsItems} bind:value={fps}>
      <SelectTrigger id="fps-select" class="option-select">
        <SelectValue placeholder="30 FPS" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="30" label="30 FPS" />
        <SelectItem
          value="60"
          label="60 FPS"
          disabled={!supports60fps}
          title={supports60fps
            ? "60 FPS is the target; actual frame delivery depends on the display and system load."
            : "60 FPS is unavailable until the capture service reports support."}
        />
      </SelectContent>
    </SelectRoot>
  </div>
</section>

<style>
  .options-section {
    display: grid;
    gap: 8px;
  }

  .option-row {
    display: grid;
    min-height: 36px;
    grid-template-columns: 1fr minmax(0, 1.18fr);
    align-items: center;
    gap: 12px;
  }

  .option-label {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 13px;
  }

  .option-label :global(svg) {
    flex: none;
  }

  :global(.option-select) {
    width: 100%;
    height: 36px;
    border-radius: 10px;
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
  }
</style>
