<script lang="ts">
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import type { HoldZoomShortcutStatus } from "../../types/shortcuts.js";

  let {
    accelerator,
    disabled = false,
    status,
    onChange
  }: {
    accelerator: string;
    disabled?: boolean;
    status: HoldZoomShortcutStatus | null;
    onChange: (accelerator: string) => Promise<boolean>;
  } = $props();

  let listening = $state(false);
  let saving = $state(false);

  function displayShortcut(value: string): string {
    const platform = typeof navigator === "undefined" ? "" : navigator.platform;
    const command = /Mac|iPhone|iPad/.test(platform) ? "⌘" : "Ctrl";
    return value
      .replace("CommandOrControl", command)
      .replace("Control", "Ctrl")
      .replaceAll("+", " + ");
  }

  function keyFromCode(code: string): string | null {
    if (code.startsWith("Key") && code.length === 4) return code.slice(3);
    if (code.startsWith("Digit") && code.length === 6) return code.slice(5);
    if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
    return null;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!listening) return;
    if (event.key === "Escape") {
      event.preventDefault();
      listening = false;
      return;
    }

    const key = keyFromCode(event.code);
    if (!key || (!event.ctrlKey && !event.metaKey && !event.altKey)) return;
    event.preventDefault();
    event.stopPropagation();

    const modifiers = [
      event.ctrlKey || event.metaKey ? "CommandOrControl" : null,
      event.altKey ? "Alt" : null,
      event.shiftKey ? "Shift" : null
    ].filter((modifier): modifier is string => modifier !== null);
    const next = [...modifiers, key].join("+");
    listening = false;
    saving = true;
    void onChange(next).finally(() => (saving = false));
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="zoom-shortcut-row" class:setting-row-disabled={disabled}>
  <Keyboard aria-hidden="true" class="setting-icon" />
  <div class="setting-copy">
    <span class="setting-label">Hold-to-zoom shortcut</span>
    <span class="setting-hint">
      {listening ? "Press Ctrl, Alt, or Command with a letter, number, or F key. Escape cancels." : "Hold while recording to follow the pointer at 2×."}
    </span>
  </div>
  <button
    type="button"
    class="zoom-shortcut-button"
    disabled={disabled || saving}
    aria-label={listening ? "Press a shortcut; Escape cancels" : `Change hold-to-zoom shortcut, current ${displayShortcut(accelerator)}`}
    aria-pressed={listening}
    title={listening ? "Press Ctrl, Alt, or Command with a letter, number, or F key. Escape cancels." : "Click to change the shortcut"}
    onclick={() => (listening = true)}
    onfocusout={() => (listening = false)}
  >
    {listening ? "Press shortcut…" : saving ? "Saving…" : displayShortcut(accelerator)}
  </button>
</div>
{#if status?.message}
  <p class="zoom-shortcut-status" role="status">{status.message}</p>
{:else if status?.enabled && !status.registered}
  <p class="zoom-shortcut-status" role="status">The hold-to-zoom shortcut is not registered.</p>
{/if}
