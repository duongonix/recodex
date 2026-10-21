<script lang="ts">
  import type { EditorSettings } from "../types.js";
  type Props = { settings: EditorSettings; onChange: (patch: Partial<EditorSettings>) => void };
  let { settings, onChange }: Props = $props();
</script>

<div class="canvas-properties">
  <h3>Canvas</h3>
  <label>Aspect ratio<select value={settings.canvasAspect} onchange={(event) => onChange({ canvasAspect: event.currentTarget.value as EditorSettings["canvasAspect"] })}>
    <option value="source">Original</option><option value="16:9">16:9</option><option value="9:16">9:16</option><option value="1:1">1:1</option>
  </select></label>
  <label>Fit<select value={settings.fit} onchange={(event) => onChange({ fit: event.currentTarget.value as EditorSettings["fit"] })}><option value="contain">Fit</option><option value="cover">Fill</option></select></label>
  <label class="color-row">Background color<input aria-label="Background color" type="color" value={settings.backgroundColor} oninput={(event) => onChange({ backgroundColor: event.currentTarget.value })} /></label>
  <p>Canvas settings are included in the exported video.</p>
</div>

<style>
  .canvas-properties { display:grid; align-content:start; gap:15px; padding:18px 14px; }
  h3 { margin:0; color:#f5f5f7; font-size:13px; }
  label { display:grid; gap:6px; color:#989aa7; font-size:11px; }
  select { height:32px; border:1px solid #292b32; border-radius:7px; background:#1b1d24; color:#f5f5f7; padding:0 8px; font:inherit; }
  .color-row { display:flex; align-items:center; justify-content:space-between; }
  input[type=color] { width:36px; height:28px; padding:2px; border:1px solid #292b32; border-radius:6px; background:#1b1d24; cursor:pointer; }
  p { color:#989aa7; font-size:11px; line-height:1.5; }
</style>
