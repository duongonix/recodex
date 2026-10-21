import { describe, expect, it } from "vitest";
import { ScreenshotEditorState, snapshotEditorValue } from "./editor-state.svelte.js";

describe("screenshot editor reactive snapshots", () => {
  it("copies the reactive drawing style and commits a pen object", () => {
    const editor = new ScreenshotEditorState();
    const style = snapshotEditorValue(editor.selectedStyle());

    style.color = "#00FF00";
    expect(editor.selectedStyle().color).toBe("#FF3038");

    editor.addObject({
      id: "pen-1",
      type: "pen",
      points: [1, 2, 3, 4],
      style,
      visible: true
    });

    expect(editor.document.objects).toHaveLength(1);
    expect(editor.document.objects[0]).toMatchObject({ type: "pen", style: { color: "#00FF00" } });
    expect(editor.canUndo).toBe(true);
  });
});
