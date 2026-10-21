# 01 — Product Requirements (MVP)

## Goal

Build a lightweight desktop screen recorder and screenshot utility for Windows, macOS and Linux, using Tauri 2. The user opens a compact window, chooses a capture mode, and starts recording. During recording the main window hides and a floating toolbar appears. Design must follow `ui/01-main-window.md`, `ui/02-recording-toolbar.md` and `ui/images/ui-reference.png`.

## MVP scope

### Video recording

- Capture full display, selected window or user-selected rectangular region.
- Multi-monitor display selection; show a picker if ambiguous.
- Start, pause, resume and stop. Elapsed timer reflects actual recorded duration (excluding paused time).
- Record microphone and system audio independently where supported. Detect device availability, permissions and unsupported combinations; never silently claim recording audio.
- Choose 30/60 FPS and 720p/1080p/2160p (UHD) where the backend reports support. 2160p is a maximum output height and never upscales a smaller source. Defaults: 1080p/30 FPS.
- Export MP4 (H.264 video; AAC audio where licensing/build permits). Prefer hardware encoder when available with a tested fallback. Show save path, progress and actionable error.
- Optional hold-to-zoom follows the native cursor at 2× during display or window recording. The user enables it and configures its global shortcut in Settings > Snap > Recording; region capture and screenshots are unaffected.
- Preserve valid partial output after crashes where feasible, by writing a recoverable intermediate format and finalizing at stop.

### Screenshots

- Full display, window, region.
- Save PNG or JPG, and copy image to clipboard.
- Region selection supports cancel via Escape; no accidental captures after cancellation.

### Desktop UX

- Compact main window; floating toolbar while recording, with elapsed time, pause/resume, stop, mic and system audio indicators/toggles and compact mode.
- Global shortcuts when registration succeeds; shortcut conflicts handled gracefully. Tray optional for first milestone but required before public MVP.
- Local settings: capture mode, device selections, output folder, format, quality, FPS, shortcuts.
- Hold-to-zoom is off by default; shortcut conflicts and unsupported platforms are reported without enabling a nonworking control.
- All user-visible actions have loading, success, failure and permission-denied states.

## Out of scope for MVP

Video timeline editor, cloud upload, accounts, OCR, scrolling screenshots, annotations, live streaming, scheduled recording and AI tools. Webcam overlay was originally out of scope for the MVP; it is now a separately approved follow-up, tracked in `specs/webcam/01-overlay-architecture.md`, and does not change the ordered MVP phase completion claims.

## Product principles

- No forced sign-in or network requirement.
- Default local storage; no telemetry or uploads without opt-in.
- Do not record the floating toolbar if OS capture exclusion is supported; otherwise show limitation and offer a safe placement/hide strategy.
- Treat Windows/macOS/Linux support as a target, not an untested promise: track platform parity in `04-platform-capture.md`.

## Acceptance criteria

- User can record, pause, resume, stop and play the saved file.
- Screenshot can be copied and saved.
- User cancellation and denied permissions return to a usable UI.
- UI states never disagree with engine state.
- No broken or misleading enabled controls on unsupported platforms.
