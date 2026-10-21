# Webcam Overlay — Architecture and Verification

**Status:** Implemented in code; live camera and MP4 smoke tests are pending.  
**UI reference:** Recodex's dark/red Settings styling in `specs/ui/images/setting.png`. No separate webcam mock is currently checked into `specs/ui/images/`.

## User flow

- Settings > Snap > Recording contains a webcam row with a switch and chevron. The row opens the configuration route inside the existing `main` Tauri window; the window remains 400 × 490 CSS px.
- The detail page keeps its header fixed and scrolls its settings body. Back restores the Snap view and its previous scroll offset. Webcam settings auto-save through the existing settings service and are shared with the recording worker.
- Camera enumeration happens on the detail page. Selecting a camera while webcam is disabled only stores its ID; capture starts after enabling the overlay or pressing Start camera preview. Turning the setting off releases capture and hides the desktop overlay. Leaving the detail page keeps the desktop overlay running only when the setting is enabled.
- The position preset picker and preview dragging are removed. Enabling the setting places the desktop overlay at the current/primary display's bottom-right. The settings preview stays bottom-right and supports resize handles only.
- At recording start, the native toolbar is placed bottom-right inside the selected display, region, or window bounds. If webcam is enabled, its desktop overlay sits below the toolbar with their right edges aligned. Toolbar move events reposition the overlay and send the same physical-pixel deltas to the recording worker, which moves the webcam compositor. Live placement is session-only; the next recording starts at the target's bottom-right.
- The toolbar's camera button appears in full and compact modes. Rust owns the enabled/available state. Turning the camera on opens it on a separate native worker so screen/audio capture keeps running; turning it off clears the recorded overlay, hides the desktop overlay, and releases the camera asynchronously. Toolbar changes persist the enabled state so the desktop overlay is restored after stop only when still enabled. When no camera is configured, the button is disabled with an explanation.

## Module boundaries

| Responsibility | Module |
|---|---|
| Persisted settings, validation and defaults | `src-tauri/src/webcam/settings.rs`, `src-tauri/src/settings/service.rs` |
| Real device listing | `src-tauri/src/webcam/devices.rs` |
| Native camera worker and latest-frame mailbox | `src-tauri/src/webcam/capture.rs` |
| Shape, border, background and alpha composition | `src-tauri/src/webcam/compositor.rs` |
| Preview lifecycle and downscaled event publication | `src-tauri/src/webcam/manager.rs` |
| Native desktop overlay window placement and lifecycle | `src-tauri/src/webcam/overlay_window.rs` |
| User-selected image validation and preview encoding | `src-tauri/src/webcam/preview.rs` |
| IPC endpoints | `src-tauri/src/commands/webcam.rs` |
| Main-window controls, settings preview and desktop overlay shell | `src/lib/components/webcam/`, `src/windows/WebcamOverlayShell.svelte` |
| Typed frontend settings and IPC | `src/lib/types/settings.ts`, `src/lib/types/webcam.ts`, `src/lib/services/ipc/webcam.ts` |

## Camera dependency and processing

- `nokhwa` 0.10.11 is enabled with `input-native` and `decoding`, with default features disabled. The existing `image` crate performs bounded resize, image filters and JPEG encoding. This reuses maintained native camera backends instead of implementing a camera API per platform in this feature.
- macOS bundles merge `src-tauri/Info.plist`, which declares `NSCameraUsageDescription` for explicit preview/recording use; the OS prompts when native capture is first opened. Windows/Linux permission failures are surfaced from the camera backend when opening the stream; there is no invented permission-status query.
- One Rust thread owns each camera session. The shared mailbox retains only the latest `Arc<WebcamFrame>`, preventing a slow preview or recorder from accumulating an unbounded queue. Settings preview negotiates up to 1280 × 720 at 30 FPS. Camera format selection prioritizes the highest available frame rate up to 30 FPS before resolution; recording then prefers a bounded format that covers 1.25 × the output webcam width (or square-crop short side), choosing the smallest covering format at that frame rate. If no format covers that target, it prefers the highest-rate bounded format. Frames are published no faster than 30 FPS; larger frames are resized to 1280 × 720 before publishing. Development logs report the selected camera mode and measured processed FPS every two seconds.
- Preview JPEGs preserve camera aspect ratio, are capped at 320 × 240, and are emitted at up to 30 FPS. This keeps preview event payload and encoding work small while avoiding the previous 10 FPS visual update cap. JavaScript receives only the encoded preview and sequence; it never processes raw video frames.
- For recording, Windows samples the latest camera frame when a WGC frame is composed, holds it until a newer camera frame arrives, then composites the overlay into the existing BGRA frame before Media Foundation encoding. This is a bounded sample-and-hold policy for independent camera and screen update rates. The common A/V origin is established from a fresh WGC frame after camera, encoder, and audio startup complete, so camera initialization time is not counted as an audio timestamp lead. WASAPI packets that straddle the origin are clipped at timeline zero by the native mixer.
- The recording compositor builds its shape/border mask once per camera session. Each video frame then copies the webcam underlay in contiguous scanlines and writes fully opaque webcam pixels directly; only partial-opacity pixels use alpha blending. This keeps per-output-frame work proportional to the webcam rectangle and avoids repeating rounded-corner/border geometry calculations at camera FPS.
- The Media Foundation Sink Writer is created with `MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS` enabled. Windows may then select an available hardware H.264 transform; the attribute does not force one, so Media Foundation can retain its software fallback when hardware is unavailable or incompatible. This setting is shared by recordings with and without webcam because both use the same encoder. The selected transform and live webcam FPS still require device-level validation.
- Tauri development builds use Rust `opt-level = 1` and optimized dependencies (`opt-level = 2`) so the camera decoder, JPEG preview encoder, and native compositor are not benchmarked at unoptimized debug speed. Release builds retain the existing `opt-level = 3` configuration. This reduces debug-build CPU overhead but does not replace live camera/FPS verification.
- The camera source is opened once per preview or recording session, not once per frame. Preview is stopped before recording starts to avoid duplicate access to a device. A camera error emits a warning, clears the last overlay, and allows screen recording to continue. Stop and app-state teardown drop the native capture handle and worker.

