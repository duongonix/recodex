import type Konva from "konva";
import type {
  ArrowObject,
  BlurObject,
  EditorObject,
  TextObject
} from "../model/document.js";
import { cropBounds, normalizeRect } from "../model/coordinates.js";
import { canStartCanvasGesture, resolvePointerGestureAction } from "./pointer-gesture.js";
import { newObjectId, snapshotEditorValue, type ScreenshotEditorState } from "../state/editor-state.svelte.js";

type KonvaApi = typeof import("konva").default;
type TextEditPosition = { left: number; top: number; width: number; height: number };

export class EditorCanvasEngine {
  private stage: Konva.Stage | null = null;
  private backgroundLayer: Konva.Layer | null = null;
  private imageLayer: Konva.Layer | null = null;
  private objectLayer: Konva.Layer | null = null;
  private overlayLayer: Konva.Layer | null = null;
  private imageGroup: Konva.Group | null = null;
  private objectGroup: Konva.Group | null = null;
  private observer: ResizeObserver | null = null;
  private gesture: { tool: string; startX: number; startY: number; node: Konva.Shape; points?: number[] } | null = null;
  private activePointerId: number | null = null;
  private activePointerButton = 0;
  private spaceDown = false;
  private panning = false;
  private panStart = { x: 0, y: 0, panX: 0, panY: 0 };
  private width = 0;
  private height = 0;
  private fitScale = 1;
  private K: KonvaApi;
  private readonly state: ScreenshotEditorState;
  private readonly onTextEdit: (object: TextObject, isNew: boolean, position: TextEditPosition) => void;
  private readonly onCropPreview: (crop: { x: number; y: number; width: number; height: number } | null) => void;
  private readonly onError: (message: string) => void;

  constructor(
    K: KonvaApi,
    state: ScreenshotEditorState,
    onTextEdit: (object: TextObject, isNew: boolean, position: TextEditPosition) => void,
    onCropPreview: (crop: { x: number; y: number; width: number; height: number } | null) => void,
    onError: (message: string) => void
  ) {
    this.K = K;
    this.state = state;
    this.onTextEdit = onTextEdit;
    this.onCropPreview = onCropPreview;
    this.onError = onError;
  }

