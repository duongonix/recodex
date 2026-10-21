# Video Editor Window

## Entry and scope

- The dedicated resizable Tauri window is labeled `recording-review`, starts at 1280 × 820 px with a 960 × 620 px minimum, and keeps the compact 400 × 490 px main window unchanged.
- The editor opens for a pending recording when `AppSettings.showRecordingPreview` is enabled. The current product flow does not provide a media library or arbitrary-file import; a recording can also be opened from the main window while it is pending review.
- Use the supplied dark + red Recodex reference: background `#101116`, panels `#17191F`, elevated controls `#1B1D24`, accent `#FF3038`, hover `#FF454D`, primary text `#F5F5F7`, secondary text `#989AA7`, and divider `#292B32`.

## Layout

- No Media/Clips sidebar. The custom 56 px header shows the Recodex mark/name and “Video Editor” on the left and minimize, maximize/restore, and close controls on the right. Pressing the left mouse button anywhere else on the header starts native Tauri window dragging; window-control clicks remain interactive.
- The flexible upper workspace places a real video preview on the left and a 320 px Video/Audio/Canvas property panel on the right. The preview preserves source aspect ratio, displays a dark canvas, and offers Fit/Fill, seek, current/total time, previous/play-pause/next, volume, and mute controls.
- The bottom timeline spans the editor width. Its toolbar contains Select, Split, Delete, Trim, Undo, Redo, zoom out/slider/in, and Fit Timeline. Below it are a time ruler, video thumbnails, a linked audio waveform when audio exists, a red playhead, and segment trim handles.
- The footer shows source dimensions, recording FPS, and edited duration, Cancel (save the original recording), and Export. The window uses a no-scroll grid layout; property values can scroll within their panel when necessary.

## Playback and editing

- The native HTML video element is the only playback clock. Play/Pause, the preview seek bar, timeline clicks, previous/next segment controls, arrow-key seeking, and the timeline playhead follow the same source/timeline mapping. Space toggles playback outside text inputs; Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z (also Ctrl/Cmd+Y) undo and redo; Delete/Backspace removes the selected segment.
- Split is non-destructive: it creates two source-time ranges at the playhead without changing the source MP4. A split requires at least the timeline model's minimum duration on both sides.
- Each segment has independent source in/out points. Dragging either red handle trims that segment; Delete removes only the selected segment and closes the gap. Video and audio use the same segment source range. The 100 ms minimum is enforced by the timeline model.
- Undo/redo snapshots contain only timeline ranges, selected segment, and editor properties, not decoded video frames. History is capped at 100 states.
- Native Media Foundation Source Reader extraction supplies bounded thumbnail JPEGs and audio waveform bins. If the source has no audio stream, the audio lane reports that state; the UI does not fabricate a waveform.

## Properties and export

- Video properties: scale, X/Y position, rotation, speed, opacity, and crop enable/left/right/top/bottom.
- Audio properties: volume and mute. Canvas properties: source/16:9/9:16/1:1 aspect, background color, and contain/cover fit.
- Preview styling follows the current controls. Export applies supported visual properties to BGRA frames in Rust, retimes video/audio to the selected speed, applies volume/mute, and writes one H.264/AAC MP4 using the existing Windows Media Foundation pipeline. No raw frame is sent to JavaScript.
- Export asks for an MP4 destination, does not replace existing files, runs on a blocking worker, reports native progress, and accepts cancellation. Cancellation removes the incomplete export and keeps the pending source recording available. Completed output is staged beside the chosen destination and published with a no-replace move.
- Cancel/close saves the untouched source recording. Successful timeline export finishes the review and returns to the main window. Native failures leave the source recording available and show the backend error.

## Platform and verification status

- The current review, thumbnail/waveform extraction, and edited MP4 export backend use Windows Media Foundation. macOS and Linux have no editor media adapter yet and must return an explicit unsupported result.
- Windows Rust compile/tests and Svelte checks verify the model, transform math, export transaction rules, and command contracts. Playback against real recordings, live drag/seek interaction, output audio/video sync, and end-to-end export/cancellation still require a Windows app smoke test before this feature can be marked complete.
