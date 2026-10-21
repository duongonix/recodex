<script lang="ts">
  import type { ScreenshotEditorState } from "../state/editor-state.svelte.js";

  let { editor }: { editor: ScreenshotEditorState } = $props();
  const selectedText = $derived(editor.selectedObject()?.type === "text" ? editor.selectedObject() as Extract<ReturnType<ScreenshotEditorState["selectedObject"]>, { type: "text" }> : null);

  function update(patch: Partial<NonNullable<typeof selectedText>>) {
    if (!selectedText) return;
    editor.updateObject(selectedText.id, (object) => object.type === "text" ? { ...object, ...patch } : object);
  }
</script>

{#if selectedText}
  <section class="text-controls">
    <label>Font family
      <select value={selectedText.fontFamily} onchange={(event) => update({ fontFamily: event.currentTarget.value })}>
        <option>Segoe UI</option><option>Arial</option><option>Verdana</option><option>Georgia</option><option>Courier New</option>
      </select>
    </label>
    <label class="range-row"><span>Size <b>{selectedText.fontSize}px</b></span>
      <input type="range" min="10" max="96" value={selectedText.fontSize} onchange={(event) => update({ fontSize: Number(event.currentTarget.value) })} />
    </label>
    <div class="text-row">
      <label class="check-control"><input type="checkbox" checked={selectedText.fontWeight === "bold"} onchange={(event) => update({ fontWeight: event.currentTarget.checked ? "bold" : "normal" })} /> Bold</label>
      <label class="color-picker" title="Text color"><span>Color</span><input type="color" value={selectedText.color} aria-label="Text color" onchange={(event) => update({ color: event.currentTarget.value })} /></label>
    </div>
    <fieldset>
      <legend>Alignment</legend>
      <div class="alignment-buttons">
        {#each ["left", "center", "right"] as alignment (alignment)}
          <button type="button" class:active={selectedText.align === alignment} aria-pressed={selectedText.align === alignment} onclick={() => update({ align: alignment as "left" | "center" | "right" })}>{alignment}</button>
        {/each}
      </div>
    </fieldset>
    <label class="range-row"><span>Text background <b>{selectedText.backgroundOpacity}%</b></span>
      <input type="range" min="0" max="100" value={selectedText.backgroundOpacity} onchange={(event) => update({ backgroundOpacity: Number(event.currentTarget.value) })} />
    </label>
    {#if selectedText.backgroundOpacity > 0}
      <label class="color-picker"><span>Background color</span><input type="color" value={selectedText.backgroundColor} aria-label="Text background color" onchange={(event) => update({ backgroundColor: event.currentTarget.value })} /></label>
    {/if}
    <label class="range-row"><span>Padding <b>{selectedText.padding}px</b></span>
      <input type="range" min="0" max="30" value={selectedText.padding} onchange={(event) => update({ padding: Number(event.currentTarget.value) })} />
    </label>
    <button type="button" class="edit-text" onclick={() => {
      editor.textEdit = { id: selectedText.id, isNew: false };
      const node = document.querySelector(".canvas-host");
      node?.dispatchEvent(new CustomEvent("recodex-edit-text", { detail: selectedText.id }));
    }}>Edit text…</button>
  </section>
{:else}
  <p class="panel-hint">Select a text annotation or choose the Text tool and click the screenshot.</p>
{/if}

<style>
  .text-controls { display:flex; flex-direction:column; gap:17px; }
  label { display:flex; flex-direction:column; gap:8px; color:#a1a4af; font-size:12px; }
  select { height:35px; padding:0 10px; border:1px solid #2b2e37; border-radius:8px; color:#f5f5f7; background:#1c1f27; }
  .range-row { gap:9px; }
  .range-row span { display:flex; justify-content:space-between; }
  .range-row b { color:#f5f5f7; font-weight:500; }
  input[type="range"] { width:100%; accent-color:#ff3038; }
  .text-row { display:flex; align-items:center; justify-content:space-between; }
  .check-control { flex-direction:row; align-items:center; color:#e0e1e5; }
  .check-control input { accent-color:#ff3038; }
  .color-picker { flex-direction:row; align-items:center; justify-content:space-between; gap:16px; }
  input[type="color"] { width:30px; height:26px; padding:2px; border:1px solid #41444c; border-radius:6px; background:#1c1f27; }
  fieldset { margin:0; padding:0; border:0; }
  legend { margin-bottom:8px; color:#a1a4af; font-size:12px; }
  .alignment-buttons { display:grid; grid-template-columns:repeat(3,1fr); gap:5px; }
  .alignment-buttons button { min-height:32px; border:1px solid transparent; border-radius:6px; color:#a1a4af; background:#1c1f27; text-transform:capitalize; font-size:11px; }
  .alignment-buttons button.active { color:#ff454d; border-color:#ff3038; background:rgba(255,48,56,.12); }
  .edit-text { align-self:flex-start; padding:7px 10px; border:1px solid #373a44; border-radius:7px; color:#f5f5f7; background:#1c1f27; }
  .panel-hint { margin:8px 0; color:#a1a4af; font-size:12px; line-height:1.5; }
</style>
