# 04 — Platform Capture & Media Pipeline

## Platform adapters

| Platform | Video capture | Audio | Notes |
|---|---|---|---|
| Windows | Windows Graphics Capture; fallback only if needed | WASAPI loopback + input | Validate per-window capture and cursor behavior |
| macOS | ScreenCaptureKit | ScreenCaptureKit system audio where available; CoreAudio mic | Screen recording/microphone permissions and OS-version differences |
| Linux | XDG Desktop Portal + PipeWire on Wayland; X11 capture where appropriate | PipeWire/PulseAudio/ALSA depending environment | Portal selection/permission behavior differs across compositors |

These are implementation candidates, not guaranteed cross-platform parity. Codex must verify OS/API version requirements and record actual tested support.

## Capture pipeline

`source selection -> OS frames/audio buffers -> timestamp/clock normalization -> optional rescale -> encoder -> muxer -> temp output -> finalized MP4`.

- Avoid moving raw frames through Tauri IPC; maintain bounded queues and backpressure.
- Allow Media Foundation hardware H.264 transforms with software fallback; the sink writer now enables `MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS`, but the actual encoder selected and sustained output cadence must be verified on each device.
- Respect source dimensions, aspect ratio and pixel format; avoid distorted output.
- Handle audio/video synchronization, variable frame delivery, paused timestamps, and device disconnects.
- Stop must drain buffers and finalize MP4 metadata; expose `finalizing` progress.
- If packaging FFmpeg or codecs, document licenses, redistribution terms and per-platform binaries. Do not assume an installed system executable.

## Recording Video Editor pipeline

- The pending recording is previewed by the editor webview through a file-scoped Tauri asset grant. Media Foundation Source Reader seeks decode bounded thumbnail frames and waveform bins asynchronously; JavaScript receives only those small display assets, never full-rate frames.
- Windows timeline export decodes timestamped BGRA video and up to two 48 kHz stereo audio streams on a Rust blocking worker, applies linked timeline ranges and supported editor settings to each stream, and writes one H.264 MP4 with separate AAC tracks through the existing Sink Writer. Only one source frame and one packet per audio stream are retained at a time.
- The export is staged in a temporary directory beside the selected destination and published with a no-replace move. Progress is emitted from the native worker; cancellation is checked between decoded samples, removes incomplete output, and leaves the source recording pending for another attempt.
- This editor decode/render path currently has no macOS or Linux adapter. Windows compilation and synthetic tests do not validate playback, export sync, device codec behavior, or cancellation during a live export.

## Webcam overlay behavior

- Native camera enumeration and frame acquisition use `nokhwa` 0.10.11 with `input-native` and decoding enabled and default features disabled. The capture worker retains only the latest decoded RGB frame, negotiates at or below 30 FPS where the backend reports formats, and bounds frames to 1280 × 720. Webcam `enabled` is reset and persisted false at every app launch. Enabling from Settings opens the camera and shows a native, always-on-top desktop overlay; capture does not start simply because the recorder is open.
- The macOS bundle includes `NSCameraUsageDescription`; Windows/Linux open-time permission and device errors are passed through without inventing a separate permission status.
- Windows recording composites the most recent camera frame into each selected screen/window/region BGRA output frame in Rust, before Media Foundation H.264 encoding. Camera and screen have independent clocks; the current synchronization strategy samples the latest camera frame for each output frame and holds it until a newer camera frame arrives. Before recording, the Settings preview session is stopped; the recording worker publishes downscaled preview frames from its own camera session. The webcam is released at recording teardown; device loss emits a warning and screen capture continues.
- The `webcam-overlay` native window starts hidden, is click-through and uses Tauri's content-protection setting on Windows. While idle it sits at the bottom-right of the current/primary display; during recording Rust places it below the toolbar inside the selected target, aligns their right edges, and follows toolbar movement. Live visibility, capture exclusion, and drag alignment remain unverified.
- Rectangle and circle masks, normalized position, output-height-scaled size, border, mirror, opacity, color/image backgrounds, and a non-segmentation blur treatment run in the native compositor. Blur creates a blurred camera backdrop with a sharp inset; it does not remove the person from the background. A missing or unreadable image background is reported and omitted.
- The Windows backend compiles and synthetic compositor/settings/MP4 tests run on Windows 10.0.19045. The Sink Writer now opts into available hardware transforms to reduce CPU pressure when camera decoding/composition are active, but live camera permissions, real-device capture, sustained webcam FPS, camera disconnect, and webcam MP4 playback have not been tested. macOS/Linux screen recording and webcam-to-recording composition are not implemented or verified; camera preview on those platforms is also untested. See `specs/webcam/01-overlay-architecture.md`.

## Capture-region behavior

