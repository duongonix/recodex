use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, SyncSender, TryRecvError},
    },
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    errors::IpcError,
    recording::{
        AudioSource, RecordingOptions, RecordingQuality, RecordingSession, SessionStatus,
        emit_progress, emit_state_changed,
    },
    webcam::{manager::RECORDING_WARNING_EVENT, overlay_window},
};

use super::webcam::{RecordingContext, RecordingWebcam};
use super::{
    ActiveClock, WorkerCommand, check_startup_cancellation, respond_with_snapshot,
    transition_and_emit, update_progress,
};

pub(super) fn run_platform(
    app: AppHandle,
    session: Arc<Mutex<RecordingSession>>,
    session_id: &str,
    options: RecordingOptions,
    commands: Receiver<WorkerCommand>,
    ready: &mut Option<SyncSender<Result<(), IpcError>>>,
) -> Result<(), IpcError> {
    use crate::{
        audio::{AudioCapture, AudioMixer},
        capture::{
            platform::windows::{CursorCompositor, WindowsCapture},
            sources::list_displays,
        },
        recording::output::VideoOutput,
    };
    let _zoom_release = ClearZoomHotkeyOnDrop(app.clone());

    let displays = list_displays(&app)?;
    let mut capture = WindowsCapture::start_for_recording(
        &options.target,
        &displays,
        options.fps,
        options.show_mouse_cursor,
        options.zoom_on_hold_enabled,
    )?;
    check_startup_cancellation(&commands)?;
    let frame_deadline = Instant::now() + Duration::from_secs(10);
    let pre_hide_frame = loop {
        check_startup_cancellation(&commands)?;
        if Instant::now() >= frame_deadline {
            return Err(IpcError::new(
                "capture_first_frame_timeout",
                "Windows Graphics Capture did not deliver a frame within 10 seconds. Refresh the selected source and try again.",
                true,
            ));
        }
        if let Some(frame) = capture.wait_for_frame(Duration::from_millis(100))? {
            break frame;
        }
    };

    let main_window = app.get_webview_window("main").ok_or_else(|| {
        IpcError::new(
            "main_window_unavailable",
            "The main window is unavailable.",
            true,
        )
    })?;
    main_window.hide().map_err(|error| {
        IpcError::new(
            "main_window_hide_failed",
            format!("Could not hide the main window before capture: {error}"),
            true,
        )
    })?;
    let frame_deadline = Instant::now() + Duration::from_secs(3);
    let mut first_frame = loop {
        check_startup_cancellation(&commands)?;
        if Instant::now() >= frame_deadline {
            return Err(IpcError::new(
                "capture_refresh_timeout",
                "Windows Graphics Capture did not deliver a fresh frame after the main window was hidden. The main window has been restored; try the source again.",
                true,
            ));
        }
        if let Some(frame) = capture.wait_for_frame(Duration::from_millis(100))?
            && frame.timestamp_100ns > pre_hide_frame.timestamp_100ns
        {
            break frame;
        }
    };
    let startup_frame_timestamp = first_frame.timestamp_100ns;
    let frame_width = first_frame.width;
    let frame_height = first_frame.height;

    let webcam_settings = app
        .try_state::<crate::app_state::AppState>()
        .and_then(|state| state.settings.get().ok())
        .map(|settings| settings.webcam)
        .unwrap_or_default();
    let mut webcam = RecordingWebcam::start(webcam_settings, frame_width, frame_height, &app);
    let mut webcam_disconnect_reported = false;

    check_startup_cancellation(&commands)?;
    let mut output = VideoOutput::create_mixed_audio(
        Path::new(&options.output_directory),
        first_frame.width,
        first_frame.height,
        options.max_height,
        options.fps,
        options.microphone_enabled || options.system_audio,
    )?;
    let audio = AudioCapture::start(
        options.microphone_device_id.as_deref(),
        options.microphone_enabled,
        options.system_audio,
    )?;
    let mut audio_mixer = AudioMixer::new(options.microphone_enabled, options.system_audio);
    let mut audio_diagnostics =
        AudioDiagnostics::new(options.microphone_enabled, options.system_audio);
    check_startup_cancellation(&commands)?;

    // Camera, encoder, and audio setup can take several seconds; establish the shared A/V origin
    // from a fresh WGC frame only after those blocking startup steps have completed.
    let frame_deadline = Instant::now() + Duration::from_secs(3);
    first_frame = loop {
        check_startup_cancellation(&commands)?;
        if Instant::now() >= frame_deadline {
            return Err(IpcError::new(
                "capture_refresh_timeout",
                "Windows Graphics Capture did not deliver a fresh frame after recording setup. The main window has been restored; try the source again.",
                true,
            ));
        }
        if let Some(frame) = capture.wait_for_frame(Duration::from_millis(100))?
            && frame.timestamp_100ns > startup_frame_timestamp
        {
            break frame;
        }
    };
    if first_frame.width != frame_width || first_frame.height != frame_height {
        return Err(IpcError::new(
            "capture_resolution_changed_during_startup",
            "The capture source resolution changed while recording was starting. Select the source again and retry.",
            true,
        ));
    }
    let first_frame_capture_timestamp = first_frame.timestamp_100ns;
    let capture_origin = first_frame_capture_timestamp;
    first_frame.timestamp_100ns = first_frame.timestamp_100ns.saturating_sub(capture_origin);
    let mut cursor_compositor = CursorCompositor::default();
    cursor_compositor.set_base_frame(&first_frame)?;
    if options.show_mouse_cursor {
        cursor_compositor.update(capture.cursor_snapshot(), Instant::now());
    }
    if webcam.enabled() {
        webcam.composite(cursor_compositor.pixels_mut())?;
    }
    output.write_frame_pixels(
        frame_width,
        frame_height,
        cursor_compositor.pixels(),
        first_frame.timestamp_100ns,
    )?;
    let _ = capture.take_dropped_frame_count();
    let mut clock = ActiveClock::start();
    let webcam_state = session
        .lock()
        .map_err(|_| IpcError::internal("The recording state lock failed."))
        .and_then(|mut session| {
            session.set_audio_availability(options.microphone_enabled, options.system_audio)?;
            session.set_webcam_state(session_id, webcam.enabled(), webcam.available())
        })
        .map_err(|error| output.recovery_error(error))?;
    let _ = emit_state_changed(&app, &webcam_state);
    transition_and_emit(
        &app,
        &session,
        session_id,
        SessionStatus::Recording,
        0,
        None,
        None,
    )
    .map_err(|error| output.recovery_error(error))?;
    let toolbar = app
        .get_webview_window("recording-toolbar")
        .ok_or_else(|| {
            IpcError::new(
                "toolbar_window_unavailable",
                "The recording toolbar window is unavailable.",
                true,
            )
        })
        .map_err(|error| output.recovery_error(error))?;
    position_toolbar_for_recording(&app, &toolbar, &options.target, &displays, &webcam)
        .map_err(|error| output.recovery_error(error))?;
    crate::capture::platform::windows::exclude_window_from_capture(&toolbar)
        .map_err(|error| output.recovery_error(error))?;
    let mut placed_webcam_enabled = webcam.enabled();
    toolbar.show().map_err(|error| {
        output.recovery_error(IpcError::new(
            "toolbar_show_failed",
            format!("Could not show the recording toolbar: {error}"),
            true,
        ))
    })?;
    ready
        .take()
        .ok_or_else(|| IpcError::internal("The recording start response was already sent."))?
        .send(Ok(()))
        .map_err(|_| {
            output.recovery_error(IpcError::new(
                "recording_start_abandoned",
                "The recording start request was cancelled before capture became ready.",
                true,
            ))
        })?;

    let mut last_progress = Instant::now();
    let mut last_quality_sample = Instant::now();
    let mut last_disk_sample = Instant::now();
    let mut captured_frames_in_sample = 0_u64;
    let mut quality = RecordingQuality {
        available_disk_bytes: crate::recording::preflight::free_bytes_for_path(Path::new(
            &options.output_directory,
        )),
        ..RecordingQuality::default()
    };
    let frame_interval = 10_000_000_i64 / i64::from(options.fps);
    let mut last_encoded_timestamp = first_frame.timestamp_100ns;
    let mut latest_capture_timestamp = first_frame_capture_timestamp;
    let mut latest_capture_arrival = Instant::now();
    let mut pending_video_frame = false;
    let pending_stop_result: Result<_, IpcError> = (|| {
        let pending_stop = loop {
            webcam.poll_tasks(&app, &session, session_id, &mut pending_video_frame);
            match commands.try_recv() {
                Ok(WorkerCommand::Pause(response)) => {
                    let current = session
                        .lock()
                        .map_err(|_| IpcError::internal("The recording state lock failed."))?
                        .snapshot();
                    if current.status != SessionStatus::Recording {
                        let _ = response.send(Err(IpcError::invalid_state(
                            "Only an active recording can be paused.",
                        )));
                        continue;
                    }
                    let pause_result = (|| {
                        if current.microphone_enabled {
                            audio.set_enabled(AudioSource::Microphone, false)?;
                        }
                        if current.system_audio_enabled {
                            audio.set_enabled(AudioSource::System, false)?;
                        }
                        write_audio_samples(
                            AudioWriteContext {
                                app: &app,
                                session: &session,
                                session_id,
                                output: &mut output,
                                mixer: &mut audio_mixer,
                                capture_origin_100ns: capture_origin,
                                paused_duration_100ns: clock.paused_duration_100ns(),
                                paused: false,
                                flush_all: false,
                                diagnostics: &mut audio_diagnostics,
                            },
                            audio.drain()?,
                        )?;
                        let duration = clock.pause();
                        transition_and_emit(
                            &app,
                            &session,
                            session_id,
                            SessionStatus::Paused,
                            duration,
                            None,
                            None,
                        )
                    })();
                    if let Err(error) = &pause_result {
                        let _ = response.send(Err(error.clone()));
                        return Err(error.clone());
                    }
                    respond_with_snapshot(&session, response, pause_result);
                }
                Ok(WorkerCommand::Resume(response)) => {
                    let current = session
                        .lock()
                        .map_err(|_| IpcError::internal("The recording state lock failed."))?
                        .snapshot();
                    if current.status != SessionStatus::Paused {
                        let _ = response.send(Err(IpcError::invalid_state(
                            "Only a paused recording can be resumed.",
                        )));
                        continue;
                    }
                    let resume_result = (|| {
                        if current.microphone_enabled {
                            audio.set_enabled(AudioSource::Microphone, true)?;
                        }
                        if current.system_audio_enabled {
                            audio.set_enabled(AudioSource::System, true)?;
                        }
                        clock.resume();
                        transition_and_emit(
                            &app,
                            &session,
                            session_id,
                            SessionStatus::Recording,
                            clock.elapsed_ms(),
                            None,
                            None,
                        )
                    })();
                    if let Err(error) = &resume_result {
                        let _ = response.send(Err(error.clone()));
                        return Err(error.clone());
                    }
                    respond_with_snapshot(&session, response, resume_result);
                }
                Ok(WorkerCommand::Stop(response)) => {
                    webcam.cancel_pending();
                    break Some(response);
                }
                Ok(WorkerCommand::Cancel) => {
                    webcam.cancel_pending();
                    break None;
                }
                Ok(WorkerCommand::SetAudio {
                    source,
                    enabled,
                    response,
                }) => {
                    let update = (|| {
                        let current = session
                            .lock()
                            .map_err(|_| IpcError::internal("The recording state lock failed."))?
                            .snapshot();
                        let available = match source {
                            AudioSource::Microphone => current.microphone_available,
                            AudioSource::System => current.system_audio_available,
                        };
                        if !available {
                            return Err(IpcError::new(
                                "audio_source_unavailable",
                                "This audio source was not enabled when the recording started.",
                                true,
                            ));
                        }
                        if current.status == SessionStatus::Recording {
                            audio.set_enabled(source, enabled)?;
                        }
                        let event = session
                            .lock()
                            .map_err(|_| IpcError::internal("The recording state lock failed."))?
                            .set_audio_enabled(session_id, source, enabled)?;
                        audio_mixer.set_source_enabled(source, enabled);
                        emit_state_changed(&app, &event)?;
                        session
                            .lock()
                            .map(|session| session.snapshot())
                            .map_err(|_| IpcError::internal("The recording state lock failed."))
                    })();
                    let _ = response.send(update);
                }
                Ok(WorkerCommand::SetWebcam { enabled, response }) => {
                    webcam.set_enabled(
                        enabled,
                        &RecordingContext {
                            app: &app,
                            session: &session,
                            session_id,
                        },
                        cursor_compositor.pixels_mut(),
                        &mut pending_video_frame,
                        response,
                    );
                }
                Ok(WorkerCommand::MoveWebcam { delta_x, delta_y }) => {
                    webcam.move_by(delta_x, delta_y, cursor_compositor.pixels_mut())?;
                    pending_video_frame = true;
                }
                Err(TryRecvError::Disconnected) => {
                    return Err(IpcError::new(
                        "recording_control_lost",
                        "Recording controls disconnected unexpectedly; capture was stopped.",
                        true,
                    ));
                }
                Err(TryRecvError::Empty) => {}
            }

            write_audio_samples(
                AudioWriteContext {
                    app: &app,
                    session: &session,
                    session_id,
                    output: &mut output,
                    mixer: &mut audio_mixer,
                    capture_origin_100ns: capture_origin,
                    paused_duration_100ns: clock.paused_duration_100ns(),
                    paused: clock.is_paused(),
                    flush_all: false,
                    diagnostics: &mut audio_diagnostics,
                },
                audio.drain()?,
            )?;

            let cursor_snapshot = capture.cursor_snapshot();
            if options.zoom_on_hold_enabled {
                let zoom_held =
                    app.try_state::<crate::app_state::AppState>()
                        .is_some_and(|state| {
                            state
                                .zoom_hotkey_held
                                .load(std::sync::atomic::Ordering::Acquire)
                        });
                capture.update_zoom_input(cursor_snapshot, zoom_held && !clock.is_paused());
            }

            if let Some(frame) = capture.wait_for_frame(Duration::from_millis(8))?
                && !clock.is_paused()
            {
                captured_frames_in_sample = captured_frames_in_sample.saturating_add(1);
                latest_capture_timestamp = frame.timestamp_100ns;
                latest_capture_arrival = Instant::now();
                cursor_compositor.set_base_frame(&frame)?;
                webcam.discard_underlay();
                pending_video_frame = true;
            }

            let webcam_error = webcam.error();
            if webcam_error.is_none() {
                webcam_disconnect_reported = false;
            }
            if let Some(message) = webcam_error
                && !webcam_disconnect_reported
            {
                webcam_disconnect_reported = true;
                emit_webcam_warning(&app, &message);
                webcam.handle_disconnect(
                    &app,
                    &session,
                    session_id,
                    cursor_compositor.pixels_mut(),
                );
                pending_video_frame = true;
            }

            if webcam.enabled() != placed_webcam_enabled {
                position_toolbar_for_recording(
                    &app,
                    &toolbar,
                    &options.target,
                    &displays,
                    &webcam,
                )?;
                placed_webcam_enabled = webcam.enabled();
            }

            if webcam.latest_sequence().is_some() {
                pending_video_frame = true;
            }

            if !clock.is_paused() {
                let now = Instant::now();
                if options.show_mouse_cursor {
                    webcam.restore_underlay(cursor_compositor.pixels_mut());
                    pending_video_frame |= cursor_compositor.update(cursor_snapshot, now);
                }
                if pending_video_frame {
                    let since_capture_100ns = now
                        .saturating_duration_since(latest_capture_arrival)
                        .as_nanos()
                        .saturating_div(100)
                        .min(i64::MAX as u128) as i64;
                    let timestamp = latest_capture_timestamp
                        .saturating_sub(capture_origin)
                        .saturating_sub(clock.paused_duration_100ns())
                        .saturating_add(since_capture_100ns)
                        .max(0);
                    let frame_tick = timestamp.div_euclid(frame_interval) * frame_interval;
                    if frame_tick.saturating_sub(last_encoded_timestamp) >= frame_interval {
                        webcam.composite(cursor_compositor.pixels_mut())?;
                        output.write_frame_pixels(
                            frame_width,
                            frame_height,
                            cursor_compositor.pixels(),
                            frame_tick,
                        )?;
                        last_encoded_timestamp = frame_tick;
                        pending_video_frame = false;
                    }
                }
            }

            if last_progress.elapsed() >= Duration::from_millis(250) {
                let now = Instant::now();
                if now.saturating_duration_since(last_quality_sample) >= Duration::from_secs(1) {
                    let elapsed = now
                        .saturating_duration_since(last_quality_sample)
                        .as_secs_f64();
                    quality.capture_fps = Some(
                        (captured_frames_in_sample as f64 / elapsed)
                            .round()
                            .clamp(0.0, f64::from(u16::MAX)) as u16,
                    );
                    captured_frames_in_sample = 0;
                    if clock.is_paused() {
                        let _ = capture.take_dropped_frame_count();
                    } else {
                        quality.dropped_video_frames = quality
                            .dropped_video_frames
                            .saturating_add(capture.take_dropped_frame_count());
                    }
                    last_quality_sample = now;
                }
                quality.audio_gaps = audio_diagnostics.audio_gap_count;
                (
                    quality.microphone_level_percent,
                    quality.system_audio_level_percent,
                ) = audio_diagnostics.recent_levels(now);
                if now.saturating_duration_since(last_disk_sample) >= Duration::from_secs(5) {
                    quality.available_disk_bytes = crate::recording::preflight::free_bytes_for_path(
                        Path::new(&options.output_directory),
                    );
                    last_disk_sample = now;
                }
                if !clock.is_paused() {
                    let event =
                        update_progress(&session, session_id, clock.elapsed_ms(), quality.clone())?;
                    let _ = emit_progress(&app, &event);
                }
                last_progress = now;
            }
        };
        Ok(pending_stop)
    })();
    let pending_stop = pending_stop_result.map_err(|error| output.recovery_error(error))?;

    let current_duration = clock.elapsed_ms();
    transition_and_emit(
        &app,
        &session,
        session_id,
        SessionStatus::Stopping,
        current_duration,
        None,
        None,
    )
    .map_err(|error| output.recovery_error(error))?;
    let trailing_audio = audio.stop().map_err(|error| output.recovery_error(error))?;
    write_audio_samples(
        AudioWriteContext {
            app: &app,
            session: &session,
            session_id,
            output: &mut output,
            mixer: &mut audio_mixer,
            capture_origin_100ns: capture_origin,
            paused_duration_100ns: clock.paused_duration_100ns(),
            paused: clock.is_paused(),
            flush_all: true,
            diagnostics: &mut audio_diagnostics,
        },
        trailing_audio,
    )?;
    audio_diagnostics.report(audio_mixer.diagnostics());
    capture
        .stop()
        .map_err(|error| output.recovery_error(error))?;
    transition_and_emit(
        &app,
        &session,
        session_id,
        SessionStatus::Finalizing,
        current_duration,
        None,
        None,
    )
    .map_err(|error| output.recovery_error(error))?;
    if let Some(toolbar) = app.get_webview_window("recording-toolbar") {
        let _ = toolbar.hide();
    }

    if options.show_preview_after_recording {
        let pending = output.finalize_for_review()?;
        let preview_path = pending.preview_path().to_owned();
        let scope = app.asset_protocol_scope();
        match scope.allow_file(&preview_path) {
            Ok(_) => {
                let state = app.state::<crate::app_state::AppState>();
                match state.recording_review.begin(
                    session_id.to_owned(),
                    current_duration,
                    options.fps,
                    pending,
                ) {
                    Ok(_) => {
                        transition_and_emit(
                            &app,
                            &session,
                            session_id,
                            SessionStatus::Reviewing,
                            current_duration,
                            None,
                            None,
                        )?;
                        if let Err(error) =
                            crate::commands::recording_review::show_review_after_recording(&app)
                        {
                            eprintln!("[RecoDex][recording-review] {error:?}");
                            crate::commands::recording_review::report_review_error(
                                &app,
                                &error.message,
                            );
                            crate::commands::recording_review::save_pending_for_quit(&app)?;
                        } else {
                            crate::commands::recording_review::emit_review_ready(&app);
                        }
                    }
                    Err(error) => {
                        let _ = scope.forbid_file(&preview_path);
                        eprintln!(
                            "[RecoDex][recording-review] Could not retain recording for review: {error:?}"
                        );
                        return Err(error);
                    }
                }
            }
            Err(error) => {
                eprintln!(
                    "[RecoDex][recording-review] Could not grant preview file access: {error}"
                );
                let mut pending = pending;
                let output_path = pending.save()?;
                register_completed_video(&app, &output_path, current_duration);
                transition_and_emit(
                    &app,
                    &session,
                    session_id,
                    SessionStatus::Completed,
                    current_duration,
                    Some(output_path.to_string_lossy().into_owned()),
                    None,
                )?;
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.show();
                }
            }
        }
    } else {
        let output_path = output.finalize()?;
        register_completed_video(&app, &output_path, current_duration);
        transition_and_emit(
            &app,
            &session,
            session_id,
            SessionStatus::Completed,
            current_duration,
            Some(output_path.to_string_lossy().into_owned()),
            None,
        )?;
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
        }
    }
    let snapshot = session
        .lock()
        .map_err(|_| IpcError::internal("The recording state lock failed."))?
        .snapshot();
    if let Some(pending_stop) = pending_stop {
        let _ = pending_stop.send(Ok(snapshot));
    }
    Ok(())
}

