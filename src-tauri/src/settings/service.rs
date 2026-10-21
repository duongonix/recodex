use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};

use crate::errors::IpcError;
use crate::webcam::settings::WebcamSettings;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    Record,
    Screenshot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureTargetKind {
    Screen,
    Window,
    Region,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScreenshotFormat {
    Png,
    Jpg,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub schema_version: u16,
    pub mode: CaptureMode,
    pub capture_target: CaptureTargetKind,
    pub fps: u16,
    pub max_height: u16,
    pub microphone_enabled: bool,
    pub system_audio_enabled: bool,
    #[serde(default)]
    pub microphone_device_id: String,
    #[serde(default)]
    pub start_with_system: bool,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_true")]
    pub show_notifications: bool,
    #[serde(default = "default_true")]
    pub show_mouse_cursor: bool,
    #[serde(default)]
    pub zoom_on_hold_enabled: bool,
    #[serde(default = "default_zoom_shortcut")]
    pub zoom_on_hold_shortcut: String,
    #[serde(default)]
    pub show_recording_preview: bool,
    pub video_output_directory: String,
    pub screenshot_format: ScreenshotFormat,
    pub screenshot_copy_to_clipboard: bool,
    pub screenshot_save_to_file: bool,
    pub screenshot_output_directory: String,
    #[serde(default = "default_true")]
    pub show_screenshot_preview: bool,
    #[serde(default)]
    pub webcam: WebcamSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsPatch {
    pub mode: Option<CaptureMode>,
    pub capture_target: Option<CaptureTargetKind>,
    pub fps: Option<u16>,
    pub max_height: Option<u16>,
    pub microphone_enabled: Option<bool>,
    pub system_audio_enabled: Option<bool>,
    pub microphone_device_id: Option<String>,
    pub start_with_system: Option<bool>,
    pub minimize_to_tray: Option<bool>,
    pub show_notifications: Option<bool>,
    pub show_mouse_cursor: Option<bool>,
    pub zoom_on_hold_enabled: Option<bool>,
    pub zoom_on_hold_shortcut: Option<String>,
    pub show_recording_preview: Option<bool>,
    pub video_output_directory: Option<String>,
    pub screenshot_format: Option<ScreenshotFormat>,
    pub screenshot_copy_to_clipboard: Option<bool>,
    pub screenshot_save_to_file: Option<bool>,
    pub screenshot_output_directory: Option<String>,
    pub show_screenshot_preview: Option<bool>,
    pub webcam: Option<WebcamSettings>,
}

pub struct SettingsService {
    path: PathBuf,
    value: Mutex<AppSettings>,
}

impl SettingsService {
    pub fn load(
        path: PathBuf,
        default_video_dir: PathBuf,
        default_screenshot_dir: PathBuf,
    ) -> io::Result<Self> {
        let (settings, legacy_audio_gain_fields) = match fs::read(&path) {
            Ok(bytes) => {
                let mut value =
                    serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Could not parse settings file: {error}"),
                        )
                    })?;
                let fields = value.as_object_mut().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Settings file must contain a JSON object.",
                    )
                })?;
                let legacy_audio_gain_fields = fields.remove("microphoneGainPercent").is_some()
                    | fields.remove("systemAudioGainPercent").is_some();
                let settings = serde_json::from_value::<AppSettings>(value).map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Could not parse settings file: {error}"),
                    )
                })?;
                (settings, legacy_audio_gain_fields)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (
                AppSettings::defaults(default_video_dir, default_screenshot_dir),
                false,
            ),
            Err(error) => return Err(error),
        };

        let mut settings = settings
            .validated()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.message))?;
        let webcam_was_enabled = settings.webcam.enabled;
        settings.webcam.enabled = false;
        let service = Self {
            path,
            value: Mutex::new(settings),
        };
        if webcam_was_enabled || legacy_audio_gain_fields {
            let settings = service
                .get()
                .map_err(|error| io::Error::other(error.message))?;
            service
                .persist(&settings)
                .map_err(|error| io::Error::other(error.message))?;
        }
        Ok(service)
    }

    pub fn get(&self) -> Result<AppSettings, IpcError> {
        self.value
            .lock()
            .map(|settings| settings.clone())
            .map_err(|_| IpcError::internal("Settings are unavailable because their lock failed."))
    }

    pub fn update(&self, patch: SettingsPatch) -> Result<AppSettings, IpcError> {
        let mut stored = self.value.lock().map_err(|_| {
            IpcError::internal("Settings are unavailable because their lock failed.")
        })?;
        let updated = stored.clone().with_patch(patch).validated()?;
        self.persist(&updated)?;
        *stored = updated.clone();
        Ok(updated)
    }

    fn persist(&self, settings: &AppSettings) -> Result<(), IpcError> {
        let parent = self.path.parent().ok_or_else(|| {
            IpcError::new(
                "settings_path_invalid",
                "Settings file path has no parent directory.",
                true,
            )
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            IpcError::new(
                "settings_write_failed",
                format!("Could not create the settings directory: {error}"),
                true,
            )
        })?;

        let temporary_path = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(settings).map_err(|error| {
            IpcError::internal(format!("Could not serialize settings: {error}"))
        })?;

        let write_result = (|| -> io::Result<()> {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&temporary_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary_path, &self.path)
        })();

        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary_path);
            return Err(IpcError::new(
                "settings_write_failed",
                format!("Could not save preferences: {error}"),
                true,
            ));
        }

        Ok(())
    }
}

