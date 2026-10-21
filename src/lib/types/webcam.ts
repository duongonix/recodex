export type WebcamDevice = {
  id: string;
  name: string;
};

export type WebcamPreviewFrame = {
  dataUrl: string;
  width: number;
  height: number;
  sequence: number;
};
