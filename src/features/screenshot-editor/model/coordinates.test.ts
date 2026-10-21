import { describe, expect, it } from "vitest";
import { clampRect, normalizeRect } from "./coordinates.js";

describe("screenshot editor coordinates", () => {
  it("normalizes reverse drag bounds without negative sizes", () => {
    expect(normalizeRect(80, 60, 10, 5)).toEqual({ x: 10, y: 5, width: 70, height: 55 });
  });

  it("clamps crop bounds to source dimensions", () => {
    expect(clampRect({ x: -20, y: 70, width: 180, height: 80 }, 100, 100)).toEqual({
      x: 0, y: 70, width: 100, height: 30
    });
  });
});

