<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Camera from "@lucide/svelte/icons/camera";
  import Undo2 from "@lucide/svelte/icons/undo-2";
  import Redo2 from "@lucide/svelte/icons/redo-2";
  import Maximize2 from "@lucide/svelte/icons/maximize-2";
  import Minimize2 from "@lucide/svelte/icons/minimize-2";
  import X from "@lucide/svelte/icons/x";
  import Clipboard from "@lucide/svelte/icons/clipboard";
  import Download from "@lucide/svelte/icons/download";
  import Save from "@lucide/svelte/icons/save";
  import Maximize from "@lucide/svelte/icons/minimize";
  import type { ScreenshotEditorState } from "../state/editor-state.svelte.js";

  let { editor, onUndo, onRedo, onCopy, onSaveAs, onSave, onClose }: {
    editor: ScreenshotEditorState;
    onUndo: () => void;
    onRedo: () => void;
    onCopy: () => void;
    onSaveAs: () => void;
    onSave: () => void;
    onClose: () => void;
  } = $props();
  let maximized = $state(false);
  let windowAvailable = $state(false);

  $effect(() => {
    if (!windowAvailable) return;
    void getCurrentWindow().isMaximized().then((value) => (maximized = value)).catch(() => {});
  });

  async function minimize() {
    try { await getCurrentWindow().minimize(); }
    catch (error) { editor.error = `Could not minimize the editor: ${message(error)}`; }
  }

  async function toggleMaximize() {
    try {
      await getCurrentWindow().toggleMaximize();
      maximized = await getCurrentWindow().isMaximized();
    } catch (error) { editor.error = `Could not resize the editor: ${message(error)}`; }
  }

  import { onMount } from "svelte";
  onMount(() => { windowAvailable = true; });
  function message(error: unknown): string { return error instanceof Error ? error.message : String(error); }
</script>

<header class="editor-topbar">
  <div class="brand" data-tauri-drag-region>
    <span class="brand-icon"><Camera size={20} aria-hidden="true" /></span>
    <span class="brand-name">RecoDex</span>
    <span class="brand-divider"></span>
    <span class="editor-title">Screenshot Editor</span>
  </div>

  <div class="edit-commands" aria-label="Edit commands">
    <button type="button" class="icon-button" aria-label="Undo" title="Undo (Ctrl+Z)" disabled={!editor.canUndo || editor.busy} onclick={onUndo}><Undo2 size={18} /></button>
    <button type="button" class="icon-button" aria-label="Redo" title="Redo (Ctrl+Shift+Z)" disabled={!editor.canRedo || editor.busy} onclick={onRedo}><Redo2 size={18} /></button>
    <span class="command-divider"></span>
    <select
      aria-label="Zoom level"
      value={editor.zoomMode === "fit" ? "fit" : String(editor.zoom)}
      disabled={!editor.image}
      onchange={(event) => {
        const value = event.currentTarget.value;
        if (value === "fit") editor.resetZoom();
        else { editor.zoomMode = "custom"; editor.zoom = Number(value); editor.panX = 0; editor.panY = 0; }
      }}
    >
      <option value="fit">Fit</option>
      {#if editor.zoomMode === "custom" && ![0.25, 0.5, 0.75, 1, 1.5, 2].includes(editor.zoom)}
        <option value={String(editor.zoom)}>{Math.round(editor.zoom * 100)}%</option>
      {/if}
      <option value="0.25">25%</option><option value="0.5">50%</option><option value="0.75">75%</option>
      <option value="1">100%</option><option value="1.5">150%</option><option value="2">200%</option>
    </select>
    <button type="button" class="fit-button" title="Fit to screen" aria-label="Fit to screen" onclick={() => editor.resetZoom()}><Maximize2 size={16} /></button>
  </div>

  <div class="export-commands">
    <button type="button" class="secondary-button" disabled={!editor.image || editor.busy} onclick={onCopy}><Clipboard size={16} /><span>Copy</span></button>
    <button type="button" class="secondary-button save-as" disabled={!editor.image || editor.busy} onclick={onSaveAs}><Download size={16} /><span>Save As…</span></button>
    <button type="button" class="save-button" disabled={!editor.image || editor.busy} onclick={onSave}><Save size={16} /><span>{editor.busy ? "Saving…" : "Save"}</span></button>
  </div>

  <div class="window-controls" aria-label="Window controls">
    <button type="button" title="Minimize" aria-label="Minimize" onclick={() => void minimize()}><Minimize2 size={14} /></button>
    <button type="button" title={maximized ? "Restore" : "Maximize"} aria-label={maximized ? "Restore" : "Maximize"} onclick={() => void toggleMaximize()}>
      {#if maximized}<Maximize size={13} />{:else}<Maximize2 size={13} />{/if}
    </button>
    <button type="button" class="close-window" title="Close editor" aria-label="Close editor" onclick={onClose}><X size={15} /></button>
  </div>
</header>

<style>
  .editor-topbar { display:flex; align-items:center; gap:15px; width:100%; min-width:0; height:68px; padding:0 14px 0 19px; background:#111319; border-bottom:1px solid rgba(255,255,255,.08); user-select:none; }
  .brand { display:flex; align-items:center; gap:9px; min-width:230px; height:100%; }
  .brand-icon { display:grid; width:28px; height:28px; place-items:center; border-radius:8px; color:#ff454d; background:rgba(255,48,56,.13); }
  .brand-name { color:#f5f5f7; font-size:14px; font-weight:700; letter-spacing:.01em; }
  .brand-divider,.command-divider { width:1px; height:23px; background:rgba(255,255,255,.11); }
  .editor-title { color:#a1a4af; font-size:12px; }
  .edit-commands,.export-commands,.window-controls { display:flex; align-items:center; gap:6px; }
  .edit-commands { margin:0 auto; }
  button { display:inline-flex; align-items:center; justify-content:center; gap:7px; height:34px; border:1px solid transparent; border-radius:8px; color:#c0c2c9; background:transparent; transition:background .14s,border-color .14s,color .14s; }
  button:hover:not(:disabled) { color:#fff; background:#20232c; }
  button:disabled { color:#5f626d; cursor:not-allowed; }
  .icon-button,.fit-button { width:34px; }
  select { height:34px; min-width:83px; padding:0 9px; border:1px solid #2b2e37; border-radius:8px; color:#f5f5f7; background:#1c1f27; font-size:12px; }
  .secondary-button { padding:0 10px; border-color:#30333d; background:#1b1e25; color:#e0e1e5; font-size:12px; }
  .save-button { padding:0 13px; color:#fff; background:#ff3038; font-size:12px; font-weight:600; }
  .save-button:hover:not(:disabled) { background:#ff454d; }
  .save-button:disabled { color:#a5a5aa; background:#672a30; }
  .window-controls { margin-left:2px; padding-left:9px; border-left:1px solid rgba(255,255,255,.09); }
  .window-controls button { width:29px; height:29px; }
  .window-controls .close-window:hover { color:#fff; background:#d92d36; }
  @media (max-width:1120px) { .brand { min-width:192px; } .save-as span { display:none; } .secondary-button { padding:0 8px; } .editor-topbar { gap:8px; padding-left:12px; } }
  @media (max-width:990px) { .editor-title,.brand-divider { display:none; } .brand { min-width:48px; } .window-controls { gap:1px; padding-left:4px; } }
</style>
