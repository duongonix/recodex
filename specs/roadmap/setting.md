# Recodex — Settings UI & Behavior Specification

**Status:** Approved UI direction · **Reference:** `specs/ui/images/setting.png`  
**Target:** Tauri 2 + Svelte 5 + Tailwind CSS + shadcn-svelte + Lucide icons  
**Authority:** Follow root `AGENTS.md` and existing `specs/` architecture and API contracts. This document overrides older Settings UI proposals if they conflict.

## 1. Non-negotiable interaction model

- **Settings is a view within the existing `main` Tauri window, not a modal, popup, overlay, tab in the recorder, or another native window.**
- The existing window remains **400 × 490 CSS px** when switching between the recorder and Settings. Do not resize or reposition it, and do not create a second `WebviewWindow`.
- Clicking the gear in the main header swaps the recorder content for Settings content in the same Svelte view hierarchy. Clicking the top-left Back arrow restores the previous recorder view, including its selected Record/Screenshot mode, capture source, and form values.
- Settings has exactly **three top tabs: `General`, `Snap`, `Hotkey`**. These tabs exist *only inside Settings*, not in the main recorder navigation.
- Default active Settings tab: `Snap` when opened (or preserve the last Settings tab during the current app session, if this behavior already exists).
- All settings changes auto-save, without a Save button. Reflect persistence failures and revert the affected control if the backend rejects the update.

## 2. Visual source of truth

Use `specs/ui/images/setting.png` as the primary visual reference for colors, spacing, typography, icons, selection styling and composition. **The image displays two states side by side only to demonstrate the transition; the running app displays one 400 × 490 window at a time.** The right-hand Settings view in the illustration is taller than the actual app: **fit the header and tabs in the fixed window and scroll the settings body vertically**. Never stretch the native window to match the image.

### Design tokens (align with existing app tokens if present)

| Token | Value / usage |
|---|---|
| Base background | `#101116` |
| Elevated surface | `#17191F` |
| Control surface | `#1B1D24` |
| Primary accent | `#FF3038` |
| Accent hover | `#FF454D` |
| Main text | `#F5F5F7` |
| Muted text | `#989AA7` |
| Subtle separator | `#292B33` |
| Corner radius | 10–12px for controls; inherit outer app radius |

Style: flat, understated, dark, red selected states; **no boxed cards or borders around every settings row**. Use subtle section dividers, and borders only where useful for interactive fields. Do not introduce a separate settings sidebar, a bright panel, gradients on every row, or new accent colors.

## 3. Fixed window layout (400 × 490)

Layout, top to bottom:

1. **Existing title bar** (approx. 44–48px): Back arrow at far left, existing Recodex logo and title, existing minimize and close controls at right. Preserve current draggable/non-draggable regions and native behavior. Do not display the main gear icon here while Settings is active.
2. **Settings heading** (compact, approx. 55–65px): small red settings icon, `Settings` title and optional single-line subtitle (`Recording & screenshot preferences`). Avoid wrapping the subtitle.
3. **Tab strip** (approx. 44px): 3 equal-width horizontal tabs with Lucide icons, `General`, `Snap`, `Hotkey`. Selected tab uses subdued red fill and red border/indicator, other tabs use dark surfaces. Keyboard accessible, visible focus states.
4. **Scroll container** fills remaining available height: section headings and flat settings rows. Keep header and tabs visible while scrolling. Use compact row heights (~40–48px) and 12–16px horizontal padding. No horizontal scroll.

If an input's full value does not fit (notably folder paths), truncate visually with ellipsis and expose full path through a tooltip/title. Do not shrink font to unreadable sizes.

## 4. Tab contents

### 4.1 General

Keep this tab minimal. Exactly these controls:

- **Start with system** — toggle, default off. Display platform-appropriate wording (`Start with Windows`, `Start at login`, etc.) rather than hard-coding Windows on macOS/Linux.
- **Minimize to system tray** — toggle, default on *if tray support is actually implemented*; otherwise disabled with explanatory hint.
- **Show notifications** — toggle, default on; applies to recording/screenshot completion or errors, not arbitrary promotional notifications.

