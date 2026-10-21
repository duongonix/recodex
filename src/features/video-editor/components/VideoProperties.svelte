<script lang="ts">
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import type { EditorSettings } from "../types.js";
  type Props = { settings: EditorSettings; onChange: (patch: Partial<EditorSettings>) => void };
  let { settings, onChange }: Props = $props();
  const numeric = (key: "positionX" | "positionY" | "cropLeft" | "cropRight" | "cropTop" | "cropBottom", event: Event) => onChange({ [key]: Number((event.currentTarget as HTMLInputElement).value) });
</script>

<div class="property-content">
  <section class="group">
    <h3>Transform</h3>
    <label class="range-row"><span>Scale</span><input type="range" min="25" max="300" step="1" value={settings.scale} oninput={(event) => onChange({ scale: Number(event.currentTarget.value) })} /><output>{settings.scale}%</output></label>
    <div class="pair-row"><label>Position X<input type="number" min="-4000" max="4000" value={settings.positionX} oninput={(event) => numeric("positionX", event)} /></label><label>Position Y<input type="number" min="-4000" max="4000" value={settings.positionY} oninput={(event) => numeric("positionY", event)} /></label></div>
    <label class="range-row"><span>Rotation</span><input type="range" min="-180" max="180" step="1" value={settings.rotation} oninput={(event) => onChange({ rotation: Number(event.currentTarget.value) })} /><output>{settings.rotation}°</output></label>
  </section>
  <section class="group">
    <h3>Video</h3>
    <label class="range-row"><span>Speed</span><input type="range" min="0.25" max="2" step="0.05" value={settings.speed} oninput={(event) => onChange({ speed: Number(event.currentTarget.value) })} /><output>{settings.speed.toFixed(2)}×</output></label>
    <label class="range-row"><span>Opacity</span><input type="range" min="0" max="100" value={settings.opacity} oninput={(event) => onChange({ opacity: Number(event.currentTarget.value) })} /><output>{settings.opacity}%</output></label>
  </section>
  <section class="group">
    <div class="section-heading"><h3>Crop</h3><label class="switch"><input type="checkbox" checked={settings.cropEnabled} onchange={(event) => onChange({ cropEnabled: event.currentTarget.checked })} /><span></span></label></div>
    <div class="crop-grid" class:disabled={!settings.cropEnabled}>
      <label>Left<input type="number" min="0" value={settings.cropLeft} disabled={!settings.cropEnabled} oninput={(event) => numeric("cropLeft", event)} /></label>
      <label>Right<input type="number" min="0" value={settings.cropRight} disabled={!settings.cropEnabled} oninput={(event) => numeric("cropRight", event)} /></label>
      <label>Top<input type="number" min="0" value={settings.cropTop} disabled={!settings.cropEnabled} oninput={(event) => numeric("cropTop", event)} /></label>
      <label>Bottom<input type="number" min="0" value={settings.cropBottom} disabled={!settings.cropEnabled} oninput={(event) => numeric("cropBottom", event)} /></label>
    </div>
    <button class="reset-button" type="button" onclick={() => onChange({ scale: 100, positionX: 0, positionY: 0, rotation: 0, opacity: 100, cropEnabled: false, cropLeft: 0, cropRight: 0, cropTop: 0, cropBottom: 0 })}><RotateCcw size={14} /> Reset transform</button>
  </section>
</div>

<style>
  .property-content { display:grid; align-content:start; gap:20px; padding:18px 14px; }
  .group { display:grid; gap:14px; }
  h3 { margin:0; color:#f5f5f7; font-size:13px; font-weight:620; }
  .range-row { display:grid; grid-template-columns:58px minmax(50px,1fr) 46px; align-items:center; gap:8px; color:#c5c6cd; font-size:11px; }
  input[type=range] { min-width:40px; accent-color:#ff3038; }
  output { color:#f5f5f7; text-align:right; font-variant-numeric:tabular-nums; }
  .pair-row,.crop-grid { display:grid; grid-template-columns:1fr 1fr; gap:8px; }
  .pair-row label,.crop-grid label { display:grid; gap:5px; color:#989aa7; font-size:10px; }
  input[type=number] { width:100%; height:30px; box-sizing:border-box; border:1px solid #292b32; border-radius:7px; background:#1b1d24; color:#f5f5f7; padding:0 8px; font:inherit; font-size:11px; }
  .section-heading { display:flex; align-items:center; justify-content:space-between; }
  .switch input { position:absolute; opacity:0; }
  .switch span { display:block; width:32px; height:18px; border-radius:999px; background:#30313a; cursor:pointer; }
  .switch span::after { display:block; width:14px; height:14px; margin:2px; border-radius:50%; background:#f5f5f7; content:""; transition:transform 150ms; }
  .switch input:checked + span { background:#ff3038; }
  .switch input:checked + span::after { transform:translateX(14px); }
  .disabled { opacity:.45; }
  .reset-button { display:flex; align-items:center; justify-content:center; gap:7px; height:30px; border:1px solid #292b32; border-radius:7px; background:#1b1d24; color:#c5c6cd; font:inherit; font-size:11px; cursor:pointer; }
  .reset-button:hover { color:white; background:#252730; }
</style>
