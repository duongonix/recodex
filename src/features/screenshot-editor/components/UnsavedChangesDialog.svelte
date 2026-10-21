<script lang="ts">
  import AlertCircle from "@lucide/svelte/icons/alert-circle";

  let { open, saving = false, onSave, onDiscard, onCancel }: {
    open: boolean;
    saving?: boolean;
    onSave: () => void;
    onDiscard: () => void;
    onCancel: () => void;
  } = $props();
</script>

{#if open}
  <div class="dialog-backdrop" role="presentation">
    <div class="dialog" role="alertdialog" tabindex="-1" aria-modal="true" aria-labelledby="unsaved-title" aria-describedby="unsaved-description">
      <span class="alert-icon"><AlertCircle size={21} aria-hidden="true" /></span>
      <h2 id="unsaved-title">Unsaved changes</h2>
      <p id="unsaved-description">Your screenshot edits have not been saved. Save them before closing?</p>
      <div class="dialog-actions">
        <button type="button" class="cancel" onclick={onCancel}>Cancel</button>
        <button type="button" class="discard" disabled={saving} onclick={onDiscard}>Discard</button>
        <button type="button" class="save" disabled={saving} onclick={onSave}>{saving ? "Saving…" : "Save"}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop { position:fixed; z-index:20; inset:0; display:grid; place-items:center; padding:20px; background:rgba(0,0,0,.66); backdrop-filter:blur(3px); }
  .dialog { width:min(420px,100%); padding:24px; border:1px solid #30333d; border-radius:14px; background:#17191f; box-shadow:0 20px 70px #000a; }
  .alert-icon { display:grid; width:38px; height:38px; place-items:center; border-radius:11px; color:#ff9f43; background:rgba(255,159,67,.12); }
  h2 { margin:16px 0 7px; color:#f5f5f7; font-size:17px; }
  p { margin:0; color:#a1a4af; font-size:13px; line-height:1.55; }
  .dialog-actions { display:flex; justify-content:flex-end; gap:8px; margin-top:22px; }
  button { height:35px; padding:0 12px; border:1px solid transparent; border-radius:8px; color:#f5f5f7; font-size:12px; }
  button:disabled { opacity:.55; }
  .cancel { background:#242730; }
  .discard { color:#ff747b; background:transparent; }
  .save { background:#ff3038; font-weight:600; }
</style>
