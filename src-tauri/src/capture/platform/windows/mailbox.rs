use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use crate::errors::IpcError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    /// Windows system-relative capture time, in 100-nanosecond units, for audio/video alignment.
    pub timestamp_100ns: i64,
    /// Packed top-down BGRA pixels, owned by Rust.
    pub pixels_bgra: Vec<u8>,
}

#[derive(Default)]
struct MailboxState {
    latest: Option<CapturedFrame>,
    closed: bool,
    error: Option<String>,
}

/// A single-slot handoff from the native capture callback to the future encoder consumer.
/// Replacing an unread frame bounds memory to one frame and drops stale frames under load.
pub struct FrameMailbox {
    state: Mutex<MailboxState>,
    available: Condvar,
    replaced_frames: AtomicU64,
}

impl FrameMailbox {
    pub(super) fn shared() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(MailboxState::default()),
            available: Condvar::new(),
            replaced_frames: AtomicU64::new(0),
        })
    }

    pub fn wait_for_latest(&self, timeout: Duration) -> Result<Option<CapturedFrame>, IpcError> {
        let state = self
            .state
            .lock()
            .map_err(|_| IpcError::internal("The native capture frame mailbox was poisoned."))?;
        let (mut state, _) = self
            .available
            .wait_timeout_while(state, timeout, |state| {
                state.latest.is_none() && !state.closed
            })
            .map_err(|_| IpcError::internal("The native capture frame mailbox was poisoned."))?;

        if let Some(frame) = state.latest.take() {
            return Ok(Some(frame));
        }
        if let Some(message) = state.error.take() {
            return Err(IpcError::new("capture_failed", message, true));
        }
        Ok(None)
    }

    pub(super) fn publish(&self, frame: CapturedFrame) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "The native capture frame mailbox was poisoned.".to_owned())?;
        if !state.closed {
            if state.latest.is_some() {
                self.replaced_frames.fetch_add(1, Ordering::Relaxed);
            }
            state.latest = Some(frame);
            self.available.notify_one();
        }
        Ok(())
    }

    pub fn take_replaced_frame_count(&self) -> u64 {
        self.replaced_frames.swap(0, Ordering::Relaxed)
    }

    pub(super) fn close(&self, error: Option<String>) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            state.error = error;
            self.available.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::{CapturedFrame, FrameMailbox};

    fn frame(timestamp_100ns: i64) -> CapturedFrame {
        CapturedFrame {
            width: 1,
            height: 1,
            timestamp_100ns,
            pixels_bgra: vec![0, 0, 0, 255],
        }
    }

    #[test]
    fn mailbox_holds_only_the_most_recent_unread_frame() {
        let mailbox = FrameMailbox::shared();
        mailbox.publish(frame(10)).unwrap();
        mailbox.publish(frame(20)).unwrap();

        assert_eq!(
            mailbox.wait_for_latest(Duration::ZERO).unwrap(),
            Some(frame(20))
        );
        assert_eq!(mailbox.wait_for_latest(Duration::ZERO).unwrap(), None);
    }

    #[test]
    fn mailbox_wakes_a_waiter_when_capture_closes() {
        let mailbox = FrameMailbox::shared();
        let waiter = std::thread::spawn({
            let mailbox = mailbox.clone();
            move || mailbox.wait_for_latest(Duration::from_secs(1)).unwrap()
        });
        thread::sleep(Duration::from_millis(10));
        mailbox.close(None);

        assert_eq!(waiter.join().unwrap(), None);
    }
}
