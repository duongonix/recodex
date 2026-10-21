# Global Agent Instructions

<!-- lean-ctx -->
## lean-ctx

Prefer lean-ctx MCP tools over native equivalents for token savings.

For compression you can rely on regardless of your Codex surface (CLI, Desktop, or Cloud) or Codex version, route shell commands through `ctx_shell` (or `C:/Users/duong/AppData/Roaming/npm/node_modules/lean-ctx-bin/bin/lean-ctx.exe -c "<cmd>"`), file reads through `ctx_read`, and code search through `ctx_search`. Hook-driven auto-compression may also be active, but the MCP/CLI tools are the path that works everywhere — otherwise large outputs (builds, `tsc`, tests, logs) can reach the model uncompressed.

Full rules: `C:\Users\duong\.codex\LEAN-CTX.md`
<!-- /lean-ctx -->

--- project-doc ---

# AGENTS.md — Screen Recorder Project Rules

> This file is the primary working agreement for Codex and other coding agents. Apply it to the entire repository. Read it before making changes. If another instruction conflicts, follow the higher-priority instruction and report the conflict.

## 1. Product and scope

Build a lightweight desktop screen recorder and screenshot app for Windows, macOS, and Linux using **Tauri 2, Rust, Svelte 5, Tailwind CSS, shadcn-svelte, and Lucide icons**.

The approved UX consists of:
- A compact main window for choosing Record / Screenshot, capture source, audio, resolution, FPS, and starting capture.
- A separate always-on-top floating toolbar while recording, with timer, pause/resume, stop, microphone/system-audio controls, and compact mode.
- A dark UI with red accents matching `specs/ui/images/ui-reference.png` and the UI specifications.

Focus on the MVP. Do not add cloud sync, streaming, advanced video editing, AI features, accounts, or other out-of-scope functionality without explicit approval.

## 2. Read before implementation

Before modifying code, read the relevant documents:
1. `specs/01-product-requirements.md`
2. `specs/02-architecture-and-structure.md`
3. `specs/03-api-and-state.md`
4. `specs/04-platform-capture.md`
5. `specs/05-engineering-rules-and-quality.md`
6. `specs/ui/01-main-window.md` and `specs/ui/02-recording-toolbar.md`
7. `specs/roadmap/mvp.md`, `features.md`, `progress.md`, `future.md`

Use the roadmap as the implementation order. If a referenced file does not exist, report it rather than inventing its contents. Do not silently override approved UI or product decisions.

## 3. Mandatory modular architecture

**Never put unrelated functionality into one large file.** Split by responsibility, not arbitrarily by line count.

- Each module, component, function, and service should have one clear responsibility.
- Prefer small, cohesive files with explicit, well-named interfaces.
- As a guideline, keep Svelte components under **200–250 lines** and Rust source files under **300–400 lines**. These are review triggers, not hard limits; refactor when cohesion or readability suffers, even below them.
- Keep functions short where practical (roughly 20–50 lines); extract complex state transitions and platform handling into named helpers.
- Avoid monolithic `App.svelte`, `lib.rs`, `main.rs`, `commands.rs`, `capture.rs`, and `utils.rs` files.
- Keep `main.rs`/`lib.rs` thin: initialize the application, register plugins/commands, and compose modules.
- Separate UI presentation, application state, Tauri IPC adapters, business logic, OS capture adapters, audio capture, encoding, and persistence.
- Keep platform-specific code behind traits/interfaces or clear adapter boundaries; do not scatter `cfg(target_os = ...)` across unrelated business logic.
- Do not introduce a generic abstraction until it has a real use case. Avoid excessive micro-files, circular dependencies, and unnecessary design patterns.

Suggested organization (adapt to the existing project and architecture spec):

```text
src/
  lib/
    components/
      recorder/
      screenshot/
      toolbar/
      shared/
    stores/
    services/
      ipc/
    types/
    utils/
src-tauri/
  src/
    app/
    commands/
    capture/
      platform/
    audio/
    encoding/
    recording/
    screenshot/
    settings/
    storage/
    events/
    errors/
    lib.rs
    main.rs
```