impl AppSettings {
    fn defaults(video_dir: PathBuf, screenshot_dir: PathBuf) -> Self {
        Self {
            schema_version: 1,
            mode: CaptureMode::Record,
            capture_target: CaptureTargetKind::Screen,
            fps: 30,
            max_height: 1080,
            microphone_enabled: false,
            system_audio_enabled: false,
            microphone_device_id: String::new(),
            start_with_system: false,
            minimize_to_tray: true,
            show_notifications: true,
            show_mouse_cursor: true,
            zoom_on_hold_enabled: false,
            zoom_on_hold_shortcut: default_zoom_shortcut(),
            show_recording_preview: false,
            video_output_directory: video_dir.to_string_lossy().into_owned(),
            screenshot_format: ScreenshotFormat::Png,
            screenshot_copy_to_clipboard: true,
            screenshot_save_to_file: true,
            screenshot_output_directory: screenshot_dir.to_string_lossy().into_owned(),
            show_screenshot_preview: true,
            webcam: WebcamSettings::default(),
        }
    }

    fn with_patch(mut self, patch: SettingsPatch) -> Self {
        if let Some(value) = patch.mode {
            self.mode = value;
        }
        if let Some(value) = patch.capture_target {
            self.capture_target = value;
        }
        if let Some(value) = patch.fps {
            self.fps = value;
        }
        if let Some(value) = patch.max_height {
            self.max_height = value;
        }
        if let Some(value) = patch.microphone_enabled {
            self.microphone_enabled = value;
        }
        if let Some(value) = patch.system_audio_enabled {
            self.system_audio_enabled = value;
        }
        if let Some(value) = patch.microphone_device_id {
            self.microphone_device_id = value;
        }
        if let Some(value) = patch.start_with_system {
            self.start_with_system = value;
        }
        if let Some(value) = patch.minimize_to_tray {
            self.minimize_to_tray = value;
        }
        if let Some(value) = patch.show_notifications {
            self.show_notifications = value;
        }
        if let Some(value) = patch.show_mouse_cursor {
            self.show_mouse_cursor = value;
        }
        if let Some(value) = patch.zoom_on_hold_enabled {
            self.zoom_on_hold_enabled = value;
        }
        if let Some(value) = patch.zoom_on_hold_shortcut {
            self.zoom_on_hold_shortcut = value;
        }
        if let Some(value) = patch.show_recording_preview {
            self.show_recording_preview = value;
        }
        if let Some(value) = patch.video_output_directory {
            self.video_output_directory = value;
        }
        if let Some(value) = patch.screenshot_format {
            self.screenshot_format = value;
        }
        if let Some(value) = patch.screenshot_copy_to_clipboard {
            self.screenshot_copy_to_clipboard = value;
        }
        if let Some(value) = patch.screenshot_save_to_file {
            self.screenshot_save_to_file = value;
        }
        if let Some(value) = patch.screenshot_output_directory {
            self.screenshot_output_directory = value;
        }
        if let Some(value) = patch.show_screenshot_preview {
            self.show_screenshot_preview = value;
        }
        if let Some(value) = patch.webcam {
            self.webcam = value;
        }
        self
    }

