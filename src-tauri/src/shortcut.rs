use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::{app_state::AppState, recording::SessionStatus};

pub const QUICK_ACCESS_ACCELERATOR: &str = "CommandOrControl+Alt+Shift+R";
pub const DEFAULT_HOLD_ZOOM_ACCELERATOR: &str = "CommandOrControl+Shift+Z";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickAccessShortcutStatus {
    pub accelerator: String,
    pub registered: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HoldZoomShortcutStatus {
    pub accelerator: String,
    pub enabled: bool,
    pub registered: bool,
    pub message: Option<String>,
}

impl Default for HoldZoomShortcutStatus {
    fn default() -> Self {
        Self {
            accelerator: DEFAULT_HOLD_ZOOM_ACCELERATOR.to_owned(),
            enabled: false,
            registered: false,
            message: None,
        }
    }
}

impl Default for QuickAccessShortcutStatus {
    fn default() -> Self {
        Self {
            accelerator: QUICK_ACCESS_ACCELERATOR.to_owned(),
            registered: false,
            message: Some("Shortcut registration has not completed.".to_owned()),
        }
    }
}

pub fn register_quick_access(app: &AppHandle) {
    let registration =
        app.global_shortcut()
            .on_shortcut(QUICK_ACCESS_ACCELERATOR, |app, _, event| {
                if event.state == ShortcutState::Pressed {
                    show_quick_access(app);
                }
            });
    let status = match registration {
        Ok(()) => QuickAccessShortcutStatus {
            accelerator: QUICK_ACCESS_ACCELERATOR.to_owned(),
            registered: true,
            message: None,
        },
        Err(error) => QuickAccessShortcutStatus {
            accelerator: QUICK_ACCESS_ACCELERATOR.to_owned(),
            registered: false,
            message: Some(format!(
                "The shortcut could not be registered: {error}. Another app may use it; use the system tray to open RecoDex."
            )),
        },
    };
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(mut stored) = state.quick_access_shortcut.lock()
    {
        *stored = status;
    }
}

pub fn register_hold_zoom_shortcut(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let settings = match state.settings.get() {
        Ok(settings) => settings,
        Err(error) => {
            set_zoom_status(
                app,
                HoldZoomShortcutStatus {
                    enabled: true,
                    registered: false,
                    message: Some(error.message),
                    ..HoldZoomShortcutStatus::default()
                },
            );
            return;
        }
    };
    let mut status = HoldZoomShortcutStatus {
        accelerator: settings.zoom_on_hold_shortcut.clone(),
        enabled: settings.zoom_on_hold_enabled,
        registered: false,
        message: None,
    };
    if settings.zoom_on_hold_enabled {
        match register_zoom_binding(app, &settings.zoom_on_hold_shortcut) {
            Ok(()) => status.registered = true,
            Err(error) => status.message = Some(error.message),
        }
    }
    set_zoom_status(app, status);
}

/// Applies shortcut registration after settings have been saved; callers serialize updates.
pub fn update_hold_zoom_shortcut(
    app: &AppHandle,
    old_enabled: bool,
    old_accelerator: &str,
    new_enabled: bool,
    new_accelerator: &str,
) -> Result<(), crate::errors::IpcError> {
    let old_shortcut = parse_zoom_shortcut(old_accelerator)?;
    let new_shortcut = parse_zoom_shortcut(new_accelerator)?;
    let same_binding = old_shortcut.id() == new_shortcut.id();
    let mut status = HoldZoomShortcutStatus {
        accelerator: new_accelerator.to_owned(),
        enabled: new_enabled,
        registered: false,
        message: None,
    };
    let currently_registered = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .hold_zoom_shortcut
                .lock()
                .ok()
                .map(|status| status.registered)
        })
        .unwrap_or(false);
    status.registered = old_enabled && same_binding && currently_registered;

    if new_enabled && !(old_enabled && same_binding && currently_registered) {
        register_zoom_binding(app, new_accelerator)?;
        status.registered = true;
    }

    if old_enabled
        && currently_registered
        && (!new_enabled || !same_binding)
        && let Err(error) = app.global_shortcut().unregister(old_accelerator)
    {
        if new_enabled && !same_binding {
            let _ = app.global_shortcut().unregister(new_accelerator);
        }
        return Err(crate::errors::IpcError::new(
            "shortcut_unregistration_failed",
            format!("Could not remove the previous hold-to-zoom shortcut: {error}"),
            true,
        ));
    }

    set_zoom_status(app, status);
    if !new_enabled && let Some(state) = app.try_state::<AppState>() {
        state
            .zoom_hotkey_held
            .store(false, std::sync::atomic::Ordering::Release);
    }
    Ok(())
}

