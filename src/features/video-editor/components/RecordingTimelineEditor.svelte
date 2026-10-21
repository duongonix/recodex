<script lang="ts">
  import MousePointer2 from "@lucide/svelte/icons/mouse-pointer-2";
  import Scissors from "@lucide/svelte/icons/scissors";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Crop from "@lucide/svelte/icons/crop";
  import Undo2 from "@lucide/svelte/icons/undo-2";
  import Redo2 from "@lucide/svelte/icons/redo-2";
  import Minus from "@lucide/svelte/icons/minus";
  import Plus from "@lucide/svelte/icons/plus";
  import Maximize from "@lucide/svelte/icons/maximize";
  import type { TimelineSegment } from "../types.js";

  type Tool = "select" | "split" | "trim";
  type Props = {
    durationMs: number;
    sourceDurationMs: number;
    currentTimeMs: number;
    audioWaveform: number[];
    videoThumbnails: string[];
    segments: TimelineSegment[];
    selectedSegmentId: string | null;
    speed: number;
    canUndo: boolean;
    canRedo: boolean;
    canSplit: boolean;
    onSeek: (time: number) => void;
    onSelect: (id: string) => void;
    onSplit: () => void;
    onDelete: () => void;
    onTrim: (id: string, edge: "start" | "end", time: number) => void;
    onUndo: () => void;
    onRedo: () => void;
  };
  let { durationMs, sourceDurationMs, currentTimeMs, audioWaveform, videoThumbnails, segments, selectedSegmentId, speed, canUndo, canRedo, canSplit, onSeek, onSelect, onSplit, onDelete, onTrim, onUndo, onRedo }: Props = $props();
  let tool = $state<Tool>("select");
  let zoom = $state(100);
  let viewport: HTMLDivElement | undefined = $state();
  let viewportWidth = $state(900);
  let dragging: { id: string; edge: "start" | "end"; pointerId: number } | null = null;

  const scale = $derived(8 * zoom / 100);
  const contentWidth = $derived(Math.max(viewportWidth, durationMs / 1000 * scale));
  const cursorX = $derived(durationMs > 0 ? currentTimeMs / durationMs * contentWidth : 0);
  const rulerStep = $derived(durationMs > 300_000 ? 60 : durationMs > 120_000 ? 30 : durationMs > 60_000 ? 15 : 10);
  const ticks = $derived(Array.from({ length: Math.floor(durationMs / (rulerStep * 1000)) + 1 }, (_, index) => index * rulerStep * 1000));

  $effect(() => {
    if (!viewport) return;
    const observer = new ResizeObserver(([entry]) => { viewportWidth = entry.contentRect.width; });
    observer.observe(viewport);
    return () => observer.disconnect();
  });

  function formatTime(milliseconds: number): string {
    const totalSeconds = Math.floor(Math.max(0, milliseconds) / 1000);
    return `${String(Math.floor(totalSeconds / 60)).padStart(2, "0")}:${String(totalSeconds % 60).padStart(2, "0")}`;
  }

  function timeAtClientX(clientX: number): number {
    if (!viewport || contentWidth <= 0) return 0;
    const rect = viewport.getBoundingClientRect();
    const x = clientX - rect.left + viewport.scrollLeft;
    return Math.max(0, Math.min(durationMs, x / contentWidth * durationMs));
  }

  function segmentOffset(id: string): number {
    let offset = 0;
    for (const segment of segments) {
      if (segment.id === id) return offset;
      offset += (segment.sourceEndMs - segment.sourceStartMs) / speed;
    }
    return offset;
  }

  function segmentLeft(id: string): number {
    return durationMs > 0 ? segmentOffset(id) / durationMs * contentWidth : 0;
  }

  function segmentWidth(segment: TimelineSegment): number {
    return durationMs > 0 ? (segment.sourceEndMs - segment.sourceStartMs) / speed / durationMs * contentWidth : 0;
  }

  function waveformFor(segment: TimelineSegment): number[] {
    if (!audioWaveform.length || sourceDurationMs <= 0) return [];
    const start = Math.floor(segment.sourceStartMs / sourceDurationMs * audioWaveform.length);
    const end = Math.max(start + 1, Math.ceil(segment.sourceEndMs / sourceDurationMs * audioWaveform.length));
    return audioWaveform.slice(start, end);
  }

  function thumbnailsFor(segment: TimelineSegment): string[] {
    if (!videoThumbnails.length || sourceDurationMs <= 0) return [];
    const start = Math.floor(segment.sourceStartMs / sourceDurationMs * videoThumbnails.length);
    const end = Math.max(start + 1, Math.ceil(segment.sourceEndMs / sourceDurationMs * videoThumbnails.length));
    const selected = videoThumbnails.slice(start, end);
    if (selected.length) return selected;
    return [videoThumbnails[Math.min(videoThumbnails.length - 1, start)]].filter(Boolean);
  }

  function trackPointerDown(event: PointerEvent) {
    if ((event.target as HTMLElement).closest("[data-trim-handle]")) return;
    const time = timeAtClientX(event.clientX);
    if (tool === "split") onSeek(time);
    else onSeek(time);
  }

  function handlePointerDown(event: PointerEvent, segment: TimelineSegment, edge: "start" | "end") {
    event.stopPropagation();
    dragging = { id: segment.id, edge, pointerId: event.pointerId };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent) {
    if (!dragging || dragging.pointerId !== event.pointerId) return;
    const timelinePosition = timeAtClientX(event.clientX);
    const segment = segments.find((item) => item.id === dragging?.id);
    if (!segment) return;
    const sourceOffset = segmentOffset(segment.id);
    onTrim(segment.id, dragging.edge, segment.sourceStartMs + (timelinePosition - sourceOffset) * speed);
  }

  function handlePointerUp() { dragging = null; }

  function zoomBy(delta: number) { zoom = Math.max(25, Math.min(400, zoom + delta)); }
  function fitTimeline() {
    if (durationMs <= 0 || viewportWidth <= 0) return;
    zoom = Math.max(25, Math.min(400, viewportWidth / (durationMs / 1000 * 0.08)));
  }

  function selectTool(next: Tool) {
    tool = next;
    if (next === "split") onSplit();
  }
