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