## Coordinates and visual controls

- The configured width is measured in pixels at 1080p output height and scales by `outputHeight / 1080`; defaults to 180 px and is limited to 80–640 px. Rectangle preserves camera aspect ratio; Circle center-crops to a square.
- The desktop overlay uses a 20 CSS px margin scaled by monitor DPI and sits at the bottom-right of the current/primary display while idle. During recording, toolbar and camera form a right-aligned stack inside the selected target bounds, with the camera below the toolbar. Moving the toolbar moves the desktop overlay in physical screen coordinates and applies matching physical-frame deltas to the recording compositor. The Settings preview uses a 1920 × 1080 model and is not the desktop overlay.
- Border color/width, rounded-rectangle radius, circle crop, opacity and horizontal mirror are applied in Rust. The screen frame is never mirrored. Border is included only while enabled and width is nonzero.
- Background `None` preserves the webcam feed. `Color` fills the webcam canvas. `Image` loads a local PNG/JPEG in Rust after checking that its path is absolute, file is no larger than 25 MB, dimensions are no larger than 16,384 × 16,384, and pixel area is no more than 16 megapixels. `Blur` draws a blurred camera backdrop and a sharp inset feed; it does not segment or remove the person from the background.
- A missing or invalid image is reported and omitted from output rather than replaced with a pretend background.

## Settings and IPC

`AppSettings.webcam` is serde-defaulted for older JSON settings files. It stores the enabled flag, selected camera ID, backward-compatible position/normalized coordinates, output-relative width, shape/radius, border, background mode/color/path, opacity and mirror. Settings loading forces `enabled=false` and persists that default on each app launch while retaining the camera selection and style. Enabling resets legacy position fields to bottom-right. Idle desktop placement uses monitor physical bounds; recording placement is relative to the selected capture target. Toolbar movement is transient. Invalid coordinates, colors, widths, radius, camera IDs and paths are rejected by settings validation.

Commands:

- `list_webcam_devices`
- `start_webcam_preview(camera_id)`
- `get_webcam_overlay_settings`
- `stop_webcam_preview`
- `get_webcam_background_preview(path)`
- Existing `get_settings` / `update_settings` persist the configuration.
- `set_recording_webcam(session_id, enabled)` updates the Rust recording snapshot and starts/stops native camera capture for the active session.
- `recording-toolbar` `WindowEvent::Moved` sends bounded `MoveWebcam` worker commands; the same `RecordingSnapshot` reports `webcamEnabled` and `webcamAvailable` to both toolbar layouts.

Events:

- `webcam://preview-frame` carries a downscaled JPEG data URL and frame sequence.
- `webcam://preview-error` reports failed preview or device loss.
- `webcam://overlay-settings-changed` sends current style settings to the dedicated overlay window.
- `webcam://recording-warning` reports a missing, inaccessible, or disconnected camera without claiming the overlay was recorded.

## Platform support and verification

| Platform | Camera preview | Webcam in recording |
|---|---|---|
| Windows | Native backend compiles; live permission/device smoke pending | Composition is integrated before Media Foundation; live device and output playback smoke pending |
| macOS | Backend is present in the selected library but not tested here | Screen recording adapter/composition is not implemented |
| Linux | Backend is present in the selected library but not tested here | Screen recording adapter/composition is not implemented |

Automated verification on the Windows build host covers Rust settings, stack geometry and compositor tests plus Svelte type checks, lint and production build. Native camera/desktop overlay interaction, camera privacy prompts, output playback, disconnect handling and FPS impact remain unverified; keep the feature limited until those smoke tests pass on a named Windows version. macOS/Linux camera runtime and recording composition have not been verified.
