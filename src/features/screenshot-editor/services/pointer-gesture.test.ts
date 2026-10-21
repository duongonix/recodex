import { describe, expect, it } from "vitest";
import {
  canStartCanvasGesture,
  isPointerButtonHeld,
  resolvePointerGestureAction
} from "./pointer-gesture.js";

describe("screenshot editor pointer gestures", () => {
  it("starts drawing only from left press and reserves middle press for pan", () => {
    expect(canStartCanvasGesture(0)).toBe(true);
    expect(canStartCanvasGesture(1)).toBe(true);
    expect(canStartCanvasGesture(2)).toBe(false);
  });

  it("recognizes the initiating button and stops when it is released", () => {
    expect(isPointerButtonHeld(1, 0)).toBe(true);
    expect(isPointerButtonHeld(0, 0)).toBe(false);
    expect(isPointerButtonHeld(4, 1)).toBe(true);
    expect(isPointerButtonHeld(0, 1)).toBe(false);
  });

  it("updates only the active held pointer, commits on release, and cancels on lost press", () => {
    expect(resolvePointerGestureAction(12, 12, 0, 1, "move")).toBe("update");
    expect(resolvePointerGestureAction(12, 12, 0, 0, "move")).toBe("cancel");
    expect(resolvePointerGestureAction(12, 99, 0, 1, "move")).toBe("ignore");
    expect(resolvePointerGestureAction(12, 12, 0, 0, "up")).toBe("finish");
    expect(resolvePointerGestureAction(12, 12, 0, 1, "cancel")).toBe("cancel");
  });
});
