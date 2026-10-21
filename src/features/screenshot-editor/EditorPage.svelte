<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { save as saveFile } from "@tauri-apps/plugin-dialog";
  import EditorTopbar from "./components/EditorTopbar.svelte";
  import ToolRail from "./components/ToolRail.svelte";
  import EditorCanvas from "./components/EditorCanvas.svelte";
  import Inspector from "./components/Inspector.svelte";
  import ScreenshotStrip from "./components/ScreenshotStrip.svelte";
  import UnsavedChangesDialog from "./components/UnsavedChangesDialog.svelte";
  import { clampRect } from "./model/coordinates.js";
  import {
    closeScreenshotEditor,
    exportScreenshotEditor,
    getScreenshotEditorMetadata,
    getScreenshotEditorSource
  } from "../../lib/services/ipc/screenshot.js";
  import { ScreenshotEditorState, snapshotEditorValue } from "./state/editor-state.svelte.js";
  import { renderScreenshot } from "./services/export.js";

  const editor = new ScreenshotEditorState();
  let sessionId = "";
  let loading = $state(true);
  let dialogOpen = $state(false);
  let closeInProgress = false;
  let removeCloseListener: (() => void) | undefined;

  onMount(() => {
    let disposed = false;
    void initialize().catch((error: unknown) => {
      if (!disposed) editor.error = errorMessage(error);
    }).finally(() => {
      if (!disposed) loading = false;
    });
    return () => {
      disposed = true;
      removeCloseListener?.();
      editor.dispose();
    };
  });

  async function initialize() {
    sessionId = new URL(window.location.href).searchParams.get("session") ?? "";
    if (!sessionId) throw new Error("This editor window has no screenshot session. Capture a screenshot again.");

    if (isTauri()) {
      removeCloseListener = await getCurrentWindow().onCloseRequested((event) => {
        event.preventDefault();
        requestClose();
      });
    }

    const metadata = await getScreenshotEditorMetadata(sessionId);
    const bytes = await getScreenshotEditorSource(sessionId);
    const sourceBuffer = new Uint8Array(bytes.byteLength);
    sourceBuffer.set(bytes);
    const bitmap = await createImageBitmap(new Blob([sourceBuffer.buffer], { type: "image/png" }));
    if (bitmap.width !== metadata.width || bitmap.height !== metadata.height) {
      bitmap.close();
      throw new Error("The screenshot dimensions do not match its editor session.");
    }
    editor.initialize(metadata, bitmap);
    window.addEventListener("keydown", handleKeyDown);
    removeCloseListener = chainCleanup(removeCloseListener, () => window.removeEventListener("keydown", handleKeyDown));
  }

  function chainCleanup(first: (() => void) | undefined, second: () => void): () => void {
    return () => { first?.(); second(); };
  }

  function requestClose() {
    if (editor.dirty) dialogOpen = true;
    else void approveClose();
  }

  async function approveClose() {
    if (closeInProgress) return;
    closeInProgress = true;
    editor.error = null;
    try {
      if (sessionId) await closeScreenshotEditor(sessionId);
      if (isTauri()) {
        const removeListeners = removeCloseListener;
        removeCloseListener = undefined;
        removeListeners?.();
        await getCurrentWindow().close();
      } else {
        window.close();
      }
    } catch (error) {
      closeInProgress = false;
      editor.error = `Could not close the screenshot editor: ${errorMessage(error)}`;
    }
  }

  async function save() {
    await exportImage("save");
  }

  async function saveAs() {
    const metadata = editor.metadata;
    if (!metadata || editor.busy) return;
    editor.error = null;
    let selectedPath: string | null;
    try {
      selectedPath = await saveFile({
        title: "Save edited screenshot",
        defaultPath: metadata.suggestedFilename,
        filters: [{ name: metadata.format.toUpperCase(), extensions: [metadata.format] }]
      });
    } catch (error) {
      editor.error = `Could not open the Save As dialog: ${errorMessage(error)}`;
      return;
    }
    if (!selectedPath) return;
    await exportImage("save-as", selectedPath);
  }

  async function copy() {
    await exportImage("copy");
  }

  async function exportImage(operation: "save" | "save-as" | "copy", outputPath?: string) {
    if (!editor.image || !editor.metadata || editor.busy) return;
    editor.busy = true;
    editor.error = null;
    editor.success = null;
    try {
      const bytes = await renderScreenshot(editor.image, editor.document, editor.metadata.format);
      const result = await exportScreenshotEditor(sessionId, operation, bytes, outputPath);
      if (operation === "copy") {
        editor.success = `Copied ${result.width} × ${result.height}px to clipboard.`;
        return;
      }
      editor.markSaved();
      if (result.outputPath) editor.success = `Saved to ${result.outputPath}`;
      if (operation === "save-as" && outputPath) {
        const name = outputPath.split(/[\\/]/).filter(Boolean).at(-1);
        if (name && editor.metadata) editor.metadata = { ...editor.metadata, suggestedFilename: name };
      }
      if (dialogOpen) {
        dialogOpen = false;
        await approveClose();
      }
    } catch (error) {
      editor.error = errorMessage(error);
    } finally {
      editor.busy = false;
    }
  }

  function applyCrop() {
    const metadata = editor.metadata;
    const requested = editor.pendingCrop;
    if (!metadata || !requested) return;
    const crop = clampRect(requested, metadata.width, metadata.height);
    editor.commit({ ...snapshotEditorValue(editor.document), crop });
    editor.pendingCrop = null;
    editor.selectedTool = "select";
    editor.resetZoom();
  }

  function cancelCrop() {
    editor.pendingCrop = null;
    editor.selectedTool = "select";
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (isTypingTarget(event.target)) return;
    const key = event.key.toLowerCase();
    const mod = event.ctrlKey || event.metaKey;
    if (event.key === "Escape") {
      if (dialogOpen) dialogOpen = false;
      else if (editor.pendingCrop) cancelCrop();
      else if (!editor.textEdit) requestClose();
      return;
    }
    if (mod && key === "z") {
      event.preventDefault();
      if (event.shiftKey) editor.redo(); else editor.undo();
    } else if (mod && key === "y") {
      event.preventDefault();
      editor.redo();
    } else if (mod && key === "s") {
      event.preventDefault();
      void save();
    } else if ((key === "delete" || key === "backspace") && editor.selectedId) {
      event.preventDefault();
      editor.removeSelected();
    } else {
      const tools = { v: "select", p: "pen", a: "arrow", r: "rectangle", t: "text", b: "blur", c: "crop" } as const;
      if (!mod && key in tools) editor.selectedTool = tools[key as keyof typeof tools];
    }
  }

  function isTypingTarget(target: EventTarget | null): boolean {
    return target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));
  }

  function errorMessage(error: unknown): string {
    if (typeof error === "string") return error;
    if (error instanceof Error) return error.message;
    return "The screenshot editor request could not be completed.";
  }
