<script lang="ts">
  import type { LibraryFilter } from "../../../lib/types/library.js";

  let {
    value = $bindable<LibraryFilter>("all"),
    count
  }: {
    value: LibraryFilter;
    count: number;
  } = $props();

  const filters: LibraryFilter[] = ["all", "video", "screenshot"];

  function label(filter: LibraryFilter): string {
    if (filter === "all") return "All";
    return filter === "video" ? "Videos" : "Screenshots";
  }
</script>

<div class="filters" role="group" aria-label="Filter captures">
  {#each filters as filter (filter)}
    <button
      type="button"
      class:chosen={value === filter}
      aria-pressed={value === filter}
      onclick={() => (value = filter)}
    >{label(filter)}</button>
  {/each}
  <span class="capture-count">{count} {count === 1 ? "item" : "items"}</span>
</div>

<style>
  .filters { display: flex; min-height: 26px; flex: none; align-items: center; gap: 6px; }
  .filters button {
    height: 25px;
    padding: 0 10px;
    border: 1px solid transparent;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 10px;
    cursor: pointer;
  }
  .filters button:hover { background: var(--surface); color: var(--text); }
  .filters button.chosen {
    border-color: color-mix(in srgb, var(--accent) 38%, var(--border));
    background: color-mix(in srgb, var(--accent) 15%, var(--surface));
    color: var(--text);
  }
  .capture-count { margin-left: auto; color: var(--muted); font-size: 9px; }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
</style>
