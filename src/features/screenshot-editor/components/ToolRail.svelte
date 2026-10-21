<script lang="ts">
  import type { Component } from "svelte";
  import MousePointer2 from "@lucide/svelte/icons/mouse-pointer-2";
  import Pencil from "@lucide/svelte/icons/pencil";
  import MoveUpRight from "@lucide/svelte/icons/move-up-right";
  import Square from "@lucide/svelte/icons/square";
  import Type from "@lucide/svelte/icons/type";
  import Scan from "@lucide/svelte/icons/scan";
  import Crop from "@lucide/svelte/icons/crop";
  import type { EditorTool } from "../model/document.js";
  import type { ScreenshotEditorState } from "../state/editor-state.svelte.js";

  let { editor }: { editor: ScreenshotEditorState } = $props();
  const tools: { id: EditorTool; label: string; shortcut: string; icon: Component }[] = [
    { id: "select", label: "Select", shortcut: "V", icon: MousePointer2 },
    { id: "pen", label: "Pen", shortcut: "P", icon: Pencil },
    { id: "arrow", label: "Arrow", shortcut: "A", icon: MoveUpRight },
    { id: "rectangle", label: "Rectangle", shortcut: "R", icon: Square },
    { id: "text", label: "Text", shortcut: "T", icon: Type },
    { id: "blur", label: "Blur", shortcut: "B", icon: Scan },
    { id: "crop", label: "Crop", shortcut: "C", icon: Crop }
  ];
</script>

<nav class="tool-rail" aria-label="Screenshot editor tools">
  <div class="tool-list">
    {#each tools as tool (tool.id)}
      <button
        type="button"
        class="tool-button"
        class:tool-active={editor.selectedTool === tool.id}
        aria-pressed={editor.selectedTool === tool.id}
        title={`${tool.label} (${tool.shortcut})`}
        onclick={() => {
          editor.selectedTool = tool.id;
          if (tool.id === "text") editor.inspectorTab = "text";
          if (tool.id === "crop") editor.inspectorTab = "style";
        }}
      >
        <tool.icon size={20} strokeWidth={1.8} aria-hidden="true" />
        <span>{tool.label}</span>
      </button>
    {/each}
  </div>
  <label class="color-control" title="Default annotation color">
    <span class="color-swatch" style:background={editor.selectedStyle().color}></span>
    <span>Color</span>
    <input
      type="color"
      aria-label="Default annotation color"
      value={editor.selectedStyle().color}
      onchange={(event) => editor.updateSelectedStyle({ color: event.currentTarget.value })}
    />
  </label>
</nav>

<style>
  .tool-rail { display:flex; flex-direction:column; justify-content:space-between; min-width:90px; padding:14px 9px 12px; background:#111319; border-right:1px solid rgba(255,255,255,.08); }
  .tool-list { display:flex; flex-direction:column; gap:5px; }
  .tool-button { position:relative; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:5px; width:70px; min-height:62px; border:1px solid transparent; border-radius:10px; color:#a1a4af; background:transparent; font-size:11px; transition:background .14s,border-color .14s,color .14s; }
  .tool-button:hover { color:#f5f5f7; background:#1c1f27; }
  .tool-button.tool-active { color:#ff454d; background:rgba(255,48,56,.12); border-color:rgba(255,48,56,.75); box-shadow:0 0 14px rgba(255,48,56,.13); }
  .tool-button:focus-visible { outline:2px solid #ff3038; outline-offset:2px; }
  .color-control { position:relative; display:flex; flex-direction:column; align-items:center; gap:5px; color:#a1a4af; font-size:10px; cursor:pointer; }
  .color-swatch { width:23px; height:23px; border:2px solid #f5f5f7; border-radius:7px; box-shadow:0 0 0 1px #373a43; }
  .color-control input { position:absolute; inset:0; width:100%; height:100%; opacity:0; cursor:pointer; }
</style>
