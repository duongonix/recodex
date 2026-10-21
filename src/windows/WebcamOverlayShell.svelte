<script lang="ts">
  import { onMount } from "svelte";
  import type { WebcamPreviewFrame } from "../lib/types/webcam.js";
  import { DEFAULT_WEBCAM_SETTINGS, type WebcamSettings } from "../lib/types/settings.js";
  import {
    getWebcamBackgroundPreview,
    getWebcamOverlaySettings,
    watchWebcamOverlaySettings,
    watchWebcamPreview
  } from "../lib/services/ipc/webcam.js";

  let settings = $state<WebcamSettings>({ ...DEFAULT_WEBCAM_SETTINGS, enabled: false });
  let frame = $state<WebcamPreviewFrame | null>(null);
  let cameraError = $state<string | null>(null);
  let backgroundPreview = $state<string | null>(null);
  let disposed = false;
  let settingsRevision = 0;
  let unlistenPreview: (() => void) | undefined;
  let unlistenSettings: (() => void) | undefined;

  onMount(() => {
    void subscribe();
    return () => {
      disposed = true;
      unlistenPreview?.();
      unlistenSettings?.();
    };
  });

  async function subscribe() {
    try {
      unlistenPreview = await watchWebcamPreview(
        (next) => { frame = next; cameraError = null; },
        (message) => { frame = null; cameraError = message; }
      );
      unlistenSettings = await watchWebcamOverlaySettings(applySettings);
      const revisionAtRequest = settingsRevision;
      const current = await getWebcamOverlaySettings();
      if (revisionAtRequest === settingsRevision) applySettings(current);
    } catch (error) {
      if (!disposed) cameraError = errorMessage(error);
    }

    if (disposed) {
      unlistenPreview?.();
      unlistenSettings?.();
    }
  }

  function applySettings(next: WebcamSettings) {
    settings = next;
    const revision = ++settingsRevision;
    backgroundPreview = null;
    if (next.backgroundMode === "image" && next.backgroundImagePath) {
      void getWebcamBackgroundPreview(next.backgroundImagePath)
        .then((preview) => {
          if (!disposed && revision === settingsRevision) backgroundPreview = preview;
        })
        .catch((error: unknown) => {
          if (!disposed && revision === settingsRevision) cameraError = errorMessage(error);
        });
    }
  }

  function errorMessage(error: unknown): string {
    if (typeof error === "string") return error;
    if (error && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
    return "Webcam overlay could not be initialized.";
  }

  const radius = $derived(settings.shape === "circle" ? "50%" : `${settings.cornerRadius}px`);
  const background = $derived(settings.backgroundMode === "color" ? settings.backgroundColor : "transparent");
  const backgroundImage = $derived(settings.backgroundMode === "image" && backgroundPreview
    ? `url("${backgroundPreview}")`
    : "none");
  const foregroundInset = $derived(settings.backgroundMode === "none" ? "0%" : "5%");
</script>

<main class="overlay-shell" aria-label="Webcam desktop overlay">
  {#if settings.enabled}
    <div
      class="overlay-frame"
      class:circle={settings.shape === "circle"}
      style:border-radius={radius}
      style:background-color={background}
      style:background-image={backgroundImage}
      style:opacity={settings.opacity / 100}
      style:box-shadow={settings.borderEnabled && settings.borderWidth > 0
        ? `inset 0 0 0 ${settings.borderWidth}px ${settings.borderColor}`
        : "none"}
    >
      {#if frame}
        {#if settings.backgroundMode === "blur"}
          <img class="camera-backdrop" src={frame.dataUrl} alt="" aria-hidden="true" draggable="false" />
        {/if}
        <img
          class="camera-frame"
          class:camera-frame-inset={settings.backgroundMode !== "none"}
          src={frame.dataUrl}
          alt=""
          aria-hidden="true"
          draggable="false"
          style:inset={foregroundInset}
          style:transform={settings.mirror ? "scaleX(-1)" : "none"}
        />
      {:else if cameraError}
        <div class="camera-error" role="status">Webcam unavailable</div>
      {/if}
    </div>
  {/if}
</main>

<style>
  :global(html), :global(body) {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    margin: 0;
    overflow: hidden;
    background: transparent;
  }

  .overlay-shell {
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: transparent;
  }

  .overlay-frame {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-position: center;
    background-size: cover;
  }

  .overlay-frame.circle { border-radius: 50%; }
  .camera-frame, .camera-backdrop { position: absolute; width: 100%; height: 100%; object-fit: cover; }
  .camera-backdrop { inset: -7%; width: 114%; height: 114%; filter: blur(12px); }
  .camera-frame-inset { width: 90%; height: 90%; border-radius: inherit; }
  .camera-error {
    display: grid;
    width: 100%;
    height: 100%;
    place-items: center;
    color: #f5f5f7;
    background: #17191f;
    font: 12px system-ui, sans-serif;
  }
</style>
