<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { WebcamBackgroundMode } from "../../types/settings.js";

  let { mode, color, imagePath, imageError = null, onChange }: {
    mode: WebcamBackgroundMode;
    color: string;
    imagePath: string | null;
    imageError?: string | null;
    onChange: (patch: { backgroundMode?: WebcamBackgroundMode; backgroundColor?: string; backgroundImagePath?: string | null }) => void;
  } = $props();

  async function chooseImage() {
    try {
      const result = await open({
        multiple: false,
        filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg"] }]
      });
      if (typeof result === "string") onChange({ backgroundImagePath: result, backgroundMode: "image" });
    } catch (error) {
      console.error("[RecoDex] Could not choose webcam background image:", error);
    }
  }
</script>

<section class="webcam-control-section" aria-labelledby="webcam-background-heading">
  <h3 id="webcam-background-heading" class="webcam-section-title">Background</h3>
  <select class="webcam-select" aria-label="Webcam background mode" value={mode} onchange={(event) => onChange({ backgroundMode: event.currentTarget.value as WebcamBackgroundMode })}>
    <option value="none">None</option>
    <option value="blur">Blur</option>
    <option value="color">Color</option>
    <option value="image">Image</option>
  </select>
  {#if mode === "blur"}
    <p class="webcam-hint">Uses a blurred expansion of the camera frame. It does not remove the person from the background.</p>
  {:else if mode === "color"}
    <label class="webcam-color-picker-row"><span>Background color</span><input type="color" value={color} aria-label="Webcam background color" oninput={(event) => onChange({ backgroundColor: event.currentTarget.value.toUpperCase() })} /></label>
  {:else if mode === "image"}
    <button type="button" class="webcam-image-button" onclick={() => { void chooseImage(); }}>{imagePath ? "Change background image" : "Choose local image"}</button>
    {#if imagePath}<p class="webcam-image-path" title={imagePath}>{imagePath}</p>{/if}
    {#if imageError}<p class="webcam-status" role="alert">{imageError}</p>{/if}
  {/if}
</section>
