<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import Download from "@lucide/svelte/icons/download";
  import CircleStop from "@lucide/svelte/icons/circle-stop";
  import X from "@lucide/svelte/icons/x";
  import type { EditorSettings, TimelineSegment } from "../types.js";
  import { errorMessage } from "../../../lib/services/ipc/settings.js";

  type Props = {
    open: boolean;
    pending: boolean;
    progress: number;
    error: string | null;
    segments: TimelineSegment[];
    settings: EditorSettings;
    onCancel: () => void;
    onCancelExport: () => Promise<void>;
    onExport: (path: string) => Promise<void>;
  };
  let { open, pending, progress, error, segments, settings, onCancel, onCancelExport, onExport }: Props = $props();
  let choosingPath = $state(false);
  let cancelRequested = $state(false);
  let dialogError = $state<string | null>(null);
  $effect(() => { if (!pending) cancelRequested = false; });

  async function exportFile() {
    if (pending || choosingPath || !segments.length) return;
    choosingPath = true;
    dialogError = null;
    try {
      const path = await save({
        title: "Export video",
        defaultPath: "RecoDex-Edited.mp4",
        filters: [{ name: "MP4 Video", extensions: ["mp4"] }]
      });
      if (path) await onExport(path);
    } catch (cause) {
      console.error("[RecoDex][video-editor] Could not open the MP4 save dialog", cause);
      dialogError = errorMessage(cause);
    } finally {
      choosingPath = false;
    }
  }

  async function requestCancelExport() {
    if (!pending || cancelRequested) return;
    cancelRequested = true;
    try { await onCancelExport(); }
    catch { cancelRequested = false; }
  }
</script>

{#if open}
  <div class="backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget && !pending) onCancel(); }}>
    <dialog open class="dialog" aria-labelledby="export-title">
      <header><div><h2 id="export-title">Export video</h2><p>Render the timeline as one MP4 file.</p></div><button class="close" type="button" aria-label="Close export dialog" disabled={pending} onclick={onCancel}><X size={17} /></button></header>
      <div class="format-row"><span>Format</span><strong>MP4 · H.264 / AAC</strong></div>
      <div class="summary">{segments.length} timeline segment{segments.length === 1 ? "" : "s"} · {settings.speed}× speed · {settings.muted ? "Audio muted" : `Audio ${settings.volume}%`}</div>
      {#if dialogError || error}<p class="error" role="alert">{dialogError ?? error}</p>{/if}
      {#if pending}
        <div class="progress-label" role="status">{cancelRequested ? "Canceling export…" : `Rendering video in Rust… ${Math.round(progress * 100)}%`}</div>
        <div class="progress-track" role="progressbar" aria-label="Video export progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(progress * 100)}><span style={`width:${Math.round(progress * 100)}%`}></span></div>
      {/if}
      <footer>
        {#if pending}
          <button class="cancel-export" type="button" disabled={cancelRequested} onclick={() => void requestCancelExport()}><CircleStop size={14} />{cancelRequested ? "Canceling…" : "Cancel export"}</button>
        {:else}
          <button class="cancel" type="button" disabled={choosingPath} onclick={onCancel}>Cancel</button>
          <button class="primary" type="button" disabled={choosingPath || !segments.length} onclick={() => void exportFile()}><Download size={15} />{choosingPath ? "Choose destination…" : "Choose destination"}</button>
        {/if}
      </footer>
    </dialog>
  </div>
{/if}

<style>
  .backdrop { position:fixed; z-index:20; inset:0; display:grid; place-items:center; background:rgb(0 0 0 / 58%); }
  .dialog { width:min(420px,calc(100vw - 36px)); padding:18px; border:1px solid #30313a; border-radius:12px; background:#17191f; box-shadow:0 16px 60px rgb(0 0 0 / 45%); }
  header { display:flex; align-items:flex-start; justify-content:space-between; gap:14px; }
  h2 { margin:0; color:#f5f5f7; font-size:15px; }
  p { margin:6px 0 0; color:#989aa7; font-size:11px; }
  .close { display:grid; width:30px; height:30px; place-items:center; border:0; border-radius:7px; background:transparent; color:#989aa7; cursor:pointer; }
  .close:hover { background:#252730; color:white; }
  .format-row { display:flex; align-items:center; justify-content:space-between; margin-top:20px; padding:12px; border-radius:8px; background:#1b1d24; color:#989aa7; font-size:11px; }
  .format-row strong { color:#f5f5f7; font-weight:600; }
  .summary { margin-top:10px; color:#c5c6cd; font-size:11px; }
  .error { padding:8px 10px; border-radius:7px; background:rgb(255 48 56 / 12%); color:#ff858a; line-height:1.4; }
  .progress-label { margin-top:14px; color:#ff777d; font-size:11px; }
  .progress-track { height:5px; margin-top:8px; overflow:hidden; border-radius:99px; background:#292b32; }
  .progress-track span { display:block; height:100%; border-radius:inherit; background:#ff3038; transition:width 150ms linear; }
  footer { display:flex; justify-content:flex-end; gap:8px; margin-top:20px; }
  footer button { display:inline-flex; height:34px; align-items:center; justify-content:center; gap:7px; padding:0 12px; border:1px solid #30313a; border-radius:8px; background:#1b1d24; color:#f5f5f7; font:inherit; font-size:11px; cursor:pointer; }
  footer button.primary { border-color:#ff3038; background:#ff3038; font-weight:600; }
  footer button.cancel-export { margin-right:auto; border-color:#553034; color:#ff9296; }
  footer button.cancel-export:hover:not(:disabled) { background:#311f23; }
  footer button:hover:not(:disabled) { background:#252730; }
  footer button.primary:hover:not(:disabled) { background:#ff454d; }
  footer button:disabled,.close:disabled { opacity:.5; cursor:not-allowed; }
  @keyframes spin { to { transform:rotate(360deg); } }
</style>