fn register_completed_video(app: &AppHandle, path: &Path, duration_ms: u64) {
    if let Some(state) = app.try_state::<crate::app_state::AppState>()
        && let Err(error) = state.capture_library.register(
            path,
            crate::library::LibraryItemKind::Video,
            Some(duration_ms),
        )
    {
        eprintln!(
            "[RecoDex][library] Could not register the saved recording: {}",
            error.message
        );
    }
}

fn emit_webcam_warning(app: &AppHandle, message: &str) {
    eprintln!("[RecoDex][webcam] {message}");
    let _ = app.emit(RECORDING_WARNING_EVENT, message.to_owned());
}

fn position_toolbar_for_recording(
    app: &AppHandle,
    toolbar: &tauri::WebviewWindow,
    target: &crate::recording::CaptureTarget,
    displays: &[crate::capture::sources::DisplaySource],
    webcam: &RecordingWebcam,
) -> Result<(), IpcError> {
    let bounds = match target {
        crate::recording::CaptureTarget::Display { display_id } => {
            let display = crate::capture::sources::find_display(displays, display_id)?;
            (display.x, display.y, display.width, display.height)
        }
        crate::recording::CaptureTarget::Region {
            x,
            y,
            width,
            height,
            ..
        } => (*x, *y, *width, *height),
        crate::recording::CaptureTarget::Window { window_id } => {
            crate::capture::platform::windows::recordable_window_bounds(window_id)?
        }
    };
    webcam.configure_overlay_window(app)?;
    overlay_window::place_recording_stack(app, toolbar, bounds, webcam.enabled())
}

