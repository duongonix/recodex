<script lang="ts">
  import type { DisplaySource, WindowSource } from "../../types/capture.js";

  let {
    open,
    kind,
    displays,
    windows,
    loading,
    error,
    onClose,
    onSelectDisplay,
    onSelectWindow
  }: {
    open: boolean;
    kind: "display" | "window";
    displays: DisplaySource[];
    windows: WindowSource[];
    loading: boolean;
    error: string | null;
    onClose: () => void;
    onSelectDisplay: (display: DisplaySource) => void;
    onSelectWindow: (window: WindowSource) => void;
  } = $props();

  let dialog = $state<HTMLDialogElement | null>(null);

  $effect(() => {
    if (open && dialog && !dialog.open) dialog.showModal();
    if (!open && dialog?.open) dialog.close();
  });
</script>

{#if open}
  <dialog
    bind:this={dialog}
    class:window-picker={kind === "window"}
    aria-labelledby="capture-source-title"
    oncancel={(event) => {
      event.preventDefault();
      onClose();
    }}
    onclick={(event) => {
      if (event.target === dialog) onClose();
    }}
  >
    <header>
      <h2 id="capture-source-title">{kind === "display" ? "Choose a display" : "Choose a window"}</h2>
      <button type="button" aria-label="Close source picker" onclick={onClose}>×</button>
    </header>

    {#if loading}
      <p class="message" role="status">Loading available sources…</p>
    {:else if error}
      <p class="message error" role="alert">{error}</p>
    {:else if kind === "display" && displays.length === 0}
      <p class="message" role="status">No displays are currently available.</p>
    {:else if kind === "window" && windows.length === 0}
      <p class="message" role="status">No visible titled windows are available.</p>
    {:else}
      <ul>
        {#if kind === "display"}
          {#each displays as display (display.displayId)}
            <li>
              <button type="button" onclick={() => onSelectDisplay(display)}>
                <span>{display.name}{display.primary ? " · Primary" : ""}</span>
                <small>{display.width} × {display.height} · {Math.round(display.scaleFactor * 100)}%</small>
              </button>
            </li>
          {/each}
        {:else}
          {#each windows as source (source.windowId)}
            <li>
              <button type="button" onclick={() => onSelectWindow(source)}>
                <span>{source.title}</span>
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    {/if}
  </dialog>
{/if}

<style>
  dialog {
    width: min(360px, calc(100vw - 32px));
    max-height: min(440px, calc(100vh - 32px));
    overflow: auto;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--bg);
    color: var(--text);
    box-shadow: 0 20px 60px rgb(0 0 0 / 45%);
  }

  dialog.window-picker {
    width: min(400px, 100vw);
    height: min(490px, 100vh);
    max-width: 100vw;
    max-height: 100vh;
    border-color: color-mix(in srgb, var(--accent) 42%, var(--border));
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 58%);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }

  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  header button {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    color: var(--muted);
    font-size: 20px;
    cursor: pointer;
  }

  ul {
    display: grid;
    max-height: 350px;
    gap: 6px;
    margin: 0;
    padding: 0;
    overflow: auto;
    list-style: none;
  }

  li button {
    display: grid;
    width: 100%;
    min-height: 42px;
    justify-items: start;
    gap: 3px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }

  li button:hover,
  li button:focus-visible,
  header button:hover,
  header button:focus-visible {
    border-color: var(--accent);
    outline: none;
  }

  li span {
    overflow: hidden;
    max-width: 100%;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  li small {
    color: var(--muted);
    font-size: 11px;
  }

  .message {
    margin: 14px 2px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }

  .error {
    color: var(--warning);
  }
</style>
