# 02 — Architecture & Folder Structure

## Required stack

Tauri 2 + Rust backend, Svelte 5 runes, TypeScript, Tailwind CSS 4, shadcn-svelte and lucide-svelte. Use established libraries rather than writing UI primitives or codec implementations from scratch.

## Architecture boundaries

1. Svelte UI renders and dispatches typed commands; never handles raw high-bandwidth video frames.
2. Tauri IPC exposes narrow, validated commands and broadcasts session events.
3. Rust application layer owns session state, permission checks, settings, output files and orchestration.
4. Capture backends provide OS-specific video/audio acquisition.
5. Media pipeline timestamps/synchronizes audio-video, encodes and muxes outputs. Use native media APIs and/or an FFmpeg-based integration with documented distribution/licensing.

`AppState` is registered on the Tauri builder before setup. Settings storage is initialized during setup after Tauri resolves the platform configuration/output directories; settings IPC returns a recoverable `settings_initializing` error during that short interval, and the frontend retries initial settings reads.

## Hold-to-zoom processing path

- Settings persist `zoomOnHoldEnabled` and `zoomOnHoldShortcut`; the main-window settings UI captures the chosen key combination and the Tauri global-shortcut plugin supplies native pressed/released events.
- Rust publishes the held state and selected target's native cursor coordinates through atomics to the Windows Graphics Capture callback. The callback applies an eased 2× crop around the cursor with a D3D11 compute shader and linear sampling before handing owned BGRA pixels to the existing Rust compositor and encoder.
- D3D11 output/readback textures are reused for the session. If shader creation or GPU resource binding is unavailable, Rust uses the already-installed `image` crate's Lanczos3 scaler. Both paths preserve the original frame dimensions; raw frames never enter JavaScript.
- The feature is gated to Windows display/window targets. Region recording and screenshot capture do not pass zoom state to the frame processor.

## Webcam overlay processing path

- `src-tauri/src/webcam/` separates settings validation, native device enumeration/capture, preview publication, desktop overlay window placement, and output-frame composition. `src/lib/components/webcam/` owns the same-window Settings page and focused controls; `src/windows/WebcamOverlayShell.svelte` renders the native overlay from downscaled preview events; typed Tauri adapters live under `src/lib/services/ipc/`.
- `nokhwa` with the native input backend owns camera access on a dedicated Rust worker. A latest-frame mailbox bounds memory; camera frames are capped at 1280 × 720 and 30 FPS before being shared with preview or recording.
- The Windows recording worker samples the most recent webcam frame for each WGC output frame and composites it into the existing BGRA frame before Media Foundation encoding. This sample-and-hold policy tolerates independent screen/camera frame rates; raw frames never enter JavaScript.
- Preview events contain only downscaled JPEG data URLs at approximately 10 FPS. `AppSettings.webcam.enabled` is reset and persisted as false at every app launch. Enabling the Settings control starts camera capture and shows the click-through desktop overlay; starting recording transfers capture to the recording worker so the camera is never opened twice.
- The position preset UI is removed. While idle, the desktop overlay starts at the current/primary display's bottom-right. While recording, Rust places it below the toolbar at the selected target's bottom-right; native toolbar move events reposition the desktop overlay and send matching deltas to the recording compositor. Positions are session-only.
- Settings persist under `AppSettings.webcam` with defaults for older JSON files. Detailed coordinates, lifecycle, limits, and validation are documented in `specs/webcam/01-overlay-architecture.md`.

## Recording review path