- Region coordinates must be normalized correctly across mixed DPI, display scale and negative monitor origins.
- The current region contract returns `x`/`y` in global desktop physical pixels and stores the selected `display_id`; dimensions are physical pixels. The native picker converts its local WebView logical pointer coordinates using the selected monitor's scale factor, clamps the rectangle to that monitor, and rejects selections smaller than 2 × 2 physical pixels.
- Hide selection overlay before recording/screenshot begins.
- Window capture handles minimized/closed/protected windows with a clear error.
- Windows applies `WDA_EXCLUDEFROMCAPTURE` to the toolbar HWND after WGC starts and reads back the native affinity before showing it. If the OS call fails, recording startup fails with a recoverable error instead of silently including the toolbar or its protected placeholder. This verifies the OS flag, not the pixels returned by every graphics driver; live WGC output smoke is still required. Other platform exclusion remains unimplemented; never promise guaranteed invisibility.

## Audio behavior

- Mic and system audio are independently selectable in Settings → Snap → Recording. There are no per-source gain controls. The Windows microphone endpoint can be selected explicitly (empty selection uses the Windows default), while loopback is opened only when System audio is enabled.
- Validate device availability, channel counts and sample rates.
- System audio support may depend on OS version and portal permissions.
- Rust aligns mic and optional loopback lanes by QPC timestamp and mixes them into one 192 kbps stereo AAC stream. A per-block peak scaler reduces the block only when the sum would exceed full scale; there are no per-source gain controls. Since QPC timestamps have 100 ns precision, the mixer snaps contiguous packet boundaries that quantize within one 48 kHz sample, unless WASAPI marks a discontinuity; larger gaps remain visible. Debug audio logs include the adjustment count. Loopback is never opened unless System audio is selected. If a physical microphone hears speakers, that acoustic bleed is part of the mic signal; use headphones or an echo-canceling microphone. Mixer/source-isolation tests pass; the synthetic mixed-track mux/decode test passed once but intermittently terminates with Windows `STATUS_ACCESS_VIOLATION`, so output playback is not yet considered reliably verified. Live device quality remains untested.

## Hold-to-zoom behavior

- Windows display/window recording can enable a configurable global hold shortcut (default `CommandOrControl+Shift+Z`). Pressing it smoothly zooms to 2× around the sampled native cursor; releasing smoothly returns to the original view. Cursor-follow smoothing reduces jitter, and the crop clamps to the source bounds near display/window edges.
- The Windows WGC callback uses a D3D11 compute shader with linear sampling and reuses the output/readback textures. It returns packed BGRA at the same capture dimensions, so the Media Foundation stream contract and aspect ratio do not change. When D3D11 shader access or shader initialization fails, the Rust `image` crate provides a Lanczos3 CPU fallback; fallback can lower the sustainable frame rate at high resolutions.
- The feature only runs when enabled at recording start and the selected target is a display or window. Region captures and screenshots remain unchanged. If the global shortcut conflicts or cannot register, Settings reports the error and restores the last saved setting.
- This implementation targets Windows only. macOS ScreenCaptureKit and Linux portal/PipeWire adapters do not implement cursor zoom. D3D11 GPU cadence, CPU fallback cadence, and final MP4 playback need a live native smoke test before marking the feature verified.

## Compatibility matrix to maintain

For every OS, record: OS version, compositor/session, full-screen, window, region, multi-monitor, mic, system audio, 30/60 FPS, MP4 playback, pause/resume, toolbar exclusion and permission denial. Mark each as `supported`, `limited`, `unsupported` or `untested` with evidence.

## Windows Phase 10–11 implementation notes

- The first capture adapter uses the maintained `windows-capture` 2.0.1 wrapper around Windows Graphics Capture. It produces top-down BGRA frames and reads each frame's `SystemRelativeTime` in Rust; a single-slot mailbox replaces unread frames to bound memory and drop stale frames under load. Media Foundation's uncompressed RGB32 input type declares a positive default stride to preserve that row orientation.
- Display targets are resolved against native monitor bounds; window targets carry both process ID and HWND and are revalidated at capture start. Region targets remain global physical pixels and are checked/converted to monitor-local crop bounds before capture.
- The Windows 10.0.19045 test host rejected both WGC's custom minimum-update interval and secondary-window inclusion. The adapter uses OS-default settings; requested 30/60 FPS output therefore needs pacing in the later recording lifecycle. The default OS capture-border behavior is used and is not promised to exclude the toolbar.
- Windows Media Foundation Sink Writer encodes BGRA frames to H.264 MP4 and finalizes container metadata. The app writes inside a temporary directory in the selected output folder, validates `ftyp`, `moov`, and `mdat`, then uses a same-folder Win32 move with write-through and no replace-existing flag. Failed finalization retains the temporary MP4 for recovery. No FFmpeg binary is bundled.
- Validation on Windows 10 Home 10.0.19045: a synthetic test window produced a real frame/timestamp; 30 synthetic frames encoded and were decoded as RGB32 samples by the Media Foundation Source Reader. Display/region frame capture, multi-monitor mixed-DPI hardware, 2160p/60 FPS output, installed-player playback, macOS and Linux remain untested. The main-window start/stop UI is wired to the Phase 12 recording worker, but the live UI-to-output flow still needs a smoke test.

## Windows Phase 12–13 implementation notes

