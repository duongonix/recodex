<script lang="ts">
  import { onMount } from "svelte";
  import ImageIcon from "@lucide/svelte/icons/image";
  import type { ScreenshotEditorState } from "../state/editor-state.svelte.js";

  let { editor }: { editor: ScreenshotEditorState } = $props();
  let thumbnail: HTMLCanvasElement;

  onMount(() => {
    let frame = 0;
    const draw = () => {
      const image = editor.image;
      if (!image || !thumbnail) return;
      thumbnail.width = 72;
      thumbnail.height = 48;
      const context = thumbnail.getContext("2d");
      if (!context) return;
      context.clearRect(0, 0, thumbnail.width, thumbnail.height);
      const scale = Math.min(thumbnail.width / image.width, thumbnail.height / image.height);
      const width = image.width * scale;
      const height = image.height * scale;
      context.drawImage(image, (thumbnail.width - width) / 2, (thumbnail.height - height) / 2, width, height);
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    const image = editor.image;
    if (thumbnail && image) {
      const context = thumbnail.getContext("2d");
      if (!context) return;
      const scale = Math.min(72 / image.width, 48 / image.height);
      const width = image.width * scale;
      const height = image.height * scale;
      thumbnail.width = 72;
      thumbnail.height = 48;
      context.clearRect(0, 0, 72, 48);
      context.drawImage(image, (72 - width) / 2, (48 - height) / 2, width, height);
    }
  });
</script>

<footer class="screenshot-strip">
  <div class="thumbnail active" aria-label="Current screenshot, selected">
    <canvas bind:this={thumbnail} aria-hidden="true"></canvas>
  </div>
  <div class="thumbnail-info">
    <div class="filename"><ImageIcon size={15} aria-hidden="true" /><strong>{editor.metadata?.suggestedFilename ?? "Screenshot"}</strong></div>
    <span>{editor.metadata?.width ?? "—"} × {editor.metadata?.height ?? "—"} px <i></i> {editor.metadata?.format.toUpperCase() ?? "PNG"}</span>
  </div>
  <div class="strip-status" role="status">
    {#if editor.error}<span class="status-error">{editor.error}</span>
    {:else if editor.success}<span>{editor.success}</span>
    {:else if editor.dirty}<span class="dirty-dot"></span><span>Unsaved changes</span>
    {:else}<span>All changes saved</span>{/if}
  </div>
</footer>

<style>
  .screenshot-strip { display:flex; align-items:center; gap:15px; min-height:84px; padding:10px 20px; border-top:1px solid rgba(255,255,255,.08); background:#111319; }
  .thumbnail { display:grid; width:78px; height:54px; flex:none; place-items:center; border:1px solid #383b45; border-radius:8px; background:#17191f; overflow:hidden; }
  .thumbnail.active { border:1px solid #ff3038; box-shadow:0 0 0 2px rgba(255,48,56,.16); }
  canvas { display:block; max-width:72px; max-height:48px; }
  .thumbnail-info { display:flex; flex-direction:column; gap:7px; min-width:0; color:#a1a4af; font-size:11px; }
  .filename { display:flex; align-items:center; gap:7px; max-width:360px; color:#f5f5f7; }
  .filename strong { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-size:12px; font-weight:500; }
  .thumbnail-info > span { display:flex; align-items:center; gap:8px; }
  .thumbnail-info i { width:3px; height:3px; border-radius:50%; background:#676b76; }
  .strip-status { display:flex; align-items:center; gap:8px; margin-left:auto; color:#a1a4af; font-size:11px; }
  .dirty-dot { width:7px; height:7px; border-radius:50%; background:#ff9f43; }
  .status-error { overflow:hidden; max-width:440px; color:#ff747b; text-overflow:ellipsis; white-space:nowrap; }
</style>