pub fn validate_hold_zoom_shortcut(accelerator: &str) -> Result<(), crate::errors::IpcError> {
    let shortcut = parse_zoom_shortcut(accelerator)?;
    let quick_access = parse_zoom_shortcut(QUICK_ACCESS_ACCELERATOR)?;
    if shortcut.id() == quick_access.id() {
        return Err(crate::errors::IpcError::new(
            "shortcut_conflict",
            "Hold-to-zoom cannot use the global quick-access shortcut.",
            true,
        ));
    }
    Ok(())
}

fn register_zoom_binding(
    app: &AppHandle,
    accelerator: &str,
) -> Result<(), crate::errors::IpcError> {
    let shortcut = parse_zoom_shortcut(accelerator)?;
    let quick_access = parse_zoom_shortcut(QUICK_ACCESS_ACCELERATOR)?;
    if shortcut.id() == quick_access.id() {
        return Err(crate::errors::IpcError::new(
            "shortcut_conflict",
            "Hold-to-zoom cannot use the global quick-access shortcut.",
            true,
        ));
    }
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if let Some(state) = app.try_state::<AppState>() {
                state.zoom_hotkey_held.store(
                    event.state == ShortcutState::Pressed,
                    std::sync::atomic::Ordering::Release,
                );
            }
        })
        .map_err(|error| {
            crate::errors::IpcError::new(
                "shortcut_registration_failed",
                format!("Could not register the hold-to-zoom shortcut: {error}"),
                true,
            )
        })
}

fn parse_zoom_shortcut(accelerator: &str) -> Result<Shortcut, crate::errors::IpcError> {
    Shortcut::try_from(accelerator).map_err(|error| {
        crate::errors::IpcError::new(
            "settings_invalid_shortcut",
            format!("The hold-to-zoom shortcut is invalid: {error}"),
            true,
        )
    })
}

fn set_zoom_status(app: &AppHandle, status: HoldZoomShortcutStatus) {
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(mut stored) = state.hold_zoom_shortcut.lock()
    {
        *stored = status;
    }
}

pub fn show_quick_access(app: &AppHandle) {
    let status = app.try_state::<AppState>().and_then(|state| {
        state
            .recording
            .lock()
            .ok()
            .map(|session| session.snapshot().status)
    });

    if status == Some(SessionStatus::Reviewing)
        && let Some(review) = app.get_webview_window("recording-review")
    {
        let _ = review.show();
        let _ = review.set_focus();
        return;
    }

    let recording_active = status.is_some_and(is_session_active);

    if recording_active
        && let (Some(main), Some(toolbar)) = (
            app.get_webview_window("main"),
            app.get_webview_window("recording-toolbar"),
        )
        && toolbar.set_always_on_top(true).is_ok()
        && main.hide().is_ok()
        && toolbar.show().is_ok()
    {
        let _ = toolbar.set_focus();
        return;
    }

    show_main_window(app);
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

fn is_session_active(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Preparing
            | SessionStatus::Recording
            | SessionStatus::Paused
            | SessionStatus::Stopping
            | SessionStatus::Finalizing
    )
}

#[cfg(test)]
mod tests {
    use super::QUICK_ACCESS_ACCELERATOR;
    use tauri_plugin_global_shortcut::Shortcut;

    #[test]
    fn quick_access_accelerator_uses_a_valid_global_shortcut_format() {
        assert!(Shortcut::try_from(QUICK_ACCESS_ACCELERATOR).is_ok());
    }

    #[test]
    fn hold_zoom_default_is_valid_and_does_not_conflict_with_quick_access() {
        assert!(Shortcut::try_from(super::DEFAULT_HOLD_ZOOM_ACCELERATOR).is_ok());
        assert!(super::validate_hold_zoom_shortcut(super::DEFAULT_HOLD_ZOOM_ACCELERATOR).is_ok());
        assert!(super::validate_hold_zoom_shortcut(QUICK_ACCESS_ACCELERATOR).is_err());
    }
}
