<script lang="ts">
  import Film from "@lucide/svelte/icons/film";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import Crop from "@lucide/svelte/icons/crop";
  import VideoProperties from "./VideoProperties.svelte";
  import AudioProperties from "./AudioProperties.svelte";
  import CanvasProperties from "./CanvasProperties.svelte";
  import type { EditorSettings } from "../types.js";
  type Props = { settings: EditorSettings; onChange: (patch: Partial<EditorSettings>) => void };
  let { settings, onChange }: Props = $props();
  let activeTab = $state<"video" | "audio" | "canvas">("video");
</script>

<aside class="property-panel" aria-label="Video properties">
  <div class="tabs" role="tablist" aria-label="Property categories">
    <button role="tab" aria-selected={activeTab === "video"} class:active={activeTab === "video"} onclick={() => activeTab = "video"}><Film size={16} /><span>Video</span></button>
    <button role="tab" aria-selected={activeTab === "audio"} class:active={activeTab === "audio"} onclick={() => activeTab = "audio"}><Volume2 size={16} /><span>Audio</span></button>
    <button role="tab" aria-selected={activeTab === "canvas"} class:active={activeTab === "canvas"} onclick={() => activeTab = "canvas"}><Crop size={16} /><span>Canvas</span></button>
  </div>
  <div class="panel-scroll" role="tabpanel">
    {#if activeTab === "video"}<VideoProperties {settings} {onChange} />{:else if activeTab === "audio"}<AudioProperties {settings} {onChange} />{:else}<CanvasProperties {settings} {onChange} />{/if}
  </div>
</aside>

<style>
  .property-panel { display:grid; min-width:280px; max-width:340px; min-height:0; grid-template-rows:60px minmax(0,1fr); background:#17191f; }
  .tabs { display:grid; grid-template-columns:repeat(3,1fr); padding:4px; border-bottom:1px solid #292b32; }
  .tabs button { display:flex; flex-direction:column; align-items:center; justify-content:center; gap:5px; border:1px solid transparent; border-radius:8px; background:transparent; color:#989aa7; font:inherit; font-size:10px; cursor:pointer; }
  .tabs button:hover { color:#f5f5f7; background:#1b1d24; }
  .tabs button.active { border-color:rgb(255 48 56 / 65%); background:rgb(255 48 56 / 16%); color:#ff6269; }
  .panel-scroll { min-height:0; overflow:auto; scrollbar-color:#484a55 #17191f; scrollbar-width:thin; }
  @media (max-width:900px) { .property-panel { min-width:240px; } }
</style>
