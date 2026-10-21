use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use image::{
    ImageBuffer, Rgba,
    imageops::{self, FilterType},
};
use windows_capture::frame::Frame;

use super::CursorSnapshot;

mod gpu;

use gpu::GpuZoomRenderer;

const ZOOM_FACTOR: f32 = 2.0;
const ZOOM_IN_TIME_SECONDS: f32 = 0.09;
const ZOOM_OUT_TIME_SECONDS: f32 = 0.16;
const CURSOR_FOLLOW_TIME_SECONDS: f32 = 0.025;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ZoomRequest {
    pub held: bool,
    pub cursor: Option<(i32, i32)>,
}

/// Lock-free state shared by the recording worker and WGC callback.
#[derive(Default)]
pub(crate) struct ZoomInput {
    held: AtomicBool,
    cursor_available: AtomicBool,
    cursor_x: AtomicI32,
    cursor_y: AtomicI32,
}

impl ZoomInput {
    pub(crate) fn update(&self, cursor: Option<CursorSnapshot>, held: bool) {
        if let Some(cursor) = cursor {
            self.cursor_x.store(cursor.x, Ordering::Relaxed);
            self.cursor_y.store(cursor.y, Ordering::Relaxed);
            self.cursor_available.store(true, Ordering::Relaxed);
        } else {
            self.cursor_available.store(false, Ordering::Relaxed);
        }
        self.held.store(held, Ordering::Release);
    }

    pub(super) fn snapshot(&self) -> ZoomRequest {
        let held = self.held.load(Ordering::Acquire);
        let cursor = self.cursor_available.load(Ordering::Relaxed).then(|| {
            (
                self.cursor_x.load(Ordering::Relaxed),
                self.cursor_y.load(Ordering::Relaxed),
            )
        });
        ZoomRequest { held, cursor }
    }
}

enum Renderer {
    Uninitialized,
    Gpu(GpuZoomRenderer),
    CpuFallback,
}

/// Applies a 2× cursor-centered zoom with eased in/out and a short cursor-follow filter.
/// The regular path leaves frames untouched and incurs no per-frame zoom work.
pub(crate) struct ZoomProcessor {
    renderer: Renderer,
    scale: f32,
    center: (f32, f32),
    center_initialized: bool,
    last_timestamp_100ns: Option<i64>,
}

impl Default for ZoomProcessor {
    fn default() -> Self {
        Self {
            renderer: Renderer::Uninitialized,
            scale: 1.0,
            center: (0.0, 0.0),
            center_initialized: false,
            last_timestamp_100ns: None,
        }
    }
}

impl ZoomProcessor {
    pub(crate) fn process(
        &mut self,
        frame: &mut Frame,
        request: ZoomRequest,
        timestamp_100ns: i64,
    ) -> Result<Option<Vec<u8>>, String> {
        if frame.width() < 2 || frame.height() < 2 {
            return Err("The captured frame is too small to apply cursor zoom.".to_owned());
        }
        self.update_transform(frame.width(), frame.height(), request, timestamp_100ns);
        if !request.held && self.scale <= 1.001 {
            self.scale = 1.0;
            self.center_initialized = false;
            return Ok(None);
        }

        if matches!(self.renderer, Renderer::Uninitialized) {
            self.renderer =
                match GpuZoomRenderer::new(frame.device(), frame.width(), frame.height()) {
                    Ok(renderer) => Renderer::Gpu(renderer),
                    Err(error) => {
                        log_gpu_fallback(&error);
                        Renderer::CpuFallback
                    }
                };
        }

        let crop = crop_rect(frame.width(), frame.height(), self.center, self.scale);
        if let Renderer::Gpu(renderer) = &mut self.renderer {
            match renderer.process(frame, crop) {
                Ok(pixels) => return Ok(Some(pixels)),
                Err(error) => {
                    log_gpu_fallback(&error);
                    self.renderer = Renderer::CpuFallback;
                }
            }
        }

        self.process_cpu(frame, crop).map(Some)
    }

    fn update_transform(
        &mut self,
        width: u32,
        height: u32,
        request: ZoomRequest,
        timestamp_100ns: i64,
    ) {
        let delta_seconds = self
            .last_timestamp_100ns
            .replace(timestamp_100ns)
            .map(|last| timestamp_100ns.saturating_sub(last).max(0) as f32 / 10_000_000.0)
            .unwrap_or(1.0 / 60.0)
            .clamp(0.0, 0.1);

        if request.held && !self.center_initialized {
            self.center = request
                .cursor
                .map(|(x, y)| (x as f32, y as f32))
                .unwrap_or((width as f32 * 0.5, height as f32 * 0.5));
            self.center_initialized = true;
        }

        if request.held
            && self.center_initialized
            && let Some((cursor_x, cursor_y)) = request.cursor
        {
            let alpha = 1.0 - (-delta_seconds / CURSOR_FOLLOW_TIME_SECONDS).exp();
            self.center.0 += (cursor_x as f32 - self.center.0) * alpha;
            self.center.1 += (cursor_y as f32 - self.center.1) * alpha;
        }

        let target_scale = if request.held { ZOOM_FACTOR } else { 1.0 };
        let transition_seconds = if request.held {
            ZOOM_IN_TIME_SECONDS
        } else {
            ZOOM_OUT_TIME_SECONDS
        };
        let alpha = 1.0 - (-delta_seconds / transition_seconds).exp();
        self.scale += (target_scale - self.scale) * alpha;
        if (self.scale - target_scale).abs() < 0.002 {
            self.scale = target_scale;
        }
    }

