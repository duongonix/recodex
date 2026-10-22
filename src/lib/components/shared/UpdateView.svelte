<script lang="ts">
  import CalendarDays from "@lucide/svelte/icons/calendar-days";
  import CircleDot from "@lucide/svelte/icons/circle-dot";
  import Download from "@lucide/svelte/icons/download";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import packageInfo from "../../../../package.json";
  import { appUpdate } from "../../stores/app-update.svelte.js";
  import { parseReleaseNotes } from "../../utils/release-notes.js";
  import { Button } from "../ui/button/index.js";

  const currentAppVersion = packageInfo.version;
  const releaseBlocks = $derived(parseReleaseNotes(appUpdate.releaseNotes));
  const releaseDateLabel = $derived.by(() => {
    if (!appUpdate.releaseDate) return null;
    const date = new Date(appUpdate.releaseDate);
    return Number.isNaN(date.getTime())
      ? null
      : new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(date);
  });
</script>

<section class="update-view" aria-labelledby="update-heading">
  <header class="update-header">
    <p class="update-eyebrow">UPDATE AVAILABLE</p>
    <div class="update-app">
      <span class="update-mark" aria-hidden="true"><CircleDot class="size-5" /></span>
      <div class="update-app-copy">
        <h2 id="update-heading">RecoDex</h2>
        <p>New version ready to install</p>
      </div>
      <span class="update-version">v{appUpdate.version ?? "—"}</span>
    </div>
    <div class="update-version-change" aria-label={`Current version ${appUpdate.currentVersion ?? currentAppVersion}, latest version ${appUpdate.version ?? "unknown"}`}>
      <span>Installed v{appUpdate.currentVersion ?? currentAppVersion}</span>
      {#if releaseDateLabel}
        <span class="release-date"><CalendarDays aria-hidden="true" class="size-3.5" />{releaseDateLabel}</span>
      {/if}
    </div>
  </header>

  <div class="release-notes" aria-label="Release information">
    <h3>Release notes</h3>
    {#if releaseBlocks.length > 0}
      <div class="release-note-content">
        {#each releaseBlocks as block, index (`${block.kind}-${index}`)}
          {#if block.kind === "heading"}
            <h4>{block.text}</h4>
          {:else if block.kind === "list-item"}
            <p class="release-list-item">{block.text}</p>
          {:else}
            <p>{block.text}</p>
          {/if}
        {/each}
      </div>
    {:else}
      <p class="release-notes-empty">Release notes are not available for this version.</p>
    {/if}
  </div>

  <footer class="update-footer">
    {#if appUpdate.error}
      <p class="update-error" role="alert">{appUpdate.error}</p>
    {/if}
    {#if appUpdate.installing}
      <div class="update-progress" aria-live="polite">
        <div class="progress-copy">
          <span>{appUpdate.downloadProgress === 100 ? "Installing update…" : "Downloading update…"}</span>
          {#if appUpdate.downloadProgress !== null && appUpdate.downloadProgress < 100}
            <span>{appUpdate.downloadProgress}%</span>
          {/if}
        </div>
        <div class="progress-track" aria-hidden="true">
          <span class:progress-indeterminate={appUpdate.downloadProgress === null} style:width={`${appUpdate.downloadProgress ?? 35}%`}></span>
        </div>
      </div>
    {/if}
    <Button class="update-action" disabled={appUpdate.installing || !appUpdate.updateAvailable} onclick={() => void appUpdate.install()}>
      {#if appUpdate.installing}
        <LoaderCircle aria-hidden="true" class="size-4 animate-spin" />
        <span>{appUpdate.downloadProgress === 100 ? "Installing…" : "Updating…"}</span>
      {:else}
        <Download aria-hidden="true" class="size-4" />
        <span>Update</span>
      {/if}
    </Button>
  </footer>
</section>

<style>
  .update-view {
    display: flex;
    min-height: 0;
    flex-direction: column;
    gap: 16px;
    padding: 8px 2px 0;
  }

  .update-header {
    display: grid;
    flex: none;
    gap: 10px;
  }

  .update-eyebrow {
    margin: 0;
    color: var(--accent);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
  }

  .update-app {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 10px;
  }

  .update-mark {
    display: grid;
    width: 36px;
    height: 36px;
    flex: none;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 54%, transparent);
    border-radius: 11px;
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
    color: var(--accent);
  }

  .update-app-copy {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .update-app-copy h2 {
    margin: 0;
    color: var(--text);
    font-size: 16px;
    font-weight: 650;
  }

  .update-app-copy p,
  .update-version-change {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
  }

  .update-version {
    margin-left: auto;
    padding: 5px 8px;
    border-radius: 7px;
    background: var(--surface-hover);
    color: var(--text);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  .update-version-change {
    display: flex;
    min-height: 22px;
    align-items: center;
    gap: 7px;
    font-variant-numeric: tabular-nums;
  }

  .release-date {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
  }

  .release-notes {
    min-height: 0;
    flex: 1;
    overflow: auto;
    padding: 12px;
    border-radius: 10px;
    background: var(--surface);
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .release-notes::-webkit-scrollbar {
    width: 6px;
  }

  .release-notes::-webkit-scrollbar-thumb {
    border-radius: 999px;
    background: var(--border);
  }

  .release-notes h3 {
    margin: 0 0 10px;
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
  }

  .release-note-content {
    display: grid;
    gap: 8px;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.55;
    overflow-wrap: anywhere;
  }

  .release-note-content p,
  .release-note-content h4,
  .release-notes-empty {
    margin: 0;
  }

  .release-note-content h4 {
    color: var(--text);
    font-size: 11px;
    font-weight: 600;
  }

  .release-list-item {
    position: relative;
    padding-left: 12px;
  }

  .release-list-item::before {
    position: absolute;
    left: 2px;
    color: var(--accent);
    content: "•";
  }

  .release-notes-empty {
    color: var(--muted);
    font-size: 11px;
    line-height: 1.5;
  }

  .update-footer {
    display: grid;
    flex: none;
    gap: 8px;
    padding-bottom: 2px;
  }

  .update-error {
    margin: 0;
    color: var(--accent);
    font-size: 11px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .update-progress {
    display: grid;
    gap: 6px;
    color: var(--muted);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .progress-copy {
    display: flex;
    justify-content: space-between;
  }

  .progress-track {
    height: 4px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--surface-hover);
  }

  .progress-track span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }

  .progress-track span.progress-indeterminate {
    width: 35%;
    animation: update-progress 1.1s ease-in-out infinite alternate;
  }

  :global(.update-action) {
    width: 100%;
    min-height: 42px;
    gap: 8px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }

  :global(.update-action:hover:not(:disabled)) {
    background: var(--accent-hover);
  }

  :global(.update-action:disabled) {
    opacity: 0.76;
  }

  @keyframes update-progress {
    from { transform: translateX(-20%); }
    to { transform: translateX(185%); }
  }

  @media (prefers-reduced-motion: reduce) {
    .progress-track span.progress-indeterminate {
      animation: none;
    }
  }
</style>
