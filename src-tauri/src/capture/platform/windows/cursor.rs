mod compositor;
mod raster;

pub(crate) use compositor::CursorCompositor;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows_sys::Win32::{
    Foundation::{POINT, RECT},
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON},
        WindowsAndMessaging::{GetCursorPos, GetWindowRect},
    },
};

#[derive(Debug, Clone, Copy)]
pub(super) enum CursorTarget {
    Screen { origin_x: i32, origin_y: i32 },
    Window { hwnd: isize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CursorSnapshot {
    pub x: i32,
    pub y: i32,
    pub left_down: bool,
    pub right_down: bool,
}

pub(super) fn sample_cursor(target: CursorTarget) -> Option<CursorSnapshot> {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return None;
    }

    let (origin_x, origin_y) = match target {
        CursorTarget::Screen { origin_x, origin_y } => (origin_x, origin_y),
        CursorTarget::Window { hwnd } => {
            let mut rect = RECT::default();
            if unsafe { GetWindowRect(hwnd as _, &mut rect) } == 0 {
                return None;
            }
            (rect.left, rect.top)
        }
    };

    let left_state = unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON)) };
    let right_state = unsafe { GetAsyncKeyState(i32::from(VK_RBUTTON)) };
    Some(CursorSnapshot {
        x: point.x.saturating_sub(origin_x),
        y: point.y.saturating_sub(origin_y),
        left_down: left_state < 0,
        right_down: right_state < 0,
    })
}

pub(super) struct CursorSampler {
    latest: Arc<Mutex<Option<CursorSnapshot>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl CursorSampler {
    pub(super) fn start(target: CursorTarget) -> Result<Self, std::io::Error> {
        let latest = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_latest = latest.clone();
        let worker_stop = stop.clone();
        let worker = thread::Builder::new()
            .name("recodex-cursor-sampler".to_owned())
            .spawn(move || {
                while !worker_stop.load(Ordering::Acquire) {
                    if let Some(snapshot) = sample_cursor(target)
                        && let Ok(mut latest) = worker_latest.lock()
                    {
                        *latest = Some(snapshot);
                    }
                    thread::sleep(Duration::from_millis(4));
                }
            })?;
        Ok(Self {
            latest,
            stop,
            worker: Some(worker),
        })
    }

    pub(super) fn snapshot(&self) -> Option<CursorSnapshot> {
        self.latest.lock().ok().and_then(|latest| *latest)
    }
}

impl Drop for CursorSampler {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
