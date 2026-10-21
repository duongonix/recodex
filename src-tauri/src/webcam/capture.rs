use std::{
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use image::RgbImage;
use nokhwa::{
    Camera,
    pixel_format::RgbFormat,
    utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType},
};

use crate::errors::IpcError;

const CAMERA_MAX_FPS: u32 = 30;
const CAMERA_FRAME_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug)]
pub struct WebcamFrame {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub rgb: RgbImage,
    pub captured_at: Instant,
}

#[derive(Default)]
struct CaptureShared {
    latest: RwLock<Option<Arc<WebcamFrame>>>,
    error: Mutex<Option<String>>,
}

#[derive(Clone, Default)]
pub struct WebcamFrameReader {
    shared: Arc<CaptureShared>,
}

impl WebcamFrameReader {
    pub fn latest(&self) -> Option<Arc<WebcamFrame>> {
        self.shared.latest.read().ok()?.clone()
    }

    pub fn error(&self) -> Option<String> {
        self.shared.error.lock().ok()?.clone()
    }
}

pub struct WebcamCapture {
    shared: Arc<CaptureShared>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl WebcamCapture {
    pub fn start(camera_id: &str) -> Result<Self, IpcError> {
        Self::start_with_recording_profile(camera_id, None)
    }

    pub fn start_for_recording(
        camera_id: &str,
        minimum_dimension: u32,
        square_crop: bool,
    ) -> Result<Self, IpcError> {
        Self::start_with_recording_profile(
            camera_id,
            Some((minimum_dimension.clamp(1, 1280), square_crop)),
        )
    }

    fn start_with_recording_profile(
        camera_id: &str,
        recording_profile: Option<(u32, bool)>,
    ) -> Result<Self, IpcError> {
        let camera_index = parse_camera_index(camera_id)?;
        let shared = Arc::new(CaptureShared::default());
        let stop = Arc::new(AtomicBool::new(false));
        let thread_shared = shared.clone();
        let thread_stop = stop.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("recodex-webcam-capture".to_owned())
            .spawn(move || {
                capture_loop(
                    camera_index,
                    recording_profile,
                    thread_shared,
                    thread_stop,
                    ready_tx,
                )
            })
            .map_err(|error| {
                IpcError::internal(format!("Could not start webcam worker: {error}"))
            })?;

        match ready_rx.recv_timeout(std::time::Duration::from_secs(8)) {
            Ok(Ok(())) => Ok(Self {
                shared,
                stop,
                worker: Some(worker),
            }),
            Ok(Err(message)) => {
                let _ = worker.join();
                Err(camera_error(message))
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                drop(worker);
                Err(IpcError::new(
                    "webcam_start_timeout",
                    format!("The camera did not provide a frame in time: {error}"),
                    true,
                ))
            }
        }
    }

    pub fn reader(&self) -> WebcamFrameReader {
        WebcamFrameReader {
            shared: self.shared.clone(),
        }
    }

    pub fn latest_frame(&self) -> Option<Arc<WebcamFrame>> {
        self.reader().latest()
    }

    pub fn error(&self) -> Option<String> {
        self.reader().error()
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for WebcamCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn capture_loop(
    camera_index: CameraIndex,
    recording_profile: Option<(u32, bool)>,
    shared: Arc<CaptureShared>,
    stop: Arc<AtomicBool>,
    ready: SyncSender<Result<(), String>>,
) {
    let result: Result<(), String> = (|| {
        let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::None);
        let mut camera = Camera::with_backend(camera_index, requested, ApiBackend::Auto)
            .map_err(|error| format!("Could not open the selected camera: {error}"))?;
        let selected_format = select_reasonable_camera_format(&mut camera, recording_profile);
        if let Some((width, height, fps)) = selected_format {
            eprintln!("[RecoDex][webcam] selected camera format {width}x{height} @ {fps} FPS");
        }
        camera
            .open_stream()
            .map_err(|error| format!("Could not start the camera stream: {error}"))?;
        let mut sequence = 0_u64;
        let mut last_decoded_at = None;
        #[cfg(debug_assertions)]
        let mut fps_window_started_at = Instant::now();
        #[cfg(debug_assertions)]
        let mut fps_window_frames = 0_u32;
        loop {
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            let buffer = camera.frame().map_err(|error| {
                format!("Camera disconnected or stopped sending frames: {error}")
            })?;
            let captured_at = Instant::now();
            if last_decoded_at
                .is_some_and(|last| captured_at.duration_since(last) < CAMERA_FRAME_INTERVAL)
            {
                continue;
            }
            last_decoded_at = Some(captured_at);
            let mut image = buffer
                .decode_image::<RgbFormat>()
                .map_err(|error| format!("Could not decode a camera frame: {error}"))?;
            image = cap_camera_frame(image);
            sequence = sequence.saturating_add(1);
            let frame = Arc::new(WebcamFrame {
                sequence,
                width: image.width(),
                height: image.height(),
                rgb: image,
                captured_at,
            });
            if let Ok(mut latest) = shared.latest.write() {
                *latest = Some(frame);
            }
            #[cfg(debug_assertions)]
            {
                fps_window_frames += 1;
                let elapsed = fps_window_started_at.elapsed();
                if elapsed >= Duration::from_secs(2) {
                    eprintln!(
                        "[RecoDex][webcam] processed capture rate {:.1} FPS",
                        f64::from(fps_window_frames) / elapsed.as_secs_f64()
                    );
                    fps_window_frames = 0;
                    fps_window_started_at = Instant::now();
                }
            }
            if sequence == 1 {
                let _ = ready.send(Ok(()));
            }
        }
    })();

    if let Err(message) = result {
        if let Ok(mut error) = shared.error.lock() {
            *error = Some(message.clone());
        }
        let _ = ready.send(Err(message));
    }
}

fn select_reasonable_camera_format(
    camera: &mut Camera,
    recording_profile: Option<(u32, bool)>,
) -> Option<(u32, u32, u32)> {
    let formats = camera.compatible_camera_formats().ok()?;
    let candidates = formats
        .iter()
        .map(|format| {
            let resolution = format.resolution();
            (resolution.x(), resolution.y(), format.frame_rate())
        })
        .collect::<Vec<_>>();
    let selected_index = select_camera_format_index(&candidates, recording_profile)?;
    let format = formats[selected_index];
    let resolution = format.resolution();
    let selected = (resolution.x(), resolution.y(), format.frame_rate());
    let request = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(format));
    if let Err(error) = camera.set_camera_requset(request) {
        eprintln!("[RecoDex][webcam] could not apply selected camera format: {error}");
        return None;
    }
    Some(selected)
}

fn select_camera_format_index(
    formats: &[(u32, u32, u32)],
    recording_profile: Option<(u32, bool)>,
) -> Option<usize> {
    let choose = |is_eligible: &dyn Fn(u32, u32, u32) -> bool| {
        formats
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, (width, height, fps))| {
                *width > 0 && *height > 0 && *fps > 0 && is_eligible(*width, *height, *fps)
            })
            .max_by_key(|(_, format)| camera_format_priority(*format, recording_profile))
            .map(|(index, _)| index)
    };

    choose(&|width, height, fps| {
        fps <= CAMERA_MAX_FPS && is_bounded_camera_dimensions(width, height)
    })
    .or_else(|| choose(&|_, _, fps| fps <= CAMERA_MAX_FPS))
    .or_else(|| choose(&|width, height, _| is_bounded_camera_dimensions(width, height)))
    .or_else(|| choose(&|_, _, _| true))
}

