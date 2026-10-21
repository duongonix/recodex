use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tempfile::{NamedTempFile, TempPath};

use crate::{errors::IpcError, settings::ScreenshotFormat};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSessionMetadata {
    pub session_id: String,
    pub width: u32,
    pub height: u32,
    pub format: ScreenshotFormat,
    pub captured_at_ms: u128,
    pub suggested_filename: String,
}

struct EditorSession {
    metadata: EditorSessionMetadata,
    source_path: TempPath,
    output_directory: PathBuf,
}

#[derive(Default)]
pub struct EditorSessionStore {
    sessions: Mutex<HashMap<String, EditorSession>>,
}

impl EditorSessionStore {
    pub fn create(
        &self,
        cache_directory: &Path,
        output_directory: PathBuf,
        format: ScreenshotFormat,
        width: u32,
        height: u32,
        source_png: &[u8],
    ) -> Result<EditorSessionMetadata, IpcError> {
        fs::create_dir_all(cache_directory).map_err(|error| {
            IpcError::new(
                "editor_session_storage_unavailable",
                format!("Could not prepare temporary screenshot storage: {error}"),
                true,
            )
        })?;

        let mut temporary = NamedTempFile::new_in(cache_directory).map_err(|error| {
            IpcError::new(
                "editor_session_storage_unavailable",
                format!("Could not create a temporary screenshot session: {error}"),
                true,
            )
        })?;
        temporary.write_all(source_png).map_err(|error| {
            IpcError::new(
                "editor_session_storage_failed",
                format!("Could not write the temporary screenshot: {error}"),
                true,
            )
        })?;
        temporary.as_file().sync_all().map_err(|error| {
            IpcError::new(
                "editor_session_storage_failed",
                format!("Could not flush the temporary screenshot: {error}"),
                true,
            )
        })?;
        let source_path = temporary.into_temp_path();

        let captured_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let sequence = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let session_id = format!("{captured_at_ms:x}-{sequence:x}");
        let extension = format.extension();
        let metadata = EditorSessionMetadata {
            session_id: session_id.clone(),
            width,
            height,
            format,
            captured_at_ms,
            suggested_filename: format!("RecoDex-{captured_at_ms}.{extension}"),
        };

        self.sessions
            .lock()
            .map_err(|_| IpcError::internal("Screenshot editor sessions are unavailable."))?
            .insert(
                session_id,
                EditorSession {
                    metadata: metadata.clone(),
                    source_path,
                    output_directory,
                },
            );
        Ok(metadata)
    }

    pub fn metadata(&self, session_id: &str) -> Result<EditorSessionMetadata, IpcError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| IpcError::internal("Screenshot editor sessions are unavailable."))?;
        sessions
            .get(session_id)
            .map(|session| session.metadata.clone())
            .ok_or_else(session_unavailable)
    }

    pub fn source(&self, session_id: &str) -> Result<Vec<u8>, IpcError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| IpcError::internal("Screenshot editor sessions are unavailable."))?;
        let session = sessions.get(session_id).ok_or_else(session_unavailable)?;
        fs::read(&session.source_path).map_err(|error| {
            IpcError::new(
                "editor_source_unavailable",
                format!("Could not read the screenshot for editing: {error}"),
                true,
            )
        })
    }

    pub fn export_location(
        &self,
        session_id: &str,
    ) -> Result<(EditorSessionMetadata, PathBuf), IpcError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| IpcError::internal("Screenshot editor sessions are unavailable."))?;
        let session = sessions.get(session_id).ok_or_else(session_unavailable)?;
        Ok((session.metadata.clone(), session.output_directory.clone()))
    }

    pub fn remove(&self, session_id: &str) -> Result<(), IpcError> {
        self.sessions
            .lock()
            .map_err(|_| IpcError::internal("Screenshot editor sessions are unavailable."))?
            .remove(session_id);
        Ok(())
    }
}

fn session_unavailable() -> IpcError {
    IpcError::new(
        "editor_session_unavailable",
        "This screenshot editor session has expired. Capture the screenshot again.",
        true,
    )
}

impl ScreenshotFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_source_is_removed_when_session_closes() {
        let cache = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let store = EditorSessionStore::default();
        let metadata = store
            .create(
                cache.path(),
                output.path().to_path_buf(),
                ScreenshotFormat::Png,
                2,
                2,
                b"source bytes",
            )
            .unwrap();
        let source_path = store
            .sessions
            .lock()
            .unwrap()
            .get(&metadata.session_id)
            .unwrap()
            .source_path
            .to_path_buf();

        assert_eq!(store.source(&metadata.session_id).unwrap(), b"source bytes");
        assert!(source_path.exists());
        store.remove(&metadata.session_id).unwrap();
        assert!(!source_path.exists());
        store.remove(&metadata.session_id).unwrap();
    }
}
