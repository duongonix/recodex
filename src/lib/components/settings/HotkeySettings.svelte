<script lang="ts">
  import AlertCircle from "@lucide/svelte/icons/alert-circle";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import type { QuickAccessShortcutStatus } from "../../types/shortcuts.js";

  let {
    shortcutStatus,
    shortcutError
  }: {
    shortcutStatus: QuickAccessShortcutStatus | null;
    shortcutError: string | null;
  } = $props();

  const shortcuts = [
    { label: "Start / Stop recording", keys: ["Ctrl", "Shift", "R"] },
    { label: "Take screenshot", keys: ["Ctrl", "Shift", "S"] },
    { label: "Capture region", keys: ["Ctrl", "Shift", "A"] }
  ];

  function shortcutLabel(accelerator: string): string {
    const commandKey = /Mac|iPhone|iPad/.test(navigator.platform) ? "⌘" : "Ctrl";
    return accelerator.replace("CommandOrControl", commandKey).replaceAll("+", " + ");
  }
</script>

<section class="settings-section" aria-label="Hotkey settings">
  <h3 class="settings-section-title">Keyboard shortcuts</h3>
  <div class="shortcut-list">
    {#each shortcuts as shortcut (shortcut.label)}
      <div class="shortcut-row">
        <Keyboard aria-hidden="true" class="setting-icon" />
        <span class="setting-label">{shortcut.label}</span>
        <span class="shortcut-keys" aria-label={shortcut.keys.join(" plus ")}>
          {#each shortcut.keys as key (key)}<kbd>{key}</kbd>{/each}
        </span>
      </div>
    {/each}
  </div>
  <p class="shortcut-note">These shortcuts are read-only. Reassignment is not implemented.</p>
  <p class="shortcut-note" role="status">
    {#if shortcutError}
      <AlertCircle aria-hidden="true" class="size-4" />
      <span>Global quick access is unavailable: {shortcutError}</span>
    {:else if shortcutStatus?.registered}
      Global quick access is registered as {shortcutLabel(shortcutStatus.accelerator)}. The three shortcuts above are not registered in this build.
    {:else if shortcutStatus}
      Global shortcut registration is unavailable. {shortcutStatus.message ?? "Use the RecoDex tray icon to open the app."}
    {:else}
      Checking global shortcut availability…
    {/if}
  </p>
</section>