fn camera_format_priority(
    (width, height, fps): (u32, u32, u32),
    recording_profile: Option<(u32, bool)>,
) -> (u32, bool, u64) {
    let covers_target = recording_profile
        .map(|(minimum_dimension, square_crop)| {
            format_covers_overlay_target(width, height, minimum_dimension, square_crop)
        })
        .unwrap_or(true);
    let area = u64::from(width) * u64::from(height);
    let resolution_priority = if recording_profile.is_some() && covers_target {
        u64::MAX - area
    } else {
        area
    };
    (fps, covers_target, resolution_priority)
}

fn is_bounded_camera_dimensions(width: u32, height: u32) -> bool {
    width <= 1280 && height <= 720
}

fn format_covers_overlay_target(
    camera_width: u32,
    camera_height: u32,
    minimum_dimension: u32,
    square_crop: bool,
) -> bool {
    if square_crop {
        camera_width.min(camera_height) >= minimum_dimension
    } else {
        camera_width >= minimum_dimension
    }
}

fn cap_camera_frame(image: image::RgbImage) -> image::RgbImage {
    if image.width() <= 1280 && image.height() <= 720 {
        return image;
    }
    let scale = (1280.0 / image.width() as f32).min(720.0 / image.height() as f32);
    let width = (image.width() as f32 * scale).round().max(1.0) as u32;
    let height = (image.height() as f32 * scale).round().max(1.0) as u32;
    image::imageops::resize(&image, width, height, image::imageops::FilterType::Triangle)
}

