use std::sync::{Arc, Mutex, OnceLock, atomic::AtomicBool, mpsc::SyncSender};

use crate::{
    capture::region::RegionPickerState,
    errors::IpcError,
    recording::RecordingSession,
    settings::{AppSettings, SettingsPatch, SettingsService},
};

#[derive(Default)]
pub struct SettingsServiceState {
    service: OnceLock<SettingsService>,
}

impl SettingsServiceState {
    pub fn initialize(&self, service: SettingsService) -> Result<(), IpcError> {
        self.service.set(service).map_err(|_| {
            IpcError::internal("Application settings were initialized more than once.")
        })
    }

    pub fn get(&self) -> Result<AppSettings, IpcError> {
        self.service
            .get()
            .ok_or_else(|| {
                IpcError::new(
                    "settings_initializing",
                    "Application settings are still initializing. Please retry shortly.",
                    true,
                )
            })?
            .get()
    }

    pub fn update(&self, patch: SettingsPatch) -> Result<AppSettings, IpcError> {
        self.service
            .get()
            .ok_or_else(|| {
                IpcError::new(
                    "settings_initializing",
                    "Application settings are still initializing. Please retry shortly.",
                    true,
                )
            })?
            .update(patch)
    }
}

pub struct AppState {
    pub recording: Arc<Mutex<RecordingSession>>,
    #[cfg(windows)]
    pub recording_review: crate::recording::review::RecordingReviewStore,
    pub recording_control: Mutex<Option<SyncSender<crate::recording::runtime::WorkerCommand>>>,
    pub toolbar_last_position: Mutex<Option<(i32, i32)>>,
    pub settings: SettingsServiceState,
    pub screenshot_editor_sessions: crate::editor::EditorSessionStore,
    pub region_picker: Mutex<Option<RegionPickerState>>,
    pub window_selection: Mutex<crate::capture::window_selection::WindowSelectionState>,
    pub quick_access_shortcut: Mutex<crate::shortcut::QuickAccessShortcutStatus>,
    pub hold_zoom_shortcut: Mutex<crate::shortcut::HoldZoomShortcutStatus>,
    pub hold_zoom_shortcut_update: Mutex<()>,
    pub zoom_hotkey_held: AtomicBool,
    pub video_editor_export_active: Arc<AtomicBool>,
    pub video_editor_export_cancelled: Arc<AtomicBool>,
    pub webcam: crate::webcam::manager::WebcamManager,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            recording: Arc::new(Mutex::new(RecordingSession::default())),
            #[cfg(windows)]
            recording_review: crate::recording::review::RecordingReviewStore::default(),
            recording_control: Mutex::new(None),
            toolbar_last_position: Mutex::new(None),
            settings: SettingsServiceState::default(),
            screenshot_editor_sessions: crate::editor::EditorSessionStore::default(),
            region_picker: Mutex::new(None),
            window_selection: Mutex::new(
                crate::capture::window_selection::WindowSelectionState::default(),
            ),
            quick_access_shortcut: Mutex::new(crate::shortcut::QuickAccessShortcutStatus::default()),
            hold_zoom_shortcut: Mutex::new(crate::shortcut::HoldZoomShortcutStatus::default()),
            hold_zoom_shortcut_update: Mutex::new(()),
            zoom_hotkey_held: AtomicBool::new(false),
            video_editor_export_active: Arc::new(AtomicBool::new(false)),
            video_editor_export_cancelled: Arc::new(AtomicBool::new(false)),
            webcam: crate::webcam::manager::WebcamManager::default(),
        }
    }
}