struct ClearZoomHotkeyOnDrop(AppHandle);

impl Drop for ClearZoomHotkeyOnDrop {
    fn drop(&mut self) {
        if let Some(state) = self.0.try_state::<crate::app_state::AppState>() {
            state
                .zoom_hotkey_held
                .store(false, std::sync::atomic::Ordering::Release);
        }
    }
}

#[cfg(windows)]
#[derive(Default)]
struct AudioDiagnostics {
    microphone_enabled: bool,
    system_enabled: bool,
    microphone_packets: u64,
    microphone_signal_packets: u64,
    microphone_peak: u16,
    system_packets: u64,
    system_signal_packets: u64,
    system_peak: u16,
    negative_timestamp_packets: u64,
    mixed_output_packets: u64,
    mixed_output_peak: u16,
    audio_gap_count: u64,
    microphone_level_percent: u8,
    microphone_level_at: Option<Instant>,
    system_level_percent: u8,
    system_level_at: Option<Instant>,
}

#[cfg(windows)]
impl AudioDiagnostics {
    fn new(microphone_enabled: bool, system_enabled: bool) -> Self {
        Self {
            microphone_enabled,
            system_enabled,
            ..Self::default()
        }
    }

    fn record_input(
        &mut self,
        source: AudioSource,
        sample: &crate::audio::AudioSample,
        relative_timestamp_100ns: i64,
    ) {
        let peak = pcm_peak(&sample.pcm_s16le);
        let level = crate::audio::measure_s16le(&sample.pcm_s16le);
        if sample.discontinuity {
            self.audio_gap_count = self.audio_gap_count.saturating_add(1);
        }
        if relative_timestamp_100ns < 0 {
            self.negative_timestamp_packets = self.negative_timestamp_packets.saturating_add(1);
        }
        let (packets, signal_packets, source_peak) = match source {
            AudioSource::Microphone => (
                &mut self.microphone_packets,
                &mut self.microphone_signal_packets,
                &mut self.microphone_peak,
            ),
            AudioSource::System => (
                &mut self.system_packets,
                &mut self.system_signal_packets,
                &mut self.system_peak,
            ),
        };
        *packets = packets.saturating_add(1);
        if peak > 0 {
            *signal_packets = signal_packets.saturating_add(1);
            *source_peak = (*source_peak).max(peak);
        }
        let now = Instant::now();
        match source {
            AudioSource::Microphone => {
                self.microphone_level_percent = level.rms_percent;
                self.microphone_level_at = Some(now);
            }
            AudioSource::System => {
                self.system_level_percent = level.rms_percent;
                self.system_level_at = Some(now);
            }
        }
    }

