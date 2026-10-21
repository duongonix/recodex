<script lang="ts">
  import { Switch } from "../ui/switch/index.js";
  const palette = ["#FF3038", "#FF8A30", "#FFD230", "#35C979", "#3E8BFF", "#9A62F5", "#F5F5F7"];
  let { enabled, color, width, onChange }: {
    enabled: boolean;
    color: string;
    width: number;
    onChange: (patch: { borderEnabled?: boolean; borderColor?: string; borderWidth?: number }) => void;
  } = $props();
</script>

<section class="webcam-control-section" aria-labelledby="webcam-border-heading">
  <div class="webcam-control-heading">
    <h3 id="webcam-border-heading" class="webcam-section-title">Border</h3>
    <div class="webcam-inline-switch"><span>{enabled ? "On" : "Off"}</span><Switch checked={enabled} aria-label="Enable webcam border" onCheckedChange={(value) => onChange({ borderEnabled: value })} /></div>
  </div>
  <div class="webcam-color-row" aria-label="Border color palette">
    {#each palette as swatch (swatch)}
      <button type="button" class="webcam-color-swatch" class:webcam-color-selected={color.toUpperCase() === swatch} style:--swatch={swatch} aria-label={`Border color ${swatch}`} aria-pressed={color.toUpperCase() === swatch} onclick={() => onChange({ borderColor: swatch })}></button>
    {/each}
    <label class="webcam-custom-color" title="Custom border color">
      <span aria-hidden="true">+</span>
      <input type="color" value={color} aria-label="Custom border color" oninput={(event) => onChange({ borderColor: event.currentTarget.value.toUpperCase() })} />
    </label>
  </div>
  <div class="webcam-control-heading webcam-radius-heading"><label for="webcam-border-width">Border width</label><output for="webcam-border-width" class="webcam-value">{width}px</output></div>
  <input id="webcam-border-width" class="webcam-range" type="range" min="0" max="12" step="1" value={width} disabled={!enabled} oninput={(event) => onChange({ borderWidth: Number(event.currentTarget.value) })} />
</section>
