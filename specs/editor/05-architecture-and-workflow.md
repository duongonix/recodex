# Screenshot Editor — Architecture, Integration, Performance and QA

## Technology baseline
Tauri 2 + Svelte 5 + Tailwind CSS + shadcn-svelte + Lucide icons; Rust for filesystem, native clipboard integration where needed, image export and platform-sensitive operations. Follow root `AGENTS.md` and reuse existing modules before creating new ones.

## Implemented integration notes
- The screenshot preference `showScreenshotPreview` defaults to ON and is persisted through the existing Rust settings service. ON creates a temporary, app-cache PNG session and opens one scoped editor window; OFF keeps the direct save/copy flow.
- The editor uses Konva (MIT) through a dynamic import for the interactive scene; no Svelte wrapper is needed. Source pixels are read once through binary Tauri IPC, while annotations and history stay as vector document state in the editor.
- Export is rendered at source resolution by the browser Canvas API to preserve browser text rendering and match the canvas preview. The resulting PNG/JPG is sent as raw binary IPC; Rust validates format/dimensions, performs atomic collision-safe file writes and native clipboard delivery. Rust does not rasterize annotation layers.
- Each session owns one temporary source PNG in the app cache. Closing removes the temporary file; closing the same session repeatedly is safe. New screenshots use distinct editor windows/sessions, and existing editor windows are hidden during capture so they cannot appear in another screenshot.
- The editor intercepts close requests while it checks for unsaved changes. After approval it removes that listener and closes through the existing `core:window:allow-close` permission; it does not need the more powerful forced-destroy permission.
- Current clipboard integration is implemented on Windows only. macOS/Linux editor clipboard export returns an explicit capability error.
- Automated coverage includes coordinate/crop normalization, undo/redo history, temporary session cleanup, PNG validation and save path behavior. Native Tauri window interaction and screenshot-to-editor visual QA still require manual Windows smoke testing; macOS/Linux capture adapters remain platform blockers.

## Proposed module layout (adapt to repository)
```text
src/
  features/screenshot-editor/
    EditorPage.svelte
    components/
      EditorTopbar.svelte
      ToolRail.svelte
      EditorCanvas.svelte
      Inspector.svelte
      StylePanel.svelte
      TextPanel.svelte
      BackgroundPanel.svelte
      ScreenshotStrip.svelte
      UnsavedChangesDialog.svelte
    state/
      editor-state.svelte.ts
      history.ts
      tools.ts
    model/
      document.ts
      objects.ts
      coordinates.ts
    services/
      editor-ipc.ts
      export.ts
src-tauri/src/
  editor/
    mod.rs
    session.rs
    commands.rs
    export.rs
    clipboard.rs
```
Do not create every suggested file if existing modules already handle it. Avoid monolithic Svelte components or giant Rust files; keep capture, editing, IPC, export and settings independent.

## Session lifecycle and IPC
1. Native screenshot capture completes and stores pixels in a scoped temporary resource or safe in-memory session.
2. When preview setting ON, create editor session with opaque ID; open/focus one editor window and send session ID through controlled Tauri event/IPC.
3. Editor requests image and metadata through safe, scoped access. Avoid embedding huge base64 strings in repeated events.
4. Editor operates on local document state; export sends an explicit render result or structured document to a validated Rust export path.
5. Save/Copy returns success only after real filesystem write/clipboard operation; show errors on failure.
6. On close, dispose object URLs, image buffers, listeners and temp resources. Handle multiple screenshots while editor is open: either queue or open new session predictably, never silently overwrite unsaved edits.

## Settings integration
- `showScreenshotPreview: boolean` persisted by existing settings system.
- `screenshotSaveFolder: string` selected via OS directory picker, validated before saving.
- `screenshotFormat` follows existing supported formats; do not invent formats.
- Preview OFF uses direct save; preview ON opens editor; both share naming/path collision policy.
- Main window stays 400 × 490 px, Settings stays in same main window view, editor is the separate resizable window.

## File safety
- Sanitize suggested filenames; avoid overwriting without confirmation; atomic write where feasible.
- Save As uses OS native save dialog, extension and encoder must agree.
- Save writes to configured directory or prompts when missing/unwritable; no success toast before completion.
- Explicit Copy copies actual flattened result with alpha behavior appropriate for OS.
- Never expose arbitrary filesystem read/write from frontend; validate paths and permissions in Rust.

## Performance
- Render at interactive resolution for preview, preserve source-quality export.
- Avoid sending full bitmap across IPC for every drag/property change.
- Throttle expensive blur preview work; cache appropriately; limit history memory (command-based history instead of full bitmap snapshots).
- Avoid memory leaks and UI freezes for 4K screenshots; test at 1920×1080 and 3840×2160.
- Prefer well-maintained existing canvas/image/clipboard libraries, verifying license and platform compatibility.

Implementation note: the current editor keeps a 100-action cap on vector snapshots and exports at the source image dimensions; 4K runtime responsiveness and peak-memory behavior have not yet been measured in a live Tauri window.

## Engineering rules
- Implement real functions only. Hide or disable future controls instead of creating decorative dead buttons.
- Use typed IPC request/response and explicit errors.
- No unrelated refactors; keep cross-platform behavior behind interfaces.
- Include loading, empty, permission and failure states; keyboard accessibility.

## Delivery phases
1. Session routing + real screenshot display + editor window.
2. Layout matching reference, zoom/pan/select and property inspector shell.
3. Pen/arrow/rectangle/text and editable objects.
4. Blur/crop + undo/redo.
5. Styling, background framing and export/copy.
6. Integration, automated tests, visual QA, resource cleanup and docs update.

### Delivery status (2026-10-18)
- Phases 1–5: implementation is present; automated checks cover session cleanup, image validation/export, coordinate transforms and history behavior.
- Phase 6: automated checks and documentation are updated. Native Windows capture-to-editor, visual comparison against `specs/ui/images/edit.png`, unsaved-close interaction, actual clipboard contents and 4K responsiveness remain unverified, so the editor is not marked verified.

## Definition of Done
- Actual capture flows correctly through ON/OFF preview settings.
- UI visually matches `specs/ui/images/edit.png`.
- Every visible enabled tool works and is undoable.
- Save, Save As and Copy produce correct composited pixels.
- No accidental loss of unsaved work; stable at 4K and high DPI.
- Typecheck, Rust checks and available tests pass; test limitations are reported.
- Update `specs/roadmap/features.md` and `specs/roadmap/progress.md` with verified status; do not mark untested platform support as complete.
