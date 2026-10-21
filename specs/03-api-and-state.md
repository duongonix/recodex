# 03 — IPC Contracts, State Machine & Persistence

## Recording state machine

`idle -> preparing -> recording <-> paused -> stopping -> finalizing -> (reviewing -> completed | completed) -> idle`.
Errors may occur during preparation, recording or finalization; use `failed` state with recovery action. Reject duplicate start/stop calls; commands must be idempotent or explicitly return `invalid_state`. UI must not optimistically claim success.

## Typed data contracts

```ts
type CaptureTarget =
  | { kind: 'display'; displayId: string }
  | { kind: 'window'; windowId: string }
  | { kind: 'region'; displayId: string; x: number; y: number; width: number; height: number };
type RecordingOptions = {
  target: CaptureTarget;
  fps: 30 | 60;
  maxHeight: 720 | 1080 | 2160;
  microphoneEnabled: boolean;
  microphoneDeviceId?: string;
  systemAudio: boolean;
  showMouseCursor: boolean;
  zoomOnHoldEnabled: boolean;
  showPreviewAfterRecording: boolean;
  outputDirectory: string;
};
type SessionStatus = 'idle'|'preparing'|'recording'|'paused'|'stopping'|'finalizing'|'reviewing'|'completed'|'failed';
type RecordingEvent = {
  sessionId: string;
  status: SessionStatus;
  recordedDurationMs: number;
  outputPath?: string;
  error?: { code: string; message: string; recoverable: boolean };
  audioWarning?: string;
  webcamEnabled: boolean;
  webcamAvailable: boolean;
  quality: RecordingQuality;
};
type RecordingQuality = {
  captureFps: number | null;
  droppedVideoFrames: number;
  audioGaps: number;
  microphoneLevelPercent: number;
  systemAudioLevelPercent: number;
  availableDiskBytes: number | null;
};
type RecordingPreflightReport = {
  canStart: boolean;
  fps: number;
  maxHeight: number;
  outputWidth: number | null;
  outputHeight: number | null;
  workloadPixelsPerSecond: number | null;
  freeDiskBytes: number | null;
  estimatedMinutesRemaining: number | null;
  checks: { key: string; label: string; status: 'ready'|'warning'|'error'|'skipped'; detail: string }[];
};
type TimelineSegment = { id: string; sourceStartMs: number; sourceEndMs: number };
type RecordingReviewInfo = { reviewId: string; videoPath: string; durationMs: number; fps: number };
type EditorSettings = {
  scale: number; positionX: number; positionY: number; rotation: number; speed: number; opacity: number;
  cropEnabled: boolean; cropLeft: number; cropRight: number; cropTop: number; cropBottom: number;
  volume: number; muted: boolean; canvasAspect: 'source'|'16:9'|'9:16'|'1:1'; backgroundColor: string;
  fit: 'contain'|'cover';
};
type RecordingTimelineAssets = { audioWaveform: number[]; videoThumbnails: string[] };
type AudioInputTestResult = {
  durationMs: number;
  microphone: { peakPercent: number; rmsPercent: number } | null;
  system: { peakPercent: number; rmsPercent: number } | null;
  microphonePackets: number;
  systemPackets: number;
};
```

## Tauri command surface (contract, names may be refined together)

- `get_capabilities()` -> supported capture modes, audio support, codecs, max FPS/resolution.
- `list_displays()` and `list_windows()` -> stable identifiers and display names.
- `list_audio_devices()` -> available inputs/loopback sources.
- `get_permissions()` and `request_permissions(kind)` -> explicit permission state.
- `start_recording(options)` -> session ID after capture truly begins.
- `pause_recording(session_id)`, `resume_recording(session_id)`, `stop_recording(session_id)` -> confirmed state/results.
- `set_recording_audio(session_id, source, enabled)` -> success or unsupported error; do not silently toggle UI.
- `set_recording_webcam(session_id, enabled)` -> confirmed native camera/compositor state for the active session.
- `get_recording_status()` -> authoritative state, supports window reload.
- `get_recording_preflight(options)` -> on-demand diagnostic shown in Settings > Snap; validates the chosen source, selected audio endpoints, enabled webcam device, writable output folder, available disk space and estimated time, and reports a high-pixel-rate warning. Start Recording does not open a preflight dialog; native start validation remains authoritative.
- `test_audio_inputs()` -> explicitly captures selected WASAPI inputs for two seconds and returns measured PCM RMS/peak levels and packet counts; it does not save or encode audio.
- `get_pending_recording_review()` -> review-window-only lookup of the finalized pending MP4 path, session ID, and duration.
- `save_recording_review(review_id)` / `discard_recording_review(review_id)` -> validate the active review session, atomically publish or discard its pending output, and finish the session.
- `trim_recording_review(review_id, start_ms, end_ms)` -> render the selected time range with Windows Media Foundation and publish one MP4. The review UI routes a complete range through `save_recording_review` so the original MP4 is kept.
- `split_recording_review(review_id, start_ms, split_ms, end_ms)` -> render two MP4 files split at the playhead inside the selected range. Both outputs are encoded before publication; each file is moved atomically, but publishing the pair is not a single filesystem transaction.
- `get_recording_review_assets(review_id, bin_count, thumbnail_count)` -> editor-window-only extraction of bounded waveform bins and JPEG video thumbnails through Media Foundation Source Reader; returns no decoded frames.
- `export_recording_timeline(review_id, segments, settings, output_path)` -> validate source ranges and editor properties, render one MP4 on a blocking worker, and publish beside the chosen destination without replacing existing files. Windows Media Foundation applies supported video/audio settings; cancellation keeps the source review recording.
- `cancel_recording_timeline_export()` -> editor-window-only, idempotent cancellation request for the active native export. The decoder checks the flag between source samples and discards incomplete output.
- `focus_recording_review()` -> main-window-only action to reopen a hidden review window.
- `capture_screenshot({target,format,copyToClipboard,saveToFile})` -> path and/or copy confirmation.
- `get_settings()` / `update_settings(patch)` -> validated persistent preferences.
- `get_hold_zoom_shortcut_status()` -> current accelerator, enabled/registered state, and registration error.
- `list_webcam_devices()` -> currently enumerated native camera IDs and display names.
- `start_webcam_preview(camera_id)` / `stop_webcam_preview()` -> start or release one native camera preview session.
- `get_webcam_overlay_settings()` -> read the settings used by the dedicated desktop webcam overlay window.
- `get_webcam_background_preview(path)` -> validate a user-selected local PNG/JPEG and return a small encoded preview.

