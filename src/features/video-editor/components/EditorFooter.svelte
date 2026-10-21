<script lang="ts">
  import Download from "@lucide/svelte/icons/download";
  type Props = { width: number; height: number; fps: number; durationMs: number; pending: boolean; onCancel: () => void; onExport: () => void };
  let { width, height, fps, durationMs, pending, onCancel, onExport }: Props = $props();
  const format = (ms: number) => {
    const seconds = Math.floor(Math.max(0, ms) / 1000);
    return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
  };
</script>

<footer class="editor-footer">
  <div class="metadata">{width || "—"} × {height || "—"}<i></i>{fps || "—"} FPS<i></i>{format(durationMs)}</div>
  <div class="actions">
    <button class="cancel" type="button" disabled={pending} onclick={onCancel}>Cancel</button>
    <button class="export" type="button" disabled={pending || durationMs <= 0} onclick={onExport}>
      {#if pending}<span class="spinner" aria-hidden="true"></span>Exporting…{:else}<Download size={16} />Export…{/if}
    </button>
  </div>
</footer>

<style>
  .editor-footer { display:flex; align-items:center; justify-content:space-between; gap:16px; padding:0 18px; border-top:1px solid #292b32; background:#101116; }
  .metadata { display:flex; align-items:center; gap:12px; color:#989aa7; font-size:11px; font-variant-numeric:tabular-nums; }
  .metadata i { width:1px; height:14px; background:#383a44; }
  .actions { display:flex; align-items:center; gap:9px; }
  button { display:inline-flex; min-width:116px; height:36px; align-items:center; justify-content:center; gap:8px; padding:0 14px; border:1px solid #292b32; border-radius:8px; background:#17191f; color:#f5f5f7; font:inherit; font-size:12px; cursor:pointer; }
  button:hover:not(:disabled) { background:#252730; }
  button.export { min-width:144px; border-color:#ff3038; background:#ff3038; color:#fff; font-weight:650; }
  button.export:hover:not(:disabled) { background:#ff454d; }
  button:disabled { opacity:.5; cursor:not-allowed; }
  .spinner { width:13px; height:13px; border:2px solid rgb(255 255 255 / 40%); border-top-color:white; border-radius:50%; animation:spin .7s linear infinite; }
  @keyframes spin { to { transform:rotate(360deg); } }
</style>
