<script lang="ts">
  import type { EditorBackground } from "../model/document.js";
  import { snapshotEditorValue, type ScreenshotEditorState } from "../state/editor-state.svelte.js";

  let { editor }: { editor: ScreenshotEditorState } = $props();
  const colors = ["transparent", "#FFFFFF", "#F2F2F2", "#17191F", "#232A37", "#1E293B", "#2D1B24"];
  const background = $derived(editor.document.background);

  function update(patch: Partial<EditorBackground>) {
    editor.commit({ ...snapshotEditorValue(editor.document), background: { ...editor.document.background, ...patch } });
  }
</script>

<section class="background-controls">
  <div class="property-group">
    <h3>Canvas color</h3>
    <div class="swatches">
      {#each colors as color (color)}
        <button type="button" class="swatch" class:transparent-swatch={color === "transparent"} class:swatch-active={background.color === color} style:background={color === "transparent" ? undefined : color} aria-label={color === "transparent" ? "Transparent canvas" : `Set canvas color ${color}`} aria-pressed={background.color === color} onclick={() => update({ color })}>{color === "transparent" ? "∅" : ""}</button>
      {/each}
      <label class="custom-color" title="Choose custom canvas color"><span>+</span><input type="color" aria-label="Choose custom canvas color" value={background.color === "transparent" ? "#17191F" : background.color} onchange={(event) => update({ color: event.currentTarget.value })} /></label>
    </div>
    {#if editor.metadata?.format === "jpg" && background.color === "transparent"}
      <p class="warning">JPEG has no transparency; export uses a white canvas.</p>
    {/if}
  </div>

  <label class="range-control"><span><span>Padding</span><b>{background.padding}px</b></span>
    <input type="range" min="0" max="200" value={background.padding} onchange={(event) => update({ padding: Number(event.currentTarget.value) })} />
  </label>
  <label class="range-control"><span><span>Image corner radius</span><b>{background.radius}px</b></span>
    <input type="range" min="0" max="40" value={background.radius} onchange={(event) => update({ radius: Number(event.currentTarget.value) })} />
  </label>

  <label class="toggle-row"><span><b>Drop shadow</b><small>Add depth around the screenshot.</small></span>
    <input type="checkbox" checked={background.shadow} onchange={(event) => update({ shadow: event.currentTarget.checked })} />
  </label>
  {#if background.shadow}
    <label class="range-control"><span><span>Shadow intensity</span><b>{background.shadowIntensity}%</b></span>
      <input type="range" min="0" max="100" value={background.shadowIntensity} onchange={(event) => update({ shadowIntensity: Number(event.currentTarget.value) })} />
    </label>
  {/if}
</section>

<style>
  .background-controls { display:flex; flex-direction:column; gap:17px; }
  .property-group { display:flex; flex-direction:column; gap:12px; }
  h3 { margin:0; color:#f5f5f7; font-size:13px; font-weight:600; }
  .swatches { display:flex; flex-wrap:wrap; gap:8px; align-items:center; }
  .swatch,.custom-color { position:relative; display:grid; width:24px; height:24px; place-items:center; border:1px solid #484b55; border-radius:7px; color:#a1a4af; background:#1c1f27; font-size:13px; }
  .transparent-swatch { background:linear-gradient(135deg, #252831 45%, #70747d 46%, #70747d 54%, #252831 55%); }
  .swatch-active { outline:2px solid #ff3038; outline-offset:2px; }
  .custom-color input { position:absolute; inset:0; width:100%; height:100%; opacity:0; cursor:pointer; }
  .range-control { display:flex; flex-direction:column; gap:9px; color:#a1a4af; font-size:12px; }
  .range-control > span { display:flex; justify-content:space-between; }
  .range-control b { color:#f5f5f7; font-weight:500; }
  input[type="range"] { width:100%; accent-color:#ff3038; }
  .toggle-row { display:flex; align-items:center; justify-content:space-between; gap:14px; color:#f5f5f7; font-size:12px; }
  .toggle-row span { display:flex; flex-direction:column; gap:4px; }
  .toggle-row small { color:#a1a4af; font-size:11px; }
  .toggle-row input { accent-color:#ff3038; }
  .warning { margin:0; color:#ffb76b; font-size:11px; line-height:1.4; }
</style>
