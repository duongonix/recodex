# Screenshot Editor — Canvas Tools and Interaction

## Document model
Use immutable source bitmap plus ordered editable layers/objects. Each object has stable ID, z-order, visibility, transform/bounds and tool-specific attributes. Do not permanently draw annotations into original pixels during editing.

## Pointer and keyboard behavior
- Convert pointer coordinates from CSS screen space into image/document space using current zoom, pan and device pixel ratio.
- Pointer capture during drag; release safely on cancellation/window blur.
- Escape cancels active gesture; Delete removes selection; Ctrl/Cmd+Z Undo; Ctrl/Cmd+Shift+Z or Ctrl+Y Redo; Ctrl/Cmd+C copies selection when appropriate or composited screenshot through explicit Copy button.
- Pan with space+drag or middle mouse; mouse wheel zoom with modifier or zoom control; trackpad behavior should be predictable.

## Select (MVP)
- Click to select one object; drag to move; drag handles to resize; click blank to deselect.
- Maintain accurate hit-testing at any zoom. Prevent objects from accidentally moving while editing text.

## Pen
- Pointer-down starts stroke, pointer-move adds points, pointer-up commits one undoable stroke.
- Smooth paths with existing rendering library functionality; adjustable color, width and opacity.
- Support mouse and pen input when available; no pressure-sensitive promise unless tested.

## Arrow
- Drag start/end points; editable endpoints after creation.
- Support stroke color/width/opacity, line style, arrowhead at start/end. Arrowheads scale with stroke width.

## Rectangle
- Drag to create, resize with handles. Stroke width/color, optional fill, opacity and corner radius where supported.
- Shift modifier optionally constrains to square. Avoid negative-size geometry bugs.

## Text
- Click to insert editable text box; double-click to edit existing box.
- Font size, weight, alignment, color; multiline, word wrap, move/resize.
- Text rendering in exported image must match editor preview as closely as possible; account for font availability.

## Blur
- Drag a rectangular region to add a non-destructive blur object masking underlying screenshot content.
- Blur must be applied to source/underlying composited pixels in the region, not just a translucent rectangle.
- Blur strength adjustable; crop/export coordinates must remain accurate. Use tested image filters rather than custom pixel loops when possible.

## Crop
- Show crop frame and dimmed outside region. Resize/drag and confirm/cancel explicitly.
- Crop operation changes output bounds and is undoable. Preserve annotation alignment and define behavior for partially cropped objects.
- Avoid changing original screenshot bytes.

## Undo / Redo
- Commands: add/delete object, move, resize, property edit, text edit, crop, background edit.
- Coalesce continuous drag into one undo action; don't push one action per pointer move.
- Clear redo history when a new edit is committed after undo.
- Track dirty state and ensure close confirmation.

## Tests
- Unit tests for coordinate mapping, hit-testing, bounds, undo stack and serialization.
- Interaction tests for every tool at zoom 50%, 100%, 200%; crop + annotations; blur + export; empty text; tiny screenshot; high-DPI display.
