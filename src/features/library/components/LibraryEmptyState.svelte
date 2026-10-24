<script lang="ts">
  import FolderOpen from "@lucide/svelte/icons/folder-open";

  let {
    loading,
    error,
    query,
    onRetry
  }: {
    loading: boolean;
    error: string | null;
    query: string;
    onRetry: () => void;
  } = $props();
</script>

{#if loading}
  <div class="empty-state"><span class="loading-dot"></span><span>Loading your captures…</span></div>
{:else if error}
  <div class="empty-state error-state">
    <p>Library could not be loaded.</p>
    <span>{error}</span>
    <button type="button" onclick={onRetry}>Try again</button>
  </div>
{:else}
  <div class="empty-state">
    <div class="library-empty-icon" aria-hidden="true"><FolderOpen class="size-6" /></div>
    <p>{query ? "No captures match your search." : "No captures yet"}</p>
    <span>{query ? "Try a different name or filter." : "Record a video or take a screenshot and it will appear here."}</span>
  </div>
{/if}

<style>
  .empty-state {
    display: flex;
    min-height: 118px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 7px;
    color: var(--muted);
    text-align: center;
  }
  .empty-state p { margin: 0; color: var(--text); font-size: 11px; font-weight: 600; }
  .empty-state > span:last-child { max-width: 245px; font-size: 9px; line-height: 1.45; overflow-wrap: anywhere; }
  .error-state > span { max-width: 250px; font-size: 9px; line-height: 1.4; overflow-wrap: anywhere; }
  .error-state button { margin-top: 3px; border: 0; background: transparent; color: var(--accent); font: inherit; font-size: 10px; cursor: pointer; }
  .library-empty-icon { display: grid; width: 40px; height: 40px; place-items: center; border-radius: 12px; background: var(--surface); color: var(--accent); }
  .loading-dot { width: 16px; height: 16px; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: library-spin 0.8s linear infinite; }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  @keyframes library-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation-duration: 0.01ms !important; } }
</style>