    fn recent_levels(&self, now: Instant) -> (u8, u8) {
        const METER_STALE_AFTER: Duration = Duration::from_millis(600);
        let recent = |level: u8, updated: Option<Instant>| {
            updated
                .filter(|updated| now.saturating_duration_since(*updated) <= METER_STALE_AFTER)
                .map_or(0, |_| level)
        };
        (
            recent(self.microphone_level_percent, self.microphone_level_at),
            recent(self.system_level_percent, self.system_level_at),
        )
    }

    fn record_mixed_output(&mut self, sample: &crate::audio::AudioMixSample) {
        self.mixed_output_packets = self.mixed_output_packets.saturating_add(1);
        self.mixed_output_peak = self.mixed_output_peak.max(pcm_peak(&sample.pcm_s16le));
    }

    fn report(&self, mixer: &crate::audio::AudioMixerDiagnostics) {
        #[cfg(debug_assertions)]
        eprintln!(
            "[RecoDex][audio] selected mic={} system={}; input packets mic={} (signal {}, peak {}), system={} (signal {}, peak {}); relative negative timestamps={}; mixed output packets={} (peak {}); mixer enabled mic={} system={}, received mic={} system={}, accepted mic={} system={}, negative={} stale={} disabled={}, boundary adjustments={}, inserted signal frames mic={} system={}, mixed output signal frames={}",
            self.microphone_enabled,
            self.system_enabled,
            self.microphone_packets,
            self.microphone_signal_packets,
            self.microphone_peak,
            self.system_packets,
            self.system_signal_packets,
            self.system_peak,
            self.negative_timestamp_packets,
            self.mixed_output_packets,
            self.mixed_output_peak,
            mixer.microphone_enabled,
            mixer.system_enabled,
            mixer.microphone_received_packets,
            mixer.system_received_packets,
            mixer.microphone_accepted_packets,
            mixer.system_accepted_packets,
            mixer.negative_timestamp_packets,
            mixer.stale_packets,
            mixer.disabled_packets,
            mixer.packet_boundary_adjustments,
            mixer.microphone_inserted_signal_frames,
            mixer.system_inserted_signal_frames,
            mixer.mixed_output_signal_frames,
        );
        #[cfg(not(debug_assertions))]
        let _ = (self, mixer);
    }
}

