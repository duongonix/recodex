use std::time::Duration;

use windows_sys::Win32::{
    System::Threading::GetCurrentProcessId,
    UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_OVERLAPPEDWINDOW, WS_VISIBLE},
};

use crate::recording::CaptureTarget;

use super::WindowsCapture;

struct SyntheticWindow(windows_sys::Win32::Foundation::HWND);

impl Drop for SyntheticWindow {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.0) };
    }
}

#[test]
fn windows_graphics_capture_returns_real_bgra_frames_and_timestamps_from_a_test_window() {
    let class = "STATIC\0".encode_utf16().collect::<Vec<_>>();
    let title = "RecoDex native capture smoke test\0"
        .encode_utf16()
        .collect::<Vec<_>>();
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            80,
            80,
            360,
            220,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        )
    };
    assert!(
        !hwnd.is_null(),
        "Windows must create the synthetic test window"
    );
    let _window = SyntheticWindow(hwnd);
    let window_id = format!(
        "hwnd:{:x}:{:x}",
        unsafe { GetCurrentProcessId() },
        hwnd as usize
    );
    let target = CaptureTarget::Window { window_id };
    let mut capture = WindowsCapture::start(&target, &[], 30)
        .expect("Windows Graphics Capture should start for the test window");
    let frame = capture
        .wait_for_frame(Duration::from_secs(10))
        .expect("native capture should not fail")
        .expect("the synthetic window should produce a frame within ten seconds");

    assert!(frame.width >= 2 && frame.height >= 2);
    assert_eq!(
        frame.pixels_bgra.len(),
        frame.width as usize * frame.height as usize * 4
    );
    assert!(frame.timestamp_100ns >= 0);
    capture
        .stop()
        .expect("capture resources should stop and join");
}
