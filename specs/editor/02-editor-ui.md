# Screenshot Editor — Pixel-Level UI Specification

**Primary visual source:** `specs/ui/images/edit.png` (approved design). Implement this as a functional UI, not as a flattened screenshot.

## Overall layout
- Separate resizable editor window; suggested initial size 1440 × 900 CSS px, clamped to current display working area; minimum 960 × 620 when display permits.
- One continuous app surface; dark almost-black chrome, subtle separators, rounded outer shell where platform supports it.
- Regions: top command bar (~76px), left tool rail (~90px), center workspace (flex), right inspector (~340px), bottom screenshot strip/status (~90px). On smaller windows, allow inspector scrolling and sensible responsive collapse.
- Main screenshot window remains unchanged at 400 × 490 px.

## Palette and typography
- App background `#0E1015`; toolbar `#111319`; inspector `#15171D`; elevated controls `#1C1F27`.
- Primary accent `#FF3038`, hover `#FF454D`, muted red tint `rgba(255,48,56,.12)`.
- Main text `#F5F5F7`, secondary `#A1A4AF`, subtle separator `rgba(255,255,255,.08)`.
- Keep **flat** controls and sections; no gratuitous card outlines. Use thin separators only when needed.
- 13–14px normal labels, 15–16px group labels; consistent Lucide icons 18–22px; 10–14px radii on interactive controls.

## Top command bar
- Left: Recodex logo/wordmark, `Screenshot Editor` caption.
- Center: Undo, Redo, zoom selector (Fit, 25, 50, 75, 100, 150, 200%), fit-to-screen icon.
- Right: Copy, Save As…, prominent red Save. Show loading/progress/disabled states while exporting.
- Provide close/minimize/maximize via platform-appropriate window controls, without duplicating native chrome.

## Left vertical toolbar
Order from top to bottom: Select, Pen, Arrow, Rectangle, Text, Blur, Crop, More (only if populated). Active tool has red-tinted background and red border/glow. Show icon + small text label. Bottom: current color swatch.
- Tooltips with keyboard shortcut; hover/focus states; tool selected persistently until changed.
- If More has no implemented tools, omit it rather than showing an inert control.

## Center workspace
- Checkerboard transparency surface around the screenshot; image centered with consistent inset.
- Zoom and pan do not alter output resolution.
- Render vector annotations over screenshot; selected element displays red dashed selection rectangle and white/red handles.
- Avoid stretching screenshot, blurry scaling at 100%, and covering it with UI.
- Crop overlay must darken outside selection and show adjustable crop handles.

## Right inspector
- Top horizontal tabs `Style`, `Text`, `Background` as reference.
- Style: color swatches + custom color, stroke width slider/input, opacity slider/input, line style (solid/dashed/dotted), start/end arrowheads where applicable, fill toggle, shadow toggle.
- Text: font family (supported fonts), size, weight, alignment, color, background/padding if supported.
- Background: solid color/transparent, padding, corner radius, shadow, optional gradient presets (only if implemented).
- Show only properties applicable to selected tool/object; explain disabled settings instead of letting them do nothing.
- Inspector scrolls independently of canvas.

## Bottom strip and status
- One screenshot thumbnail with red active state, optional metadata `1920 × 1080 | PNG | size` derived from actual image.
- A plus/add button must not appear active until multi-image sessions are supported.

## Responsiveness and accessibility
- Keyboard focus visible, semantic labels, tooltips, Esc behavior, no keyboard traps.
- Respect OS scaling and small screens; avoid nested horizontal scroll for primary actions.
- Animation 120–180ms and reduced-motion preference.

## UI verification
Capture screenshot of implemented editor at target resolution and compare visually to `edit.png`: layout, spacing, colors, iconography, hierarchy, and selected-state styling. Document justified differences due to platform or accessibility.