    fn validated(mut self) -> Result<Self, IpcError> {
        if self.schema_version != 1 {
            return Err(IpcError::new(
                "settings_version_unsupported",
                format!(
                    "Settings schema version {} is not supported.",
                    self.schema_version
                ),
                false,
            ));
        }
        if !matches!(self.fps, 30 | 60) {
            return Err(IpcError::new(
                "settings_invalid",
                "FPS must be 30 or 60.",
                true,
            ));
        }
        if !matches!(self.max_height, 720 | 1080 | 2160) {
            return Err(IpcError::new(
                "settings_invalid",
                "Maximum video height must be 720, 1080, or 2160.",
                true,
            ));
        }
        if self.microphone_device_id.len() > 1024
            || self.microphone_device_id.chars().any(char::is_control)
            || (!self.microphone_device_id.is_empty()
                && self.microphone_device_id.trim().is_empty())
        {
            return Err(IpcError::new(
                "settings_invalid_audio_device",
                "The selected microphone device ID is invalid.",
                true,
            ));
        }
        if self.zoom_on_hold_shortcut.trim().is_empty()
            || self.zoom_on_hold_shortcut.len() > 128
            || self.zoom_on_hold_shortcut.chars().any(char::is_control)
        {
            return Err(IpcError::new(
                "settings_invalid_shortcut",
                "The hold-to-zoom shortcut must be a valid keyboard shortcut.",
                true,
            ));
        }

        self.webcam.validate()?;

        self.video_output_directory = validate_output_directory(&self.video_output_directory)?;
        self.screenshot_output_directory =
            validate_output_directory(&self.screenshot_output_directory)?;
        Ok(self)
    }
}

fn default_true() -> bool {
    true
}

fn default_zoom_shortcut() -> String {
    "CommandOrControl+Shift+Z".to_owned()
}

