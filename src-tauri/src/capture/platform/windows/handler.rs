use std::sync::Arc;

use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
};

use super::{
    mailbox::{CapturedFrame, FrameMailbox},
    target::CropRect,
    zoom::{ZoomInput, ZoomProcessor},
};

pub(super) struct HandlerFlags {
    pub mailbox: Arc<FrameMailbox>,
    pub crop: Option<CropRect>,
    pub zoom_input: Option<Arc<ZoomInput>>,
}

pub(super) struct WindowsFrameHandler {
    mailbox: Arc<FrameMailbox>,
    crop: Option<CropRect>,
    zoom_input: Option<Arc<ZoomInput>>,
    zoom_processor: ZoomProcessor,
    last_timestamp_100ns: Option<i64>,
}

impl GraphicsCaptureApiHandler for WindowsFrameHandler {
    type Flags = HandlerFlags;
    type Error = String;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            mailbox: ctx.flags.mailbox,
            crop: ctx.flags.crop,
            zoom_input: ctx.flags.zoom_input,
            zoom_processor: ZoomProcessor::default(),
            last_timestamp_100ns: None,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let timestamp = frame
            .timestamp()
            .map_err(|error| format!("Could not read the native capture timestamp: {error}"))?
            .Duration;
        if timestamp < 0
            || self
                .last_timestamp_100ns
                .is_some_and(|last| timestamp <= last)
        {
            return Err("Windows Graphics Capture returned a non-monotonic timestamp.".to_owned());
        }
        let timestamp_100ns = timestamp;
        let zoomed_pixels = if let Some(zoom_input) = &self.zoom_input {
            self.zoom_processor
                .process(frame, zoom_input.snapshot(), timestamp_100ns)?
        } else {
            None
        };
        let (width, height, pixels_bgra) = if let Some(pixels) = zoomed_pixels {
            (frame.width(), frame.height(), pixels)
        } else {
            copy_frame_pixels(frame, self.crop)?
        };
        self.mailbox.publish(CapturedFrame {
            width,
            height,
            timestamp_100ns,
            pixels_bgra,
        })?;
        self.last_timestamp_100ns = Some(timestamp);
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        self.mailbox.close(None);
        Ok(())
    }
}

fn copy_frame_pixels(
    frame: &mut Frame,
    crop: Option<CropRect>,
) -> Result<(u32, u32, Vec<u8>), String> {
    let (width, height, buffer) = if let Some(crop) = crop {
        let right = crop
            .left
            .checked_add(crop.width)
            .filter(|right| *right <= frame.width())
            .ok_or_else(|| {
                "The selected region no longer fits the captured display frame.".to_owned()
            })?;
        let bottom = crop
            .top
            .checked_add(crop.height)
            .filter(|bottom| *bottom <= frame.height())
            .ok_or_else(|| {
                "The selected region no longer fits the captured display frame.".to_owned()
            })?;
        (
            crop.width,
            crop.height,
            frame
                .buffer_crop(crop.left, crop.top, right, bottom)
                .map_err(|error| format!("Could not crop the selected display region: {error}"))?,
        )
    } else {
        (
            frame.width(),
            frame.height(),
            frame.buffer().map_err(|error| {
                format!("Could not read the Windows Graphics Capture frame: {error}")
            })?,
        )
    };

    if width < 2 || height < 2 {
        return Err(
            "Windows Graphics Capture returned a frame smaller than 2 × 2 pixels.".to_owned(),
        );
    }
    let mut packed_pixels = Vec::new();
    let pixels_bgra = buffer.as_nopadding_buffer(&mut packed_pixels).to_vec();
    let expected_length = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "The captured frame is too large for a CPU pixel buffer.".to_owned())?;
    if pixels_bgra.len() != expected_length {
        return Err(format!(
            "Windows returned {} pixel bytes for a {width} × {height} BGRA frame.",
            pixels_bgra.len()
        ));
    }
    Ok((width, height, pixels_bgra))
}