- `AppSettings.showRecordingPreview` controls post-recording review and defaults to false for existing users. When enabled, the existing recording review becomes the separate, resizable `recording-review` Video Editor window; the main recorder remains unchanged. The current flow edits a pending recording and does not provide arbitrary media import.
- Rust retains the finalized MP4 in `RecordingReviewStore`, transitions the authoritative session to `reviewing`, grants the Tauri asset protocol access to that exact MP4 path, and opens the editor. The webview uses one native HTML video element for preview and playhead timing; raw video frames stay in Rust.
- `src/features/video-editor/` owns typed editor settings and the non-destructive timeline model, plus focused Svelte components for header, playback, properties, timeline and export dialog. `src/windows/VideoEditorWindow.svelte` connects that view to one playback engine and the typed `src/lib/services/ipc/recording-review.ts` service.
- `get_recording_review_assets` runs Media Foundation Source Reader seeks on a blocking worker to produce a bounded set of JPEG thumbnails and waveform bins. The timeline stores source-time ranges; split, trim and delete do not modify the original. Audio and video share each range, and the UI history stores timeline/settings snapshots rather than decoded frames.
- `export_recording_timeline` validates and renders ordered ranges on a blocking worker. The Windows Source Reader feeds one decoded BGRA frame and one 48 kHz stereo PCM packet at a time into the existing Media Foundation H.264/AAC writer. Rust applies scale, position, rotation, crop, fit, canvas, opacity, playback speed, volume and mute; it then stages the MP4 next to the user-selected destination and publishes without replacing an existing file. A cancellation atomic is checked in the decode loop, the incomplete export is removed, and the source review transaction remains available. Progress is emitted through `recording-review://export-progress`.
- Successful export or saving the original completes the review, revokes the file-scoped asset permission, and returns to the main window. Failed/canceled export leaves the pending source available. Closing the editor and choosing Quit while review is pending save the original by default.
- The editor backend is implemented for Windows Media Foundation only. macOS/Linux must report unsupported until native media adapters exist; compile checks do not count as playback/export smoke tests.

## Suggested repository structure

```text
/
├── package.json
├── src/
│   ├── app.css
│   ├── App.svelte
│   ├── lib/
│   │   ├── components/
│   │   │   ├── ui/                 # shadcn-svelte generated components
│   │   │   ├── recorder/
│   │   │   ├── screenshot/
│   │   │   └── toolbar/
│   │   ├── stores/                 # Svelte 5 state and derived state
│   │   ├── services/               # typed Tauri IPC adapters
│   │   ├── types/
│   │   └── utils/
│   └── windows/
│       ├── MainWindow.svelte
│       ├── RecordingToolbar.svelte
│       └── RegionOverlay.svelte
├── src-tauri/
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── commands/
│       ├── app_state/
│       ├── recording/
│       │   ├── session.rs
│       │   ├── capture/
│       │   │   ├── mod.rs
│       │   │   ├── windows.rs
│       │   │   ├── macos.rs
│       │   │   └── linux.rs
│       │   ├── audio/
│       │   ├── encoding/
│       │   └── output/
│       ├── screenshot/
│       ├── webcam/                 # camera capture, desktop overlay window, preview, settings, compositor
│       ├── permissions/
│       ├── settings/
│       ├── shortcuts/
│       └── windows/
├── tests/
│   ├── e2e/
│   └── fixtures/
└── specs/
    ├── 01-product-requirements.md
    ├── 02-architecture-and-structure.md
    ├── 03-api-and-state.md
    ├── 04-platform-capture.md
    ├── 05-engineering-rules-and-quality.md
    ├── ui/
    └── roadmap/
```

## Engineering rules

- Split files by responsibility; avoid giant `lib.rs`, `App.svelte` or single-file capture engines.
- Put platform-specific logic behind Rust traits/interfaces. No OS-specific conditional code inside UI components.
- Share design tokens between windows. Windows have distinct Tauri labels: `main`, `recording-toolbar`, `recording-review`, `webcam-overlay`, `region-overlay`.
- `recording-toolbar` is always-on-top, draggable, borderless where supported. On Windows the recording worker reapplies and reads back `WDA_EXCLUDEFROMCAPTURE` after WGC starts and before the toolbar is shown. `webcam-overlay` starts hidden, is click-through and capture-protected, and is shown only while webcam is enabled. During recording it follows below the toolbar. Region overlay must not persist after cancellation.
- Backend is source of truth for recording state. UI only renders confirmed state.
- Keep native APIs and FFmpeg wrapper isolated so the implementation can change without rewriting UI.
