# Choose a Window

The window picker is a separate, borderless Tauri window that follows [images/chosewindow.png](images/chosewindow.png). Keep its dark charcoal surfaces, restrained red selection accent, compact controls, and three-column window-card grid consistent with that reference.

## Window and layout

- Open a centered window-picker route at approximately 1000 × 900 logical pixels, capped to the available monitor work area. It may resize to fit smaller displays.
- Keep the custom title area, content list, and bottom action row visually inside one rounded dark surface. The title area supports dragging the frameless window.
- Use three card columns at desktop size, two at medium widths, and one at narrow widths. The cards show a 16:9 native preview above the application icon/name and window title.
- Keep search/sort controls above the list and the visible-only filter plus Cancel/Select Window actions at the bottom.

## Native window data

- Enumerate live top-level OS windows through the Rust capture adapter. Never use fixture entries or browser-only window APIs.
- Show each window's real application name, title, icon where the OS can provide one, visibility/minimized state, and native preview where capture permits it.
- The visible-only checkbox starts checked. Turning it off may reveal hidden or minimized entries, which remain visibly unavailable and cannot be selected.
- Sort modes are Last used, Application name, and Window title. Last used reflects confirmed user selections and stays unset for windows that have never been selected.
- Previews load lazily and use bounded native work. Pixel capture, resizing, and encoding remain in Rust.

## Selection and recovery

- A user can select one recordable window. The Select Window action stays disabled until one is selected; double-click confirms it.
- Escape and Cancel close the picker and keep the previously committed capture source. Confirm revalidates the window before changing the source; recording start validates it again.
- Refresh recovers from a stale list. Show explicit loading, empty, and error states; preview failures are local to that card and do not invent a successful image.
- Support keyboard navigation, visible focus, accessible labels, and a clear selected state.

## Platform support

The current native enumeration, application icon extraction, and WGC thumbnail adapter are Windows-only. Other platforms must report the native capability as unsupported until their adapters and permissions exist; the interface must not display fabricated windows.
