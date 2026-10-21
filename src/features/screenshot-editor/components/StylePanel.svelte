<script lang="ts">
  import type { ScreenshotEditorState } from "../state/editor-state.svelte.js";

  let { editor }: { editor: ScreenshotEditorState } = $props();
  const swatches = ["#FF3038", "#FF8A3D", "#FFD43B", "#42D392", "#4BA3FF", "#986BFF", "#F5F5F7"];
  const selected = $derived(editor.selectedObject());
  const style = $derived(editor.selectedStyle());
  const selectedRectangle = $derived(selected?.type === "rectangle" ? selected : null);
  const selectedBlur = $derived(selected?.type === "blur" ? selected : null);

  function updateStyle(patch: Partial<typeof style>) { editor.updateSelectedStyle(patch); }
  function updateRectangleRadius(value: number) {
    if (selectedRectangle) editor.updateObject(selectedRectangle.id, (object) => object.type === "rectangle" ? { ...object, radius: value } : object);
    else updateStyle({ radius: value });
  }
</script>

{#if selected?.type === "text"}
  <p class="panel-hint">Text appearance is edited in the Text tab.</p>
{:else if selectedBlur}
  <label class="range-control">
    <span><span>Blur strength</span><b>{selectedBlur.strength}px</b></span>
    <input type="range" min="2" max="40" value={selectedBlur.strength} onchange={(event) => {
      const value = Number(event.currentTarget.value);
      editor.updateObject(selectedBlur!.id, (object) => object.type === "blur" ? { ...object, strength: value } : object);
    }} />
  </label>
  <p class="panel-hint">The selected area is blurred from the original screenshot during export.</p>
{:else}
  <section class="property-group">
    <h3>Color</h3>
    <div class="swatches" aria-label="Stroke color presets">
      {#each swatches as color (color)}
        <button type="button" class="swatch" class:swatch-active={style.color === color} style:background={color} aria-label={`Set color ${color}`} aria-pressed={style.color === color} onclick={() => updateStyle({ color })}></button>
      {/each}
      <label class="custom-color" title="Choose custom color">
        <span>+</span>
        <input type="color" aria-label="Choose custom stroke color" value={style.color} onchange={(event) => updateStyle({ color: event.currentTarget.value })} />
      </label>
    </div>
  </section>

  <label class="range-control">
    <span><span>Stroke width</span><b>{style.width}px</b></span>
    <input type="range" min="1" max="24" value={style.width} onchange={(event) => updateStyle({ width: Number(event.currentTarget.value) })} />
  </label>
  <label class="range-control">
    <span><span>Opacity</span><b>{style.opacity}%</b></span>
    <input type="range" min="0" max="100" value={style.opacity} onchange={(event) => updateStyle({ opacity: Number(event.currentTarget.value) })} />
  </label>

  <label class="select-control">Line style
    <select value={style.lineStyle} onchange={(event) => updateStyle({ lineStyle: event.currentTarget.value as typeof style.lineStyle })}>
      <option value="solid">Solid</option><option value="dashed">Dashed</option><option value="dotted">Dotted</option>
    </select>
  </label>

  {#if selected?.type === "rectangle"}
    <div class="inline-control">
      <label class="check-control"><input type="checkbox" checked={style.fillOpacity > 0} onchange={(event) => updateStyle({ fillOpacity: event.currentTarget.checked ? 28 : 0 })} /> Fill</label>
      {#if style.fillOpacity > 0}
        <label class="color-mini"><input type="color" value={style.fillColor} aria-label="Rectangle fill color" onchange={(event) => updateStyle({ fillColor: event.currentTarget.value })} /></label>
      {/if}
    </div>
    <label class="range-control">
      <span><span>Corner radius</span><b>{selectedRectangle?.radius ?? 0}px</b></span>
      <input type="range" min="0" max="40" value={selectedRectangle?.radius ?? 0} onchange={(event) => updateRectangleRadius(Number(event.currentTarget.value))} />
    </label>
  {/if}

  {#if selected?.type === "arrow"}
    <section class="property-group arrow-heads">
      <h3>Arrowheads</h3>
      <label class="check-control"><input type="checkbox" checked={style.startArrow} onchange={(event) => updateStyle({ startArrow: event.currentTarget.checked })} /> Start</label>
      <label class="check-control"><input type="checkbox" checked={style.endArrow} onchange={(event) => updateStyle({ endArrow: event.currentTarget.checked })} /> End</label>
    </section>
  {/if}
  {#if !selected}
    <p class="panel-hint">Default style applies to the next shape you draw.</p>
  {/if}
{/if}

<style>
  .property-group { display:flex; flex-direction:column; gap:11px; margin-bottom:19px; }
  .property-group h3 { margin:0; color:#f5f5f7; font-size:13px; font-weight:600; }
  .swatches { display:flex; flex-wrap:wrap; gap:8px; align-items:center; }
  .swatch { width:22px; height:22px; border:2px solid transparent; border-radius:7px; box-shadow:0 0 0 1px #3b3e47; }
  .swatch-active { outline:2px solid #ff3038; outline-offset:2px; }
  .custom-color,.color-mini { position:relative; display:grid; width:24px; height:24px; place-items:center; border:1px solid #484b55; border-radius:7px; color:#f5f5f7; background:#1c1f27; cursor:pointer; }
  .custom-color input,.color-mini input { position:absolute; inset:0; width:100%; height:100%; opacity:0; cursor:pointer; }
  .range-control { display:flex; flex-direction:column; gap:9px; margin:17px 0; color:#a1a4af; font-size:12px; }
  .range-control > span { display:flex; justify-content:space-between; align-items:center; }
  .range-control b { color:#f5f5f7; font-weight:500; }
  input[type="range"] { width:100%; accent-color:#ff3038; }
  .select-control { display:flex; flex-direction:column; gap:8px; margin:16px 0; color:#a1a4af; font-size:12px; }
  select { width:100%; height:35px; padding:0 10px; border:1px solid #2b2e37; border-radius:8px; color:#f5f5f7; background:#1c1f27; }
  .inline-control,.arrow-heads { display:flex; align-items:center; justify-content:space-between; gap:12px; }
  .check-control { display:flex; align-items:center; gap:8px; color:#d0d1d6; font-size:12px; }
  .check-control input { accent-color:#ff3038; }
  .panel-hint { margin:14px 0; color:#a1a4af; font-size:12px; line-height:1.5; }
</style>
