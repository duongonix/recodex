<script lang="ts">
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";

  let {
    loading,
    onBack,
    onRefresh
  }: {
    loading: boolean;
    onBack: () => void;
    onRefresh: () => void;
  } = $props();
</script>

<div class="library-heading">
  <button class="back" type="button" aria-label="Back to recorder" title="Back" onclick={onBack}>
    <ArrowLeft aria-hidden="true" class="size-[17px]" />
  </button>
  <div class="heading-copy">
    <h2 id="library-title">Library</h2>
    <p>Your recent recordings and screenshots</p>
  </div>
  <button class="refresh" type="button" aria-label="Refresh library" title="Refresh" disabled={loading} onclick={onRefresh}>
    <span class:spinning={loading}><RefreshCw aria-hidden="true" class="size-4" /></span>
  </button>
</div>

<style>
  .library-heading { display: flex; min-height: 38px; flex: none; align-items: center; gap: 9px; }
  .back,
  .refresh {
    display: grid;
    width: 30px;
    height: 30px;
    flex: none;
    place-items: center;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .back:hover,
  .refresh:hover:not(:disabled) { border-color: var(--border); background: var(--surface); color: var(--text); }
  .heading-copy { min-width: 0; flex: 1; }
  h2 { margin: 0; color: var(--text); font-size: 15px; font-weight: 650; letter-spacing: -0.02em; }
  .heading-copy p { overflow: hidden; margin: 3px 0 0; color: var(--muted); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
  .refresh:disabled { cursor: default; opacity: 0.65; }
  .refresh .spinning { animation: library-spin 1s linear infinite; }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  @keyframes library-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .refresh .spinning { animation-duration: 0.01ms; } }
</style>