#[cfg(windows)]
fn pcm_peak(pcm_s16le: &[u8]) -> u16 {
    pcm_s16le
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]).unsigned_abs())
        .max()
        .unwrap_or(0)
}

#[cfg(windows)]
struct AudioWriteContext<'a> {
    app: &'a AppHandle,
    session: &'a Arc<Mutex<RecordingSession>>,
    session_id: &'a str,
    output: &'a mut crate::recording::output::VideoOutput,
    mixer: &'a mut crate::audio::AudioMixer,
    capture_origin_100ns: i64,
    paused_duration_100ns: i64,
    paused: bool,
    flush_all: bool,
    diagnostics: &'a mut AudioDiagnostics,
}

#[cfg(windows)]
fn write_audio_samples(
    context: AudioWriteContext<'_>,
    samples: impl IntoIterator<Item = crate::audio::AudioSample>,
) -> Result<(), IpcError> {
    let AudioWriteContext {
        app,
        session,
        session_id,
        output,
        mixer,
        capture_origin_100ns,
        paused_duration_100ns,
        paused,
        flush_all,
        diagnostics,
    } = context;
    for sample in samples {
        if sample.discontinuity {
            emit_audio_gap_warning(app, session, session_id, sample.source)?;
        }
        let timestamp_100ns = sample
            .timestamp_100ns
            .saturating_sub(capture_origin_100ns)
            .saturating_sub(paused_duration_100ns);
        diagnostics.record_input(sample.source, &sample, timestamp_100ns);
        mixer.push(sample, timestamp_100ns)?;
    }
    flush_audio_samples(output, mixer, paused, flush_all, diagnostics)
}

