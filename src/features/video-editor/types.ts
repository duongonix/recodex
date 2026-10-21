export type TimelineSegment = {
  id: string;
  sourceStartMs: number;
  sourceEndMs: number;
};

export type EditorHistorySnapshot = {
  segments: TimelineSegment[];
  selectedSegmentId: string | null;
  settings: EditorSettings;
};

export type EditorSettings = {
  scale: number;
  positionX: number;
  positionY: number;
  rotation: number;
  speed: number;
  opacity: number;
  cropEnabled: boolean;
  cropLeft: number;
  cropRight: number;
  cropTop: number;
  cropBottom: number;
  volume: number;
  muted: boolean;
  canvasAspect: "source" | "16:9" | "9:16" | "1:1";
  backgroundColor: string;
  fit: "contain" | "cover";
};

export function createDefaultEditorSettings(): EditorSettings {
  return {
    scale: 100, positionX: 0, positionY: 0, rotation: 0, speed: 1, opacity: 100,
    cropEnabled: false, cropLeft: 0, cropRight: 0, cropTop: 0, cropBottom: 0,
    volume: 100, muted: false, canvasAspect: "source", backgroundColor: "#08090c", fit: "contain"
  };
}
