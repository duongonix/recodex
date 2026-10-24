use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::{errors::IpcError, settings::AppSettings};

const STORE_VERSION: u8 = 1;
const MAX_LIBRARY_ITEMS: usize = 2_000;
const MAX_LIST_ITEMS: usize = 500;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LibraryItemKind {
    Video,
    Screenshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub path: String,
    pub name: String,
    pub kind: LibraryItemKind,
    pub modified_at_ms: u64,
    pub size_bytes: u64,
    pub duration_ms: Option<u64>,
    pub is_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryDocument {
    version: u8,
    items: Vec<LibraryItem>,
}

#[derive(Default)]
pub struct CaptureLibrary {
    path: OnceLock<PathBuf>,
    items: Mutex<Vec<LibraryItem>>,
}

impl CaptureLibrary {
    pub fn initialize(&self, path: PathBuf) -> Result<(), IpcError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                IpcError::new(
                    "library_storage_unavailable",
                    format!("Could not create local library storage: {error}"),
                    true,
                )
            })?;
        }

        let items = match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<LibraryDocument>(&bytes) {
                Ok(document) if document.version == STORE_VERSION => document.items,
                Ok(_) => Vec::new(),
                Err(error) => {
                    eprintln!("[RecoDex][library] Ignoring unreadable local index: {error}");
                    Vec::new()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                eprintln!("[RecoDex][library] Could not read the local index: {error}");
                Vec::new()
            }
        };

        self.items
            .lock()
            .map_err(|_| IpcError::internal("The capture library lock failed."))?
            .extend(items.into_iter().take(MAX_LIBRARY_ITEMS));
        self.path
            .set(path)
            .map_err(|_| IpcError::internal("The capture library was initialized more than once."))
    }

    pub fn register(
        &self,
        path: &Path,
        kind: LibraryItemKind,
        duration_ms: Option<u64>,
    ) -> Result<LibraryItem, IpcError> {
        let path = path.canonicalize().map_err(|error| {
            IpcError::new(
                "library_item_unavailable",
                format!("Could not locate the saved capture: {error}"),
                true,
            )
        })?;
        if !path.is_file() {
            return Err(IpcError::new(
                "library_item_unavailable",
                "The saved capture is not a file.",
                true,
            ));
        }
        let item = item_from_path(&path, kind, duration_ms)?;
        let mut items = self
            .items
            .lock()
            .map_err(|_| IpcError::internal("The capture library lock failed."))?;
        upsert(&mut items, item.clone());
        self.persist(&items)?;
        Ok(item)
    }

    pub fn list(&self, settings: &AppSettings) -> Result<Vec<LibraryItem>, IpcError> {
        self.discover_directory(
            Path::new(&settings.video_output_directory),
            LibraryItemKind::Video,
        )?;
        self.discover_directory(
            Path::new(&settings.screenshot_output_directory),
            LibraryItemKind::Screenshot,
        )?;

        let mut items = self
            .items
            .lock()
            .map_err(|_| IpcError::internal("The capture library lock failed."))?
            .clone();
        for item in &mut items {
            item.is_available = Path::new(&item.path).is_file();
            if item.is_available
                && let Ok(metadata) = fs::metadata(&item.path)
            {
                item.size_bytes = metadata.len();
                item.modified_at_ms = modified_at_ms(&metadata);
            }
        }
        items.sort_by_key(|item| std::cmp::Reverse(item.modified_at_ms));
        items.truncate(MAX_LIST_ITEMS);
        Ok(items)
    }

    pub fn validate_item_path(&self, path: &Path) -> Result<PathBuf, IpcError> {
        let path = path.canonicalize().map_err(|error| {
            IpcError::new(
                "library_item_unavailable",
                format!("The selected capture is no longer available: {error}"),
                true,
            )
        })?;
        if !path.is_file() {
            return Err(IpcError::new(
                "library_item_unavailable",
                "The selected capture is no longer available.",
                true,
            ));
        }
        let items = self
            .items
            .lock()
            .map_err(|_| IpcError::internal("The capture library lock failed."))?;
        if items
            .iter()
            .any(|item| item.path == path.to_string_lossy().as_ref())
        {
            Ok(path)
        } else {
            Err(IpcError::new(
                "library_item_unknown",
                "The selected file is not in the RecoDex capture library.",
                false,
            ))
        }
    }

    pub fn kind_for_path(&self, path: &Path) -> Result<LibraryItemKind, IpcError> {
        let path = path.to_string_lossy();
        self.items
            .lock()
            .map_err(|_| IpcError::internal("The capture library lock failed."))?
            .iter()
            .find(|item| item.path == path.as_ref())
            .map(|item| item.kind)
            .ok_or_else(|| {
                IpcError::new(
                    "library_item_unknown",
                    "The selected file is not in the RecoDex capture library.",
                    false,
                )
            })
    }

    fn discover_directory(&self, directory: &Path, kind: LibraryItemKind) -> Result<(), IpcError> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                eprintln!("[RecoDex][library] Could not scan an output folder: {error}");
                return Ok(());
            }
        };

        let mut discovered = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_recodex_capture(&path, kind) {
                continue;
            }
            if let Ok(canonical) = path.canonicalize()
                && let Ok(item) = item_from_path(&canonical, kind, None)
            {
                discovered.push(item);
            }
        }
        if discovered.is_empty() {
            return Ok(());
        }

        let mut items = self
            .items
            .lock()
            .map_err(|_| IpcError::internal("The capture library lock failed."))?;
        let old_len = items.len();
        for item in discovered {
            if !items.iter().any(|existing| existing.path == item.path) {
                items.push(item);
            }
        }
        if items.len() != old_len {
            sort_and_limit(&mut items);
            self.persist(&items)?;
        }
        Ok(())
    }

    fn persist(&self, items: &[LibraryItem]) -> Result<(), IpcError> {
        let path = self.path.get().ok_or_else(|| {
            IpcError::new(
                "library_storage_initializing",
                "The local capture library is still initializing.",
                true,
            )
        })?;
        let parent = path.parent().ok_or_else(|| {
            IpcError::internal("The local capture library path has no parent folder.")
        })?;
        let mut temporary = NamedTempFile::new_in(parent).map_err(|error| {
            IpcError::new(
                "library_storage_failed",
                format!("Could not stage the local capture index: {error}"),
                true,
            )
        })?;
        serde_json::to_writer(
            temporary.as_file_mut(),
            &LibraryDocument {
                version: STORE_VERSION,
                items: items.to_vec(),
            },
        )
        .map_err(|error| {
            IpcError::new(
                "library_storage_failed",
                format!("Could not serialize the local capture index: {error}"),
                true,
            )
        })?;
        temporary.as_file().sync_all().map_err(|error| {
            IpcError::new(
                "library_storage_failed",
                format!("Could not flush the local capture index: {error}"),
                true,
            )
        })?;
        temporary.persist(path).map_err(|error| {
            IpcError::new(
                "library_storage_failed",
                format!(
                    "Could not finalize the local capture index: {}",
                    error.error
                ),
                true,
            )
        })?;
        Ok(())
    }
}

