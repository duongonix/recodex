export type QuickAccessShortcutStatus = {
  accelerator: string;
  registered: boolean;
  message: string | null;
};

export type HoldZoomShortcutStatus = {
  accelerator: string;
  enabled: boolean;
  registered: boolean;
  message: string | null;
};

export type CaptureHotkeyAction =
  | "record-screen"
  | "record-window"
  | "record-region"
  | "screenshot-screen"
  | "screenshot-window"
  | "screenshot-region";

export type CaptureHotkeyStatus = {
  action: CaptureHotkeyAction;
  accelerator: string;
  registered: boolean;
  message: string | null;
};

export type CaptureHotkeyRequest = {
  id: number;
  action: CaptureHotkeyAction;
};
