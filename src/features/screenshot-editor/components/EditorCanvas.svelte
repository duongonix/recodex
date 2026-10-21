<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import X from "@lucide/svelte/icons/x";
  import Check from "@lucide/svelte/icons/check";
  import { EditorCanvasEngine } from "../services/canvas-engine.js";
  import type { TextObject } from "../model/document.js";
  import type { ScreenshotEditorState } from "../state/editor-state.svelte.js";

  let { editor }: { editor: ScreenshotEditorState } = $props();
  let container: HTMLDivElement;
  let textarea = $state<HTMLTextAreaElement | undefined>();
  let engine: EditorCanvasEngine | null = null;
  let textValue = $state("");
  let editPosition = $state({ left: 0, top: 0, width: 220, height: 72 });
  let editingId = $state<string | null>(null);
  let originalText = "";
  let newText = false;

  onMount(() => {
    let disposed = false;
    void import("konva").then(({ default: Konva }) => {
      if (disposed) return;
      engine = new EditorCanvasEngine(
        Konva,
        editor,
        beginTextEdit,
        (crop) => (editor.pendingCrop = crop),
        (message) => (editor.error = message)
      );
      engine.mount(container);
      engine.update();
      container.addEventListener("recodex-edit-text", handleEditTextRequest);
    }).catch((error: unknown) => {
      editor.error = `Could not initialize the screenshot canvas: ${error instanceof Error ? error.message : String(error)}`;
    });
    return () => {
      disposed = true;
      container?.removeEventListener("recodex-edit-text", handleEditTextRequest);
      engine?.destroy();
      engine = null;
    };
  });

  $effect(() => {
    const revision = JSON.stringify([
      editor.revision,
      editor.selectedId,
      editor.selectedTool,
      editor.pendingCrop,
      editor.zoom,
      editor.zoomMode,
      editor.document.background
    ]);
    if (!revision || !engine) return;
    engine?.update();
  });

  onDestroy(() => engine?.destroy());

  function beginTextEdit(object: TextObject, isNew: boolean, position: { left: number; top: number; width: number; height: number }) {
    editingId = object.id;
    newText = isNew;
    originalText = object.text;
    textValue = object.text;
    editPosition = {
      left: Math.max(0, Math.min(container.clientWidth - 180, position.left)),
      top: Math.max(0, Math.min(container.clientHeight - 76, position.top)),
      width: Math.max(180, Math.min(container.clientWidth - position.left, position.width)),
      height: Math.max(64, Math.min(container.clientHeight - position.top, position.height))
    };
    editor.textEdit = { id: object.id, isNew };
    void tick().then(() => textarea?.focus());
  }

  function handleEditTextRequest(event: Event) {
    if (!(event instanceof CustomEvent) || typeof event.detail !== "string" || !engine) return;
    const object = editor.document.objects.find((candidate) => candidate.id === event.detail);
    if (object?.type === "text") {
      beginTextEdit(object, false, engine.screenPosition(object.x, object.y, object.width, object.height));
    }
  }

  function commitText() {
    if (!editingId) return;
    const id = editingId;
    if (newText && !textValue.trim()) {
      editor.selectedId = id;
      editor.removeSelected();
    } else {
      editor.updateObject(id, (object) => object.type === "text"
        ? { ...object, text: textValue, height: Math.max(object.height, textValue.split("\n").length * object.fontSize * 1.25 + object.padding * 2) }
        : object);
    }
    finishTextEdit();
  }

  function cancelText() {
    if (newText && editingId) {
      editor.selectedId = editingId;
      editor.removeSelected();
    }
    textValue = originalText;
    finishTextEdit();
  }

  function finishTextEdit() {
    editingId = null;
    editor.textEdit = null;
  }

  function handleTextKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      cancelText();
    } else if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      commitText();
    }
  }
</script>

<div class="canvas-region">
  <div class="canvas-host" bind:this={container} aria-label="Screenshot editing canvas"></div>
  {#if editingId}
    <div
      class="text-editor"
      style:left="{editPosition.left}px"
      style:top="{editPosition.top}px"
      style:width="{editPosition.width}px"
      style:min-height="{editPosition.height}px"
    >
      <textarea
        bind:this={textarea}
        bind:value={textValue}
        aria-label="Edit screenshot annotation"
        placeholder="Type annotation…"
        onkeydown={handleTextKeydown}
      ></textarea>
      <div class="text-editor-actions">
        <span>Ctrl + Enter to apply · Esc to cancel</span>
        <button type="button" class="text-cancel" aria-label="Cancel text edit" onclick={cancelText}><X size={14} /></button>
        <button type="button" class="text-apply" aria-label="Apply text edit" onclick={commitText}><Check size={14} /></button>
      </div>
    </div>
  {/if}
</div>

<style>
  .canvas-region { position: relative; width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; }
  .canvas-host {
    width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden;
    background-color: #101218;
    background-image: linear-gradient(45deg, #1a1d25 25%, transparent 25%), linear-gradient(-45deg, #1a1d25 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #1a1d25 75%), linear-gradient(-45deg, transparent 75%, #1a1d25 75%);
    background-size: 20px 20px;
    background-position: 0 0, 0 10px, 10px -10px, -10px 0;
  }
  .canvas-host :global(canvas) { display: block; }
  .text-editor {
    position: absolute; z-index: 4; display: flex; flex-direction: column; gap: 6px;
    max-width: calc(100% - 16px); padding: 6px; border: 1px solid #ff3038;
    border-radius: 8px; background: rgba(17, 19, 25, .96); box-shadow: 0 8px 28px #0009;
  }
  textarea {
    width: 100%; min-width: 160px; min-height: 42px; resize: both; border: 0; outline: none;
    color: #f5f5f7; background: transparent; font: 16px/1.3 "Segoe UI", system-ui, sans-serif;
  }
  .text-editor-actions { display: flex; align-items: center; justify-content: flex-end; gap: 6px; color: #a1a4af; font-size: 11px; }
  .text-editor-actions span { margin-right: auto; }
  .text-editor-actions button { display: grid; width: 24px; height: 24px; place-items: center; border: 0; border-radius: 5px; color: white; }
  .text-cancel { background: #282b34; }
  .text-apply { background: #ff3038; }
</style>
