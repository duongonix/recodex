<script lang="ts">
  import { onMount } from "svelte";
  import Minus from "@lucide/svelte/icons/minus";
  import Square from "@lucide/svelte/icons/square";
  import X from "@lucide/svelte/icons/x";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  type Props = { onClose: () => void };
  let { onClose }: Props = $props();
  let maximized = $state(false);
  let headerElement: HTMLElement | undefined = $state();

  onMount(() => {
    const handleMouseDown = (event: MouseEvent) => {
      if (event.button !== 0 || (event.target instanceof Element && event.target.closest(".window-controls"))) return;
      void getCurrentWindow().startDragging().catch((cause: unknown) => {
        console.error("[RecoDex][video-editor] Could not start dragging the editor window", cause);
      });
    };

    headerElement?.addEventListener("mousedown", handleMouseDown);
    return () => headerElement?.removeEventListener("mousedown", handleMouseDown);
  });

  async function minimize() {
    try {
      await getCurrentWindow().minimize();
    } catch (cause) {
      console.error("[RecoDex][video-editor] Could not minimize the editor window", cause);
    }
  }

  async function toggleMaximize() {
    const window = getCurrentWindow();
    try {
      await window.toggleMaximize();
      maximized = await window.isMaximized();
    } catch (cause) {
      console.error("[RecoDex][video-editor] Could not toggle editor window maximize", cause);
    }
  }

</script>

<header class="editor-header" bind:this={headerElement}>
  <div class="brand">
    <div class="brand-mark" aria-hidden="true"><span></span></div>
    <span class="brand-name">Recodex</span>
    <span class="divider"></span>
    <span class="editor-label">Video Editor</span>
  </div>
  <div class="window-controls">
    <button type="button" aria-label="Minimize" title="Minimize" onclick={() => void minimize()}><Minus size={15} /></button>
    <button type="button" aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"} onclick={() => void toggleMaximize()}><Square size={12} /></button>
    <button class="close" type="button" aria-label="Save and close editor" title="Save and close" onclick={onClose}><X size={17} /></button>
  </div>
</header>

<style>
  .editor-header { display:flex; height:56px; align-items:center; justify-content:space-between; padding:0 16px; border-bottom:1px solid #292b32; background:#101116; user-select:none; }
  .brand { display:flex; min-width:0; height:100%; align-items:center; gap:12px; }
  .brand-mark { display:grid; width:32px; height:32px; place-items:center; border:1px solid rgb(255 48 56 / 55%); border-radius:8px; background:rgb(255 48 56 / 10%); }
  .brand-mark span { width:14px; height:14px; border:2px solid #ff3038; border-radius:50%; box-shadow:0 0 0 3px rgb(255 48 56 / 18%); }
  .brand-name { color:#f5f5f7; font-size:14px; font-weight:650; }
  .divider { width:1px; height:20px; background:#30313a; }
  .editor-label { color:#c5c6cd; font-size:13px; }
  .window-controls { display:flex; height:100%; align-items:center; gap:4px; }
  .window-controls button { display:grid; width:34px; height:30px; place-items:center; border:0; border-radius:6px; background:transparent; color:#a0a2ad; cursor:pointer; }
  .window-controls button:hover { background:#252730; color:#f5f5f7; }
  .window-controls button.close:hover { background:#ff3038; color:white; }
  @media (prefers-reduced-motion:no-preference) { .window-controls button { transition:background 150ms,color 150ms; } }
</style>
