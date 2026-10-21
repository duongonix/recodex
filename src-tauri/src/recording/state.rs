use serde::{Deserialize, Serialize};

use crate::errors::IpcError;

pub const STATE_CHANGED_EVENT: &str = "recording://state-changed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Idle,
    Preparing,
    Recording,
    Paused,
    Stopping,
    Finalizing,
    Reviewing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CaptureTarget {
    Display {
        display_id: String,
    },
    Window {
        window_id: String,
    },
    Region {
        display_id: String,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordingOptions {
    pub target: CaptureTarget,
    pub fps: u16,
    pub max_height: u16,
    pub microphone_enabled: bool,
    pub microphone_device_id: Option<String>,
    pub system_audio: bool,
    #[serde(default = "default_show_mouse_cursor")]
    pub show_mouse_cursor: bool,
    #[serde(default)]
    pub zoom_on_hold_enabled: bool,
    #[serde(default)]
    pub show_preview_after_recording: bool,
    pub output_directory: String,
}

fn default_show_mouse_cursor() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioSource {
    Microphone,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingQuality {
    pub capture_fps: Option<u16>,
    pub dropped_video_frames: u64,
    pub audio_gaps: u64,
    pub microphone_level_percent: u8,
    pub system_audio_level_percent: u8,
    pub available_disk_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSnapshot {
    pub session_id: Option<String>,
    pub status: SessionStatus,
    pub recorded_duration_ms: u64,
    pub output_path: Option<String>,
    pub error: Option<RecordingError>,
    pub audio_warning: Option<String>,
    pub microphone_enabled: bool,
    pub system_audio_enabled: bool,
    pub microphone_available: bool,
    pub system_audio_available: bool,
    pub webcam_enabled: bool,
    pub webcam_available: bool,
    pub quality: RecordingQuality,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingEvent {
    pub session_id: String,
    pub status: SessionStatus,
    pub recorded_duration_ms: u64,
    pub output_path: Option<String>,
    pub error: Option<RecordingError>,
    pub audio_warning: Option<String>,
    pub microphone_enabled: bool,
    pub system_audio_enabled: bool,
    pub microphone_available: bool,
    pub system_audio_available: bool,
    pub webcam_enabled: bool,
    pub webcam_available: bool,
    pub quality: RecordingQuality,
    pub revision: u64,
}

#[derive(Debug, Clone)]
pub struct RecordingSession {
    snapshot: RecordingSnapshot,
}

impl Default for RecordingSession {
    fn default() -> Self {
        Self {
            snapshot: RecordingSnapshot {
                session_id: None,
                status: SessionStatus::Idle,
                recorded_duration_ms: 0,
                output_path: None,
                error: None,
                audio_warning: None,
                microphone_enabled: false,
                system_audio_enabled: false,
                microphone_available: false,
                system_audio_available: false,
                webcam_enabled: false,
                webcam_available: false,
                quality: RecordingQuality::default(),
                revision: 0,
            },
        }
    }
}

impl RecordingSession {
    pub fn snapshot(&self) -> RecordingSnapshot {
        self.snapshot.clone()
    }

    pub fn transition(
        &mut self,
        next: SessionStatus,
        session_id: Option<&str>,
        recorded_duration_ms: u64,
        output_path: Option<String>,
        error: Option<RecordingError>,
    ) -> Result<RecordingEvent, IpcError> {
        if !is_allowed_transition(&self.snapshot.status, &next) {
            return Err(IpcError::invalid_state(format!(
                "Cannot transition recording from {:?} to {:?}.",
                self.snapshot.status, next
            )));
        }

        let current_id = self.snapshot.session_id.as_deref();
        let transition_id = match (&self.snapshot.status, &next) {
            (SessionStatus::Idle, SessionStatus::Preparing) => session_id
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| {
                    IpcError::invalid_state("A session ID is required to prepare recording.")
                })?,
            (_, SessionStatus::Idle) => current_id.or(session_id).ok_or_else(|| {
                IpcError::invalid_state("A session ID is required to finish a session.")
            })?,
            _ => {
                let supplied = session_id.ok_or_else(|| {
                    IpcError::invalid_state("A session ID is required for this transition.")
                })?;
                if current_id != Some(supplied) {
                    return Err(IpcError::invalid_state(
                        "The session ID does not match the active recording.",
                    ));
                }
                supplied
            }
        }
        .to_owned();

        if recorded_duration_ms < self.snapshot.recorded_duration_ms {
            return Err(IpcError::invalid_state(
                "Recorded duration cannot move backwards.",
            ));
        }
        if next == SessionStatus::Failed && error.is_none() {
            return Err(IpcError::invalid_state(
                "A failed session requires an error description.",
            ));
        }
        if next != SessionStatus::Failed && error.is_some() {
            return Err(IpcError::invalid_state(
                "Error details are valid only in the failed state.",
            ));
        }

        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
        self.snapshot.status = next.clone();
        self.snapshot.session_id = if next == SessionStatus::Idle {
            None
        } else {
            Some(transition_id.clone())
        };
        self.snapshot.recorded_duration_ms = if next == SessionStatus::Idle {
            0
        } else {
            recorded_duration_ms
        };
        self.snapshot.output_path = output_path.clone();
        self.snapshot.error = error.clone();
        if next == SessionStatus::Preparing || next == SessionStatus::Idle {
            self.snapshot.audio_warning = None;
            self.snapshot.quality = RecordingQuality::default();
        }
        if next == SessionStatus::Idle {
            self.snapshot.microphone_enabled = false;
            self.snapshot.system_audio_enabled = false;
            self.snapshot.microphone_available = false;
            self.snapshot.system_audio_available = false;
            self.snapshot.webcam_enabled = false;
            self.snapshot.webcam_available = false;
        }

        Ok(RecordingEvent {
            session_id: transition_id,
            status: next,
            recorded_duration_ms,
            output_path,
            error,
            audio_warning: self.snapshot.audio_warning.clone(),
            microphone_enabled: self.snapshot.microphone_enabled,
            system_audio_enabled: self.snapshot.system_audio_enabled,
            microphone_available: self.snapshot.microphone_available,
            system_audio_available: self.snapshot.system_audio_available,
            webcam_enabled: self.snapshot.webcam_enabled,
            webcam_available: self.snapshot.webcam_available,
            quality: self.snapshot.quality.clone(),
            revision: self.snapshot.revision,
        })
    }

    pub fn update_progress(
        &mut self,
        session_id: &str,
        recorded_duration_ms: u64,
    ) -> Result<RecordingEvent, IpcError> {
        self.update_progress_with_quality(session_id, recorded_duration_ms, None)
    }

    pub fn update_progress_with_quality(
        &mut self,
        session_id: &str,
        recorded_duration_ms: u64,
        quality: Option<RecordingQuality>,
    ) -> Result<RecordingEvent, IpcError> {
        if !matches!(
            self.snapshot.status,
            SessionStatus::Recording | SessionStatus::Paused
        ) || self.snapshot.session_id.as_deref() != Some(session_id)
        {
            return Err(IpcError::invalid_state(
                "Progress can only be updated for the active recording session.",
            ));
        }
        if recorded_duration_ms < self.snapshot.recorded_duration_ms {
            return Err(IpcError::invalid_state(
                "Recorded duration cannot move backwards.",
            ));
        }

        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
        self.snapshot.recorded_duration_ms = recorded_duration_ms;
        if let Some(quality) = quality {
            self.snapshot.quality = quality;
        }
        Ok(RecordingEvent {
            session_id: session_id.to_owned(),
            status: self.snapshot.status.clone(),
            recorded_duration_ms,
            output_path: self.snapshot.output_path.clone(),
            error: self.snapshot.error.clone(),
            audio_warning: self.snapshot.audio_warning.clone(),
            microphone_enabled: self.snapshot.microphone_enabled,
            system_audio_enabled: self.snapshot.system_audio_enabled,
            microphone_available: self.snapshot.microphone_available,
            system_audio_available: self.snapshot.system_audio_available,
            webcam_enabled: self.snapshot.webcam_enabled,
            webcam_available: self.snapshot.webcam_available,
            quality: self.snapshot.quality.clone(),
            revision: self.snapshot.revision,
        })
    }

    pub fn set_audio_warning(
        &mut self,
        session_id: &str,
        warning: String,
    ) -> Result<Option<RecordingEvent>, IpcError> {
        if !matches!(
            self.snapshot.status,
            SessionStatus::Recording | SessionStatus::Paused | SessionStatus::Stopping
        ) || self.snapshot.session_id.as_deref() != Some(session_id)
        {
            return Err(IpcError::invalid_state(
                "Audio warnings can only be set for the active recording session.",
            ));
        }
        if self.snapshot.audio_warning.is_some() {
            return Ok(None);
        }
        self.snapshot.audio_warning = Some(warning);
        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
        Ok(Some(self.current_event(session_id)))
    }

    pub fn set_audio_availability(
        &mut self,
        microphone_enabled: bool,
        system_audio_enabled: bool,
    ) -> Result<(), IpcError> {
        if self.snapshot.status != SessionStatus::Preparing {
            return Err(IpcError::invalid_state(
                "Audio source availability can only be confirmed while preparing a recording.",
            ));
        }
        self.snapshot.microphone_enabled = microphone_enabled;
        self.snapshot.system_audio_enabled = system_audio_enabled;
        self.snapshot.microphone_available = microphone_enabled;
        self.snapshot.system_audio_available = system_audio_enabled;
        Ok(())
    }

    pub fn set_webcam_state(
        &mut self,
        session_id: &str,
        enabled: bool,
        available: bool,
    ) -> Result<RecordingEvent, IpcError> {
        if !matches!(
            self.snapshot.status,
            SessionStatus::Preparing | SessionStatus::Recording | SessionStatus::Paused
        ) || self.snapshot.session_id.as_deref() != Some(session_id)
        {
            return Err(IpcError::invalid_state(
                "Webcam can only be changed during the active recording session.",
            ));
        }
        if enabled && !available {
            return Err(IpcError::new(
                "webcam_unavailable",
                "Choose an available camera in Webcam Overlay settings before enabling it.",
                true,
            ));
        }
        self.snapshot.webcam_enabled = enabled;
        self.snapshot.webcam_available = available;
        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
        Ok(self.current_event(session_id))
    }

    pub fn set_audio_enabled(
        &mut self,
        session_id: &str,
        source: AudioSource,
        enabled: bool,
    ) -> Result<RecordingEvent, IpcError> {
        if !matches!(
            self.snapshot.status,
            SessionStatus::Recording | SessionStatus::Paused
        ) || self.snapshot.session_id.as_deref() != Some(session_id)
        {
            return Err(IpcError::invalid_state(
                "Audio can only be changed in the active recording session.",
            ));
        }
        match source {
            AudioSource::Microphone if self.snapshot.microphone_available => {
                self.snapshot.microphone_enabled = enabled;
            }
            AudioSource::System if self.snapshot.system_audio_available => {
                self.snapshot.system_audio_enabled = enabled;
            }
            _ => {
                return Err(IpcError::new(
                    "audio_source_unavailable",
                    "This audio source was not enabled when the recording started.",
                    true,
                ));
            }
        }
        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
        Ok(self.current_event(session_id))
    }

    fn current_event(&self, session_id: &str) -> RecordingEvent {
        RecordingEvent {
            session_id: session_id.to_owned(),
            status: self.snapshot.status.clone(),
            recorded_duration_ms: self.snapshot.recorded_duration_ms,
            output_path: self.snapshot.output_path.clone(),
            error: self.snapshot.error.clone(),
            audio_warning: self.snapshot.audio_warning.clone(),
            microphone_enabled: self.snapshot.microphone_enabled,
            system_audio_enabled: self.snapshot.system_audio_enabled,
            microphone_available: self.snapshot.microphone_available,
            system_audio_available: self.snapshot.system_audio_available,
            webcam_enabled: self.snapshot.webcam_enabled,
            webcam_available: self.snapshot.webcam_available,
            quality: self.snapshot.quality.clone(),
            revision: self.snapshot.revision,
        }
    }
}

fn is_allowed_transition(current: &SessionStatus, next: &SessionStatus) -> bool {
    matches!(
        (current, next),
        (SessionStatus::Idle, SessionStatus::Preparing)
            | (SessionStatus::Preparing, SessionStatus::Recording)
            | (SessionStatus::Preparing, SessionStatus::Stopping)
            | (SessionStatus::Preparing, SessionStatus::Failed)
            | (SessionStatus::Recording, SessionStatus::Paused)
            | (SessionStatus::Recording, SessionStatus::Stopping)
            | (SessionStatus::Recording, SessionStatus::Failed)
            | (SessionStatus::Paused, SessionStatus::Recording)
            | (SessionStatus::Paused, SessionStatus::Stopping)
            | (SessionStatus::Paused, SessionStatus::Failed)
            | (SessionStatus::Stopping, SessionStatus::Finalizing)
            | (SessionStatus::Stopping, SessionStatus::Failed)
            | (SessionStatus::Finalizing, SessionStatus::Reviewing)
            | (SessionStatus::Finalizing, SessionStatus::Completed)
            | (SessionStatus::Finalizing, SessionStatus::Failed)
            | (SessionStatus::Reviewing, SessionStatus::Completed)
            | (SessionStatus::Completed, SessionStatus::Idle)
            | (SessionStatus::Failed, SessionStatus::Idle)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance_to_recording(session: &mut RecordingSession) {
        session
            .transition(SessionStatus::Preparing, Some("session-1"), 0, None, None)
            .expect("idle can prepare");
        session
            .transition(SessionStatus::Recording, Some("session-1"), 10, None, None)
            .expect("prepared session can record");
    }

    #[test]
    fn accepts_happy_path_and_increments_revision() {
        let mut session = RecordingSession::default();
        advance_to_recording(&mut session);
        let paused = session
            .transition(SessionStatus::Paused, Some("session-1"), 500, None, None)
            .expect("recording can pause");

        assert_eq!(paused.revision, 3);
        assert_eq!(paused.status, SessionStatus::Paused);
        assert_eq!(session.snapshot().recorded_duration_ms, 500);
    }

    #[test]
    fn rejects_duplicate_start_and_wrong_session_id() {
        let mut session = RecordingSession::default();
        advance_to_recording(&mut session);

        let duplicate_start =
            session.transition(SessionStatus::Preparing, Some("session-2"), 10, None, None);
        assert_eq!(duplicate_start.unwrap_err().code, "invalid_state");

        let wrong_session =
            session.transition(SessionStatus::Paused, Some("session-2"), 10, None, None);
        assert_eq!(wrong_session.unwrap_err().code, "invalid_state");
    }

    #[test]
    fn requires_error_details_for_failed_state() {
        let mut session = RecordingSession::default();
        session
            .transition(SessionStatus::Preparing, Some("session-1"), 0, None, None)
            .expect("idle can prepare");

        let result = session.transition(SessionStatus::Failed, Some("session-1"), 0, None, None);
        assert_eq!(result.unwrap_err().code, "invalid_state");
    }

    #[test]
    fn idle_snapshot_has_no_session_and_events_use_camel_case() {
        let mut session = RecordingSession::default();
        let event = session
            .transition(SessionStatus::Preparing, Some("session-1"), 0, None, None)
            .expect("idle can prepare");
        let json = serde_json::to_value(event).expect("event serializes");

        assert_eq!(session.snapshot().session_id.as_deref(), Some("session-1"));
        assert_eq!(json["sessionId"], "session-1");
        assert_eq!(json["recordedDurationMs"], 0);
    }

    #[test]
    fn progress_requires_active_session_and_never_moves_backwards() {
        let mut session = RecordingSession::default();
        advance_to_recording(&mut session);
        let event = session.update_progress("session-1", 1_000).unwrap();
        assert_eq!(event.recorded_duration_ms, 1_000);
        assert_eq!(event.revision, 3);
        assert_eq!(
            session.update_progress("session-1", 999).unwrap_err().code,
            "invalid_state"
        );
        assert_eq!(
            session
                .update_progress("wrong-session", 1_000)
                .unwrap_err()
                .code,
            "invalid_state"
        );
    }

    #[test]
    fn audio_discontinuity_warning_keeps_the_active_recording() {
        let mut session = RecordingSession::default();
        advance_to_recording(&mut session);
        let warning = session
            .set_audio_warning("session-1", "Audio gap; video continues.".to_owned())
            .unwrap()
            .expect("first warning updates state");

        assert_eq!(warning.status, SessionStatus::Recording);
        assert_eq!(
            warning.audio_warning.as_deref(),
            Some("Audio gap; video continues.")
        );
        assert_eq!(session.snapshot().status, SessionStatus::Recording);
        assert!(
            session
                .set_audio_warning("session-1", "Another gap.".to_owned())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn audio_controls_only_cover_sources_selected_before_start() {
        let mut session = RecordingSession::default();
        session
            .transition(SessionStatus::Preparing, Some("session-1"), 0, None, None)
            .unwrap();
        session.set_audio_availability(true, false).unwrap();
        session
            .transition(SessionStatus::Recording, Some("session-1"), 0, None, None)
            .unwrap();

        let muted = session
            .set_audio_enabled("session-1", AudioSource::Microphone, false)
            .unwrap();
        assert!(!muted.microphone_enabled);
        assert!(muted.microphone_available);
        assert!(!muted.system_audio_available);
        assert_eq!(
            session
                .set_audio_enabled("session-1", AudioSource::System, true)
                .unwrap_err()
                .code,
            "audio_source_unavailable"
        );
    }

    #[test]
    fn webcam_state_is_reported_and_can_change_while_recording() {
        let mut session = RecordingSession::default();
        session
            .transition(SessionStatus::Preparing, Some("session-1"), 0, None, None)
            .unwrap();
        let enabled = session.set_webcam_state("session-1", true, true).unwrap();
        assert!(enabled.webcam_enabled);
        assert!(enabled.webcam_available);

        session
            .transition(SessionStatus::Recording, Some("session-1"), 0, None, None)
            .unwrap();
        let disabled = session.set_webcam_state("session-1", false, true).unwrap();
        assert!(!disabled.webcam_enabled);
        assert!(disabled.webcam_available);
        assert!(!session.snapshot().webcam_enabled);
    }
}
