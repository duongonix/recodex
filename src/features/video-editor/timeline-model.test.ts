import { describe, expect, it } from "vitest";
import {
  createInitialSegment,
  deleteSegment,
  getUsableSourceDurationMs,
  getTimelineDuration,
  nextTimelineBoundary,
  previousTimelineBoundary,
  sourceTimeAtTimelinePosition,
  splitAtTimelinePosition,
  timelinePositionAtSourceTime,
  trimSegment
} from "./timeline-model.js";

describe("video editor timeline model", () => {
  it("uses a source duration no longer than both the media and recording metadata", () => {
    expect(getUsableSourceDurationMs(10_001.8, 10_000)).toBe(10_000);
    expect(getUsableSourceDurationMs(10_001.8, 0)).toBe(10_001);
  });

  it("maps between source time and the joined timeline across split segments", () => {
    const segments = splitAtTimelinePosition(createInitialSegment(10_000), 4_000, "split")!;
    expect(segments.map(({ sourceStartMs, sourceEndMs }) => [sourceStartMs, sourceEndMs])).toEqual([
      [0, 4_000], [4_000, 10_000]
    ]);
    expect(sourceTimeAtTimelinePosition(segments, 6_000)).toBe(6_000);
    expect(timelinePositionAtSourceTime(segments, 6_000)).toBe(6_000);
  });

  it("joins timeline ranges after deleting a segment and keeps duration accurate", () => {
    const split = splitAtTimelinePosition(createInitialSegment(10_000), 4_000, "split")!;
    const removed = deleteSegment(split, split[0].id)!;
    expect(getTimelineDuration(removed)).toBe(6_000);
    expect(sourceTimeAtTimelinePosition(removed, 0)).toBe(4_000);
  });

  it("trims only the selected source range and rejects sub-minimum edits", () => {
    const initial = createInitialSegment(10_000);
    expect(trimSegment(initial, "segment-1", "start", 1_000, 10_000)?.[0]).toMatchObject({
      sourceStartMs: 1_000,
      sourceEndMs: 10_000
    });
    expect(trimSegment(initial, "segment-1", "start", 9_950, 10_000)?.[0].sourceStartMs).toBe(9_900);
  });

  it("clamps trim handles to source duration and neighboring segments", () => {
    const split = splitAtTimelinePosition(createInitialSegment(10_000), 4_000, "split")!;
    expect(trimSegment(split, split[0].id, "end", 9_000, 10_000)).toBeNull();
    expect(trimSegment(split, split[1].id, "start", 1_000, 10_000)).toBeNull();
    const gap = [{ id: "a", sourceStartMs: 0, sourceEndMs: 2_000 }, { id: "b", sourceStartMs: 4_000, sourceEndMs: 8_000 }];
    expect(trimSegment(gap, "b", "start", 0, 9_500)?.[1].sourceStartMs).toBe(2_000);
    expect(trimSegment(gap, "b", "end", 10_000, 9_500)?.[1].sourceEndMs).toBe(9_500);
  });

  it("maps playback speed into output timeline duration", () => {
    const segments = createInitialSegment(10_000);
    expect(getTimelineDuration(segments, 2)).toBe(5_000);
    expect(sourceTimeAtTimelinePosition(segments, 2_000, 2)).toBe(4_000);
  });

  it("seeks previous and next to boundaries around the playhead", () => {
    const segments = splitAtTimelinePosition(createInitialSegment(10_000), 4_000, "split")!;
    expect(previousTimelineBoundary(segments, 3_999)).toBe(0);
    expect(nextTimelineBoundary(segments, 3_999)).toBe(4_000);
    expect(previousTimelineBoundary(segments, 4_000)).toBe(0);
    expect(nextTimelineBoundary(segments, 4_000)).toBe(10_000);
    expect(previousTimelineBoundary(segments, 5_000)).toBe(4_000);
    expect(nextTimelineBoundary(segments, 5_000)).toBe(10_000);
  });
});
