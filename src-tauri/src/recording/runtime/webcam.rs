use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender, SyncSender, TryRecvError},
    },
    thread,
};

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    errors::IpcError,
    recording::{RecordingSession, RecordingSnapshot, emit_state_changed},
    webcam::{
        capture::WebcamCapture,
        compositor::WebcamCompositor,
        manager::{PreviewPublisher, RECORDING_WARNING_EVENT},
        overlay_window,
        settings::{WebcamPosition, WebcamSettings, WebcamShape},
    },
};

enum WebcamTaskResult {
    Started(Result<WebcamCapture, IpcError>),
    Stopped,
}

pub(super) struct RecordingWebcam {
    settings: WebcamSettings,
    frame_width: u32,
    frame_height: u32,
    camera_dimensions: Option<(u32, u32)>,
    camera_min_dimension: u32,
    camera_square_crop: bool,
    capture: Option<WebcamCapture>,
    preview_publisher: Option<PreviewPublisher>,
    compositor: Option<WebcamCompositor>,
    enabled: bool,
    available: bool,
    last_sequence: u64,
    moved_x: i64,
    moved_y: i64,
    task_tx: Sender<WebcamTaskResult>,
    task_rx: Receiver<WebcamTaskResult>,
    pending_response: Option<SyncSender<Result<RecordingSnapshot, IpcError>>>,
    shutdown_pending: bool,
}

pub(super) struct RecordingContext<'a> {
    pub app: &'a AppHandle,
    pub session: &'a Arc<Mutex<RecordingSession>>,
    pub session_id: &'a str,
}