#[cfg(windows)]
fn emit_audio_gap_warning(
    app: &AppHandle,
    session: &Arc<Mutex<RecordingSession>>,
    session_id: &str,
    source: AudioSource,
) -> Result<(), IpcError> {
    let source = match source {
        AudioSource::Microphone => "microphone",
        AudioSource::System => "system",
    };
    let event = session
        .lock()
        .map_err(|_| IpcError::internal("The recording state lock failed."))?
        .set_audio_warning(
            session_id,
            format!(
                "Windows reported a gap in {source} audio. Recording continues, but some audio may be missing."
            ),
        )?;
    if let Some(event) = event {
        let _ = emit_state_changed(app, &event);
    }
    Ok(())
}

#[cfg(windows)]
fn flush_audio_samples(
    output: &mut crate::recording::output::VideoOutput,
    mixer: &mut crate::audio::AudioMixer,
    paused: bool,
    flush_all: bool,
    diagnostics: &mut AudioDiagnostics,
) -> Result<(), IpcError> {
    let samples = if flush_all {
        mixer.flush_all()
    } else if paused {
        Vec::new()
    } else {
        mixer.flush_available()
    };
    for sample in samples {
        diagnostics.record_mixed_output(&sample);
        output.write_mixed_audio(&sample)?;
    }
    Ok(())
}
