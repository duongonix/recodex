# Screenshot Editor — Styles, Borders, Text and Background

## General principles
- Match the Style/Text/Background inspector from `specs/ui/images/edit.png`.
- Properties must update the selected object or the default style for the next object (clearly differentiated).
- Each committed change is undoable. Preserve settings when switching tools, without unintentionally overwriting existing objects.

## Stroke and border
- Color swatches: red `#FF3038`, orange, yellow, green, blue, purple, white; add custom color picker.
- Stroke width: 1–24 px (document-space) with slider and numeric field, validated and synchronized.
- Opacity: 0–100%; solid, dashed and dotted lines.
- Rectangle borders: width, color, opacity, corner radius; optional fill with separate fill color/opacity.
- Arrow properties: none/arrow/other implemented endpoint styles; no inert presets.
- Shadow (if enabled): blur, offset, color/opacity with sensible defaults.

## Text
- Text editor properties: font family (OS-safe fonts), size (e.g. 10–96 px), weight, alignment, text color, optional fill background.
- Support multiline editing, correct cursor and selection behavior, and resizing text box without destructive rasterization.
- Escape commits or cancels editing according to a consistent documented rule; prevent accidental keyboard shortcuts while typing.

## Background editor
The background is a separate output framing layer **behind** the screenshot, not a destructive recolor of screenshot pixels.
- Transparent / solid color / optional gradient presets.
- Padding around screenshot: 0–200px per side (or unified setting initially).
- Corner radius of placed screenshot: 0–40px; ensure clipping and shadow are correct.
- Optional shadow on screenshot, adjustable intensity.
- Background canvas dimensions = screenshot dimensions + padding, unless crop/resize explicitly changes output.
- If background is transparent and exported as JPEG, show warning or composite onto user-selected solid color.

## Rendering order
1. Background color/gradient.
2. Screenshot bitmap with chosen framing/radius/shadow.
3. Blur regions where applicable (defined against correct underlying pixels).
4. Shapes, arrows, pen strokes, text in z-order.
5. Editor-only selection handles/guides (never exported).

## Output fidelity
- Export at original screenshot resolution plus configured padding, independent of preview zoom.
- Respect alpha channel for PNG; handle JPEG flattening and quality appropriately.
- Avoid accidental scaling or blurry text. Verify result dimensions and compare against preview.

## Suggested reusable libraries
Evaluate Konva (via `konva` and Svelte integration where maintained), Fabric.js, or PixiJS for scene interactions, and an established image library such as Rust `image` for export/filters. Choose based on actual project dependencies, maintained compatibility and feature coverage. Do not mix multiple canvas engines without a compelling reason.

## Acceptance tests
- Change stroke width/color/line style, undo, redo and export.
- Text with Vietnamese diacritics exports correctly.
- Rounded image with background padding/shadow matches preview.
- Transparency and JPEG flattening are correct.
- Changing inspector values never silently changes unrelated objects.
