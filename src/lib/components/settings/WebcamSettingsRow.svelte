<script lang="ts">
  import Camera from "@lucide/svelte/icons/camera";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import { Switch } from "../ui/switch/index.js";

  let { enabled, selected = false, disabled = false, disabledReason = "", onToggle, onOpen }: {
    enabled: boolean;
    selected?: boolean;
    disabled?: boolean;
    disabledReason?: string;
    onToggle: (enabled: boolean) => Promise<boolean>;
    onOpen: () => void;
  } = $props();

  let pending = $state(false);

  async function toggle(value: boolean) {
    if (pending || disabled || value === enabled) return;
    pending = true;
    try { await onToggle(value); }
    finally { pending = false; }
  }
</script>

<div class="webcam-setting-row" class:webcam-setting-row-selected={selected}>
  <Camera aria-hidden="true" class="setting-icon" />
  <button type="button" class="webcam-setting-open" onclick={onOpen} aria-label="Configure Webcam Overlay">
    <span class="setting-label">Webcam overlay</span>
    <ChevronRight aria-hidden="true" class="webcam-setting-chevron" />
  </button>
  <Switch checked={enabled} disabled={pending || disabled} aria-label="Enable Webcam Overlay" title={disabled ? disabledReason : undefined} onCheckedChange={(value) => { void toggle(value); }} />
</div>
