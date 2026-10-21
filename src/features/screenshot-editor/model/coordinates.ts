import type { CropRect, EditorDocument } from "./document.js";

export function cropBounds(document: EditorDocument, width: number, height: number): CropRect {
  return document.crop ?? { x: 0, y: 0, width, height };
}

export function normalizeRect(x1: number, y1: number, x2: number, y2: number): CropRect {
  const x = Math.min(x1, x2);
  const y = Math.min(y1, y2);
  return { x, y, width: Math.abs(x2 - x1), height: Math.abs(y2 - y1) };
}

export function clampRect(rect: CropRect, width: number, height: number): CropRect {
  const x = Math.max(0, Math.min(width - 1, rect.x));
  const y = Math.max(0, Math.min(height - 1, rect.y));
  const right = Math.max(x + 1, Math.min(width, rect.x + rect.width));
  const bottom = Math.max(y + 1, Math.min(height, rect.y + rect.height));
  return { x, y, width: right - x, height: bottom - y };
}