## Events

- `recording://state-changed` on every transition, nonfatal audio warning, and webcam toggle. A WASAPI data gap preserves the packet timestamp, keeps video capture running, and sets `audioWarning`; unreliable timestamps remain fatal.
- `recording://progress` throttled (e.g. 4 Hz), not every frame; its quality snapshot includes measured WGC capture FPS, mailbox-overwritten frame count, audio discontinuity count, recent mic/system RMS level, and free bytes sampled every five seconds.
- `recording://error` for actionable failures.
- `recording-review://ready` and `recording-review://error` to refresh the separate review window after finalization or report a recoverable preview issue.
- `recording-review://export-progress` carries a fraction from `0.0` to `1.0` for the native timeline export; progress is emitted from the blocking worker and consumed only by the editor window.
- `capture://sources-changed` when device/source list changes, if supported.
- `webcam://preview-frame`, `webcam://preview-error`, `webcam://recording-warning`, and `webcam://overlay-settings-changed` for downscaled preview images, camera lifecycle errors, and desktop overlay settings; full camera frames remain native.

## Persistence and paths

- Store preferences using Tauri store or a Rust config file in the platform app-config directory; never store device secrets.
- `AppSettings` also persists startup, minimize-to-tray, notification, cursor-visibility, hold-to-zoom enabled state and shortcut, default quality/FPS/audio, selected microphone endpoint ID (empty means Windows default), and output-folder preferences. Legacy `microphoneGainPercent` and `systemAudioGainPercent` keys are discarded and migrated out on load. Keep frontend types synchronized with the Rust settings contract.
- Windows recording aligns microphone and optional system loopback by QPC timestamp, then mixes enabled sources into one 192 kbps stereo AAC stream in the MP4. Because QPC timestamps have 100 ns precision while 48 kHz sample boundaries are fractional at that scale, the mixer snaps contiguous packet boundaries that differ by at most one sample unless WASAPI flags a discontinuity; larger gaps are preserved. Debug audio logs report how many boundary adjustments occurred. A native peak scaler prevents integer clipping when simultaneous sources exceed full scale; it does not apply per-source gain. The System audio toggle determines whether a WASAPI render-loopback endpoint is opened at all. Older recordings with two audio streams remain readable by the editor.
- `AppSettings.showRecordingPreview` defaults to false and controls whether a finalized recording is auto-saved or held for user review. While reviewing, the output transaction remains in the selected output folder's temporary directory. The Tauri asset protocol starts with an empty static scope and Rust grants read access only to the exact pending MP4; Save, Trim, Split, Discard, and app Quit revoke that grant.
- Review trimming/splitting accepts nonnegative millisecond ranges. The native decoder clips video at frame presentation timestamps and each audio track on 48 kHz sample boundaries, then re-encodes with the existing H.264/AAC settings. Up to two source tracks are preserved independently and editor volume/mute settings apply to each. Encode work runs outside the async command thread; no video frames are sent to JavaScript. If a split's second atomic publication fails after the first succeeds, the command reports the first saved path and retains the original review output.
- The Video Editor stores split/trim/delete as ordered source-time ranges and keeps linked audio/video boundaries. Its undo history stores range and property values, never media frames. Timeline export applies scale, position, rotation, crop, fit/canvas color, opacity, speed, volume and mute in Rust; one native HTML video element remains the preview clock. Export output is staged in the selected destination folder and moved into place without replacing an existing file. A cancellation removes partial export output and leaves the original pending review transaction available. Current editor media commands are Windows-only.
- `zoomOnHoldEnabled` is evaluated at recording start and only activates for display/window targets. The global shortcut has pressed/released semantics and a separate runtime status; settings updates register a replacement before removing the previous shortcut and roll back persistence if registration fails.
- `AppSettings.webcam` stores the selected camera ID and overlay controls; loading settings at app startup forces `enabled=false` and persists that privacy default while retaining camera/style preferences. Enabling in Settings opens the native desktop overlay. Webcam style settings are snapshotted when recording starts; the toolbar can start/stop the native camera during the session, and its drag deltas move both the desktop overlay and recording compositor without persisting a new position. After stop, preview resumes only if enabled remains true. A selected camera that cannot open produces an actionable error/warning while screen recording continues. See `specs/webcam/01-overlay-architecture.md` for the contract fields and platform limits.
- Save recordings/screenshots to a user-chosen directory; default to platform Videos/Pictures or documented fallback.
- Generate collision-safe names with timestamp + suffix; write to temporary output, then atomically finalize/rename where possible.
- Do not add SQLite until a recording library needs indexed metadata.

## UI transition requirements

- Start: validate -> permission/source selection -> prepare -> recording confirmed -> hide main/show toolbar.
- Stop: disable repeated stop -> finalize -> either atomically save and reopen main or enter `reviewing` and open the separate preview window; Save/Discard then finishes the session.
- On error: preserve recoverable output, show error and retry/open-folder action.
- On app restart: detect unfinished sessions and offer recovery where supported.
