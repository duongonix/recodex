# 05 — Codex Rules, Quality Gates & Definition of Done

## Mandatory rules

1. Read all specs in `specs/` and UI reference PNG before implementation.
2. Follow Tauri 2 + Svelte 5 runes + TypeScript + Tailwind 4 + shadcn-svelte + lucide-svelte.
3. Reuse maintained libraries for dialogs, controls, encoding and media APIs when suitable. Inspect license, maintenance and platform support first.
4. Create folders/modules freely; keep modules cohesive. Do not place all recording logic in one Rust file or all UI in one Svelte component.
5. Never fabricate functionality: controls must invoke real commands, or clearly show disabled/coming-later status. No fake recording timer as a substitute for an actual engine session.
6. Backend owns recording state; UI subscribes to typed events and reconciles with `get_recording_status()`.
7. Handle permission denied, missing devices, unsupported features, cancellation, disk full, interrupted output and multi-monitor edge cases.
8. Keep the UI visually faithful to `specs/ui/images/ui-reference.png` and exact layout specs; no unrequested redesign.
9. Never log microphone audio, screen pixels, credentials or sensitive paths unnecessarily. No silent uploads.
10. Update roadmap files after each completed phase; do not mark work complete without validation.

## Code conventions

- Rust: `cargo fmt`, `cargo clippy`, typed errors, explicit ownership, unit tests for session transitions.
- Svelte/TS: strict types, reusable components, keyboard focus and accessibility labels, no direct OS API access.
- Centralize UI colors/spacing tokens and command/event schemas.
- Avoid blocking Tauri main thread; run capture/encode asynchronously or on dedicated worker threads.
- Do not add dependencies without documenting purpose and licensing considerations.

## Testing

- Unit: state machine, settings validation, target coordinate conversion, output naming.
- Integration: IPC start/pause/resume/stop, cancel source picker, denied permissions, missing audio devices.
- Manual platform smoke: short 10s recording with audio, 2min recording, region capture, screenshot clipboard, app close/reopen.
- Performance: check dropped frames, CPU/RAM growth and A/V drift; document test hardware and thresholds instead of inventing performance guarantees.
- Hold-to-zoom: verify native shortcut press/release, smooth return, cursor alignment on display/window edges, unchanged region/screenshot behavior, and MP4 output at 30/60 FPS. Measure GPU path cadence and memory; separately report CPU fallback performance rather than assuming it meets 4K/60 targets.
- UI: verify compact dimensions, overflow, keyboard shortcuts, contrast, and toolbar states.

## Definition of Done for a phase

- Feature works end-to-end on its declared platform(s).
- Tests/checks run and results recorded; unsupported platforms flagged explicitly.
- User-visible errors are actionable.
- No stale mock UI pretending to be complete.
- `roadmap/progress.md` updated with completed tasks, tests, blockers and next phase.
- `roadmap/features.md` updated for feature/platform status.

## Execution order

Implement `roadmap/mvp.md` sequentially. A phase may be split into small PRs but do not skip dependencies or claim a later phase finished prematurely.
