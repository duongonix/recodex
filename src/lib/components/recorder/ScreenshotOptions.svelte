<script lang="ts">
  import Clipboard from "@lucide/svelte/icons/clipboard";
  import Image from "@lucide/svelte/icons/image";
  import Save from "@lucide/svelte/icons/save";
  import {
    Root as SelectRoot,
    Trigger as SelectTrigger,
    Value as SelectValue,
    Content as SelectContent,
    Item as SelectItem
  } from "../ui/select/index.js";
  import { Switch } from "../ui/switch/index.js";

  type ImageFormat = "png" | "jpg";
  const formatItems: { value: ImageFormat; label: string }[] = [
    { value: "png", label: "PNG" },
    { value: "jpg", label: "JPG" }
  ];

  let {
    format = $bindable("png"),
    copyToClipboard = $bindable(true),
    saveToFile = $bindable(true)
  }: {
    format?: ImageFormat;
    copyToClipboard?: boolean;
    saveToFile?: boolean;
  } = $props();
</script>

<section aria-label="Screenshot options" class="screenshot-options">
  <div class="format-row">
    <label for="format-select" class="option-label">
      <Image aria-hidden="true" class="size-5" />
      <span>Format</span>
    </label>
    <SelectRoot type="single" items={formatItems} bind:value={format}>
      <SelectTrigger id="format-select" class="option-select">
        <SelectValue placeholder="PNG" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="png" label="PNG" />
        <SelectItem value="jpg" label="JPG" />
      </SelectContent>
    </SelectRoot>
  </div>

  <div class="delivery-options">
    <div class="delivery-option">
      <Clipboard aria-hidden="true" class="size-5" />
      <span id="copy-option-label">Copy to clipboard</span>
      <Switch
        bind:checked={copyToClipboard}
        aria-labelledby="copy-option-label"
      />
    </div>
    <div class="delivery-option">
      <Save aria-hidden="true" class="size-5" />
      <span id="save-option-label">Save to file</span>
      <Switch bind:checked={saveToFile} aria-labelledby="save-option-label" />
    </div>
  </div>
</section>

<style>
  .screenshot-options {
    display: grid;
    gap: 10px;
  }

  .format-row {
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

  .delivery-options {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .delivery-option {
    display: flex;
    min-width: 0;
    height: 44px;
    align-items: center;
    gap: 8px;
    padding: 0 9px;
    border-radius: 10px;
    background: var(--surface);
    color: var(--text);
    font-size: 11px;
  }

  .delivery-option span {
    flex: 1;
    white-space: nowrap;
  }

  .delivery-option :global(svg),
  .delivery-option :global([data-slot="switch"]) {
    flex: none;
  }
</style>
