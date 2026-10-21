import { getWindowThumbnail } from "./ipc/capture.js";

const MAX_CONCURRENT_PREVIEWS = 2;
let activePreviews = 0;
const waiting: Array<() => void> = [];

export async function loadWindowPreview(windowId: string): Promise<string | null> {
  if (activePreviews >= MAX_CONCURRENT_PREVIEWS) {
    await new Promise<void>((resolve) => waiting.push(resolve));
  }

  activePreviews += 1;
  try {
    return await getWindowThumbnail(windowId);
  } finally {
    activePreviews -= 1;
    waiting.shift()?.();
  }
}