  mount(container: HTMLDivElement): void {
    this.stage = new this.K.Stage({ container, width: 320, height: 240 });
    this.backgroundLayer = new this.K.Layer({ listening: false });
    this.imageLayer = new this.K.Layer({ listening: false });
    this.objectLayer = new this.K.Layer();
    this.overlayLayer = new this.K.Layer();
    this.stage.add(this.backgroundLayer, this.imageLayer, this.objectLayer, this.overlayLayer);
    this.stage.container().style.touchAction = "none";
    this.stage.on("pointerdown", this.handlePointerDown);
    this.stage.on("wheel", this.handleWheel);
    window.addEventListener("pointermove", this.handlePointerMove, true);
    // Let Konva finish its own pointerup processing before we replace the draft node.
    // Window bubbling still receives releases outside the canvas (including captured pointers).
    window.addEventListener("pointerup", this.handlePointerUp);
    window.addEventListener("pointercancel", this.handlePointerCancel, true);
    window.addEventListener("lostpointercapture", this.handleLostPointerCapture, true);
    window.addEventListener("keydown", this.handleKeyDown);
    window.addEventListener("keyup", this.handleKeyUp);
    window.addEventListener("blur", this.cancelGesture);
    this.observer = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect;
      if (!rect || rect.width < 1 || rect.height < 1) return;
      this.width = rect.width;
      this.height = rect.height;
      this.stage?.size({ width: this.width, height: this.height });
      this.render();
    });
    this.observer.observe(container);
  }

  update(): void { this.render(); }

  destroy(): void {
    this.observer?.disconnect();
    window.removeEventListener("keydown", this.handleKeyDown);
    window.removeEventListener("keyup", this.handleKeyUp);
    window.removeEventListener("blur", this.cancelGesture);
    window.removeEventListener("pointermove", this.handlePointerMove, true);
    window.removeEventListener("pointerup", this.handlePointerUp);
    window.removeEventListener("pointercancel", this.handlePointerCancel, true);
    window.removeEventListener("lostpointercapture", this.handleLostPointerCapture, true);
    this.stage?.destroy();
    this.stage = null;
  }

  screenPosition(x: number, y: number, objectWidth = 1, objectHeight = 1): TextEditPosition {
    const crop = this.state.metadata
      ? cropBounds(this.state.document, this.state.metadata.width, this.state.metadata.height)
      : { x: 0, y: 0, width: 1, height: 1 };
    const scale = this.currentScale(crop.width, crop.height);
    const padding = this.state.document.background.padding;
    const transform = this.transform(crop.x, crop.y, crop.width + padding * 2, crop.height + padding * 2, scale);
    return {
      left: transform.x + x * scale,
      top: transform.y + y * scale,
      width: Math.max(1, objectWidth * scale),
      height: Math.max(1, objectHeight * scale)
    };
  }

  private readonly render = (): void => {
    const metadata = this.state.metadata;
    const image = this.state.image;
    if (!this.stage || !this.backgroundLayer || !this.imageLayer || !this.objectLayer || !this.overlayLayer || !metadata || !image) return;

    const crop = cropBounds(this.state.document, metadata.width, metadata.height);
    const background = this.state.document.background;
    const outputWidth = crop.width + background.padding * 2;
    const outputHeight = crop.height + background.padding * 2;
    const scale = this.currentScale(outputWidth, outputHeight);
    const transform = this.transform(crop.x, crop.y, outputWidth, outputHeight, scale);
    this.backgroundLayer.position(transform);
    this.backgroundLayer.scale({ x: scale, y: scale });
    this.imageLayer.position(transform);
    this.imageLayer.scale({ x: scale, y: scale });
    this.objectLayer.position(transform);
    this.objectLayer.scale({ x: scale, y: scale });

    this.backgroundLayer.destroyChildren();
    if (background.color !== "transparent") {
      this.backgroundLayer.add(new this.K.Rect({
        x: crop.x - background.padding,
        y: crop.y - background.padding,
        width: outputWidth,
        height: outputHeight,
        fill: background.color,
        listening: false
      }));
    }
    if (background.shadow) {
      this.backgroundLayer.add(new this.K.Rect({
        x: crop.x,
        y: crop.y,
        width: crop.width,
        height: crop.height,
        cornerRadius: background.radius,
        fill: background.color === "transparent" ? "#000000" : background.color,
        opacity: background.color === "transparent" ? 0.001 : 1,
        shadowColor: "#000000",
        shadowOpacity: background.shadowIntensity / 100,
        shadowBlur: background.shadowIntensity * 0.65,
        shadowOffsetY: background.shadowIntensity * 0.2,
        listening: false
      }));
    }

    this.imageLayer.destroyChildren();
    const viewport = new this.K.Group({
      clipFunc: (context) => {
        context.beginPath();
        context.roundRect(crop.x, crop.y, crop.width, crop.height, background.radius);
        context.closePath();
      },
      listening: false
    });
    viewport.add(new this.K.Image({
      image,
      x: 0,
      y: 0,
      width: metadata.width,
      height: metadata.height,
      listening: false
    }));
    this.imageLayer.add(viewport);

    this.objectLayer.destroyChildren();
    this.objectGroup = new this.K.Group({
      clipX: crop.x,
      clipY: crop.y,
      clipWidth: crop.width,
      clipHeight: crop.height
    });
    const objects = this.state.document.objects;
    for (const object of objects) {
      if (!object.visible) continue;
      if (object.type === "blur") {
        this.objectGroup.add(this.createBlurNode(object, image));
      }
    }
    for (const object of objects) {
      if (!object.visible || object.type === "blur") continue;
      this.objectGroup.add(this.createObjectNode(object));
    }
    this.objectLayer.add(this.objectGroup);

    this.overlayLayer.destroyChildren();
    if (this.state.selectedTool === "crop") this.drawCropOverlay(crop);
    const selected = objects.find((object) => object.id === this.state.selectedId);
    if (selected && this.state.selectedTool === "select") {
      const node = this.objectGroup.findOne(`#${escapeSelector(selected.id)}`);
      if (selected.type === "arrow") {
        this.addArrowEndpointHandle(selected, "start");
        this.addArrowEndpointHandle(selected, "end");
      } else if (node) {
        this.overlayLayer.add(new this.K.Transformer({
          nodes: [node],
          rotateEnabled: false,
          flipEnabled: false,
          anchorSize: 8,
          anchorStroke: "#FFFFFF",
          anchorFill: "#FF3038",
          borderStroke: "#FF3038",
          borderDash: [5, 4],
          boundBoxFunc: (oldBox, newBox) => newBox.width < 3 || newBox.height < 3 ? oldBox : newBox
        }));
      }
    }
    this.stage.batchDraw();
  };

  private createBlurNode(object: BlurObject, image: ImageBitmap): Konva.Group {
    const group = new this.K.Group({
      x: object.x,
      y: object.y,
      draggable: this.state.selectedTool === "select",
      name: "editor-object",
      id: object.id
    });
    const blur = new this.K.Image({
      image,
      x: 0,
      y: 0,
      width: object.width,
      height: object.height,
      crop: { x: object.x, y: object.y, width: object.width, height: object.height },
      filters: [this.K.Filters.Blur],
      blurRadius: object.strength,
      listening: true
    });
    blur.cache({ pixelRatio: 1 });
    group.add(blur);
    this.attachObjectEvents(group, object);
    return group;
  }

  private createObjectNode(object: Exclude<EditorObject, { type: "blur" }>): Konva.Group {
    const coordinates = objectOrigin(object);
    const group = new this.K.Group({
      x: coordinates.x,
      y: coordinates.y,
      draggable: this.state.selectedTool === "select",
      name: "editor-object",
      id: object.id
    });
    if (object.type === "pen") {
      const points = object.points.map((value, index) => value - (index % 2 === 0 ? coordinates.x : coordinates.y));
      group.add(new this.K.Line({
        points,
        stroke: object.style.color,
        strokeWidth: object.style.width,
        opacity: object.style.opacity / 100,
        dash: dashArray(object.style.lineStyle),
        lineCap: "round",
        lineJoin: "round",
        hitStrokeWidth: Math.max(12, object.style.width + 8)
      }));
    } else if (object.type === "arrow") {
      group.add(new this.K.Arrow({
        points: [object.x1 - coordinates.x, object.y1 - coordinates.y, object.x2 - coordinates.x, object.y2 - coordinates.y],
        stroke: object.style.color,
        fill: object.style.color,
        strokeWidth: object.style.width,
        opacity: object.style.opacity / 100,
        dash: dashArray(object.style.lineStyle),
        pointerAtBeginning: object.style.startArrow,
        pointerAtEnding: object.style.endArrow,
        pointerLength: Math.max(10, object.style.width * 3.8),
        pointerWidth: Math.max(10, object.style.width * 3.8),
        hitStrokeWidth: Math.max(12, object.style.width + 8)
      }));
    } else if (object.type === "rectangle") {
      group.add(new this.K.Rect({
        x: 0,
        y: 0,
        width: object.width,
        height: object.height,
        cornerRadius: object.radius,
        stroke: object.style.color,
        strokeWidth: object.style.width,
        opacity: object.style.opacity / 100,
        fill: object.style.fillColor,
        fillOpacity: object.style.fillOpacity / 100,
        dash: dashArray(object.style.lineStyle),
        hitStrokeWidth: Math.max(12, object.style.width + 8)
      }));
    } else {
      if (object.backgroundOpacity > 0) {
        group.add(new this.K.Rect({
          x: 0, y: 0, width: object.width, height: object.height,
          fill: object.backgroundColor,
          opacity: object.backgroundOpacity / 100,
          listening: false
        }));
      }
      group.add(new this.K.Text({
        x: object.padding,
        y: object.padding,
        text: object.text,
        width: Math.max(1, object.width - object.padding * 2),
        height: Math.max(1, object.height - object.padding * 2),
        fontFamily: object.fontFamily,
        fontSize: object.fontSize,
        fontStyle: object.fontWeight,
        align: object.align,
        verticalAlign: "top",
        fill: object.color,
        wrap: "word",
        lineHeight: 1.25
      }));
      group.on("dblclick dbltap", () => this.onTextEdit(
        object,
        false,
        this.screenPosition(object.x, object.y, object.width, object.height)
      ));
    }
    this.attachObjectEvents(group, object);
    return group;
  }

  private attachObjectEvents(group: Konva.Group, object: EditorObject): void {
    group.on("pointerdown", (event) => {
      if (this.state.selectedTool !== "select" || this.spaceDown || event.evt.button === 1) return;
      event.cancelBubble = true;
      this.state.selectedId = object.id;
      this.state.inspectorTab = object.type === "text" ? "text" : "style";
    });
    group.on("click tap", (event) => {
      if (this.state.selectedTool !== "select") return;
      event.cancelBubble = true;
      this.state.selectedId = object.id;
      this.state.inspectorTab = object.type === "text" ? "text" : "style";
      this.render();
    });
    group.on("dragstart", () => {
      this.state.selectedId = object.id;
    });
    group.on("dragend", () => {
      const moveX = group.x() - objectOrigin(object).x;
      const moveY = group.y() - objectOrigin(object).y;
      if (Math.abs(moveX) < 0.01 && Math.abs(moveY) < 0.01) return;
      this.state.updateObject(object.id, (current) => moveObject(current, moveX, moveY));
    });
    group.on("transformend", () => this.commitTransform(group, object));
  }

  private commitTransform(group: Konva.Group, original: EditorObject): void {
    const scaleX = group.scaleX();
    const scaleY = group.scaleY();
    const x = group.x();
    const y = group.y();
    group.scale({ x: 1, y: 1 });
    this.state.updateObject(original.id, (current) => transformObject(current, original, x, y, scaleX, scaleY));
  }

  private addArrowEndpointHandle(arrow: ArrowObject, endpoint: "start" | "end"): void {
    if (!this.overlayLayer) return;
    const initial = endpoint === "start"
      ? { x: arrow.x1, y: arrow.y1 }
      : { x: arrow.x2, y: arrow.y2 };
    let latest = initial;
    const screen = this.sourceToScreen(initial.x, initial.y);
    const handle = new this.K.Circle({
      x: screen.x,
      y: screen.y,
      radius: 6,
      fill: "#FFFFFF",
      stroke: "#FF3038",
      strokeWidth: 2,
      draggable: true,
      shadowColor: "#000000",
      shadowOpacity: 0.45,
      shadowBlur: 5
    });
    handle.on("pointerdown", (event) => { event.cancelBubble = true; });
    handle.on("dragmove", () => {
      latest = this.toSourcePoint(handle.position());
      const group = this.objectGroup?.findOne(`#${escapeSelector(arrow.id)}`) as Konva.Group | undefined;
      const shape = group?.findOne("Arrow") as Konva.Arrow | undefined;
      if (!group || !shape) return;
      const other = endpoint === "start" ? { x: arrow.x2, y: arrow.y2 } : { x: arrow.x1, y: arrow.y1 };
      const start = endpoint === "start" ? latest : other;
      const end = endpoint === "end" ? latest : other;
      const originX = Math.min(start.x, end.x);
      const originY = Math.min(start.y, end.y);
      group.position({ x: originX, y: originY });
      shape.points([start.x - originX, start.y - originY, end.x - originX, end.y - originY]);
      this.objectLayer?.batchDraw();
    });
    handle.on("dragend", () => {
      this.state.updateObject(arrow.id, (object) => object.type === "arrow"
        ? endpoint === "start" ? { ...object, x1: latest.x, y1: latest.y } : { ...object, x2: latest.x, y2: latest.y }
        : object);
    });
    this.overlayLayer.add(handle);
  }

  private readonly handlePointerDown = (event: Konva.KonvaEventObject<PointerEvent>): void => {
    if (!this.stage || !this.state.metadata || !this.objectGroup) return;
    const nativeEvent = event.evt;
    if (this.activePointerId !== null || !canStartCanvasGesture(nativeEvent.button)) return;
    const pointer = this.stage.getPointerPosition();
    if (!pointer) return;
    if (this.spaceDown || nativeEvent.button === 1) {
      this.panning = true;
      this.capturePointer(nativeEvent);
      this.panStart = { x: pointer.x, y: pointer.y, panX: this.state.panX, panY: this.state.panY };
      this.stage.container().style.cursor = "grabbing";
      nativeEvent.preventDefault();
      return;
    }

    const position = this.toSourcePoint(pointer);
    if (this.state.selectedTool === "select") {
      if (event.target === this.stage) {
        this.state.selectedId = null;
        this.render();
      }
      return;
    }
    if (this.state.selectedTool === "text") {
      this.createText(position.x, position.y);
      return;
    }
    if (!["pen", "arrow", "rectangle", "blur", "crop"].includes(this.state.selectedTool)) return;

    const tool = this.state.selectedTool;
    const style = this.state.selectedStyle();
    let node: Konva.Shape;
    if (tool === "pen") {
      node = new this.K.Line({ points: [position.x, position.y], stroke: style.color, strokeWidth: style.width, opacity: style.opacity / 100, lineCap: "round", lineJoin: "round" });
      this.gesture = { tool, startX: position.x, startY: position.y, node, points: [position.x, position.y] };
    } else if (tool === "arrow") {
      node = new this.K.Arrow({ points: [position.x, position.y, position.x, position.y], stroke: style.color, fill: style.color, strokeWidth: style.width, pointerAtEnding: true, pointerLength: Math.max(10, style.width * 3.8), pointerWidth: Math.max(10, style.width * 3.8) });
      this.gesture = { tool, startX: position.x, startY: position.y, node };
    } else if (tool === "rectangle") {
      node = new this.K.Rect({ x: position.x, y: position.y, width: 1, height: 1, stroke: style.color, strokeWidth: style.width, opacity: style.opacity / 100, dash: dashArray(style.lineStyle), fill: style.fillColor, fillOpacity: style.fillOpacity / 100 });
      this.gesture = { tool, startX: position.x, startY: position.y, node };
    } else if (tool === "blur") {
      node = new this.K.Rect({ x: position.x, y: position.y, width: 1, height: 1, fill: "rgba(255,48,56,.22)", stroke: "#FF3038", dash: [6, 4] });
      this.gesture = { tool, startX: position.x, startY: position.y, node };
    } else {
      node = new this.K.Rect({ x: position.x, y: position.y, width: 1, height: 1, fill: "rgba(255,48,56,.12)", stroke: "#FF3038", dash: [8, 5] });
      this.gesture = { tool, startX: position.x, startY: position.y, node };
    }
    this.objectGroup.add(node);
    this.objectLayer?.batchDraw();
    this.capturePointer(nativeEvent);
    nativeEvent.preventDefault();
  };

  private readonly handlePointerMove = (event: PointerEvent): void => {
    const action = resolvePointerGestureAction(
      this.activePointerId,
      event.pointerId,
      this.activePointerButton,
      event.buttons,
      "move"
    );
    if (action === "ignore") return;
    if (action === "cancel") {
      this.cancelGesture();
      return;
    }
    const pointer = this.stagePosition(event);
    if (!pointer) return;
    this.updateGestureAt(pointer);
  };

  private updateGestureAt(pointer: { x: number; y: number }): void {
    if (this.panning) {
      this.state.panX = this.panStart.panX + pointer.x - this.panStart.x;
      this.state.panY = this.panStart.panY + pointer.y - this.panStart.y;
      this.applyTransforms();
      return;
    }
    if (!this.gesture) return;
    const point = this.toSourcePoint(pointer);
    const { node, startX, startY, tool } = this.gesture;
    if (tool === "pen" && this.gesture.points) {
      this.gesture.points.push(point.x, point.y);
      (node as Konva.Line).points(this.gesture.points);
    } else if (tool === "arrow") {
      (node as Konva.Arrow).points([startX, startY, point.x, point.y]);
    } else {
      const rect = normalizeRect(startX, startY, point.x, point.y);
      node.setAttrs(rect);
    }
    this.objectLayer?.batchDraw();
  }

  private readonly handlePointerUp = (event: PointerEvent): void => {
    const action = resolvePointerGestureAction(
      this.activePointerId,
      event.pointerId,
      this.activePointerButton,
      event.buttons,
      "up"
    );
    if (action !== "finish") return;
    const pointer = this.stagePosition(event);
    if (pointer) this.updateGestureAt(pointer);
    this.activePointerId = null;
    this.activePointerButton = 0;
    if (this.panning) {
      this.panning = false;
      if (this.stage) this.stage.container().style.cursor = this.spaceDown ? "grab" : "default";
      return;
    }
    const gesture = this.gesture;
    if (!gesture) return;
    this.gesture = null;
    gesture.node.destroy();
    const end = pointer ? this.toSourcePoint(pointer) : { x: gesture.startX, y: gesture.startY };
    if (gesture.tool === "crop") {
      const rect = normalizeRect(gesture.startX, gesture.startY, end.x, end.y);
      if (rect.width > 2 && rect.height > 2) this.onCropPreview(rect);
      this.render();
      return;
    }
    if (Math.hypot(end.x - gesture.startX, end.y - gesture.startY) < 2) {
      this.objectLayer?.batchDraw();
      return;
    }
    const style = snapshotEditorValue(this.state.selectedStyle());
    if (gesture.tool === "pen" && gesture.points && gesture.points.length >= 4) {
      this.state.addObject({ id: newObjectId(), type: "pen", points: gesture.points, style, visible: true });
    } else if (gesture.tool === "arrow") {
      this.state.addObject({ id: newObjectId(), type: "arrow", x1: gesture.startX, y1: gesture.startY, x2: end.x, y2: end.y, style, visible: true });
    } else {
      const rect = normalizeRect(gesture.startX, gesture.startY, end.x, end.y);
      if (gesture.tool === "rectangle") {
        this.state.addObject({ id: newObjectId(), type: "rectangle", ...rect, radius: style.radius, style, visible: true });
      } else if (gesture.tool === "blur") {
        this.state.addObject({ id: newObjectId(), type: "blur", ...rect, strength: 14, visible: true });
      }
    }
    this.render();
  };

  private readonly handlePointerCancel = (event: PointerEvent): void => {
    const action = resolvePointerGestureAction(
      this.activePointerId,
      event.pointerId,
      this.activePointerButton,
      event.buttons,
      "cancel"
    );
    if (action === "cancel") this.cancelGesture();
  };

  private readonly handleLostPointerCapture = (event: Event): void => {
    const pointerEvent = event as PointerEvent;
    const action = resolvePointerGestureAction(
      this.activePointerId,
      pointerEvent.pointerId,
      this.activePointerButton,
      pointerEvent.buttons,
      "cancel"
    );
    if (action === "cancel") this.cancelGesture();
  };

  private capturePointer(event: PointerEvent): void {
    this.activePointerId = event.pointerId;
    this.activePointerButton = event.button;
    try {
      this.stage?.container().setPointerCapture(event.pointerId);
    } catch {
      // Window-level pointer listeners still finish or cancel the drag if capture is unavailable.
    }
  }

  private stagePosition(event: Pick<PointerEvent, "clientX" | "clientY">): { x: number; y: number } | null {
    const container = this.stage?.container();
    if (!container) return null;
    const bounds = container.getBoundingClientRect();
    return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  }

  private readonly handleWheel = (event: Konva.KonvaEventObject<WheelEvent>): void => {
    const native = event.evt;
    if (!native.ctrlKey && !native.metaKey) return;
    native.preventDefault();
    const pointer = this.stage?.getPointerPosition();
    const metadata = this.state.metadata;
    if (!pointer || !metadata) return;
    const crop = cropBounds(this.state.document, metadata.width, metadata.height);
    const padding = this.state.document.background.padding;
    const oldScale = this.currentScale(crop.width + padding * 2, crop.height + padding * 2);
    const source = this.toSourcePoint(pointer);
    const nextScale = clamp(oldScale * (native.deltaY < 0 ? 1.1 : 1 / 1.1), 0.1, 8);
    this.state.zoomMode = "custom";
    this.state.zoom = nextScale;
    const nextTransform = this.transform(
      crop.x,
      crop.y,
      crop.width + padding * 2,
      crop.height + padding * 2,
      nextScale
    );
    this.state.panX = pointer.x - source.x * nextScale - (nextTransform.x - this.state.panX);
    this.state.panY = pointer.y - source.y * nextScale - (nextTransform.y - this.state.panY);
    this.render();
  };

  private readonly handleKeyDown = (event: KeyboardEvent): void => {
    if (event.code === "Space" && !isTypingTarget(event.target)) {
      this.spaceDown = true;
      if (this.stage) this.stage.container().style.cursor = "grab";
      this.setObjectsDraggable(false);
    }
  };

  private readonly handleKeyUp = (event: KeyboardEvent): void => {
    if (event.code === "Space") {
      this.spaceDown = false;
      if (!this.panning && this.stage) this.stage.container().style.cursor = "default";
      this.setObjectsDraggable(this.state.selectedTool === "select");
    }
  };

  private setObjectsDraggable(enabled: boolean): void {
    this.objectGroup?.find(".editor-object").forEach((node) => {
      if (node instanceof this.K.Group) node.draggable(enabled);
    });
  }

  private readonly cancelGesture = (): void => {
    this.gesture?.node.destroy();
    this.gesture = null;
    this.panning = false;
    this.activePointerId = null;
    this.activePointerButton = 0;
    if (this.stage) this.stage.container().style.cursor = this.spaceDown ? "grab" : "default";
    this.onCropPreview(null);
    this.objectLayer?.batchDraw();
  };

  private createText(x: number, y: number): void {
    const style = this.state.selectedStyle();
    const object: TextObject = {
      id: newObjectId(),
      type: "text",
      x,
      y,
      width: 280,
      height: 68,
      text: "",
      fontFamily: "Segoe UI",
      fontSize: 32,
      fontWeight: "normal",
      align: "left",
      color: style.color,
      backgroundColor: "#17191F",
      backgroundOpacity: 0,
      padding: 4,
      visible: true
    };
    this.state.addObject(object);
    this.state.inspectorTab = "text";
    this.onTextEdit(object, true, this.screenPosition(x, y, object.width, object.height));
  }

  private drawCropOverlay(crop: { x: number; y: number; width: number; height: number }): void {
    const metadata = this.state.metadata;
    if (!metadata || !this.overlayLayer) return;
    const current = this.state.pendingCrop ?? crop;
    const topLeft = this.sourceToScreen(current.x, current.y);
    const bottomRight = this.sourceToScreen(current.x + current.width, current.y + current.height);
    const left = Math.min(topLeft.x, bottomRight.x);
    const top = Math.min(topLeft.y, bottomRight.y);
    const width = Math.abs(bottomRight.x - topLeft.x);
    const height = Math.abs(bottomRight.y - topLeft.y);
    const stageWidth = this.width;
    const stageHeight = this.height;
    const mask = new this.K.Shape({
      listening: false,
      sceneFunc: (context, shape) => {
        const raw = context._context;
        raw.fillStyle = "rgba(0,0,0,.58)";
        raw.fillRect(0, 0, stageWidth, stageHeight);
        raw.globalCompositeOperation = "destination-out";
        raw.fillRect(left, top, width, height);
        raw.globalCompositeOperation = "source-over";
        context.fillStrokeShape(shape);
      }
    });
    this.overlayLayer.add(mask);
    this.overlayLayer.add(new this.K.Rect({
      x: left,
      y: top,
      width,
      height,
      stroke: "#FF3038",
      strokeWidth: 1.5,
      dash: [6, 4],
      listening: false
    }));
    const text = new this.K.Text({
      x: left,
      y: Math.max(8, top - 25),
      text: `${Math.round(current.width)} × ${Math.round(current.height)} px`,
      fontFamily: "Segoe UI",
      fontSize: 13,
      fill: "#FFFFFF",
      listening: false
    });
    this.overlayLayer.add(text);
  }

  private currentScale(outputWidth: number, outputHeight: number): number {
    const fit = Math.min((this.width - 132) / outputWidth, (this.height - 120) / outputHeight);
    this.fitScale = clamp(fit || 1, 0.01, 8);
    return this.state.zoomMode === "fit" ? this.fitScale : this.state.zoom;
  }

  private transform(cropX: number, cropY: number, outputWidth: number, outputHeight: number, scale: number): { x: number; y: number } {
    const padding = this.state.document.background.padding;
    return {
      x: this.width / 2 - outputWidth * scale / 2 + (padding - cropX) * scale + this.state.panX,
      y: this.height / 2 - outputHeight * scale / 2 + (padding - cropY) * scale + this.state.panY
    };
  }

  private applyTransforms(): void {
    const metadata = this.state.metadata;
    if (!metadata) return;
    const crop = cropBounds(this.state.document, metadata.width, metadata.height);
    const padding = this.state.document.background.padding;
    const scale = this.currentScale(crop.width + padding * 2, crop.height + padding * 2);
    const transform = this.transform(crop.x, crop.y, crop.width + padding * 2, crop.height + padding * 2, scale);
    for (const layer of [this.backgroundLayer, this.imageLayer, this.objectLayer]) {
      layer?.position(transform);
      layer?.scale({ x: scale, y: scale });
    }
    this.stage?.batchDraw();
  }

  private toSourcePoint(point: { x: number; y: number }): { x: number; y: number } {
    const transformed = this.objectGroup?.getAbsoluteTransform().copy().invert().point(point);
    return transformed ?? { x: 0, y: 0 };
  }

  private sourceToScreen(x: number, y: number): { x: number; y: number } {
    return this.objectGroup?.getAbsoluteTransform().point({ x, y }) ?? { x: 0, y: 0 };
  }
}