A suggested path is not permission to restructure an existing working repository unnecessarily. Respect existing conventions and the architecture spec.

## 4. Library-first policy (Rust and Svelte)

**Use proven, maintained libraries and platform APIs instead of implementing mature functionality from scratch.** This applies equally to Rust and the Svelte frontend.

Before implementing a substantial feature:
1. Check existing dependencies and built-in Tauri capabilities.
2. Evaluate maintained libraries/crates/components that already solve the problem.
3. Prefer a small, compatible dependency with an appropriate license, active maintenance, good documentation, and acceptable security/performance characteristics.
4. Reuse existing dependencies when suitable; do not add overlapping libraries without justification.
5. Document major dependency choices and tradeoffs in the relevant spec or progress log.

Preferred tools where suitable:
- **Frontend:** Svelte 5 runes, Tailwind CSS, shadcn-svelte, Lucide icons, Tauri official JS APIs/plugins.
- **Rust:** official Tauri 2 APIs/plugins, `serde`, `thiserror`, `tracing`, and mature crates for platform integration and media handling as justified.
- **Capture:** Windows Graphics Capture, ScreenCaptureKit, and PipeWire/portal-compatible approaches where appropriate. Prefer reliable native bindings or maintained wrappers.
- **Encoding:** use established media encoders/FFmpeg integrations or native hardware encoders; do not write video codecs, muxers, resamplers, or container formats from scratch.
- **Settings/storage:** use an appropriate existing Tauri plugin or mature Rust library; avoid creating a custom persistence framework.

Do **not** blindly install libraries. Verify Tauri 2 / Svelte 5 compatibility, OS support, build requirements, license, and maintenance. Use the project's package manager and lockfiles. Do not introduce a large dependency solely to replace a trivial helper.

## 5. UI implementation rules

- Match the approved **dark + red** design; use design tokens/CSS variables instead of scattering hard-coded colors.
- Main window should remain compact; toolbar should be a separate Tauri window, not a mock overlay in the main window.
- Reuse shadcn-svelte components when suitable and customize them consistently.
- Break the main window into independent components (tabs, source picker, audio toggles, quality/FPS selectors, primary action).
- Break the toolbar into independent components (timer/status, transport controls, audio controls, compact mode).
- Use Lucide icons consistently. Ensure keyboard access, tooltips/labels, focus indicators, and disabled/loading states.
- UI state must reflect the real backend state. Never fake elapsed time, recording success, audio availability, or completed exports.
- When capture starts, hide the main window and show the toolbar only after the backend confirms the appropriate transition. Handle errors and rollback cleanly.
- Do not assume `alwaysOnTop` prevents the toolbar from appearing in captured video. Implement platform-appropriate exclusion where supported, and document limitations otherwise.

## 6. Rust/backend and IPC rules

- Capture, audio processing, encoding, and file writing belong in Rust/native code, not in the WebView's JavaScript thread.
- Define typed IPC command arguments, return values, events, and errors. Keep frontend TypeScript types synchronized with Rust contracts.
- Use a single authoritative recording state machine (e.g. Idle, Preparing, Recording, Paused, Stopping, Saving, Error) and validate every transition.
- Never block the UI or Tauri command handler with long-running synchronous capture/encoding work.
- Use bounded queues/backpressure for media frames and audio buffers; avoid unbounded memory growth.
- Release native handles, microphones, capture sessions, encoders, and temporary files on stop, error, window close, and app exit.
- Protect against duplicate Start/Stop requests, rapid clicks, lost devices, revoked permissions, disk-full errors, and encoder failures.
- Keep logs structured and useful, but never log captured pixels, raw microphone content, secrets, or private file contents.
- Keep permissions/capabilities minimal. Request OS screen/microphone permissions at the correct time and provide actionable errors.

## 7. Cross-platform discipline