- Phase 12 adds a dedicated Rust recording worker around the WGC mailbox and Media Foundation output. It paces frames from system-relative timestamps, uses a monotonic active-time clock that excludes pauses, validates state transitions, and emits progress at 4 Hz. Start waits for a confirmed capture transition before hiding `main` and showing `recording-toolbar`; stop drains/finalizes the output before reopening `main`.
- Phase 13 enumerates WASAPI input and render endpoints. Capture uses the Windows Core Audio COM bindings for shared-mode 48 kHz stereo PCM; selected microphone and default-render loopback run on dedicated threads with a bounded 512-packet queue and QPC timestamps. The first `GetBuffer` after `Start` may set `DATA_DISCONTINUITY` with device position zero; that undefined initial flag is ignored, while later gaps produce a nonfatal toolbar warning and preserve capture. Pause stops selected streams; toolbar controls are enabled only for sources opened at session start.
- The Rust mixer aligns enabled 48 kHz stereo lanes by QPC timestamp with a 150 ms safety window, then sums them into one PCM stream. Contiguous packet boundaries within one sample are snapped to account for 100 ns QPC quantization; WASAPI discontinuities and gaps larger than one sample are preserved and counted for debug diagnostics. A block peak scaler attenuates only when the sum exceeds the signed 16-bit range, avoiding integer clipping without per-source gain controls. Recording creates one 192 kbps stereo AAC stream whenever either source is enabled; System audio off means the render-loopback endpoint is not opened. Older MP4s with two AAC streams remain readable/exportable by the editor. Mixer source-isolation, timestamp alignment, boundary-jitter and peak-scaling tests pass. The synthetic Media Foundation mixed-track mux/decode test passed once but intermittently terminates with Windows `STATUS_ACCESS_VIOLATION`; investigate before treating output playback as verified. Live dual-device quality and A/V sync remain untested.
- The `wasapi` 0.26.0 crate remains responsible for endpoint enumeration. Audio packet reads use the maintained `windows` 0.62.2 binding because the crate's `read_from_device` helper does not special-case WASAPI's silent-buffer/null-pointer case; the adapter copies silent packets as zeros without dereferencing that pointer. Both dependencies are MIT or MIT/Apache-2.0 licensed.
- Windows does not expose a dependable app-level microphone permission query through this flow. Endpoint presence is reported as limited capability; actual access is checked when the user starts capture, and failures point to Windows microphone privacy settings.
- Tests do not exercise a live microphone, live system loopback, denied permission, device removal, Tauri window visibility, or end-to-end A/V sync. Debug builds log the resolved endpoint friendly name; Settings warns for common names such as Stereo Mix and What U Hear, which can capture desktop playback as a mic input.

## Windows Phase 14–15 implementation notes

- Screenshot IPC accepts a typed target and format. Before Windows Graphics Capture starts, the native command temporarily enables Tauri's Windows capture exclusion (`WDA_EXCLUDEFROMCAPTURE`) for the main window and hides it; after capture it restores window visibility and capture visibility, including error paths. This exclusion is supported on Windows 10 version 2004 and later; older versions treat the flag like `WDA_MONITOR` and may leave a blank protected window rectangle. BGRA pixels stay in Rust, encode as PNG/JPEG with `image` 0.25.10, save through same-directory `tempfile` and `persist_noclobber` without replacing existing files, and optionally copy as RGBA with `arboard` 3.7.0. A requested recording in progress blocks screenshots; partial save/clipboard failures are returned as warnings when the other destination succeeds.
- The screenshot button is enabled only when WGC capability, a selected target, and at least one output destination are available. It reports the actual output path and clipboard result. macOS/Linux screenshot adapters remain unimplemented.
- Toolbar compact/expand calls Tauri's logical window sizing API with the `core:window:allow-set-size` permission. Hiding/reopening uses window-authorized Rust commands; close requests while recording switch between the main window and toolbar instead of terminating the session. The toolbar is reasserted always-on-top before display, and the main window explains that it can appear in a full-display recording while open.
- Verification on Windows 10 Home 10.0.19045: Svelte check/lint/build, `cargo check`, Clippy, Tauri MSI/NSIS build and all 45 Rust tests pass. Tests decode synthetic PNG/JPEG images, verify BGRA channel order, check collision-safe file output, and include the synthetic WGC test-window flow. The recent fix that temporarily excludes RecoDex from WGC output still needs a live Tauri screenshot smoke test; user screen contents were not captured and the existing clipboard was not overwritten. macOS/Linux are untested.

## Windows recording cursor follow-up

- Recording starts Windows Graphics Capture with cursor capture disabled. A short-lived native Rust sampler reads the selected target's screen position and left/right button states; display and region targets use global physical-pixel origins, while window targets read the current HWND bounds.
- Rust composites an enlarged black pointer with a white outline into video frames, adds a fading click ripple, and shows a pulsing ring while dragging. Releasing a sustained drag adds a drop ripple. The compositor restores only the small previous-cursor patch when the pointer moves; video pixels never pass through JavaScript.
- Screenshot and window-thumbnail capture keep WGC's default cursor behavior. This custom cursor path is Windows-only; its live recording output and A/V behavior have not yet been smoke-tested.
