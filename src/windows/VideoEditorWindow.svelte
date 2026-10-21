<script lang="ts">
  import { onMount, tick } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import EditorHeader from "../features/video-editor/components/EditorHeader.svelte";
  import VideoPreview from "../features/video-editor/components/VideoPreview.svelte";
  import PropertyPanel from "../features/video-editor/components/PropertyPanel.svelte";
  import RecordingTimelineEditor from "../features/video-editor/components/RecordingTimelineEditor.svelte";
  import EditorFooter from "../features/video-editor/components/EditorFooter.svelte";
  import ExportDialog from "../features/video-editor/components/ExportDialog.svelte";
  import { cancelRecordingTimelineExport, exportRecordingTimeline, getPendingRecordingReview, getRecordingReviewAssets, saveRecordingReview, type RecordingReviewInfo } from "../lib/services/ipc/recording-review.js";
  import { errorMessage } from "../lib/services/ipc/settings.js";
  import { createInitialSegment, deleteSegment, getTimelineDuration, getUsableSourceDurationMs, nextTimelineBoundary, previousTimelineBoundary, sourceTimeAtTimelinePosition, splitAtTimelinePosition, timelinePositionAtSourceTime, trimSegment } from "../features/video-editor/timeline-model.js";
  import { createDefaultEditorSettings, type EditorHistorySnapshot, type EditorSettings, type TimelineSegment } from "../features/video-editor/types.js";

  const READY_EVENT = "recording-review://ready";
  const ERROR_EVENT = "recording-review://error";
  const EXPORT_PROGRESS_EVENT = "recording-review://export-progress";
  const MAX_HISTORY = 100;
  let review = $state<RecordingReviewInfo | null>(null);
  let videoSource = $state<string | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let pending = $state(false);
  let exportProgress = $state(0);
  let exportOpen = $state(false);
  let videoElement: HTMLVideoElement | undefined = $state();
  let sourceDurationMs = $state(0);
  let currentTimeMs = $state(0);
  let sourceWidth = $state(0);
  let sourceHeight = $state(0);
  let playing = $state(false);
  let audioWaveform = $state<number[]>([]);
  let videoThumbnails = $state<string[]>([]);
  let waveformReviewId: string | null = null;
  let segments = $state<TimelineSegment[]>([]);
  let selectedSegmentId = $state<string | null>(null);
  let settings = $state<EditorSettings>(createDefaultEditorSettings());
  let undoStack = $state<EditorHistorySnapshot[]>([]);
  let redoStack = $state<EditorHistorySnapshot[]>([]);
  let nextSegmentId = 2;
  let lastCheckpoint = 0;

  const timelineDurationMs = $derived(getTimelineDuration(segments, settings.speed));
  const canSplit = $derived(Boolean(splitAtTimelinePosition(segments, currentTimeMs, "split-preview", settings.speed)));
  const canUndo = $derived(undoStack.length > 0);
  const canRedo = $derived(redoStack.length > 0);
  const canvasRatio = $derived(settings.canvasAspect === "source"
    ? (sourceWidth > 0 && sourceHeight > 0 ? sourceWidth / sourceHeight : 16 / 9)
    : settings.canvasAspect === "16:9" ? 16 / 9 : settings.canvasAspect === "9:16" ? 9 / 16 : 1);
  const canvasStyle = $derived(`--ratio:${canvasRatio};background:${settings.backgroundColor};`);
  const videoStyle = $derived(makeVideoStyle(settings, sourceWidth, sourceHeight));

  function makeVideoStyle(value: EditorSettings, width: number, height: number): string {
    const l = width ? Math.max(0, Math.min(width - 2, value.cropLeft)) / width * 100 : 0;
    const r = width ? Math.max(0, Math.min(width - 2, value.cropRight)) / width * 100 : 0;
    const t = height ? Math.max(0, Math.min(height - 2, value.cropTop)) / height * 100 : 0;
    const b = height ? Math.max(0, Math.min(height - 2, value.cropBottom)) / height * 100 : 0;
    return `--fit:${value.fit};opacity:${value.opacity / 100};transform:translate(${width ? value.positionX / width * 100 : 0}cqw,${height ? value.positionY / height * 100 : 0}cqh) scale(${value.scale / 100}) rotate(${value.rotation}deg);clip-path:${value.cropEnabled ? `inset(${t}% ${r}% ${b}% ${l}%)` : "none"}`;
  }

  function snapshot(): EditorHistorySnapshot {
    return { segments: segments.map((segment) => ({ ...segment })), selectedSegmentId, settings: { ...settings } };
  }
  function checkpoint() {
    const now = performance.now();
    if (lastCheckpoint && now - lastCheckpoint < 180) return;
    undoStack = [...undoStack.slice(-(MAX_HISTORY - 1)), snapshot()];
    redoStack = [];
    lastCheckpoint = now;
  }
  function applyPlaybackSettings() {
    if (!videoElement) return;
    videoElement.playbackRate = settings.speed;
    videoElement.volume = Math.max(0, Math.min(1, settings.volume / 100));
    videoElement.muted = settings.muted;
  }
  function updateSettings(patch: Partial<EditorSettings>) {
    checkpoint();
    settings = { ...settings, ...patch };
    applyPlaybackSettings();
  }
  function restoreSnapshot(value: EditorHistorySnapshot) {
    segments = value.segments.map((segment) => ({ ...segment }));
    selectedSegmentId = value.selectedSegmentId;
    settings = { ...value.settings };
    applyPlaybackSettings();
    seekTo(Math.min(currentTimeMs, getTimelineDuration(segments, settings.speed)));
  }
  function undo() {
    const previous = undoStack.at(-1);
    if (!previous) return;
    redoStack = [...redoStack, snapshot()];
    undoStack = undoStack.slice(0, -1);
    restoreSnapshot(previous);
  }
  function redo() {
    const next = redoStack.at(-1);
    if (!next) return;
    undoStack = [...undoStack, snapshot()];
    redoStack = redoStack.slice(0, -1);
    restoreSnapshot(next);
  }

  onMount(() => {
    let disposed = false;
    let unlistenReady: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;
    let unlistenExportProgress: (() => void) | undefined;
    const load = async () => {
      loading = true;
      try {
        const next = await getPendingRecordingReview();
        if (next?.reviewId !== review?.reviewId) {
          sourceDurationMs = 0; currentTimeMs = 0; sourceWidth = 0; sourceHeight = 0;
          segments = []; selectedSegmentId = null; settings = createDefaultEditorSettings(); audioWaveform = []; videoThumbnails = []; waveformReviewId = null;
          undoStack = []; redoStack = []; nextSegmentId = 2;
        }
        review = next;
        videoSource = review ? convertFileSrc(review.videoPath) : null;
        error = null;
        if (review && waveformReviewId !== review.reviewId) {
          const reviewId = review.reviewId;
          waveformReviewId = reviewId;
          void getRecordingReviewAssets(reviewId, 240, 24).then((assets) => {
            if (review?.reviewId === reviewId) {
              audioWaveform = assets.audioWaveform;
              videoThumbnails = assets.videoThumbnails;
            }
          }).catch((cause: unknown) => {
            console.warn("[RecoDex][video-editor] Could not load native timeline assets", errorMessage(cause));
          });
        }
      } catch (cause) { error = errorMessage(cause); }
      finally { loading = false; }
    };
    const setup = async () => {
      try {
        unlistenReady = await listen(READY_EVENT, () => { void load(); });
        unlistenError = await listen<string>(ERROR_EVENT, ({ payload }) => { error = payload; });
        unlistenExportProgress = await listen<number>(EXPORT_PROGRESS_EVENT, ({ payload }) => {
          if (Number.isFinite(payload)) exportProgress = Math.max(0, Math.min(1, payload));
        });
        if (!disposed) await load();
      } catch (cause) { error = errorMessage(cause); loading = false; }
    };
    void setup();
    return () => { disposed = true; unlistenReady?.(); unlistenError?.(); unlistenExportProgress?.(); };
  });

  function onVideoMetadata() {
    if (!videoElement || !Number.isFinite(videoElement.duration)) return;
    sourceDurationMs = getUsableSourceDurationMs(videoElement.duration * 1000, review?.durationMs ?? 0);
    sourceWidth = videoElement.videoWidth;
    sourceHeight = videoElement.videoHeight;
    if (!segments.length) {
      segments = createInitialSegment(sourceDurationMs);
      selectedSegmentId = segments[0]?.id ?? null;
    }
    applyPlaybackSettings();
    videoElement.currentTime = sourceTimeAtTimelinePosition(segments, currentTimeMs, settings.speed) / 1000;
  }
  function seekTo(timelineMs: number) {
    const position = Math.max(0, Math.min(timelineDurationMs, timelineMs));
    currentTimeMs = position;
    if (videoElement && Number.isFinite(videoElement.duration) && videoElement.duration > 0) {
      videoElement.currentTime = sourceTimeAtTimelinePosition(segments, position, settings.speed) / 1000;
    }
  }
  function onVideoTimeUpdate() {
    if (!videoElement || !Number.isFinite(videoElement.currentTime)) return;
    const sourceMs = videoElement.currentTime * 1000;
    const position = timelinePositionAtSourceTime(segments, sourceMs, settings.speed);
    if (position !== null) { currentTimeMs = position; return; }
    const next = segments.find((segment) => segment.sourceStartMs > sourceMs);
    if (next && playing) {
      const nextPosition = timelinePositionAtSourceTime(segments, next.sourceStartMs, settings.speed);
      if (nextPosition !== null) currentTimeMs = nextPosition;
      videoElement.currentTime = next.sourceStartMs / 1000;
    } else if (playing) { videoElement.pause(); currentTimeMs = timelineDurationMs; }
  }
  async function togglePlayback() {
    if (!videoElement) return;
    if (videoElement.paused) {
      if (currentTimeMs >= timelineDurationMs && timelineDurationMs > 0) seekTo(0);
      try { await videoElement.play(); } catch (cause) { error = errorMessage(cause); }
    } else videoElement.pause();
  }
  function previousClip() {
    seekTo(previousTimelineBoundary(segments, currentTimeMs, settings.speed));
  }
  function nextClip() {
    seekTo(nextTimelineBoundary(segments, currentTimeMs, settings.speed));
  }
  function splitAtPlayhead() {
    const next = splitAtTimelinePosition(segments, currentTimeMs, `segment-${nextSegmentId++}`, settings.speed);
    if (!next) return;
    checkpoint(); segments = next;
    const offset = currentTimeMs;
    let cursor = 0;
    selectedSegmentId = next[0]?.id ?? null;
    for (const segment of next) {
      const end = cursor + (segment.sourceEndMs - segment.sourceStartMs) / settings.speed;
      if (offset <= end) { selectedSegmentId = segment.id; break; }
      cursor = end;
    }
  }
  function trimSelected(id: string, edge: "start" | "end", sourceTimeMs: number) {
    const next = trimSegment(segments, id, edge, sourceTimeMs, sourceDurationMs);
    if (!next) return;
    checkpoint(); segments = next;
    seekTo(Math.min(currentTimeMs, getTimelineDuration(next, settings.speed)));
  }
  function deleteSelected() {
    if (!selectedSegmentId) return;
    const next = deleteSegment(segments, selectedSegmentId);
    if (!next) return;
    checkpoint(); segments = next; selectedSegmentId = next[0]?.id ?? null;
    seekTo(Math.min(currentTimeMs, getTimelineDuration(next, settings.speed)));
  }
  async function releaseVideo() {
    videoElement?.pause(); videoElement?.removeAttribute("src"); videoElement?.load();
    videoSource = null; await tick();
  }
  async function restoreVideo() {
    if (!review) return;
    videoSource = convertFileSrc(review.videoPath); await tick(); applyPlaybackSettings();
  }
  async function saveOriginal() {
    if (pending) return;
    if (!review) { await getCurrentWindow().close(); return; }
    pending = true; error = null; const id = review.reviewId; await releaseVideo();
    try { await saveRecordingReview(id); review = null; }
    catch (cause) { error = errorMessage(cause); await restoreVideo(); }
    finally { pending = false; }
  }
  async function exportEdited(path: string) {
    if (!review || pending || !segments.length) return;
    pending = true; exportProgress = 0; error = null; const id = review.reviewId; await releaseVideo();
    try {
      const output = await exportRecordingTimeline(id, segments.map(({ sourceStartMs, sourceEndMs }) => ({ startMs: sourceStartMs, endMs: sourceEndMs })), settings, path);
      console.info(`[RecoDex][video-editor] Exported edited recording: ${output}`);
      review = null; exportOpen = false;
    } catch (cause) { exportProgress = 0; error = errorMessage(cause); await restoreVideo(); }
    finally { pending = false; }
  }
  async function cancelExport() {
    try { await cancelRecordingTimelineExport(); }
    catch (cause) { error = errorMessage(cause); throw cause; }
  }
  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && exportOpen && !pending) { exportOpen = false; return; }
    if ((event.target as HTMLElement | null)?.matches("input,textarea,select,[contenteditable='true']")) return;
    const mod = event.ctrlKey || event.metaKey;
    if (mod && event.shiftKey && event.key.toLowerCase() === "z") { event.preventDefault(); redo(); return; }
    if (mod && event.key.toLowerCase() === "z") { event.preventDefault(); undo(); return; }
    if (mod && event.key.toLowerCase() === "y") { event.preventDefault(); redo(); return; }
    if (event.key === " " && !mod) { event.preventDefault(); void togglePlayback(); return; }
    if (event.key === "Delete" || event.key === "Backspace") { event.preventDefault(); deleteSelected(); return; }
    if (event.key === "ArrowLeft") { event.preventDefault(); seekTo(currentTimeMs - (event.shiftKey ? 5_000 : 1_000)); }
    if (event.key === "ArrowRight") { event.preventDefault(); seekTo(currentTimeMs + (event.shiftKey ? 5_000 : 1_000)); }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<main class="editor-window">
  <EditorHeader onClose={() => void saveOriginal()} />
  <div class="workspace">
    <VideoPreview bind:videoElement source={videoSource} durationMs={timelineDurationMs} {currentTimeMs} {playing} volume={settings.volume} muted={settings.muted} {error} {canvasStyle} {videoStyle}
      onMetadata={onVideoMetadata} onTimeUpdate={onVideoTimeUpdate} onPlay={() => playing = true} onPause={() => playing = false} onSeek={seekTo} onTogglePlayback={() => void togglePlayback()}
      onPrevious={previousClip} onNext={nextClip} onVolume={(volume) => updateSettings({ volume })} onToggleMute={() => updateSettings({ muted: !settings.muted })} />
    <PropertyPanel {settings} onChange={updateSettings} />
  </div>
  <RecordingTimelineEditor durationMs={timelineDurationMs} {sourceDurationMs} {currentTimeMs} {audioWaveform} {videoThumbnails} {segments} {selectedSegmentId} speed={settings.speed} {canUndo} {canRedo} {canSplit}
    onSeek={seekTo} onSelect={(id) => selectedSegmentId = id} onSplit={splitAtPlayhead} onDelete={deleteSelected} onTrim={trimSelected} onUndo={undo} onRedo={redo} />
  <EditorFooter width={sourceWidth} height={sourceHeight} fps={review?.fps ?? 0} durationMs={timelineDurationMs} {pending} onCancel={() => void saveOriginal()} onExport={() => exportOpen = true} />
  {#if loading}<div class="loading" role="status"><span></span>Loading recording…</div>{/if}
</main>
<ExportDialog open={exportOpen} {pending} progress={exportProgress} {error} {segments} {settings} onCancel={() => { if (!pending) exportOpen = false; }} onCancelExport={cancelExport} onExport={exportEdited} />

<style>
  :global(html), :global(body) { width:100%; height:100%; margin:0; overflow:hidden; background:#101116; color:#f5f5f7; }
  .editor-window { position:relative; display:grid; width:100%; height:100%; grid-template-rows:56px minmax(280px,1fr) 228px 58px; overflow:hidden; background:#101116; font-family:inherit; }
  .workspace { display:grid; min-width:0; min-height:0; grid-template-columns:minmax(0,1fr) 320px; overflow:hidden; }
  .loading { position:absolute; z-index:10; inset:56px 0 0; display:flex; align-items:center; justify-content:center; gap:10px; background:rgb(16 17 22 / 70%); color:#c5c6cd; font-size:12px; pointer-events:none; }
  .loading span { width:14px; height:14px; border:2px solid #30313a; border-top-color:#ff3038; border-radius:50%; animation:spin .7s linear infinite; }
  @keyframes spin { to { transform:rotate(360deg); } }
  @media (max-width:1000px) { .workspace { grid-template-columns:minmax(0,1fr) 280px; } }
  @media (max-height:680px) { .editor-window { grid-template-rows:48px minmax(220px,1fr) 198px 52px; } }
</style>