- Design platform-neutral contracts first, then implement OS-specific adapters.
- Do not claim Windows, macOS, or Linux support until tested on that OS.
- Consider Windows capture/audio APIs, macOS Screen Recording and Microphone permissions, Linux Wayland/X11 differences, portals, and PipeWire availability.
- Treat unsupported operations as explicit capability errors; never silently simulate them.
- Feature-detect hardware encoding, audio loopback, capture exclusion, and available screens/windows.
- Avoid OS-specific paths and shell commands in shared code. Use proper path APIs and platform directories.

## 8. Reliability, privacy, and security

- Capture only the source explicitly selected by the user. No recording before user action and required permission.
- Make recording state obvious; do not secretly record in the background.
- Validate user-selected paths and handle filename collisions safely.
- Prefer safe, atomic finalization of output files when possible; do not report success before the file is valid and finalized.
- Ensure Stop, Save, and failure paths cannot corrupt existing user files.
- Do not transmit screenshots, audio, video, telemetry, or file paths to external services without explicit product requirements and user consent.
- Do not hard-code tokens, credentials, absolute developer paths, or machine-specific configuration.
- Avoid `unwrap()`/`expect()` in recoverable runtime paths; use meaningful error propagation.

## 9. Tests and quality gates

For each phase, run applicable checks and report results:
- Frontend type checking, linting, formatting, and build.
- Rust formatting, Clippy, unit/integration tests, and compile checks.
- Manual smoke tests for supported capture flows on the available OS.
- Tests for state transitions, IPC error mapping, settings persistence, timer behavior, cancellation, and cleanup.
- Check window behavior: opening, closing, hiding, toolbar positioning, always-on-top, and compact/paused modes.
- Test recording outputs by opening the resulting files, verifying duration, video, and audio where applicable.

Do not fabricate passing tests. If a test cannot run due to missing OS/hardware/permissions, mark it **Not tested** with the reason. Do not mark a feature complete solely because the UI renders.

## 10. Roadmap and progress reporting

- Implement phases in `specs/roadmap/mvp.md` sequentially unless the user changes priority.
- For each phase: inspect existing code, plan the smallest coherent change, implement, test, and document.
- Update `specs/roadmap/progress.md` with work completed, changed files, test commands/results, blockers, and next steps.
- Update `specs/roadmap/features.md` only for genuinely working features; distinguish Planned / In progress / Blocked / Done.
- Put non-MVP proposals in `specs/roadmap/future.md`; do not implement them opportunistically.
- Do not mark a phase Done if critical acceptance criteria are unmet.
- Preserve existing working features and public contracts; describe and justify any breaking changes.

## 11. Agent working rules

**DO:**
- Read relevant code and specs before editing.
- Reuse existing code, libraries, components, and styles.
- Make focused, reviewable changes and keep modules cohesive.
- Add/update tests for meaningful logic changes.
- Update documentation when APIs, behaviors, or architecture change.
- Explain any dependency additions and major tradeoffs.
- Ask for clarification when an irreversible or substantial product decision is missing.

**DO NOT:**
- Rewrite the whole project for a small feature.
- Put hundreds of lines of unrelated logic into a single file.
- Build custom UI primitives or media infrastructure when suitable maintained solutions exist.
- Add fake buttons, fake recordings, mock-only IPC, or placeholder success states and claim completion.
- Introduce unrelated features, broad refactors, or dependencies without a reason.
- Suppress compiler, linter, or type errors with unsafe casts or blanket ignores.
- Delete tests, user data, or existing working functionality to make a build pass.
- Change the approved design, architecture, or roadmap silently.
- Claim cross-platform behavior or performance without evidence.

## 12. Definition of Done for each phase

A phase is complete only when:
1. Its documented acceptance criteria are met.
2. Code is split into appropriate modules with clear ownership.
3. Existing libraries have been reused where practical.
4. Relevant checks and tests pass, or limitations are explicitly recorded.
5. UI interactions are wired to real behavior when the phase requires it.
6. Error and cleanup paths are handled.
7. Roadmap progress and feature tracking are updated.
8. The final report names changed files, tests, limitations, and next phase.

**Guiding principle:** Build a small, fast, reliable recorder with clean architecture. Favor reusable, maintained tools over reinventing components, and favor working functionality over superficial demos.
