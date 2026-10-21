import { describe, expect, it } from "vitest";
import { DocumentHistory } from "./history.js";

describe("screenshot editor history", () => {
  it("moves across undo and redo snapshots", () => {
    const history = new DocumentHistory<string>((value) => value);
    history.push("before");
    expect(history.undo("after")).toBe("before");
    expect(history.redo("before")).toBe("after");
  });

  it("clears redo when a new edit follows undo", () => {
    const history = new DocumentHistory<string>((value) => value);
    history.push("first");
    expect(history.undo("second")).toBe("first");
    history.push("new branch");
    expect(history.canRedo).toBe(false);
    expect(history.redo("new branch")).toBeNull();
  });
});