fn validate_output_directory(value: &str) -> Result<String, IpcError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(IpcError::new(
            "settings_invalid_path",
            "Output folder must be a valid absolute directory path.",
            true,
        ));
    }

    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(IpcError::new(
            "settings_invalid_path",
            "Output folder must be an absolute directory path.",
            true,
        ));
    }
    let metadata = fs::metadata(path).map_err(|error| {
        IpcError::new(
            "settings_invalid_path",
            format!("Output folder is not available: {error}"),
            true,
        )
    })?;
    if !metadata.is_dir() {
        return Err(IpcError::new(
            "settings_invalid_path",
            "Output folder must point to an existing directory.",
            true,
        ));
    }

    path.canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| {
            IpcError::new(
                "settings_invalid_path",
                format!("Could not resolve output folder: {error}"),
                true,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("recodex-settings-{nonce}"));
        fs::create_dir_all(&path).expect("create test directory");
        path
    }

    #[test]
    fn defaults_use_product_choices_and_round_trip_to_disk() {
        let root = temporary_directory();
        let video = root.join("videos");
        let screenshots = root.join("screenshots");
        fs::create_dir_all(&video).expect("create video directory");
        fs::create_dir_all(&screenshots).expect("create screenshot directory");
        let file = root.join("settings.json");
        let service = SettingsService::load(file.clone(), video.clone(), screenshots).unwrap();

        assert_eq!(service.get().unwrap().fps, 30);
        assert_eq!(service.get().unwrap().max_height, 1080);
        assert!(service.get().unwrap().microphone_device_id.is_empty());
        assert!(!service.get().unwrap().zoom_on_hold_enabled);
        assert!(service.get().unwrap().show_screenshot_preview);
        assert_eq!(
            service.get().unwrap().zoom_on_hold_shortcut,
            "CommandOrControl+Shift+Z"
        );
        service
            .update(SettingsPatch {
                fps: Some(60),
                microphone_device_id: Some("microphone-test-device".to_owned()),
                zoom_on_hold_enabled: Some(true),
                zoom_on_hold_shortcut: Some("CommandOrControl+Alt+Z".to_owned()),
                video_output_directory: Some(video.to_string_lossy().into_owned()),
                ..SettingsPatch::default()
            })
            .unwrap();
        service
            .update(SettingsPatch {
                screenshot_format: Some(ScreenshotFormat::Jpg),
                show_screenshot_preview: Some(false),
                ..SettingsPatch::default()
            })
            .unwrap();
        drop(service);

        let reloaded = SettingsService::load(file, video, root.join("screenshots")).unwrap();
        assert_eq!(reloaded.get().unwrap().fps, 60);
        assert_eq!(
            reloaded.get().unwrap().microphone_device_id,
            "microphone-test-device"
        );
        assert!(reloaded.get().unwrap().zoom_on_hold_enabled);
        assert_eq!(
            reloaded.get().unwrap().zoom_on_hold_shortcut,
            "CommandOrControl+Alt+Z"
        );
        assert_eq!(
            reloaded.get().unwrap().screenshot_format,
            ScreenshotFormat::Jpg
        );
        assert!(!reloaded.get().unwrap().show_screenshot_preview);
        assert!(root.join("settings.json").is_file());
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn legacy_settings_drop_removed_audio_gain_fields_and_get_safe_defaults() {
        let root = temporary_directory();
        let video = root.join("videos");
        let screenshots = root.join("screenshots");
        fs::create_dir_all(&video).unwrap();
        fs::create_dir_all(&screenshots).unwrap();

        let mut legacy =
            serde_json::to_value(AppSettings::defaults(video.clone(), screenshots.clone()))
                .unwrap();
        let fields = legacy.as_object_mut().unwrap();
        fields.remove("zoomOnHoldEnabled");
        fields.remove("zoomOnHoldShortcut");
        fields.remove("showScreenshotPreview");
        fields.remove("webcam");
        fields.remove("microphoneDeviceId");
        fields.insert("microphoneGainPercent".to_owned(), serde_json::json!(75));
        fields.insert("systemAudioGainPercent".to_owned(), serde_json::json!(140));
        let file = root.join("settings.json");
        fs::write(&file, serde_json::to_vec(&legacy).unwrap()).unwrap();

        let loaded = SettingsService::load(file.clone(), video, screenshots).unwrap();
        let settings = loaded.get().unwrap();
        assert!(!settings.zoom_on_hold_enabled);
        assert!(settings.show_screenshot_preview);
        assert_eq!(settings.zoom_on_hold_shortcut, "CommandOrControl+Shift+Z");
        assert!(!settings.webcam.enabled);
        assert!(settings.microphone_device_id.is_empty());
        let persisted: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        assert!(persisted.get("microphoneGainPercent").is_none());
        assert!(persisted.get("systemAudioGainPercent").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn webcam_is_disabled_and_persisted_off_on_every_app_start() {
        let root = temporary_directory();
        let video = root.join("videos");
        let screenshots = root.join("screenshots");
        fs::create_dir_all(&video).unwrap();
        fs::create_dir_all(&screenshots).unwrap();
        let file = root.join("settings.json");
        let service =
            SettingsService::load(file.clone(), video.clone(), screenshots.clone()).unwrap();
        let mut webcam = service.get().unwrap().webcam;
        webcam.enabled = true;
        service
            .update(SettingsPatch {
                webcam: Some(webcam),
                ..SettingsPatch::default()
            })
            .unwrap();
        drop(service);

        let reloaded = SettingsService::load(file.clone(), video, screenshots).unwrap();
        assert!(!reloaded.get().unwrap().webcam.enabled);
        let persisted: AppSettings = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        assert!(!persisted.webcam.enabled);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_settings_are_rejected_without_mutating_persisted_values() {
        let root = temporary_directory();
        let video = root.join("videos");
        let screenshots = root.join("screenshots");
        fs::create_dir_all(&video).unwrap();
        fs::create_dir_all(&screenshots).unwrap();
        let service =
            SettingsService::load(root.join("settings.json"), video, screenshots).unwrap();

        let result = service.update(SettingsPatch {
            fps: Some(24),
            ..SettingsPatch::default()
        });
        assert_eq!(result.unwrap_err().code, "settings_invalid");
        assert_eq!(service.get().unwrap().fps, 30);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_relative_or_missing_output_directories() {
        let relative = validate_output_directory("output").unwrap_err();
        assert_eq!(relative.code, "settings_invalid_path");

        let root = temporary_directory();
        let missing = root.join("missing");
        let missing = validate_output_directory(&missing.to_string_lossy()).unwrap_err();
        assert_eq!(missing.code, "settings_invalid_path");
        fs::remove_dir_all(root).unwrap();
    }
}
