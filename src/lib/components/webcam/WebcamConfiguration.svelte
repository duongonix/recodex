<script lang="ts">
  import { onMount } from "svelte";
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import { Switch } from "../ui/switch/index.js";
  import CameraSelector from "./CameraSelector.svelte";
  import WebcamSizeControl from "./WebcamSizeControl.svelte";
  import ShapeSelector from "./ShapeSelector.svelte";
  import BorderControls from "./BorderControls.svelte";
  import BackgroundControls from "./BackgroundControls.svelte";
  import WebcamPreview from "./WebcamPreview.svelte";
  import { getWebcamBackgroundPreview, listWebcamDevices, startWebcamPreview, stopWebcamPreview, watchWebcamPreview } from "../../services/ipc/webcam.js";
  import { DEFAULT_WEBCAM_SETTINGS, type WebcamSettings } from "../../types/settings.js";
  import type { WebcamDevice, WebcamPreviewFrame } from "../../types/webcam.js";

  let { settings, saving = false, captureActive = false, onBack, onChange }: {
    settings: WebcamSettings;
    saving?: boolean;
    captureActive?: boolean;
    onBack: () => void;
    onChange: (settings: WebcamSettings) => Promise<boolean>;
  } = $props();

  let overrides = $state<Partial<WebcamSettings>>({});
  let local = $derived<WebcamSettings>({ ...DEFAULT_WEBCAM_SETTINGS, ...settings, ...overrides });
  let devices = $state<WebcamDevice[]>([]);
  let cameraLoading = $state(true);
  let cameraError = $state<string | null>(null);
  let previewError = $state<string | null>(null);
  let previewFrame = $state<WebcamPreviewFrame | null>(null);
  let backgroundImagePreview = $state<string | null>(null);
  let backgroundImageError = $state<string | null>(null);
  let previewStarting = $state(false);
  let previewActive = $state(false);
  let previewCameraId: string | null = null;
  let persistTimer: ReturnType<typeof setTimeout> | undefined;
  let unlistenPreview: (() => void) | undefined;
  let disposed = false;

  let revision = 0;

  onMount(() => {
    void initializeCamera();
    return () => {
      disposed = true;
      unlistenPreview?.();
      if (persistTimer) {
        clearTimeout(persistTimer);
        void onChange(local);
      }
      if (!local.enabled) {
        void stopWebcamPreview().catch((error: unknown) => console.error("[RecoDex] Could not stop camera preview:", error));
      }
    };
  });

  async function initializeCamera() {
    try {
      unlistenPreview = await watchWebcamPreview(
        (frame) => { previewFrame = frame; previewError = null; previewActive = true; previewStarting = false; },
        (message) => { previewError = classifyCameraError(message); previewActive = false; previewFrame = null; }
      );
      if (local.backgroundMode === "image" && local.backgroundImagePath) {
        await loadBackgroundPreview(local.backgroundImagePath);
      }
      const available = await listWebcamDevices();
      if (disposed) return;
      devices = available;
      cameraError = available.length ? null : "No camera detected.";
      cameraLoading = false;
      if (local.enabled && local.cameraId && available.some((device) => device.id === local.cameraId)) {
        await beginPreview(local.cameraId);
      } else if (local.cameraId && !available.some((device) => device.id === local.cameraId)) {
        cameraError = "The selected camera is disconnected. Connect it or choose another camera.";
      }
    } catch (error) {
      cameraLoading = false;
      cameraError = classifyCameraError(errorMessage(error));
    }
  }

  function update(patch: Partial<WebcamSettings>, immediate = false) {
    overrides = { ...overrides, ...patch };
    revision += 1;
    if ("backgroundImagePath" in patch) {
      backgroundImagePreview = null;
      backgroundImageError = null;
      if (patch.backgroundImagePath) void loadBackgroundPreview(patch.backgroundImagePath);
    }
    if (persistTimer) clearTimeout(persistTimer);
    if (immediate) {
      void persist();
      return;
    }
    persistTimer = setTimeout(() => { void persist(); }, 180);
  }

  async function loadBackgroundPreview(path: string) {
    backgroundImageError = null;
    try {
      const preview = await getWebcamBackgroundPreview(path);
      if (local.backgroundImagePath === path && !disposed) backgroundImagePreview = preview;
    } catch (error) {
      if (local.backgroundImagePath === path && !disposed) backgroundImageError = errorMessage(error);
    }
  }

  async function persist(): Promise<boolean> {
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = undefined;
    const currentRevision = revision;
    const saved = await onChange({ ...local });
    if (revision === currentRevision) overrides = {};
    return saved;
  }

  async function setEnabled(enabled: boolean) {
    if (captureActive || saving) return;
    const patch = enabled
      ? { enabled, position: "bottom-right" as const, x: 1, y: 1 }
      : { enabled };
    overrides = { ...overrides, ...patch };
    revision += 1;
    if (persistTimer) clearTimeout(persistTimer);
    const saved = await persist();
    if (!saved) return;
    previewActive = false;
    previewStarting = enabled && Boolean(local.cameraId);
    if (!enabled) previewFrame = null;
  }

  async function selectCamera(cameraId: string | null) {
    update({ cameraId }, true);
    previewError = null;
    previewFrame = null;
    if (local.enabled) {
      previewStarting = Boolean(cameraId);
    } else {
      await endPreview();
    }
  }

  async function beginPreview(cameraId: string) {
    if (previewStarting || previewActive) {
      if (previewActive && previewCameraId === cameraId && !previewError) return;
      await endPreview();
    }
    previewStarting = true;
    previewError = null;
    try {
      await startWebcamPreview(cameraId);
      previewActive = true;
      previewCameraId = cameraId;
    } catch (error) {
      previewError = classifyCameraError(errorMessage(error));
      previewActive = false;
    } finally {
      previewStarting = false;
    }
  }

  async function endPreview() {
    previewActive = false;
    previewCameraId = null;
    previewFrame = null;
    try {
      await stopWebcamPreview();
    } catch (error) {
      console.error("[RecoDex] Could not release webcam preview:", error);
    }
  }

  async function refreshCameras() {
    cameraLoading = true;
    cameraError = null;
    try {
      devices = await listWebcamDevices();
      cameraError = devices.length ? null : "No camera detected.";
    } catch (error) {
      cameraError = classifyCameraError(errorMessage(error));
    } finally {
      cameraLoading = false;
    }
  }

  function classifyCameraError(message: string) {
    const lower = message.toLowerCase();
    if (lower.includes("permission") || lower.includes("denied") || lower.includes("access is denied")) return `Camera permission denied: ${message}`;
    if (lower.includes("disconnect") || lower.includes("not available") || lower.includes("not found")) return `Camera unavailable: ${message}`;
    return message;
  }

  function errorMessage(error: unknown): string {
    if (typeof error === "string") return error;
    if (error && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
    return "Camera request failed.";
  }
</script>

<section class="webcam-settings-page" aria-label="Webcam Overlay settings">
  <header class="webcam-page-header">
    <button type="button" class="webcam-back-button" onclick={onBack} aria-label="Back to Snap settings"><ArrowLeft aria-hidden="true" class="size-4" /></button>
    <div class="webcam-page-heading-copy">
      <h2>Webcam Overlay</h2>
      <p>Display webcam while recording</p>
    </div>
    <Switch checked={local.enabled} disabled={saving || captureActive || (!local.cameraId && !local.enabled)} aria-label="Enable Webcam Overlay" title={!local.cameraId ? "Choose a camera below first." : undefined} onCheckedChange={(enabled) => { void setEnabled(enabled); }} />
  </header>

  <div class="webcam-settings-body">
    <CameraSelector
      devices={devices}
      loading={cameraLoading}
      selected={local.cameraId}
      status={previewError ?? cameraError}
      onChange={(id) => { void selectCamera(id); }}
      onRefresh={() => { void refreshCameras(); }}
    />
    {#if local.enabled}
      <p class="webcam-status" role="status">
        {previewActive ? "Webcam overlay is visible on your desktop." : previewError ?? "Opening webcam overlay…"}
      </p>
    {:else if local.cameraId && !previewActive && !previewStarting}
      <button type="button" class="webcam-preview-action" disabled={cameraLoading} onclick={() => { void beginPreview(local.cameraId!); }}>Start camera preview</button>
    {:else if previewStarting}
      <p class="webcam-status" role="status">Opening camera preview…</p>
    {:else if previewActive}
      <button type="button" class="webcam-preview-action webcam-preview-stop" onclick={() => { void endPreview(); }}>Stop camera preview</button>
    {/if}

    <WebcamPreview
      settings={local}
      frame={previewFrame}
      {backgroundImagePreview}
      previewMessage={previewError ?? (local.cameraId ? "Camera preview is off." : cameraError)}
      onChange={(patch) => update(patch, true)}
    />
    <WebcamSizeControl value={local.width} onInput={(width) => update({ width })} />
    <ShapeSelector value={local.shape} onChange={(shape) => update({ shape }, true)} />
    {#if local.shape === "rectangle"}
      <section class="webcam-control-section" aria-labelledby="webcam-radius-heading">
        <div class="webcam-control-heading"><h3 id="webcam-radius-heading" class="webcam-section-title">Corner radius</h3><output for="webcam-radius" class="webcam-value">{local.cornerRadius}px</output></div>
        <input id="webcam-radius" class="webcam-range" type="range" min="0" max="50" step="1" value={local.cornerRadius} oninput={(event) => update({ cornerRadius: Number(event.currentTarget.value) })} />
      </section>
    {/if}
    <BorderControls
      enabled={local.borderEnabled}
      color={local.borderColor}
      width={local.borderWidth}
      onChange={(patch) => update(patch, true)}
    />
    <BackgroundControls
      mode={local.backgroundMode}
      color={local.backgroundColor}
      imagePath={local.backgroundImagePath}
      imageError={backgroundImageError}
      onChange={(patch) => update(patch, true)}
    />
    <section class="webcam-control-section" aria-labelledby="webcam-opacity-heading">
      <div class="webcam-control-heading"><h3 id="webcam-opacity-heading" class="webcam-section-title">Opacity</h3><output for="webcam-opacity" class="webcam-value">{local.opacity}%</output></div>
      <input id="webcam-opacity" class="webcam-range" type="range" min="0" max="100" step="1" value={local.opacity} oninput={(event) => update({ opacity: Number(event.currentTarget.value) })} />
    </section>
    <section class="webcam-control-section webcam-mirror-row" aria-label="Mirror webcam preview">
      <div><h3 class="webcam-section-title">Mirror</h3><p class="webcam-hint">Flip webcam horizontally in the recording.</p></div>
      <Switch checked={local.mirror} aria-label="Mirror (flip horizontally)" onCheckedChange={(mirror) => update({ mirror }, true)} />
    </section>
  </div>
</section>
