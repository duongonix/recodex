use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Register command state before setup so every webview sees it as soon as it can invoke IPC.
        .manage(app_state::AppState::new())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir)?;

            let output_fallback = config_dir.join("output");
            std::fs::create_dir_all(&output_fallback)?;

            let video_dir = app
                .path()
                .video_dir()
                .ok()
                .filter(|path| path.is_dir())
                .map(|path| path.join("RecoDex"))
                .unwrap_or_else(|| output_fallback.join("recordings"));
            let screenshot_dir = app
                .path()
                .picture_dir()
                .ok()
                .filter(|path| path.is_dir())
                .map(|path| path.join("RecoDex"))
                .unwrap_or_else(|| output_fallback.join("screenshots"));
            std::fs::create_dir_all(&video_dir)?;
            std::fs::create_dir_all(&screenshot_dir)?;

            let settings = settings::SettingsService::load(
                config_dir.join("settings.json"),
                video_dir,
                screenshot_dir,
            )?;
            app.state::<app_state::AppState>()
                .settings
                .initialize(settings)
                .map_err(|error| std::io::Error::other(error.message))?;
            shortcut::register_quick_access(app.handle());
            shortcut::register_hold_zoom_shortcut(app.handle());
            system_tray::install(app)?;
            Ok(())
        })
        .on_window_event(commands::recording_windows::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::audio::list_audio_devices,
            commands::audio::test_audio_inputs,
            commands::capture::get_capture_capabilities,
            commands::capture::get_capture_permissions,
            commands::capture::request_capture_permission,
            commands::sources::list_displays,
            commands::window_picker::list_windows,
            commands::window_picker::get_window_thumbnail,
            commands::window_picker::get_selected_window,
            commands::window_picker::open_window_picker,
            commands::window_picker::close_window_picker,
            commands::window_picker::select_capture_window,
            commands::region::open_region_picker,
            commands::region::get_region_picker_state,
            commands::region::complete_region_selection,
            commands::region::cancel_region_picker,
            commands::recording::get_recording_status,
            commands::recording::get_recording_preflight,
            commands::recording::start_recording,
            commands::recording::pause_recording,
            commands::recording::resume_recording,
            commands::recording::stop_recording,
            commands::recording::set_recording_audio,
            commands::recording::set_recording_webcam,
            commands::recording_review::get_pending_recording_review,
            commands::recording_review::get_recording_review_assets,
            commands::recording_review::save_recording_review,
            commands::recording_review::discard_recording_review,
            commands::recording_review::trim_recording_review,
            commands::recording_review::split_recording_review,
            commands::recording_review::export_recording_timeline,
            commands::recording_review::cancel_recording_timeline_export,
            commands::recording_review::focus_recording_review,
            commands::recording_windows::show_recording_toolbar,
            commands::recording_windows::hide_recording_toolbar,
            commands::screenshot::capture_screenshot,
            editor::commands::get_screenshot_editor_metadata,
            editor::commands::get_screenshot_editor_source,
            editor::commands::save_screenshot_editor,
            editor::commands::close_screenshot_editor,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::webcam::list_webcam_devices,
            commands::webcam::get_webcam_overlay_settings,
            commands::webcam::start_webcam_preview,
            commands::webcam::stop_webcam_preview,
            commands::webcam::get_webcam_background_preview,
            commands::shortcuts::get_quick_access_shortcut_status,
            commands::shortcuts::get_hold_zoom_shortcut_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod app_state;
pub mod audio;
pub mod capture;
mod commands;
pub mod editor;
mod errors;
pub mod recording;
pub mod screenshot;
pub mod settings;
mod shortcut;
mod system_tray;
pub mod webcam;
