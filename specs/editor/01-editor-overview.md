# Recodex Screenshot Editor — Product & Behavior

> Status: approved UI. Visual reference: `specs/ui/images/edit.png`.
> Read `AGENTS.md`, `specs/ui/setting.md`, and existing capture specifications before implementing.

## Purpose
A focused, non-destructive editor for a screenshot immediately after capture. It must look like the approved Recodex Dark + Red reference, not a general-purpose photo editor. Main Recodex window remains 400 × 490 px; editor is a separate, resizable Tauri window.

## Entry conditions
- `Settings > Snap > Screenshot > Show screenshot preview` ON: capture image, create temporary screenshot session, then open editor with actual captured pixels.
- OFF: save screenshot to configured screenshot folder directly; never open editor.
- On screenshot failure, do not open an empty editor. Display a useful error.
- Opening editor must not block capture or cause the editor to appear inside its own screenshot.
- A screenshot session contains source dimensions, pixel data/reference, capture time, suggested filename, output folder, dirty flag and edit document.

## User journey
1. Capture screenshot (full/window/region).
2. Editor opens showing the actual image centered on a checkerboard workspace, fitted to view.
3. Choose a tool from left toolbar, edit image, configure properties in right inspector.
4. Undo/redo, zoom, pan, select and modify objects.
5. Copy composited result to clipboard, Save to configured folder, or Save As to custom location.
6. On close with unsaved changes, show Save / Discard / Cancel confirmation. No silent loss.

## Scope (MVP)
- Select, freehand pen, arrow, rectangle, text, blur and crop.
- Stroke/fill/opacity and tool-appropriate styling; text properties; background padding/color and optional shadow.
- Zoom to fit, zoom percentage, pan; undo/redo; copy; save; save as.
- Support one image per editor session. Reference shows a bottom thumbnail strip; MVP may display one thumbnail only. Plus icon must be hidden/disabled unless adding screenshots is implemented; never fake multi-image support.

## State and persistence
- Edits are non-destructive until exported. Keep original screenshot immutable.
- A screenshot editor session is transient. Export path and selected tool may be remembered; do not persist screenshot pixels in settings.
- Settings are shared with main window; changes must survive app restart.
- Save must produce actual image bytes with all visible edits, not just the source image.

## Acceptance criteria
- ON/OFF switch changes actual post-capture behavior.
- Editor opens with a real screenshot and correct dimensions.
- Every visible enabled tool works on the real image.
- Save/Copy match the canvas output; original remains intact unless explicitly overwriting a chosen path.
- No misleading functional UI, crashes or unsaved data loss.
