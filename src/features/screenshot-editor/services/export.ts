import type { ScreenshotFormat } from "../../../lib/types/settings.js";
import type { EditorDocument, EditorObject, TextObject } from "../model/document.js";
import { cropBounds } from "../model/coordinates.js";

const MAX_PIXELS = 64_000_000;

export async function renderScreenshot(
  source: ImageBitmap,
  model: EditorDocument,
  format: ScreenshotFormat
): Promise<Uint8Array> {
  const crop = cropBounds(model, source.width, source.height);
  const padding = model.background.padding;
  const width = Math.ceil(crop.width + padding * 2);
  const height = Math.ceil(crop.height + padding * 2);
  if (width < 2 || height < 2 || width * height > MAX_PIXELS) {
    throw new Error("The edited screenshot exceeds the supported export dimensions.");
  }

  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d", { alpha: format === "png" });
  if (!context) throw new Error("Could not initialize the screenshot exporter.");

  if (format === "jpg" && model.background.color === "transparent") {
    context.fillStyle = "#FFFFFF";
    context.fillRect(0, 0, width, height);
  } else if (model.background.color !== "transparent") {
    context.fillStyle = model.background.color;
    context.fillRect(0, 0, width, height);
  }

  drawScreenshot(context, source, model, crop, padding);
  for (const object of model.objects) {
    if (!object.visible || object.type !== "blur") continue;
    drawBlur(context, source, object, crop, padding);
  }
  for (const object of model.objects) {
    if (!object.visible || object.type === "blur") continue;
    drawObject(context, object, crop, padding);
  }

  const blob = await canvasToBlob(canvas, format);
  return new Uint8Array(await blob.arrayBuffer());
}

function drawScreenshot(
  context: CanvasRenderingContext2D,
  source: ImageBitmap,
  model: EditorDocument,
  crop: { x: number; y: number; width: number; height: number },
  padding: number
): void {
  const x = padding;
  const y = padding;
  context.save();
  if (model.background.shadow) {
    const opacity = Math.max(0, Math.min(1, model.background.shadowIntensity / 100));
    context.shadowColor = `rgba(0, 0, 0, ${opacity})`;
    context.shadowBlur = Math.max(1, model.background.shadowIntensity * 0.65);
    context.shadowOffsetY = Math.max(1, model.background.shadowIntensity * 0.2);
    context.fillStyle = model.background.color === "transparent" ? "rgba(0,0,0,.001)" : model.background.color;
    context.beginPath();
    context.roundRect(x, y, crop.width, crop.height, model.background.radius);
    context.fill();
  }
  context.restore();
  context.save();
  context.beginPath();
  context.roundRect(x, y, crop.width, crop.height, model.background.radius);
  context.clip();
  context.drawImage(source, crop.x, crop.y, crop.width, crop.height, x, y, crop.width, crop.height);
  context.restore();
}

function drawBlur(
  context: CanvasRenderingContext2D,
  source: ImageBitmap,
  object: Extract<EditorObject, { type: "blur" }>,
  crop: { x: number; y: number; width: number; height: number },
  padding: number
): void {
  const x = object.x - crop.x + padding;
  const y = object.y - crop.y + padding;
  context.save();
  context.beginPath();
  context.rect(x, y, object.width, object.height);
  context.clip();
  context.filter = `blur(${Math.max(1, object.strength)}px)`;
  context.drawImage(source, padding - crop.x, padding - crop.y);
  context.restore();
}

function drawObject(
  context: CanvasRenderingContext2D,
  object: Exclude<EditorObject, { type: "blur" }>,
  crop: { x: number; y: number; width: number; height: number },
  padding: number
): void {
  context.save();
  const offsetX = padding - crop.x;
  const offsetY = padding - crop.y;
  if (object.type === "pen") {
    if (object.points.length < 4) return context.restore();
    applyLineStyle(context, object.style);
    context.beginPath();
    context.moveTo(object.points[0] + offsetX, object.points[1] + offsetY);
    for (let index = 2; index < object.points.length; index += 2) {
      context.lineTo(object.points[index] + offsetX, object.points[index + 1] + offsetY);
    }
    context.stroke();
  } else if (object.type === "arrow") {
    applyLineStyle(context, object.style);
    const x1 = object.x1 + offsetX;
    const y1 = object.y1 + offsetY;
    const x2 = object.x2 + offsetX;
    const y2 = object.y2 + offsetY;
    context.beginPath();
    context.moveTo(x1, y1);
    context.lineTo(x2, y2);
    context.stroke();
    if (object.style.startArrow) drawArrowHead(context, x2, y2, x1, y1, object.style.width);
    if (object.style.endArrow) drawArrowHead(context, x1, y1, x2, y2, object.style.width);
  } else if (object.type === "rectangle") {
    const x = object.x + offsetX;
    const y = object.y + offsetY;
    context.globalAlpha = object.style.fillOpacity / 100;
    if (object.style.fillOpacity > 0) {
      context.fillStyle = object.style.fillColor;
      context.beginPath();
      context.roundRect(x, y, object.width, object.height, object.radius);
      context.fill();
    }
    context.globalAlpha = object.style.opacity / 100;
    context.strokeStyle = object.style.color;
    context.lineWidth = object.style.width;
    setLineDash(context, object.style.lineStyle);
    context.beginPath();
    context.roundRect(x, y, object.width, object.height, object.radius);
    context.stroke();
  } else {
    drawText(context, object, offsetX, offsetY);
  }
  context.restore();
}