</script>

<svelte:head>
  <title>Screenshot Editor · RecoDex</title>
  <meta name="description" content="Edit and export a screenshot in RecoDex." />
</svelte:head>

<main class="editor-window" aria-label="RecoDex Screenshot Editor">
  <EditorTopbar
    {editor}
    onUndo={() => editor.undo()}
    onRedo={() => editor.redo()}
    onCopy={() => void copy()}
    onSaveAs={() => void saveAs()}
    onSave={() => void save()}
    onClose={requestClose}
  />
  <section class="editor-workspace" aria-label="Screenshot workspace">
    <ToolRail {editor} />
    <div class="canvas-column">
      <div class="canvas-area">
        {#if editor.image}
          <EditorCanvas {editor} />
        {:else}
          <div class="loading-editor" role="status">
            {#if loading}<span class="loading-spinner"></span><p>Loading screenshot…</p>
            {:else}<div class="load-error"><strong>Screenshot editor could not open</strong><p>{editor.error ?? "The screenshot source is unavailable."}</p></div>{/if}
          </div>
        {/if}
        {#if editor.selectedTool === "crop"}
          <div class="crop-actions">
            {#if editor.pendingCrop}
              <span>{Math.round(editor.pendingCrop.width)} × {Math.round(editor.pendingCrop.height)} px</span>
              <button type="button" class="crop-apply" onclick={applyCrop}>Apply crop</button>
              <button type="button" class="crop-cancel" onclick={cancelCrop}>Cancel</button>
            {:else}
              <span>Drag over the screenshot to set crop bounds.</span>
              <button type="button" class="crop-cancel" onclick={cancelCrop}>Cancel</button>
            {/if}
          </div>
        {/if}
      </div>
      <ScreenshotStrip {editor} />
    </div>
    <Inspector {editor} />
  </section>
  <UnsavedChangesDialog
    open={dialogOpen}
    saving={editor.busy}
    onSave={() => void save()}
    onDiscard={() => { dialogOpen = false; void approveClose(); }}
    onCancel={() => (dialogOpen = false)}
  />
</main>

<style>
  :global(html), :global(body) { width:100%; height:100%; min-width:960px; min-height:620px; overflow:hidden; background:#0e1015; }
  :global(body) { margin:0; }
  .editor-window { display:grid; width:100vw; height:100vh; min-width:960px; min-height:620px; grid-template-rows:68px minmax(0,1fr); overflow:hidden; color:#f5f5f7; background:#0e1015; }
  .editor-workspace { display:grid; min-width:0; min-height:0; grid-template-columns:90px minmax(0,1fr) 340px; }
  .canvas-column { display:grid; min-width:0; min-height:0; grid-template-rows:minmax(0,1fr) 84px; }
  .canvas-area { position:relative; min-width:0; min-height:0; overflow:hidden; }
  .loading-editor { display:grid; width:100%; height:100%; place-content:center; justify-items:center; gap:13px; background:radial-gradient(ellipse at center,#1a1d25 0,#0e1015 65%); color:#a1a4af; text-align:center; }
  .loading-editor p { margin:0; font-size:13px; }
  .loading-spinner { width:25px; height:25px; border:2px solid #343741; border-top-color:#ff3038; border-radius:50%; animation:spin .8s linear infinite; }
  .load-error { max-width:460px; padding:22px; }
  .load-error strong { color:#ff747b; font-size:15px; }
  .load-error p { margin-top:8px; line-height:1.5; }
  .crop-actions { position:absolute; z-index:6; left:50%; bottom:18px; display:flex; align-items:center; gap:7px; max-width:calc(100% - 30px); padding:7px 9px 7px 13px; border:1px solid #373a43; border-radius:9px; background:rgba(20,22,29,.96); box-shadow:0 8px 28px #0008; transform:translateX(-50%); color:#cacbd1; font-size:11px; }
  .crop-actions span { margin-right:5px; white-space:nowrap; }
  .crop-actions button { height:29px; padding:0 10px; border:1px solid transparent; border-radius:6px; color:#f5f5f7; font-size:11px; }
  .crop-apply { background:#ff3038; }
  .crop-cancel { background:#242730; }
  @keyframes spin { to { transform:rotate(360deg); } }
  @media (max-width:1140px) { .editor-workspace { grid-template-columns:82px minmax(0,1fr) 300px; } }
</style>