impl RecordingWebcam {
    pub fn start(
        mut settings: WebcamSettings,
        frame_width: u32,
        frame_height: u32,
        app: &AppHandle,
    ) -> Self {
        // Placement now follows the floating toolbar, so persisted positions from older builds
        // must not override the bottom-right recording default.
        settings.position = WebcamPosition::BottomRight;
        settings.x = 1.0;
        settings.y = 1.0;
        let (camera_min_dimension, camera_square_crop) =
            recording_capture_profile(&settings, frame_height);
        let (task_tx, task_rx) = mpsc::channel();
        let mut webcam = Self {
            available: settings.camera_id.is_some(),
            enabled: false,
            settings,
            frame_width,
            frame_height,
            camera_dimensions: None,
            camera_min_dimension,
            camera_square_crop,
            capture: None,
            preview_publisher: None,
            compositor: None,
            last_sequence: 0,
            moved_x: 0,
            moved_y: 0,
            task_tx,
            task_rx,
            pending_response: None,
            shutdown_pending: false,
        };

        if webcam.settings.enabled {
            if let Some(camera_id) = webcam.settings.camera_id.clone() {
                match WebcamCapture::start_for_recording(
                    &camera_id,
                    webcam.camera_min_dimension,
                    webcam.camera_square_crop,
                ) {
                    Ok(camera) => {
                        if let Err(error) = webcam.install_capture(camera, app) {
                            emit_warning(
                                app,
                                &format!("Webcam overlay was disabled: {}", error.message),
                            );
                        } else if let Some(warning) = webcam.image_warning() {
                            emit_warning(app, warning);
                        }
                    }
                    Err(error) => emit_warning(app, &error.message),
                }
            } else {
                emit_warning(
                    app,
                    "Choose a camera in Settings > Snap > Webcam Overlay to include it in the recording.",
                );
            }
        }
        webcam
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn available(&self) -> bool {
        self.available
    }

    pub fn configure_overlay_window(&self, app: &AppHandle) -> Result<(), IpcError> {
        if !self.enabled {
            return Ok(());
        }
        let window = app.get_webview_window("webcam-overlay").ok_or_else(|| {
            IpcError::new(
                "webcam_overlay_unavailable",
                "The webcam overlay window is unavailable.",
                true,
            )
        })?;
        overlay_window::configure_size(&window, &self.settings, self.camera_dimensions)
    }

    pub fn latest_sequence(&mut self) -> Option<u64> {
        let sequence = self.capture.as_ref()?.latest_frame()?.sequence;
        if sequence == self.last_sequence {
            return None;
        }
        self.last_sequence = sequence;
        Some(sequence)
    }

    pub fn error(&self) -> Option<String> {
        self.capture.as_ref()?.error()
    }

    pub fn discard_underlay(&mut self) {
        if let Some(compositor) = &mut self.compositor {
            compositor.discard_underlay();
        }
    }

    pub fn restore_underlay(&mut self, screen_bgra: &mut [u8]) {
        if let Some(compositor) = &mut self.compositor {
            compositor.restore_underlay(screen_bgra);
        }
    }

    pub fn composite(&mut self, screen_bgra: &mut [u8]) -> Result<(), IpcError> {
        if !self.enabled {
            return Ok(());
        }
        let frame = self.capture.as_ref().and_then(WebcamCapture::latest_frame);
        if let Some(compositor) = &mut self.compositor {
            compositor.composite(screen_bgra, frame.as_deref())?;
        }
        Ok(())
    }

    pub fn move_by(&mut self, dx: i32, dy: i32, screen_bgra: &mut [u8]) -> Result<(), IpcError> {
        if dx == 0 && dy == 0 {
            return Ok(());
        }
        self.moved_x = self.moved_x.saturating_add(i64::from(dx));
        self.moved_y = self.moved_y.saturating_add(i64::from(dy));
        if let Some(compositor) = &mut self.compositor {
            compositor.restore_underlay(screen_bgra);
            compositor.move_by(dx, dy);
        }
        Ok(())
    }

    pub fn clear(&mut self, screen_bgra: &mut [u8]) {
        if let Some(compositor) = &mut self.compositor {
            compositor.clear(screen_bgra);
        }
    }

    pub fn set_enabled(
        &mut self,
        enabled: bool,
        context: &RecordingContext<'_>,
        screen_bgra: &mut [u8],
        pending_video_frame: &mut bool,
        response: SyncSender<Result<RecordingSnapshot, IpcError>>,
    ) {
        if enabled == self.enabled && self.pending_response.is_none() {
            let _ = response.send(snapshot(context.session));
            return;
        }
        if self.pending_response.is_some() || self.shutdown_pending {
            let _ = response.send(Err(IpcError::invalid_state(
                "Wait for the webcam to finish changing state before toggling it again.",
            )));
            return;
        }
        if enabled {
            if !self.available {
                let _ = response.send(Err(IpcError::new(
                    "webcam_unavailable",
                    "Choose an available camera in Webcam Overlay settings before enabling it.",
                    true,
                )));
                return;
            }
            if let Some(camera_id) = self.settings.camera_id.clone() {
                let sender = self.task_tx.clone();
                let minimum_dimension = self.camera_min_dimension;
                let square_crop = self.camera_square_crop;
                let spawn = thread::Builder::new()
                    .name("recodex-webcam-start".to_owned())
                    .spawn(move || {
                        let _ = sender.send(WebcamTaskResult::Started(
                            WebcamCapture::start_for_recording(
                                &camera_id,
                                minimum_dimension,
                                square_crop,
                            ),
                        ));
                    });
                match spawn {
                    Ok(_) => self.pending_response = Some(response),
                    Err(error) => {
                        let _ = response.send(Err(IpcError::new(
                            "webcam_start_failed",
                            format!("Could not start the webcam worker: {error}"),
                            true,
                        )));
                    }
                }
            } else {
                let _ = response.send(Err(IpcError::new(
                    "webcam_camera_unselected",
                    "Choose a camera in Webcam Overlay settings before enabling it.",
                    true,
                )));
            }
            return;
        }

        self.enabled = false;
        self.clear(screen_bgra);
        *pending_video_frame = true;
        if let Some(mut capture) = self.capture.take() {
            let publisher = self.preview_publisher.take();
            self.shutdown_pending = true;
            let sender = self.task_tx.clone();
            match thread::Builder::new()
                .name("recodex-webcam-stop".to_owned())
                .spawn(move || {
                    if let Some(mut publisher) = publisher {
                        publisher.stop();
                    }
                    capture.stop();
                    let _ = sender.send(WebcamTaskResult::Stopped);
                }) {
                Ok(_) => {}
                Err(error) => {
                    emit_warning(
                        context.app,
                        &format!("Could not release webcam capture on a worker thread: {error}"),
                    );
                    // Do not join a camera worker from the recording loop.
                    self.shutdown_pending = false;
                }
            }
        }
        let result = update_state(
            context.app,
            context.session,
            context.session_id,
            false,
            self.available,
        );
        let _ = response.send(result);
    }

    pub fn poll_tasks(
        &mut self,
        app: &AppHandle,
        session: &Arc<Mutex<RecordingSession>>,
        session_id: &str,
        pending_video_frame: &mut bool,
    ) {
        loop {
            match self.task_rx.try_recv() {
                Ok(WebcamTaskResult::Started(Ok(camera))) => {
                    match self.install_capture(camera, app) {
                        Ok(()) => {
                            if let Some(warning) = self.image_warning() {
                                emit_warning(app, warning);
                            }
                            self.enabled = true;
                            *pending_video_frame = true;
                            self.finish_pending(app, session, session_id, Ok(()));
                        }
                        Err(error) => {
                            emit_warning(app, &error.message);
                            self.enabled = false;
                            self.finish_pending(app, session, session_id, Err(error));
                        }
                    }
                }
                Ok(WebcamTaskResult::Started(Err(error))) => {
                    emit_warning(app, &error.message);
                    self.finish_pending(app, session, session_id, Err(error));
                }
                Ok(WebcamTaskResult::Stopped) => self.shutdown_pending = false,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    pub fn cancel_pending(&mut self) {
        if let Some(response) = self.pending_response.take() {
            let _ = response.send(Err(IpcError::invalid_state(
                "Recording stopped before the webcam change completed.",
            )));
        }
    }

    pub fn handle_disconnect(
        &mut self,
        app: &AppHandle,
        session: &Arc<Mutex<RecordingSession>>,
        session_id: &str,
        screen_bgra: &mut [u8],
    ) {
        self.enabled = false;
        self.clear(screen_bgra);
        if let Some(mut capture) = self.capture.take() {
            let publisher = self.preview_publisher.take();
            self.shutdown_pending = true;
            let sender = self.task_tx.clone();
            if thread::Builder::new()
                .name("recodex-webcam-disconnect".to_owned())
                .spawn(move || {
                    if let Some(mut publisher) = publisher {
                        publisher.stop();
                    }
                    capture.stop();
                    let _ = sender.send(WebcamTaskResult::Stopped);
                })
                .is_err()
            {
                self.shutdown_pending = false;
            }
        }
        if let Ok(event) = session
            .lock()
            .map_err(|_| IpcError::internal("Recording state lock failed."))
            .and_then(|mut session| session.set_webcam_state(session_id, false, self.available))
        {
            let _ = emit_state_changed(app, &event);
        }
    }

    fn install_capture(&mut self, camera: WebcamCapture, app: &AppHandle) -> Result<(), IpcError> {
        let frame = camera.latest_frame().ok_or_else(|| {
            IpcError::new(
                "webcam_frame_unavailable",
                "The camera did not provide a frame.",
                true,
            )
        })?;
        let mut compositor = WebcamCompositor::new(
            self.settings.clone(),
            self.frame_width,
            self.frame_height,
            frame.width,
            frame.height,
        )?;
        let publisher = PreviewPublisher::start(app.clone(), camera.reader())?;
        compositor.move_by(
            self.moved_x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            self.moved_y.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        );
        self.last_sequence = frame.sequence;
        self.camera_dimensions = Some((frame.width, frame.height));
        self.compositor = Some(compositor);
        self.preview_publisher = Some(publisher);
        self.capture = Some(camera);
        self.available = true;
        Ok(())
    }

    fn image_warning(&self) -> Option<&str> {
        self.compositor.as_ref()?.image_warning()
    }

    fn finish_pending(
        &mut self,
        app: &AppHandle,
        session: &Arc<Mutex<RecordingSession>>,
        session_id: &str,
        outcome: Result<(), IpcError>,
    ) {
        let Some(response) = self.pending_response.take() else {
            return;
        };
        let result = outcome
            .and_then(|()| update_state(app, session, session_id, self.enabled, self.available));
        let _ = response.send(result);
    }
}

fn recording_capture_profile(settings: &WebcamSettings, frame_height: u32) -> (u32, bool) {
    let output_width = (u32::from(settings.width) * frame_height / 1080).max(1);
    let minimum_dimension = output_width.saturating_mul(5).div_ceil(4).clamp(320, 1280);
    (minimum_dimension, settings.shape == WebcamShape::Circle)
}

fn update_state(
    app: &AppHandle,
    session: &Arc<Mutex<RecordingSession>>,
    session_id: &str,
    enabled: bool,
    available: bool,
) -> Result<RecordingSnapshot, IpcError> {
    let event = session
        .lock()
        .map_err(|_| IpcError::internal("Recording state lock failed."))?
        .set_webcam_state(session_id, enabled, available)?;
    emit_state_changed(app, &event)?;
    snapshot(session)
}

fn snapshot(session: &Arc<Mutex<RecordingSession>>) -> Result<RecordingSnapshot, IpcError> {
    session
        .lock()
        .map(|session| session.snapshot())
        .map_err(|_| IpcError::internal("Recording state lock failed."))
}

fn emit_warning(app: &AppHandle, message: &str) {
    let _ = app.emit(RECORDING_WARNING_EVENT, message.to_owned());
}

#[cfg(test)]
mod tests {
    use super::recording_capture_profile;
    use crate::webcam::settings::{WebcamSettings, WebcamShape};

    #[test]
    fn recording_camera_profile_scales_with_output_resolution() {
        let settings = WebcamSettings::default();
        assert_eq!(recording_capture_profile(&settings, 1080), (320, false));
        assert_eq!(recording_capture_profile(&settings, 2160), (450, false));
    }

    #[test]
    fn recording_camera_profile_caps_at_supported_capture_bounds_and_tracks_circle_crop() {
        let settings = WebcamSettings {
            width: 640,
            shape: WebcamShape::Circle,
            ..WebcamSettings::default()
        };
        assert_eq!(recording_capture_profile(&settings, 2160), (1280, true));
    }
}
