<script lang="ts">
  import type { Component } from "svelte";
  import Palette from "@lucide/svelte/icons/palette";
  import Type from "@lucide/svelte/icons/type";
  import PanelsTopLeft from "@lucide/svelte/icons/panels-top-left";
  import StylePanel from "./StylePanel.svelte";
  import TextPanel from "./TextPanel.svelte";
  import BackgroundPanel from "./BackgroundPanel.svelte";
  import type { InspectorTab } from "../model/document.js";
  import type { ScreenshotEditorState } from "../state/editor-state.svelte.js";

  let { editor }: { editor: ScreenshotEditorState } = $props();
  const tabs: { id: InspectorTab; label: string; icon: Component }[] = [
    { id: "style", label: "Style", icon: Palette },
    { id: "text", label: "Text", icon: Type },
    { id: "background", label: "Background", icon: PanelsTopLeft }
  ];
</script>

<aside class="inspector" aria-label="Screenshot properties">
  <div class="inspector-tabs" role="tablist" aria-label="Property groups">
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        class:active={editor.inspectorTab === tab.id}
        role="tab"
        aria-selected={editor.inspectorTab === tab.id}
        onclick={() => (editor.inspectorTab = tab.id)}
      ><tab.icon size={16} aria-hidden="true" /><span>{tab.label}</span></button>
    {/each}
  </div>
  <div class="inspector-content">
    {#if editor.inspectorTab === "style"}
      <StylePanel {editor} />
    {:else if editor.inspectorTab === "text"}
      <TextPanel {editor} />
    {:else}
      <BackgroundPanel {editor} />
    {/if}
  </div>
  <div class="inspector-footer">
    {#if editor.selectedObject()}
      <span class="object-marker"></span>
      <span>{editor.selectedObject()?.type}</span>
    {:else}
      <span>{editor.selectedTool === "select" ? "Select an object to edit" : `Ready · ${editor.selectedTool}`}</span>
    {/if}
  </div>
</aside>

<style>
  .inspector { display:flex; flex-direction:column; min-width:0; min-height:0; background:#15171d; border-left:1px solid rgba(255,255,255,.08); }
  .inspector-tabs { flex:none; display:grid; grid-template-columns:repeat(3,1fr); height:52px; border-bottom:1px solid rgba(255,255,255,.08); }
  .inspector-tabs button { display:flex; align-items:center; justify-content:center; gap:7px; border:0; border-bottom:2px solid transparent; color:#a1a4af; background:transparent; font-size:12px; transition:color .15s,background .15s,border-color .15s; }
  .inspector-tabs button:hover { color:#f5f5f7; background:#1b1e26; }
  .inspector-tabs button.active { color:#ff454d; border-bottom-color:#ff3038; background:rgba(255,48,56,.08); }
  .inspector-content { flex:1; min-height:0; overflow:auto; padding:18px 18px 22px; }
  .inspector-footer { display:flex; align-items:center; gap:8px; flex:none; height:35px; padding:0 16px; border-top:1px solid rgba(255,255,255,.07); color:#9296a1; font-size:11px; text-transform:capitalize; }
  .object-marker { width:7px; height:7px; border-radius:50%; background:#ff3038; box-shadow:0 0 8px #ff303877; }
  @media (max-width:1150px) { .inspector-tabs button { gap:4px; font-size:11px; } .inspector-content { padding:14px 13px; } }
</style>