    fn process_cpu(&self, frame: &mut Frame, crop: CropRect) -> Result<Vec<u8>, String> {
        let width = frame.width();
        let height = frame.height();
        let mut packed_pixels = Vec::new();
        let buffer = frame
            .buffer()
            .map_err(|error| format!("Could not read a frame for CPU zoom scaling: {error}"))?;
        let pixels = buffer.as_nopadding_buffer(&mut packed_pixels);
        let image = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixels.to_vec())
            .ok_or_else(|| "The captured BGRA frame has an invalid image layout.".to_owned())?;
        let left = crop.left.floor() as u32;
        let top = crop.top.floor() as u32;
        let crop_width = (crop.width.round() as u32).clamp(2, width.saturating_sub(left));
        let crop_height = (crop.height.round() as u32).clamp(2, height.saturating_sub(top));
        let cropped = imageops::crop_imm(&image, left, top, crop_width, crop_height).to_image();
        Ok(imageops::resize(&cropped, width, height, FilterType::Lanczos3).into_raw())
    }
}

#[derive(Clone, Copy)]
struct CropRect {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

fn crop_rect(width: u32, height: u32, center: (f32, f32), scale: f32) -> CropRect {
    let width_f = width as f32;
    let height_f = height as f32;
    let crop_width = (width_f / scale).clamp(2.0, width_f);
    let crop_height = (height_f / scale).clamp(2.0, height_f);
    let max_left = (width_f - crop_width).max(0.0);
    let max_top = (height_f - crop_height).max(0.0);
    CropRect {
        left: (center.0 - crop_width * 0.5).clamp(0.0, max_left),
        top: (center.1 - crop_height * 0.5).clamp(0.0, max_top),
        width: crop_width,
        height: crop_height,
    }
}

fn log_gpu_fallback(error: &str) {
    #[cfg(debug_assertions)]
    eprintln!("[RecoDex][zoom] D3D11 GPU scaling unavailable; using Rust CPU scaler: {error}");
    #[cfg(not(debug_assertions))]
    let _ = error;
}

#[cfg(test)]
mod tests {
    use super::{CropRect, ZoomInput, ZoomProcessor, ZoomRequest, crop_rect};
    use crate::capture::platform::windows::CursorSnapshot;
    use std::sync::atomic::Ordering;

    #[test]
    fn zoom_crop_tracks_cursor_and_clamps_at_frame_edges() {
        let crop = crop_rect(1920, 1080, (0.0, 1079.0), 2.0);

        assert_eq!(crop.left, 0.0);
        assert_eq!(crop.top, 540.0);
        assert_eq!(crop.width, 960.0);
        assert_eq!(crop.height, 540.0);
    }

    #[test]
    fn zoom_transition_moves_in_and_releases_to_the_original_frame() {
        let mut processor = ZoomProcessor::default();
        processor.update_transform(
            1920,
            1080,
            ZoomRequest {
                held: true,
                cursor: Some((400, 300)),
            },
            0,
        );
        assert!(processor.scale > 1.0);
        assert_eq!(processor.center, (400.0, 300.0));

        processor.update_transform(
            1920,
            1080,
            ZoomRequest {
                held: false,
                cursor: Some((400, 300)),
            },
            1_000_000,
        );
        assert!(processor.scale < 2.0);
        assert!(processor.scale > 1.0);
    }

    #[test]
    fn crop_rect_keeps_output_source_bounds_valid() {
        let crop: CropRect = crop_rect(640, 360, (5000.0, -300.0), 2.0);

        assert!(crop.left >= 0.0 && crop.top >= 0.0);
        assert!(crop.left + crop.width <= 640.0);
        assert!(crop.top + crop.height <= 360.0);
    }

    #[test]
    fn zoom_input_publishes_cursor_coordinates_and_release_state() {
        let input = ZoomInput::default();
        input.update(
            Some(CursorSnapshot {
                x: 123,
                y: 456,
                left_down: false,
                right_down: false,
            }),
            true,
        );

        assert!(input.held.load(Ordering::Acquire));
        assert_eq!(input.snapshot().cursor, Some((123, 456)));

        input.update(None, false);
        assert!(!input.held.load(Ordering::Acquire));
        assert_eq!(input.snapshot().cursor, None);
    }
}
