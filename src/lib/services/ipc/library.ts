import { invoke } from "@tauri-apps/api/core";
import type { LibraryItem, LibraryItemKind } from "../../types/library.js";

export function listLibraryItems(): Promise<LibraryItem[]> {
  return invoke<LibraryItem[]>("list_library_items");
}

export function getLibraryThumbnail(path: string): Promise<string | null> {
  return invoke<string | null>("get_library_thumbnail", { path });
}

export function openLibraryItem(path: string): Promise<void> {
  return invoke("open_library_item", { path });
}

export function revealLibraryItem(path: string): Promise<void> {
  return invoke("reveal_library_item", { path });
}

export function openLibraryFolder(kind: LibraryItemKind): Promise<void> {
  return invoke("open_library_folder", { kind });
}
