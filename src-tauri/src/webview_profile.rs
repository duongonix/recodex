#[cfg(target_os = "windows")]
use std::{env, fs, path::PathBuf};

/// Selects a dedicated WebView2 profile before Tauri creates any webviews.
#[cfg(target_os = "windows")]
pub fn prepare(app_identifier: &str) -> Result<(), String> {
    if env::var_os("WEBVIEW2_USER_DATA_FOLDER").is_some() {
        return Ok(());
    }

    let local_data_dir = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "Windows did not provide LOCALAPPDATA".to_owned())?;
    let profile_dir = local_data_dir.join(app_identifier).join("WebView2");

    fs::create_dir_all(&profile_dir).map_err(|error| {
        format!(
            "Could not create WebView2 profile at {}: {error}",
            profile_dir.display()
        )
    })?;

    // WebView2 reads this process environment variable when creating its native
    // environment. This runs before Tauri starts threads or creates webviews.
    unsafe { env::set_var("WEBVIEW2_USER_DATA_FOLDER", profile_dir) };

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn prepare(_app_identifier: &str) -> Result<(), String> {
    Ok(())
}
