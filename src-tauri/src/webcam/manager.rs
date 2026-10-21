use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{ImageEncoder, codecs::jpeg::JpegEncoder, imageops::FilterType};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::errors::IpcError;

use super::capture::{WebcamCapture, WebcamFrame, WebcamFrameReader};

pub const PREVIEW_FRAME_EVENT: &str = "webcam://preview-frame";
pub const PREVIEW_STOPPED_EVENT: &str = "webcam://preview-stopped";
pub const PREVIEW_ERROR_EVENT: &str = "webcam://preview-error";
pub const RECORDING_WARNING_EVENT: &str = "webcam://recording-warning";
const PREVIEW_INTERVAL: Duration = Duration::from_millis(33);
const PREVIEW_MAX_WIDTH: u32 = 320;
const PREVIEW_MAX_HEIGHT: u32 = 240;

#[derive(Clone, Default)]
pub struct WebcamManager {
    inner: Arc<Mutex<Option<PreviewSession>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebcamPreviewFrame {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub sequence: u64,
}

struct PreviewSession {
    camera_id: String,
    capture: Option<WebcamCapture>,
    publisher: Option<PreviewPublisher>,
}

pub struct PreviewPublisher {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl WebcamManager {
    pub fn start_preview(&self, app: AppHandle, camera_id: String) -> Result<(), IpcError> {
        let mut current = self.inner.lock().map_err(|_| {
            IpcError::internal("Webcam preview state is unavailable because its lock failed.")
        })?;
        if current.as_ref().is_some_and(|preview| {
            preview.camera_id == camera_id
                && preview
                    .capture
                    .as_ref()
                    .is_some_and(|capture| capture.error().is_none())
        }) {
            return Ok(());
        }
        if let Some(mut previous) = current.take() {
            previous.stop();
        }
        let capture = WebcamCapture::start(&camera_id)?;
        let reader = capture.reader();
        let publisher = PreviewPublisher::start(app, reader)?;
        *current = Some(PreviewSession {
            camera_id,
            capture: Some(capture),
            publisher: Some(publisher),
        });
        Ok(())
    }

    pub fn stop_preview(&self) -> Result<(), IpcError> {
        let mut current = self.inner.lock().map_err(|_| {
            IpcError::internal("Webcam preview state is unavailable because its lock failed.")
        })?;
        if let Some(mut preview) = current.take() {
            preview.stop();
        }
        Ok(())
    }

    pub fn latest_dimensions(&self) -> Option<(u32, u32)> {
        let current = self.inner.lock().ok()?;
        let frame = current.as_ref()?.capture.as_ref()?.latest_frame()?;
        Some((frame.width, frame.height))
    }
}

impl PreviewPublisher {
    pub fn start(app: AppHandle, reader: WebcamFrameReader) -> Result<Self, IpcError> {
        let stop = Arc::new(AtomicBool::new(false));
        let publisher_stop = stop.clone();
        let worker = thread::Builder::new()
            .name("recodex-webcam-preview".to_owned())
            .spawn(move || publish_preview(app, reader, publisher_stop))
            .map_err(|error| {
                IpcError::internal(format!("Could not start webcam preview: {error}"))
            })?;
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for PreviewPublisher {
    fn drop(&mut self) {
        self.stop();
    }
}

impl PreviewSession {
    fn stop(&mut self) {
        if let Some(mut publisher) = self.publisher.take() {
            publisher.stop();
        }
        if let Some(mut capture) = self.capture.take() {
            capture.stop();
        }
    }
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        self.stop();
    }
}

fn publish_preview(app: AppHandle, reader: WebcamFrameReader, stop: Arc<AtomicBool>) {
    let mut last_sequence = 0;
    while !stop.load(Ordering::Acquire) {
        if let Some(frame) = reader.latest()
            && frame.sequence != last_sequence
        {
            if let Err(error) = publish_preview_frame(&app, &frame) {
                let _ = app.emit(PREVIEW_ERROR_EVENT, error.message);
                break;
            }
            last_sequence = frame.sequence;
        }
        if let Some(error) = reader.error() {
            let _ = app.emit(PREVIEW_ERROR_EVENT, error);
            break;
        }
        thread::sleep(PREVIEW_INTERVAL);
    }
}

pub fn publish_preview_frame(app: &AppHandle, frame: &WebcamFrame) -> Result<(), IpcError> {
    let (data_url, width, height) = encode_preview(&frame.rgb)?;
    let payload = WebcamPreviewFrame {
        data_url,
        width,
        height,
        sequence: frame.sequence,
    };
    app.emit(PREVIEW_FRAME_EVENT, payload)
        .map_err(|error| IpcError::internal(format!("Could not publish webcam preview: {error}")))
}

fn encode_preview(image: &image::RgbImage) -> Result<(String, u32, u32), IpcError> {
    let scale = (PREVIEW_MAX_WIDTH as f32 / image.width().max(1) as f32)
        .min(PREVIEW_MAX_HEIGHT as f32 / image.height().max(1) as f32)
        .min(1.0);
    let width = (image.width() as f32 * scale).round().max(1.0) as u32;
    let height = (image.height() as f32 * scale).round().max(1.0) as u32;
    let preview = if image.dimensions() == (width, height) {
        image.clone()
    } else {
        image::imageops::resize(image, width, height, FilterType::Triangle)
    };
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 70)
        .write_image(
            preview.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| IpcError::internal(format!("Could not encode camera preview: {error}")))?;
    let data_url = format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg));
    Ok((data_url, width, height))
}

#[cfg(test)]
mod tests {
    use super::encode_preview;

    #[test]
    fn preview_encoding_limits_size_without_changing_aspect_ratio() {
        let image = image::RgbImage::new(1280, 720);

        let (_, width, height) = encode_preview(&image).unwrap();

        assert_eq!((width, height), (320, 180));
    }

    #[test]
    fn preview_encoding_preserves_small_native_frames() {
        let image = image::RgbImage::new(320, 240);

        let (_, width, height) = encode_preview(&image).unwrap();

        assert_eq!((width, height), (320, 240));
    }
}