function objectOrigin(object: EditorObject): { x: number; y: number } {
  if (object.type === "arrow") return { x: Math.min(object.x1, object.x2), y: Math.min(object.y1, object.y2) };
  if (object.type === "pen") {
    const xs = object.points.filter((_, index) => index % 2 === 0);
    const ys = object.points.filter((_, index) => index % 2 === 1);
    return { x: Math.min(...xs), y: Math.min(...ys) };
  }
  return { x: object.x, y: object.y };
}

function moveObject(object: EditorObject, dx: number, dy: number): EditorObject {
  if (object.type === "pen") {
    return { ...object, points: object.points.map((value, index) => value + (index % 2 === 0 ? dx : dy)) };
  }
  if (object.type === "arrow") {
    return { ...object, x1: object.x1 + dx, y1: object.y1 + dy, x2: object.x2 + dx, y2: object.y2 + dy };
  }
  return { ...object, x: object.x + dx, y: object.y + dy };
}

function transformObject(
  current: EditorObject,
  original: EditorObject,
  x: number,
  y: number,
  scaleX: number,
  scaleY: number
): EditorObject {
  const origin = objectOrigin(original);
  if (current.type === "pen" && original.type === "pen") {
    return { ...current, points: original.points.map((value, index) => index % 2 === 0
      ? x + (value - origin.x) * scaleX
      : y + (value - origin.y) * scaleY) };
  }
  if (current.type === "arrow" && original.type === "arrow") {
    return {
      ...current,
      x1: x + (original.x1 - origin.x) * scaleX,
      y1: y + (original.y1 - origin.y) * scaleY,
      x2: x + (original.x2 - origin.x) * scaleX,
      y2: y + (original.y2 - origin.y) * scaleY
    };
  }
  if ((current.type === "rectangle" || current.type === "text" || current.type === "blur")
    && (original.type === current.type)) {
    return { ...current, x, y, width: Math.max(4, original.width * scaleX), height: Math.max(4, original.height * scaleY) };
  }
  return current;
}

function dashArray(style: "solid" | "dashed" | "dotted"): number[] {
  return style === "dashed" ? [12, 8] : style === "dotted" ? [2, 8] : [];
}

function escapeSelector(value: string): string { return value.replaceAll("#", "\\#"); }
function clamp(value: number, min: number, max: number): number { return Math.min(max, Math.max(min, value)); }
function isTypingTarget(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));
}