function drawText(
  context: CanvasRenderingContext2D,
  object: TextObject,
  offsetX: number,
  offsetY: number
): void {
  const x = object.x + offsetX;
  const y = object.y + offsetY;
  const lines = wrapText(context, object);
  const lineHeight = Math.ceil(object.fontSize * 1.25);
  const textHeight = lines.length * lineHeight;
  if (object.backgroundOpacity > 0) {
    context.globalAlpha = object.backgroundOpacity / 100;
    context.fillStyle = object.backgroundColor;
    context.fillRect(x, y, object.width, Math.max(object.height, textHeight + object.padding * 2));
  }
  context.globalAlpha = 1;
  context.fillStyle = object.color;
  context.font = `${object.fontWeight} ${object.fontSize}px ${quoteFontFamily(object.fontFamily)}`;
  context.textBaseline = "top";
  context.textAlign = object.align;
  const textX = object.align === "left" ? x + object.padding
    : object.align === "center" ? x + object.width / 2
      : x + object.width - object.padding;
  lines.forEach((line, index) => context.fillText(line, textX, y + object.padding + index * lineHeight));
}

function wrapText(context: CanvasRenderingContext2D, object: TextObject): string[] {
  context.font = `${object.fontWeight} ${object.fontSize}px ${quoteFontFamily(object.fontFamily)}`;
  const maxWidth = Math.max(1, object.width - object.padding * 2);
  const output: string[] = [];
  for (const paragraph of object.text.split("\n")) {
    let line = "";
    for (const word of paragraph.split(/\s+/)) {
      const candidate = line ? `${line} ${word}` : word;
      if (line && context.measureText(candidate).width > maxWidth) {
        output.push(line);
        line = word;
      } else line = candidate;
    }
    output.push(line);
  }
  return output;
}

function applyLineStyle(context: CanvasRenderingContext2D, style: {
  color: string; width: number; opacity: number; lineStyle: "solid" | "dashed" | "dotted";
}): void {
  context.globalAlpha = style.opacity / 100;
  context.strokeStyle = style.color;
  context.lineWidth = style.width;
  context.lineCap = "round";
  context.lineJoin = "round";
  setLineDash(context, style.lineStyle);
}

function setLineDash(context: CanvasRenderingContext2D, style: "solid" | "dashed" | "dotted"): void {
  context.setLineDash(style === "dashed" ? [12, 8] : style === "dotted" ? [2, 8] : []);
}

function drawArrowHead(
  context: CanvasRenderingContext2D,
  fromX: number,
  fromY: number,
  toX: number,
  toY: number,
  width: number
): void {
  const angle = Math.atan2(toY - fromY, toX - fromX);
  const size = Math.max(10, width * 3.8);
  context.beginPath();
  context.moveTo(toX, toY);
  context.lineTo(toX - size * Math.cos(angle - Math.PI / 6), toY - size * Math.sin(angle - Math.PI / 6));
  context.moveTo(toX, toY);
  context.lineTo(toX - size * Math.cos(angle + Math.PI / 6), toY - size * Math.sin(angle + Math.PI / 6));
  context.stroke();
}

function quoteFontFamily(family: string): string {
  return `"${family.replaceAll('"', "")}"`;
}

function canvasToBlob(canvas: HTMLCanvasElement, format: ScreenshotFormat): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => blob ? resolve(blob) : reject(new Error("The screenshot could not be encoded.")),
      format === "png" ? "image/png" : "image/jpeg",
      format === "jpg" ? 0.92 : undefined
    );
  });
}

