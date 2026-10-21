import { invoke } from "@tauri-apps/api/core";
import type { ScreenshotRequest, ScreenshotResult } from "../../types/screenshot.js";

export type EditorSessionMetadata = {
  sessionId: string;
  width: number;
  height: number;
  format: "png" | "jpg";
  capturedAtMs: number;
  suggestedFilename: string;
};

export type EditorExportResult = {
  width: number;
  height: number;
  outputPath: string | null;
  copiedToClipboard: boolean;
};

export function captureScreenshot(request: ScreenshotRequest): Promise<ScreenshotResult> {
  return invoke<ScreenshotResult>("capture_screenshot", { request });
}

export function getScreenshotEditorMetadata(sessionId: string): Promise<EditorSessionMetadata> {
  return invoke<EditorSessionMetadata>("get_screenshot_editor_metadata", { sessionId });
}

export async function getScreenshotEditorSource(sessionId: string): Promise<Uint8Array> {
  return new Uint8Array(await invoke<ArrayBuffer>("get_screenshot_editor_source", { sessionId }));
}

export function exportScreenshotEditor(
  sessionId: string,
  operation: "save" | "save-as" | "copy",
  imageBytes: Uint8Array,
  outputPath?: string
): Promise<EditorExportResult> {
  const headers: Record<string, string> = {
    "x-session-id": sessionId,
    "x-operation": operation
  };
  if (outputPath) headers["x-output-path"] = encodeHeader(outputPath);
  return invoke<EditorExportResult>("save_screenshot_editor", imageBytes, { headers });
}

export function closeScreenshotEditor(sessionId: string): Promise<void> {
  return invoke("close_screenshot_editor", { sessionId });
}

function encodeHeader(value: string): string {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}
