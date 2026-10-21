<script lang="ts">
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import type { WebcamDevice } from "../../types/webcam.js";

  let { devices, loading, selected, status, onChange, onRefresh }: {
    devices: WebcamDevice[];
    loading: boolean;
    selected: string | null;
    status: string | null;
    onChange: (id: string | null) => void;
    onRefresh: () => void;
  } = $props();
</script>

<section class="webcam-control-section" aria-labelledby="webcam-camera-heading">
  <h3 id="webcam-camera-heading" class="webcam-section-title">Camera</h3>
  <div class="webcam-camera-row">
    <select
      aria-label="Select camera"
      class="webcam-select"
      value={selected ?? ""}
      disabled={loading || devices.length === 0}
      onchange={(event) => onChange(event.currentTarget.value || null)}
    >
      <option value="">{loading ? "Loading cameras…" : devices.length ? "Select a camera" : "No camera detected"}</option>
      {#each devices as device (device.id)}
        <option value={device.id}>{device.name}</option>
      {/each}
    </select>
    <button type="button" class="webcam-device-settings" disabled={loading} title="Refresh camera list" aria-label="Refresh camera list" onclick={onRefresh}>
      <RefreshCw aria-hidden="true" class="size-4" />
    </button>
  </div>
  {#if status}
    <p class="webcam-status" role="status">{status}</p>
  {/if}
</section>
