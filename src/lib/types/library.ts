export type LibraryItemKind = "video" | "screenshot";
export type LibraryFilter = "all" | LibraryItemKind;

export type LibraryItem = {
  path: string;
  name: string;
  kind: LibraryItemKind;
  modifiedAtMs: number;
  sizeBytes: number;
  durationMs: number | null;
  isAvailable: boolean;
};