</script>

<section class="timeline-editor" aria-label="Timeline editor">
  <div class="timeline-toolbar">
    <div class="tool-group" role="toolbar" aria-label="Timeline editing tools">
      <button class:active={tool === "select"} type="button" aria-pressed={tool === "select"} onclick={() => selectTool("select")}><MousePointer2 size={15} /> Select</button>
      <button class:active={tool === "split"} type="button" aria-pressed={tool === "split"} disabled={!canSplit} onclick={() => selectTool("split")}><Scissors size={15} /> Split</button>
      <button type="button" disabled={!selectedSegmentId} onclick={onDelete}><Trash2 size={15} /> Delete</button>
      <button class:active={tool === "trim"} type="button" aria-pressed={tool === "trim"} onclick={() => selectTool(tool === "trim" ? "select" : "trim")}><Crop size={15} /> Trim</button>
      <span class="toolbar-divider"></span>
      <button class="icon" type="button" aria-label="Undo" title="Undo (Ctrl+Z)" disabled={!canUndo} onclick={onUndo}><Undo2 size={16} /></button>
      <button class="icon" type="button" aria-label="Redo" title="Redo (Ctrl+Shift+Z)" disabled={!canRedo} onclick={onRedo}><Redo2 size={16} /></button>
    </div>
    <div class="zoom-group" aria-label="Timeline zoom">
      <button class="icon" aria-label="Zoom out" type="button" onclick={() => zoomBy(-25)}><Minus size={15} /></button>
      <input aria-label="Timeline zoom" type="range" min="25" max="400" step="5" bind:value={zoom} />
      <button class="icon" aria-label="Zoom in" type="button" onclick={() => zoomBy(25)}><Plus size={15} /></button>
      <button class="icon fit" aria-label="Fit timeline" title="Fit timeline" type="button" onclick={fitTimeline}><Maximize size={15} /></button>
    </div>
  </div>
  <div class="timeline-scroll" bind:this={viewport}>
    <div class="timeline-content" style={`width:${contentWidth}px`}>
      <div class="ruler" aria-label="Time ruler">
        {#each ticks as tick (tick)}
          <span class="tick" style={`left:${durationMs ? tick / durationMs * contentWidth : 0}px`}>{formatTime(tick)}<i></i></span>
        {/each}
      </div>
      <div class="tracks" role="group" aria-label="Timeline tracks" onpointerdown={trackPointerDown} onpointermove={handlePointerMove} onpointerup={handlePointerUp} onpointercancel={handlePointerUp}>
        <div class="track video-track" aria-label="Video track">
          {#each segments as segment, index (segment.id)}
            {@const left = segmentLeft(segment.id)}
            {@const width = segmentWidth(segment)}
            {@const thumbs = thumbnailsFor(segment)}
            <button class="video-clip" class:selected={selectedSegmentId === segment.id} type="button" aria-label={`Video segment ${index + 1}, ${formatTime(segment.sourceStartMs)} to ${formatTime(segment.sourceEndMs)}`} style={`left:${left}px;width:${width}px`} onclick={() => { onSelect(segment.id); onSeek(segmentOffset(segment.id)); }}>
              {#if thumbs.length}<span class="thumbnail-strip" aria-hidden="true">{#each thumbs as thumbnail, frame (frame)}<img src={`data:image/jpeg;base64,${thumbnail}`} alt="" draggable="false" />{/each}</span>{/if}
              <span class="clip-name">Video {index + 1}</span>
              <span class="clip-duration">{formatTime((segment.sourceEndMs - segment.sourceStartMs) / speed)}</span>
            </button>
            <button class="trim-handle start" data-trim-handle aria-label={`Trim start of segment ${index + 1}`} style={`left:${left}px`} onpointerdown={(event) => handlePointerDown(event, segment, "start")} onpointermove={handlePointerMove} onpointerup={handlePointerUp} onpointercancel={handlePointerUp}></button>
            <button class="trim-handle end" data-trim-handle aria-label={`Trim end of segment ${index + 1}`} style={`left:${left + width}px`} onpointerdown={(event) => handlePointerDown(event, segment, "end")} onpointermove={handlePointerMove} onpointerup={handlePointerUp} onpointercancel={handlePointerUp}></button>
          {/each}
        </div>
        <div class="track audio-track" aria-label="Linked audio track">
          {#each segments as segment, index (segment.id)}
            {@const wave = waveformFor(segment)}
            <div class="audio-clip" aria-label={wave.length ? `Audio remains linked to video segment ${index + 1}` : "No audio stream in this recording"} style={`left:${segmentLeft(segment.id)}px;width:${segmentWidth(segment)}px`}>
              {#if wave.length}
                <div class="waveform" aria-hidden="true">{#each wave as amplitude, bin (bin)}<i style={`height:${Math.max(2, Math.min(28, amplitude * 28))}px`}></i>{/each}</div>
                <span>Audio {index + 1} · linked</span>
              {:else}<span>No audio track</span>{/if}
            </div>
          {/each}
        </div>
        <div class="playhead" style={`left:${cursorX}px`} aria-hidden="true"><span>{formatTime(currentTimeMs)}</span></div>
      </div>
    </div>
  </div>
</section>

<style>
  .timeline-editor { display:grid; min-height:220px; grid-template-rows:52px minmax(0,1fr); border-top:1px solid #292b32; background:#101116; }
  .timeline-toolbar { display:flex; min-width:0; align-items:center; justify-content:space-between; gap:12px; padding:0 16px; border-bottom:1px solid #22242b; }
  .tool-group,.zoom-group { display:flex; align-items:center; gap:6px; }
  button { display:inline-flex; height:34px; align-items:center; justify-content:center; gap:7px; padding:0 10px; border:1px solid transparent; border-radius:8px; background:#1b1d24; color:#c5c6cd; font:inherit; font-size:11px; white-space:nowrap; cursor:pointer; }
  button:hover:not(:disabled) { border-color:#42444e; color:#fff; background:#252730; }
  button.active { border-color:rgb(255 48 56 / 75%); background:rgb(255 48 56 / 16%); color:#ff696f; }
  button:disabled { cursor:not-allowed; opacity:.4; }
  button.icon { width:32px; padding:0; }
  button.fit { margin-left:2px; }
  .toolbar-divider { width:1px; height:22px; margin:0 3px; background:#30313a; }
  .zoom-group input { width:100px; accent-color:#ff3038; }
  .timeline-scroll { min-width:0; overflow:auto hidden; scrollbar-color:#454751 #17191f; scrollbar-width:thin; }
  .timeline-content { position:relative; min-width:100%; min-height:160px; padding:0 14px 14px; box-sizing:border-box; }
  .ruler { position:relative; height:34px; border-bottom:1px solid #292b32; color:#898b97; font-size:10px; font-variant-numeric:tabular-nums; }
  .tick { position:absolute; top:9px; transform:translateX(-50%); white-space:nowrap; }
  .tick:first-child { transform:none; }
  .tick i { position:absolute; top:16px; left:50%; width:1px; height:9px; background:#383a44; }
  .tracks { position:relative; display:grid; gap:7px; padding-top:12px; touch-action:none; }
  .track { position:relative; height:46px; box-sizing:border-box; }
  .video-track { border-radius:8px; background:#17191f; }
  .video-clip { position:absolute; top:2px; height:42px; overflow:hidden; justify-content:space-between; border:1px solid rgb(255 48 56 / 80%); border-radius:5px; background:#381b20; color:#f5f5f7; }
  .thumbnail-strip { position:absolute; inset:0; display:flex; overflow:hidden; }
  .thumbnail-strip img { min-width:0; flex:1 1 0; object-fit:cover; opacity:.78; }
  .clip-name,.clip-duration { position:relative; z-index:1; padding:2px 3px; border-radius:3px; background:rgb(16 17 22 / 72%); }
  .video-clip.selected { box-shadow:0 0 0 1px #ff3038; }
  .clip-name,.clip-duration { overflow:hidden; font-size:10px; text-overflow:ellipsis; }
  .trim-handle { position:absolute; z-index:4; top:3px; width:9px; height:40px; padding:0; transform:translateX(-50%); border:0; border-radius:3px; background:#ff3038; touch-action:none; cursor:ew-resize; }
  .trim-handle:hover { background:#ff7379; }
  .audio-track { border:1px solid rgb(255 48 56 / 58%); border-radius:8px; background:#251519; }
  .audio-clip { position:absolute; top:0; bottom:0; overflow:hidden; border-right:1px solid #713139; }
  .waveform { position:absolute; inset:2px 6px; display:flex; align-items:center; justify-content:space-between; gap:1px; opacity:.68; pointer-events:none; }
  .waveform i { width:2px; min-width:1px; border-radius:2px; background:#ff4c54; }
  .audio-clip span { position:relative; display:block; padding:14px 10px; color:#e56a70; font-size:10px; text-shadow:0 1px 4px #101116; white-space:nowrap; pointer-events:none; }
  .playhead { position:absolute; z-index:5; top:-10px; bottom:-4px; width:2px; background:#ff3038; pointer-events:none; }
  .playhead span { position:absolute; top:-1px; left:50%; transform:translateX(-50%); padding:3px 6px; border-radius:5px; background:#ff3038; color:white; font-size:9px; font-variant-numeric:tabular-nums; white-space:nowrap; }
  @media (max-width:740px) { .timeline-toolbar { align-items:flex-start; flex-direction:column; height:auto; padding:8px; } .timeline-editor { grid-template-rows:auto minmax(0,1fr); } .zoom-group input { width:70px; } }
</style>
