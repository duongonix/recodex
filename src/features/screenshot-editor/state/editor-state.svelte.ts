import type { EditorSessionMetadata } from "../../../lib/services/ipc/screenshot.js";
import {
  createDocument,
  defaultStyle,
  type EditorDocument,
  type EditorObject,
  type EditorTool,
  type InspectorTab,
  type ObjectStyle
} from "../model/document.js";
import { DocumentHistory } from "./history.js";

export class ScreenshotEditorState {
  metadata = $state<EditorSessionMetadata | null>(null);
  image = $state<ImageBitmap | null>(null);
  document = $state<EditorDocument>(createDocument());
  selectedTool = $state<EditorTool>("select");
  inspectorTab = $state<InspectorTab>("style");
  selectedId = $state<string | null>(null);
  pendingCrop = $state<{ x: number; y: number; width: number; height: number } | null>(null);
  zoomMode = $state<"fit" | "custom">("fit");
  zoom = $state(1);
  panX = $state(0);
  panY = $state(0);
  busy = $state(false);
  error = $state<string | null>(null);
  success = $state<string | null>(null);
  textEdit = $state<{ id: string; isNew: boolean } | null>(null);
  revision = $state(0);
  dirty = $state(false);
  canUndo = $state(false);
  canRedo = $state(false);
  private savedSnapshot = JSON.stringify(this.document);
  private history = new DocumentHistory<EditorDocument>((value) => snapshotEditorValue(value));

  initialize(metadata: EditorSessionMetadata, image: ImageBitmap): void {
    this.metadata = metadata;
    this.image = image;
    this.document = createDocument();
    this.savedSnapshot = JSON.stringify(this.document);
    this.dirty = false;
    this.revision += 1;
  }

  commit(next: EditorDocument): void {
    this.history.push(this.document);
    this.document = snapshotEditorValue(next);
    this.revision += 1;
    this.refreshHistoryState();
    this.refreshDirtyState();
    this.error = null;
    this.success = null;
  }

  updateObject(id: string, update: (object: EditorObject) => EditorObject): void {
    const next = snapshotEditorValue(this.document);
    const index = next.objects.findIndex((object) => object.id === id);
    if (index < 0) return;
    next.objects[index] = update(next.objects[index]);
    this.commit(next);
  }

  updateSelectedStyle(update: Partial<ObjectStyle>): void {
    const selected = this.document.objects.find((object) => object.id === this.selectedId);
    if (!selected) {
      const next = snapshotEditorValue(this.document);
      next.defaultStyle = { ...next.defaultStyle, ...update };
      this.commit(next);
      return;
    }
    if (selected.type === "text" || selected.type === "blur") return;
    this.updateObject(selected.id, (object) => {
      if (object.type === "text" || object.type === "blur") return object;
      return { ...object, style: { ...object.style, ...update } };
    });
  }

  selectedObject(): EditorObject | null {
    return this.document.objects.find((object) => object.id === this.selectedId) ?? null;
  }

  selectedStyle(): ObjectStyle {
    const selected = this.selectedObject();
    return selected && selected.type !== "text" && selected.type !== "blur"
      ? selected.style
      : this.document.defaultStyle;
  }

  addObject(object: EditorObject): void {
    this.commit({ ...snapshotEditorValue(this.document), objects: [...this.document.objects, object] });
    this.selectedId = object.id;
  }

  removeSelected(): void {
    if (!this.selectedId) return;
    this.commit({
      ...snapshotEditorValue(this.document),
      objects: this.document.objects.filter((object) => object.id !== this.selectedId)
    });
    this.selectedId = null;
  }

  undo(): void {
    const previous = this.history.undo(this.document);
    if (!previous) return;
    this.document = previous;
    this.revision += 1;
    this.refreshHistoryState();
    this.refreshDirtyState();
    this.selectedId = null;
  }

  redo(): void {
    const next = this.history.redo(this.document);
    if (!next) return;
    this.document = next;
    this.revision += 1;
    this.refreshHistoryState();
    this.refreshDirtyState();
    this.selectedId = null;
  }

  markSaved(): void {
    this.savedSnapshot = JSON.stringify(this.document);
    this.dirty = false;
    this.success = "Screenshot saved.";
  }

  resetZoom(): void {
    this.zoomMode = "fit";
    this.zoom = 1;
    this.panX = 0;
    this.panY = 0;
  }

  newStyle(): ObjectStyle { return defaultStyle(); }

  dispose(): void {
    this.image?.close();
    this.image = null;
  }

  private refreshHistoryState(): void {
    this.canUndo = this.history.canUndo;
    this.canRedo = this.history.canRedo;
  }

  private refreshDirtyState(): void {
    this.dirty = JSON.stringify(this.document) !== this.savedSnapshot;
  }
}

export function newObjectId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `object-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export function snapshotEditorValue<T>(value: T) {
  return $state.snapshot(value);
}

