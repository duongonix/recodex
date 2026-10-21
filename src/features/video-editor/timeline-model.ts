import type { TimelineSegment } from "./types.js";

export const MIN_SEGMENT_MS = 100;

export function getUsableSourceDurationMs(mediaDurationMs: number, recordingDurationMs: number): number {
  const mediaDuration = Number.isFinite(mediaDurationMs) ? Math.max(0, Math.floor(mediaDurationMs)) : 0;
  const recordingDuration = Number.isFinite(recordingDurationMs) ? Math.max(0, Math.floor(recordingDurationMs)) : 0;
  return recordingDuration > 0 ? Math.min(mediaDuration, recordingDuration) : mediaDuration;
}

export function createInitialSegment(durationMs: number): TimelineSegment[] {
  return durationMs > 0 ? [{ id: "segment-1", sourceStartMs: 0, sourceEndMs: durationMs }] : [];
}

export function getTimelineDuration(segments: TimelineSegment[], speed = 1): number {
  const safeSpeed = Math.max(0.01, speed);
  return segments.reduce((total, segment) => total + (segment.sourceEndMs - segment.sourceStartMs) / safeSpeed, 0);
}

export function previousTimelineBoundary(segments: TimelineSegment[], positionMs: number, speed = 1): number {
  const safeSpeed = Math.max(0.01, speed);
  let boundary = 0;
  let previous = 0;
  for (let index = 0; index < segments.length - 1; index += 1) {
    boundary += (segments[index].sourceEndMs - segments[index].sourceStartMs) / safeSpeed;
    if (boundary >= positionMs) break;
    previous = boundary;
  }
  return previous;
}

export function nextTimelineBoundary(segments: TimelineSegment[], positionMs: number, speed = 1): number {
  const safeSpeed = Math.max(0.01, speed);
  let boundary = 0;
  for (const segment of segments) {
    boundary += (segment.sourceEndMs - segment.sourceStartMs) / safeSpeed;
    if (boundary > positionMs) return boundary;
  }
  return boundary;
}

export function sourceTimeAtTimelinePosition(segments: TimelineSegment[], positionMs: number, speed = 1): number {
  const safeSpeed = Math.max(0.01, speed);
  const position = Math.max(0, Math.min(getTimelineDuration(segments, safeSpeed), positionMs));
  let offset = 0;
  for (const segment of segments) {
    const length = (segment.sourceEndMs - segment.sourceStartMs) / safeSpeed;
    if (position <= offset + length) return segment.sourceStartMs + Math.max(0, position - offset) * safeSpeed;
    offset += length;
  }
  return segments.at(-1)?.sourceEndMs ?? 0;
}

export function timelinePositionAtSourceTime(segments: TimelineSegment[], sourceTimeMs: number, speed = 1): number | null {
  const safeSpeed = Math.max(0.01, speed);
  let offset = 0;
  for (const segment of segments) {
    if (sourceTimeMs >= segment.sourceStartMs && sourceTimeMs <= segment.sourceEndMs) {
      return offset + (sourceTimeMs - segment.sourceStartMs) / safeSpeed;
    }
    offset += (segment.sourceEndMs - segment.sourceStartMs) / safeSpeed;
  }
  return null;
}

export function splitAtTimelinePosition(
  segments: TimelineSegment[],
  positionMs: number,
  id: string,
  speed = 1
): TimelineSegment[] | null {
  const safeSpeed = Math.max(0.01, speed);
  let offset = 0;
  for (let index = 0; index < segments.length; index += 1) {
    const segment = segments[index];
    const length = (segment.sourceEndMs - segment.sourceStartMs) / safeSpeed;
    const splitOffset = positionMs - offset;
    if (splitOffset > MIN_SEGMENT_MS / safeSpeed && splitOffset < length - MIN_SEGMENT_MS / safeSpeed) {
      const minimumSplit = segment.sourceStartMs + MIN_SEGMENT_MS;
      const maximumSplit = segment.sourceEndMs - MIN_SEGMENT_MS;
      const sourceSplitMs = Math.max(
        minimumSplit,
        Math.min(Math.round(segment.sourceStartMs + splitOffset * safeSpeed), maximumSplit)
      );
      return [
        ...segments.slice(0, index),
        { id: `${id}-a`, sourceStartMs: segment.sourceStartMs, sourceEndMs: sourceSplitMs },
        { id: `${id}-b`, sourceStartMs: sourceSplitMs, sourceEndMs: segment.sourceEndMs },
        ...segments.slice(index + 1)
      ];
    }
    offset += length;
  }
  return null;
}

export function trimSegment(
  segments: TimelineSegment[],
  id: string,
  edge: "start" | "end",
  sourceTimeMs: number,
  sourceDurationMs = Math.max(0, ...segments.map((segment) => segment.sourceEndMs))
): TimelineSegment[] | null {
  const index = segments.findIndex((segment) => segment.id === id);
  if (index < 0 || !Number.isFinite(sourceTimeMs)) return null;
  const segment = segments[index];
  const previousEnd = index > 0 ? segments[index - 1].sourceEndMs : 0;
  const nextStart = index < segments.length - 1
    ? segments[index + 1].sourceStartMs
    : sourceDurationMs;
  const candidate = Math.round(sourceTimeMs);
  let next: TimelineSegment;
  if (edge === "start") {
    const maximumStart = segment.sourceEndMs - MIN_SEGMENT_MS;
    if (previousEnd > maximumStart) return null;
    next = { ...segment, sourceStartMs: Math.max(previousEnd, Math.min(candidate, maximumStart)) };
  } else {
    const minimumEnd = segment.sourceStartMs + MIN_SEGMENT_MS;
    const maximumEnd = Math.min(nextStart, sourceDurationMs);
    if (minimumEnd > maximumEnd) return null;
    next = { ...segment, sourceEndMs: Math.min(maximumEnd, Math.max(candidate, minimumEnd)) };
  }
  if (next.sourceStartMs === segment.sourceStartMs && next.sourceEndMs === segment.sourceEndMs) return null;
  return segments.map((item, itemIndex) => itemIndex === index ? next : item);
}

export function deleteSegment(segments: TimelineSegment[], id: string): TimelineSegment[] | null {
  const next = segments.filter((segment) => segment.id !== id);
  return next.length === segments.length ? null : next;
}