Do not add theme switcher, color picker, language selector, updater, account settings, analytics, or cloud controls for this MVP.

### 4.2 Snap

Two flat sections, separated by a subtle divider.

**Recording**

1. **Default save folder** — path field (read-only) and folder icon button opening the native OS directory picker. Store the chosen directory persistently. Initial value: a valid app subfolder under the OS Videos directory, or a safe OS-appropriate fallback. Never hard-code a personal Windows path from the reference image.
2. **Show mouse cursor** — switch; default on. Must affect actual recording behavior if the capture backend supports it; if unsupported on a platform, disable with an explanation instead of pretending it works.
3. **System audio** — switch; reflect the same setting/state used by the recorder main screen.
4. **Microphone** — switch; reflect the same setting/state used by the recorder main screen.
5. **Default quality** — dropdown; use only resolutions actually supported by the current capture/encode pipeline (e.g. 720p, 1080p, 4K where available).
6. **Default FPS** — dropdown; offer supported rates (e.g. 30 and 60 FPS).
7. **Hold-to-zoom** — off-by-default switch. When enabled, holding its shortcut smoothly zooms to 2× around the cursor in Windows display/window recordings. Region and screenshot capture are unaffected; unsupported platforms show an explanatory disabled state.
8. **Hold-to-zoom shortcut** — editable key-combination field, default `CommandOrControl+Shift+Z`. It captures Ctrl/Command or Alt plus a letter, number, or F key; Escape cancels listening. Registration conflicts, including the global quick-access shortcut, are rejected and the last saved value is retained.
9. **Webcam overlay** — a camera-icon row with an on/off switch and chevron opens a detailed configuration view in this same 400 × 490 main window. The detail view has a fixed header, Back navigation to Snap, and a vertically scrolling body. Webcam is forced off at app launch; enabling it starts camera capture and shows a native desktop overlay at the bottom-right. During recording it sits below the toolbar, and dragging the toolbar moves both together. Camera capture starts only after enabling the overlay or the user explicitly starts preview. The current native implementation and its unverified hardware/platform cases are documented in `specs/webcam/01-overlay-architecture.md`.

**Screenshot**

1. **Default save folder** — path field (read-only) and folder picker button. Initial value: valid app subfolder under OS Pictures directory, or safe fallback.
2. **Show screenshot preview** — editor-specific workflow preference, default ON. ON opens the screenshot editor after capture; OFF uses the direct save/copy destinations. This preference is specified in `specs/editor/05-architecture-and-workflow.md` and is the only screenshot control added for the editor workflow.

The reference image is the visual source of truth, but **the exact controls above are the functional source of truth**, with the editor-specific preview preference documented above. Do not add screenshot format, clipboard, cursor highlight, countdown, or other options.

### 4.3 Hotkey

Display exactly three shortcut rows, matching the established shortcuts where available:

- **Start / Stop recording** — `Ctrl + Shift + R` (platform-aware equivalent if needed).
- **Take screenshot** — `Ctrl + Shift + S`.
- **Capture region** — `Ctrl + Shift + A`.

Use a compact key-combination control (dark flat surface). If hotkey reassignment is already supported, clicking a field enters a clear listening state (`Press shortcut…`), supports Escape to cancel, checks for duplicates/conflicts, and saves only after successful OS registration. If editing is **not** implemented, show read-only shortcuts rather than nonfunctional clickable inputs. Provide a short note if global shortcut permissions are unavailable.

## 5. State and backend integration

