# MVP Roadmap — Ordered Implementation Phases

Complete phases in order. Each phase requires validation and updates to `progress.md` and `features.md`. Do not mark a phase done until tested on a named OS.

- [x] **01 — Project bootstrap:** Tauri 2, Svelte 5, Tailwind 4, shadcn-svelte, lucide-svelte, lint/build scripts.
- [x] **02 — Window shell:** compact `main`, `recording-toolbar`, region overlay and Tauri window permissions.
- [x] **03 — UI design system:** tokens, buttons, selects, switches, tabs, icons, dark/red theme.
- [x] **04 — Main UI:** UI implemented against `ui/01-main-window.md` and PNG reference; capture capabilities and native source selection remain for their dependent phases.
- [x] **05 — Toolbar UI:** state-driven recording/paused/compact/finalizing layouts implemented against `ui/02-recording-toolbar.md`; recording actions and native window lifecycle remain in their dependent phases.
- [x] **06 — Typed IPC/state:** typed commands and events, session state machine, UI/backend status synchronization.
- [x] **07 — Settings:** persistent quality/FPS/audio/output preferences and validation.
- [x] **08 — Capability/permission layer:** OS detection, permission status and actionable errors; unsupported capture remains explicit.
- [x] **09 — Capture source picker:** native display/window sources, dedicated dark/red Choose a Window window with live Windows metadata/previews, region overlay and mixed-DPI coordinate normalization implemented; native picker interaction and runtime monitor-layout smoke remain recorded as validation limits.
- [x] **10 — First-platform video capture:** Windows Graphics Capture adapter returns real BGRA frames and normalized system-relative timestamps on the Windows test host; monitor/region runtime smoke remains limited.
- [x] **11 — Encoding/export:** Windows Media Foundation writes H.264 MP4; validated temp output is atomically moved without overwriting, and the Windows Media Foundation Source Reader decodes the result.
- [ ] **12 — Recording lifecycle (implementation in progress):** start/pause/resume/stop, true timer, reliable transitions; native Tauri runtime smoke remains pending.
- [ ] **13 — Audio capture (implementation in progress):** microphone + system audio as supported, sync and permissions; live endpoint and A/V smoke remains pending.
- [ ] **14 — Screenshot capture (implementation in progress):** Windows native WGC full/window/region path, PNG/JPG encoding, clipboard, and collision-safe save are implemented; UI-to-native smoke remains pending.
- [ ] **15 — Floating toolbar integration (implementation in progress):** main hide/show, always-on-top, compact resize, hide/reopen, and stop feedback are wired; native active-recording window smoke remains pending.
- [ ] **16 — Shortcuts/tray (implementation in progress):** global quick-access shortcut, startup conflict reporting, system-tray access and safe Quit; native UI smoke remains pending.
- [ ] **17 — Robustness (implementation in progress):** startup cancellation, graceful stop on Quit, disk-full/device-loss errors, close-to-tray behavior and recoverable output; fault-injection and native runtime smoke remain pending.
- [ ] **18 — Platform parity:** port and verify macOS/Windows/Linux adapters; document limitations.
- [ ] **19 — QA/performance:** multi-monitor, long recording, CPU/RAM, frame drops, A/V sync.
- [ ] **20 — Packaging/release:** platform installers, icons, release notes, license review, manual verification.

## MVP exit criteria

- End-to-end video recording with pause/resume/stop and playable saved output.
- Screenshot save and clipboard.
- Faithful compact UI and functional toolbar.
- No fake enabled controls, clear platform support matrix, release artifacts validated.
