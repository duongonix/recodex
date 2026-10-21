export type EditorTool = "select" | "pen" | "arrow" | "rectangle" | "text" | "blur" | "crop";
export type InspectorTab = "style" | "text" | "background";
export type LineStyle = "solid" | "dashed" | "dotted";

export type ObjectStyle = {
  color: string;
  width: number;
  opacity: number;
  lineStyle: LineStyle;
  fillColor: string;
  fillOpacity: number;
  radius: number;
  startArrow: boolean;
  endArrow: boolean;
};

type BaseObject = { id: string; visible: boolean };

export type PenObject = BaseObject & {
  type: "pen";
  points: number[];
  style: ObjectStyle;
};

export type ArrowObject = BaseObject & {
  type: "arrow";
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  style: ObjectStyle;
};

export type RectangleObject = BaseObject & {
  type: "rectangle";
  x: number;
  y: number;
  width: number;
  height: number;
  radius: number;
  style: ObjectStyle;
};

export type TextObject = BaseObject & {
  type: "text";
  x: number;
  y: number;
  width: number;
  height: number;
  text: string;
  fontFamily: string;
  fontSize: number;
  fontWeight: "normal" | "bold";
  align: "left" | "center" | "right";
  color: string;
  backgroundColor: string;
  backgroundOpacity: number;
  padding: number;
};

export type BlurObject = BaseObject & {
  type: "blur";
  x: number;
  y: number;
  width: number;
  height: number;
  strength: number;
};

export type EditorObject = PenObject | ArrowObject | RectangleObject | TextObject | BlurObject;

export type CropRect = { x: number; y: number; width: number; height: number };

export type EditorBackground = {
  color: string;
  padding: number;
  radius: number;
  shadow: boolean;
  shadowIntensity: number;
};

export type EditorDocument = {
  version: 1;
  objects: EditorObject[];
  crop: CropRect | null;
  background: EditorBackground;
  defaultStyle: ObjectStyle;
};

export const defaultStyle = (): ObjectStyle => ({
  color: "#FF3038",
  width: 5,
  opacity: 100,
  lineStyle: "solid",
  fillColor: "#FF3038",
  fillOpacity: 0,
  radius: 0,
  startArrow: false,
  endArrow: true
});

export function createDocument(): EditorDocument {
  return {
    version: 1,
    objects: [],
    crop: null,
    background: {
      color: "#181B22",
      padding: 40,
      radius: 16,
      shadow: true,
      shadowIntensity: 28
    },
    defaultStyle: defaultStyle()
  };
}