- Use existing shared Svelte 5 reactive state for current view, selected Settings tab, recording preferences, and screenshot preferences. Do not duplicate source-of-truth values between main UI and Settings.
- On entry, load settings from existing Tauri/Rust persistence (or the existing Tauri store plugin). Show sensible loading placeholders; do not overwrite stored values with defaults during asynchronous hydration.
- Each toggle/select commits changes immediately; on error show a small non-blocking message and restore the last confirmed value.
- Directory picker: use the project's existing Tauri dialog plugin or install the official `@tauri-apps/plugin-dialog` if absent, with corresponding Rust plugin registration and permissions. Validate the selected directory before saving; cancellation leaves the previous path unchanged.
- Recording and screenshot output routines must actually use the persisted directories. Existing main-window Quality/FPS and audio toggles must remain synchronized with the Snap tab.
- Do not silently change active capture settings in the middle of a recording. Apply options that require restarting capture to the next session and communicate this if relevant.
- Preserve user settings across app restarts. Do not use fake paths or mock success states.
- Respect platform constraints for startup, tray, shortcuts, cursor capture, and directory access. Surface unsupported functionality accurately.

## 6. Suggested Svelte module split

Adapt to existing project layout; do not duplicate existing shared components:

```text
src/
  features/
    settings/
      SettingsView.svelte           # composition and Back navigation only
      SettingsTabs.svelte           # three-tab strip
      GeneralSettings.svelte
      SnapSettings.svelte
      HotkeySettings.svelte
      components/
        SettingRow.svelte
        FolderPickerRow.svelte
        ShortcutField.svelte
      settings-state.svelte.ts      # only if no existing shared store
      settings-api.ts               # Tauri IPC/store adapter
```

Prefer existing shadcn-svelte `Tabs`, `Switch`, `Select`, `Button`, and accessible primitives; use Lucide icons. Do not hand-build components already available and suitable. Keep files small and focused as required by `AGENTS.md`. Use CSS variables/Tailwind tokens rather than duplicating hard-coded colors across components.

## 7. Behavior and accessibility

- Back button works via mouse and keyboard, and returns to the prior main view without resetting selections.
- Tabs switch instantly without spawning windows, changing window dimensions, or remounting the entire Tauri application.
- Only the Settings content area scrolls; title bar and tabs stay fixed.
- Tab key reaches all controls in logical order. Each switch and input has a programmatic label, with visible focus indicator.
- Folder picker is keyboard accessible. Disabled options explain why they are disabled.
- Long translations/paths must not cause overflow.
- Avoid layout shifts while loading. Respect reduced-motion preferences.

## 8. Acceptance checklist

- [ ] Settings replaces the existing main content in the **same** 400 × 490 Tauri window.
- [ ] Back returns to the previous recorder view with its state preserved.
- [ ] Exactly 3 tabs: General, Snap, Hotkey.
- [ ] Style matches `specs/ui/images/setting.png` while remaining compact and flat.
- [ ] Header and tab strip remain visible while settings content scrolls.
- [ ] General has exactly 3 listed controls.
- [ ] Snap Recording has folder, cursor, system audio, microphone, quality, FPS, hold-to-zoom, and its shortcut.
- [ ] Snap Screenshot has its save-folder picker and editor-specific Show screenshot preview preference.
- [ ] Hotkey shows 3 real shortcuts; editable only when registration is implemented.
- [ ] Folder pickers choose real OS directories; cancel does not mutate values.
- [ ] Settings persist and are reflected in real capture/save behavior.
- [ ] Hold-to-zoom is off by default, uses a registered pressed/released shortcut, and is applied only to supported screen/window recordings.
- [ ] UI remains usable without clipping or horizontal scroll at 400 × 490.
- [ ] Svelte/typecheck, Rust checks and app build pass; manual interaction is verified on the current OS.

## 9. Codex delivery instructions

1. Read `AGENTS.md`, existing `specs/ui/` and relevant architecture/API specs; inspect the current implementation before editing.
2. Reuse installed libraries and components. Split UI/state/platform logic into focused modules.
3. Implement the exact navigation and content above, not the older large Settings window or floating overlay proposals.
4. Verify the fixed 400 × 490 layout with real data and long paths. Test settings persistence and actual output folder usage.
5. Update relevant `specs/roadmap/` progress and feature records only for verified work. Report unsupported platform behaviors and any remaining blockers.