fn item_from_path(
    path: &Path,
    kind: LibraryItemKind,
    duration_ms: Option<u64>,
) -> Result<LibraryItem, IpcError> {
    let metadata = fs::metadata(path).map_err(|error| {
        IpcError::new(
            "library_item_metadata_failed",
            format!("Could not inspect a saved capture: {error}"),
            true,
        )
    })?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| IpcError::internal("A saved capture has no filename."))?;
    Ok(LibraryItem {
        path: path.to_string_lossy().into_owned(),
        name,
        kind,
        modified_at_ms: modified_at_ms(&metadata),
        size_bytes: metadata.len(),
        duration_ms,
        is_available: true,
    })
}

fn modified_at_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn is_recodex_capture(path: &Path, kind: LibraryItemKind) -> bool {
    if !path.is_file() {
        return false;
    }
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !name.starts_with("recodex-") {
        return false;
    }
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match kind {
        LibraryItemKind::Video => extension == "mp4",
        LibraryItemKind::Screenshot => matches!(extension.as_str(), "png" | "jpg" | "jpeg"),
    }
}

fn upsert(items: &mut Vec<LibraryItem>, item: LibraryItem) {
    if let Some(existing) = items.iter_mut().find(|existing| existing.path == item.path) {
        *existing = item;
    } else {
        items.push(item);
    }
    sort_and_limit(items);
}

fn sort_and_limit(items: &mut Vec<LibraryItem>) {
    items.sort_by_key(|item| std::cmp::Reverse(item.modified_at_ms));
    items.truncate(MAX_LIBRARY_ITEMS);
}
