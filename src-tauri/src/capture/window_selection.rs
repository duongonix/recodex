use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::capture::sources::WindowSource;

/// Holds the currently confirmed window and real selection timestamps for this app session.
#[derive(Default)]
pub struct WindowSelectionState {
    selected: Option<WindowSource>,
    last_used_at: HashMap<String, u64>,
}

impl WindowSelectionState {
    pub fn selected(&self) -> Option<WindowSource> {
        self.selected.clone()
    }

    pub fn annotate_last_used(&self, windows: &mut [WindowSource]) {
        for window in windows {
            window.last_used_at = self.last_used_at.get(&window.window_id).copied();
        }
    }

    pub fn select(&mut self, mut window: WindowSource) -> WindowSource {
        let used_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
        if let Some(used_at) = used_at {
            self.last_used_at.insert(window.window_id.clone(), used_at);
        }
        window.last_used_at = used_at;
        self.selected = Some(window.clone());
        window
    }
}

#[cfg(test)]
mod tests {
    use super::WindowSelectionState;
    use crate::capture::sources::WindowSource;

    fn window(id: &str, title: &str) -> WindowSource {
        WindowSource {
            window_id: id.to_owned(),
            title: title.to_owned(),
            app_name: "test".to_owned(),
            app_icon: None,
            is_visible: true,
            is_minimized: false,
            is_recordable: true,
            last_used_at: None,
        }
    }

    #[test]
    fn confirms_window_and_tracks_actual_recent_use() {
        let mut state = WindowSelectionState::default();
        let selected = state.select(window("hwnd:1:2", "Editor"));
        assert_eq!(
            state
                .selected()
                .as_ref()
                .map(|value| value.window_id.as_str()),
            Some("hwnd:1:2")
        );
        assert_eq!(
            selected.last_used_at,
            state.selected().and_then(|value| value.last_used_at)
        );

        let mut listed = vec![window("hwnd:1:2", "Editor"), window("hwnd:2:3", "Browser")];
        state.annotate_last_used(&mut listed);
        assert_eq!(listed[0].last_used_at, selected.last_used_at);
        assert_eq!(listed[1].last_used_at, None);
    }
}
