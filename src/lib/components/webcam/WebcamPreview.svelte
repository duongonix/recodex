<script lang="ts">
  import { onMount } from "svelte";
  import type { WebcamPreviewFrame } from "../../types/webcam.js";
  import type { WebcamSettings } from "../../types/settings.js";

  let { settings, frame, backgroundImagePreview, previewMessage, onChange }: {
    settings: WebcamSettings;
    frame: WebcamPreviewFrame | null;
    backgroundImagePreview: string | null;
    previewMessage: string | null;
    onChange: (patch: Partial<Pick<WebcamSettings, "position" | "x" | "y" | "width">>) => void;
  } = $props();

  type Geometry = { left: number; top: number; width: number; height: number; maxX: number; maxY: number };
  type DragState = {
    pointerId: number;
    clientX: number;
    clientY: number;
    width: number;
    left: number;
    top: number;
    boxWidth: number;
    boxHeight: number;
    resizeX: -1 | 0 | 1;
    resizeY: -1 | 0 | 1;
  };
  let stage = $state<HTMLDivElement>();
  let stageSize = $state({ width: 320, height: 180 });
  let dragState = $state<DragState | null>(null);
  let draft = $state<{ width: number } | null>(null);

  onMount(() => {
    if (!stage) return;
    const observer = new ResizeObserver(([entry]) => {
      stageSize = { width: entry.contentRect.width, height: entry.contentRect.height };
    });
    observer.observe(stage);
    return () => observer.disconnect();
  });

  const geometry = $derived.by((): Geometry => {
    const width = ((draft?.width ?? settings.width) * stageSize.width) / 1920;
    const ratio = frame ? frame.width / Math.max(1, frame.height) : 4 / 3;
    const height = settings.shape === "circle" ? width : width / ratio;
    const maxX = Math.max(0, stageSize.width - width);
    const maxY = Math.max(0, stageSize.height - height);
    const marginX = (24 * stageSize.width) / 1920;
    const marginY = (24 * stageSize.height) / 1080;
    const left = Math.max(0, maxX - marginX);
    const top = Math.max(0, maxY - marginY);
    return { left, top, width, height, maxX, maxY };
  });

  function begin(event: PointerEvent, resizeX: -1 | 0 | 1, resizeY: -1 | 0 | 1) {
    if (!stage) return;
    event.preventDefault();
    event.stopPropagation();
    draft = { width: settings.width };
    dragState = {
      pointerId: event.pointerId, clientX: event.clientX, clientY: event.clientY,
      width: settings.width,
      left: geometry.left, top: geometry.top, boxWidth: geometry.width, boxHeight: geometry.height,
      resizeX, resizeY
    };
    stage.setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent) {
    if (!dragState || event.pointerId !== dragState.pointerId) return;
    const deltaX = event.clientX - dragState.clientX;
    const deltaY = event.clientY - dragState.clientY;
    const aspect = dragState.boxWidth / Math.max(1, dragState.boxHeight);
    const horizontal = deltaX * dragState.resizeX;
    const vertical = deltaY * dragState.resizeY * aspect;
    const delta = Math.abs(horizontal) >= Math.abs(vertical) ? horizontal : vertical;
    const width = Math.min(640, Math.max(80, dragState.width + (delta * 1920) / Math.max(1, stageSize.width)));
    draft = { width };
  }

  function finish(event: PointerEvent) {
    if (!dragState || event.pointerId !== dragState.pointerId || !draft) return;
    onChange({ width: Math.round(draft.width / 10) * 10 });
    dragState = null;
    draft = null;
  }

  const radius = $derived(settings.shape === "circle" ? "50%" : `${Math.min(50, settings.cornerRadius) * stageSize.width / 1920}px`);
  const foregroundInset = $derived(settings.backgroundMode === "none" ? "0%" : "5%");
</script>

