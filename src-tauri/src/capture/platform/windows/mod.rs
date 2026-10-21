mod cursor;
mod display_affinity;
mod handler;
mod mailbox;
mod target;
mod window_discovery;
mod window_icon;
mod window_preview;
mod zoom;

pub(crate) use cursor::CursorCompositor;
pub(crate) use display_affinity::exclude_window_from_capture;
pub use mailbox::{CapturedFrame, FrameMailbox};
pub use window_discovery::{find_recordable_window, list_windows, recordable_window_bounds};
pub use window_preview::capture_window_thumbnail;

use std::{sync::Arc, time::Duration};

use windows_capture::{
    capture::{CaptureControl, GraphicsCaptureApiHandler},
    graphics_capture_api::GraphicsCaptureApi,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
};

use crate::{capture::sources::DisplaySource, errors::IpcError, recording::CaptureTarget};

use self::{
    cursor::{CursorSampler, CursorSnapshot},
    handler::WindowsFrameHandler,
    target::{CaptureItem, resolve_target},
    zoom::ZoomInput,
};

/// Windows Graphics Capture adapter. Frames and pixels stay inside Rust.
pub struct WindowsCapture {
    control: Option<CaptureControl<WindowsFrameHandler, String>>,
    mailbox: Arc<FrameMailbox>,
    cursor_sampler: Option<CursorSampler>,
    zoom_input: Option<Arc<ZoomInput>>,
}

impl WindowsCapture {
    pub fn start(
        target: &CaptureTarget,
        displays: &[DisplaySource],
        fps: u16,
    ) -> Result<Self, IpcError> {
        Self::start_internal(
            target,
            displays,
            fps,
            CursorCaptureSettings::Default,
            false,
            false,
        )
    }

    /// Starts capture without WGC's system cursor; recording composes its own cursor in Rust.
    pub fn start_for_recording(
        target: &CaptureTarget,
        displays: &[DisplaySource],
        fps: u16,
        show_mouse_cursor: bool,
        zoom_on_hold_enabled: bool,
    ) -> Result<Self, IpcError> {
        Self::start_internal(
            target,
            displays,
            fps,
            CursorCaptureSettings::WithoutCursor,
            show_mouse_cursor,
            zoom_on_hold_enabled,
        )
    }

    fn start_internal(
        target: &CaptureTarget,
        displays: &[DisplaySource],
        fps: u16,
        cursor_settings: CursorCaptureSettings,
        custom_cursor: bool,
        zoom_on_hold_enabled: bool,
    ) -> Result<Self, IpcError> {
        if !matches!(fps, 30 | 60) {
            return Err(IpcError::new(
                "capture_options_invalid",
                "Windows Graphics Capture accepts a requested rate of 30 or 60 FPS.",
                true,
            ));
        }
        if !GraphicsCaptureApi::is_supported().map_err(|error| {
            IpcError::new(
                "capture_api_unavailable",
                format!("Windows Graphics Capture is unavailable: {error}"),
                true,
            )
        })? {
            return Err(IpcError::new(
                "capture_api_unavailable",
                "Windows Graphics Capture requires a supported Windows 10 version and graphics driver.",
                true,
            ));
        }

        let resolved = resolve_target(target, displays)?;
        let zoom_enabled = zoom_on_hold_enabled
            && matches!(
                target,
                CaptureTarget::Display { .. } | CaptureTarget::Window { .. }
            );
        let zoom_input = zoom_enabled.then(|| Arc::new(ZoomInput::default()));
        let cursor_sampler = if custom_cursor || zoom_enabled {
            Some(
                CursorSampler::start(resolved.cursor_target).map_err(|error| {
                    IpcError::new(
                        "cursor_sampler_start_failed",
                        format!("Could not start the native cursor sampler: {error}"),
                        true,
                    )
                })?,
            )
        } else {
            None
        };
        let mailbox = FrameMailbox::shared();
        let flags = handler::HandlerFlags {
            mailbox: mailbox.clone(),
            crop: resolved.crop,
            zoom_input: zoom_input.clone(),
        };
        let control = start_resolved(resolved, flags, cursor_settings);
        control.map(|control| Self {
            control: Some(control),
            mailbox,
            cursor_sampler,
            zoom_input,
        })
    }

    pub fn wait_for_frame(&mut self, timeout: Duration) -> Result<Option<CapturedFrame>, IpcError> {
        let frame = self.mailbox.wait_for_latest(timeout)?;
        if frame.is_none()
            && self
                .control
                .as_ref()
                .is_some_and(|control| control.is_finished())
        {
            if let Some(control) = self.control.take() {
                control.wait().map_err(|error| {
                    IpcError::new(
                        "capture_failed",
                        format!("Windows Graphics Capture stopped unexpectedly: {error}"),
                        true,
                    )
                })?;
            }
            self.mailbox.close(None);
        }
        Ok(frame)
    }

    pub fn take_dropped_frame_count(&self) -> u64 {
        self.mailbox.take_replaced_frame_count()
    }

    pub fn stop(mut self) -> Result<(), IpcError> {
        let result = self
            .control
            .take()
            .map(|control| {
                control.stop().map_err(|error| {
                    IpcError::new(
                        "capture_stop_failed",
                        format!("Could not stop Windows Graphics Capture cleanly: {error}"),
                        true,
                    )
                })
            })
            .unwrap_or(Ok(()));
        self.mailbox.close(None);
        result
    }

    pub fn mailbox(&self) -> Arc<FrameMailbox> {
        self.mailbox.clone()
    }

    pub(crate) fn cursor_snapshot(&self) -> Option<CursorSnapshot> {
        self.cursor_sampler
            .as_ref()
            .and_then(CursorSampler::snapshot)
    }

    pub(crate) fn update_zoom_input(&self, cursor: Option<CursorSnapshot>, held: bool) {
        if let Some(input) = &self.zoom_input {
            input.update(cursor, held);
        }
    }
}

impl Drop for WindowsCapture {
    fn drop(&mut self) {
        if let Some(control) = self.control.take()
            && let Err(error) = control.stop()
        {
            self.mailbox
                .close(Some(format!("Windows capture cleanup failed: {error}")));
            return;
        }
        self.mailbox.close(None);
    }
}

fn start_resolved(
    resolved: target::ResolvedTarget,
    flags: handler::HandlerFlags,
    cursor_settings: CursorCaptureSettings,
) -> Result<CaptureControl<WindowsFrameHandler, String>, IpcError> {
    match resolved.item {
        CaptureItem::Monitor(monitor) => start_item(monitor, flags, cursor_settings),
        CaptureItem::Window(window) => start_item(window, flags, cursor_settings),
    }
}

fn start_item<T>(
    item: T,
    flags: handler::HandlerFlags,
    cursor_settings: CursorCaptureSettings,
) -> Result<CaptureControl<WindowsFrameHandler, String>, IpcError>
where
    T: TryInto<windows_capture::settings::GraphicsCaptureItemType> + Send + 'static,
{
    let settings = Settings::new(
        item,
        cursor_settings,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        flags,
    );
    WindowsFrameHandler::start_free_threaded(settings).map_err(|error| {
        IpcError::new(
            "capture_start_failed",
            format!("Could not start Windows Graphics Capture: {error}"),
            true,
        )
    })
}

#[cfg(test)]
mod tests;