fn parse_camera_index(camera_id: &str) -> Result<CameraIndex, IpcError> {
    if let Ok(index) = camera_id.parse::<u32>() {
        return Ok(CameraIndex::Index(index));
    }
    if camera_id.trim().is_empty()
        || camera_id.len() > 256
        || camera_id.chars().any(char::is_control)
    {
        return Err(IpcError::new(
            "webcam_camera_invalid",
            "The selected camera is no longer available.",
            true,
        ));
    }
    Ok(CameraIndex::String(camera_id.to_owned()))
}

fn camera_error(message: String) -> IpcError {
    let lower = message.to_lowercase();
    let code = if lower.contains("permission")
        || lower.contains("access denied")
        || lower.contains("denied")
    {
        "webcam_permission_denied"
    } else if lower.contains("not found")
        || lower.contains("no device")
        || lower.contains("unavailable")
    {
        "webcam_camera_unavailable"
    } else {
        "webcam_capture_failed"
    };
    IpcError::new(code, message, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_ids_support_native_indices_and_backend_identifiers() {
        assert_eq!(parse_camera_index("2").unwrap(), CameraIndex::Index(2));
        assert_eq!(
            parse_camera_index("/dev/video4").unwrap(),
            CameraIndex::String("/dev/video4".to_owned())
        );
    }

    #[test]
    fn camera_ids_reject_empty_or_control_characters() {
        assert!(parse_camera_index(" ").is_err());
        assert!(parse_camera_index("camera\n0").is_err());
    }

    #[test]
    fn camera_frames_are_bounded_to_1280_by_720() {
        let image = image::RgbImage::new(1920, 1080);
        let capped = cap_camera_frame(image);
        assert_eq!(capped.dimensions(), (1280, 720));
    }

    #[test]
    fn camera_format_profile_covers_rectangular_and_square_webcam_outputs() {
        assert!(format_covers_overlay_target(640, 360, 450, false));
        assert!(format_covers_overlay_target(640, 480, 450, true));
        assert!(!format_covers_overlay_target(640, 360, 450, true));
        assert!(!format_covers_overlay_target(320, 240, 450, false));
    }

    #[test]
    fn camera_format_selection_prioritizes_frame_rate_before_resolution() {
        let formats = [
            (320, 240, 10),
            (320, 240, 15),
            (640, 480, 30),
            (1280, 720, 30),
        ];

        assert_eq!(
            select_camera_format_index(&formats, Some((320, false))),
            Some(2)
        );
    }

    #[test]
    fn preview_selects_the_highest_resolution_at_the_best_supported_frame_rate() {
        let formats = [(640, 480, 30), (1280, 720, 30), (1280, 720, 15)];

        assert_eq!(select_camera_format_index(&formats, None), Some(1));
    }

    #[test]
    fn camera_format_selection_falls_back_to_the_fastest_available_mode() {
        let formats = [(320, 240, 5), (640, 480, 15)];

        assert_eq!(
            select_camera_format_index(&formats, Some((1280, false))),
            Some(1)
        );
    }
}