<section class="webcam-control-section" aria-labelledby="webcam-preview-heading">
  <div class="webcam-control-heading">
    <h3 id="webcam-preview-heading" class="webcam-section-title">Preview</h3>
    <span class="webcam-preview-live" class:webcam-preview-connected={frame !== null}>{frame ? "LIVE" : "PREVIEW"}</span>
  </div>
  <div class="webcam-preview-stage" bind:this={stage} role="group" aria-label="Webcam preview. Use the handles to resize the camera overlay." onpointermove={move} onpointerup={finish} onpointercancel={finish}>
    <div class="webcam-preview-screen-label">1920 × 1080</div>
    {#if previewMessage && !frame}
      <div class="webcam-preview-message" role="status">{previewMessage}</div>
    {:else if !frame}
      <div class="webcam-preview-message" role="status">Select a camera and start preview to see it here.</div>
    {/if}
    <div
      class="webcam-preview-overlay"
      class:webcam-preview-circle={settings.shape === "circle"}
      class:webcam-preview-selected={frame !== null}
      style:left={`${geometry.left}px`}
      style:top={`${geometry.top}px`}
      style:width={`${geometry.width}px`}
      style:height={`${geometry.height}px`}
      style:border-radius={radius}
      style:opacity={settings.opacity / 100}
      style:background-color={settings.backgroundMode === "color" ? settings.backgroundColor : "transparent"}
      style:background-image={settings.backgroundMode === "image" && backgroundImagePreview ? `url("${backgroundImagePreview}")` : "none"}
      style:background-size="cover"
      aria-label="Webcam preview positioned at bottom right"
    >
      {#if frame}
        {#if settings.backgroundMode === "blur"}
          <img class="webcam-preview-backdrop" src={frame.dataUrl} alt="" aria-hidden="true" draggable="false" />
        {/if}
        <img
          class="webcam-preview-camera"
          class:webcam-preview-camera-inset={settings.backgroundMode !== "none"}
          src={frame.dataUrl}
          alt="Live webcam preview"
          draggable="false"
          style:inset={foregroundInset}
          style:transform={settings.mirror ? "scaleX(-1)" : "none"}
        />
      {:else}
        <span class="webcam-preview-placeholder" aria-hidden="true"></span>
      {/if}
      {#if settings.borderEnabled && settings.borderWidth > 0}
        <span class="webcam-preview-border" style:border-color={settings.borderColor} style:border-width={`${settings.borderWidth * stageSize.height / 1080}px`} aria-hidden="true"></span>
      {/if}
      {#if frame}
        <button class="webcam-resize-handle webcam-resize-top-left" aria-label="Resize webcam overlay from top left" onpointerdown={(event) => begin(event, -1, -1)}></button>
        <button class="webcam-resize-handle webcam-resize-top" aria-label="Resize webcam overlay from top" onpointerdown={(event) => begin(event, 0, -1)}></button>
        <button class="webcam-resize-handle webcam-resize-top-right" aria-label="Resize webcam overlay from top right" onpointerdown={(event) => begin(event, 1, -1)}></button>
        <button class="webcam-resize-handle webcam-resize-right" aria-label="Resize webcam overlay from right" onpointerdown={(event) => begin(event, 1, 0)}></button>
        <button class="webcam-resize-handle webcam-resize-bottom-right" aria-label="Resize webcam overlay from bottom right" onpointerdown={(event) => begin(event, 1, 1)}></button>
        <button class="webcam-resize-handle webcam-resize-bottom" aria-label="Resize webcam overlay from bottom" onpointerdown={(event) => begin(event, 0, 1)}></button>
        <button class="webcam-resize-handle webcam-resize-bottom-left" aria-label="Resize webcam overlay from bottom left" onpointerdown={(event) => begin(event, -1, 1)}></button>
        <button class="webcam-resize-handle webcam-resize-left" aria-label="Resize webcam overlay from left" onpointerdown={(event) => begin(event, -1, 0)}></button>
      {/if}
    </div>
  </div>
</section>
