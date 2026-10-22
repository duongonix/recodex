<script lang="ts">
  import AlertCircle from "@lucide/svelte/icons/alert-circle";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import type {
    CaptureHotkeyAction,
    CaptureHotkeyStatus,
    QuickAccessShortcutStatus
  } from "../../types/shortcuts.js";

  let {
    shortcutStatus,
    captureHotkeyStatus,
    shortcutError
  }: {
    shortcutStatus: QuickAccessShortcutStatus | null;
    captureHotkeyStatus: CaptureHotkeyStatus[] | null;
    shortcutError: string | null;
  } = $props();

  const actions: { id: CaptureHotkeyAction; label: string }[] = [
    { id: "record-screen", label: "Record screen" },
    { id: "record-window", label: "Record window" },
    { id: "record-region", label: "Record area" },
    { id: "screenshot-screen", label: "Screenshot screen" },
    { id: "screenshot-window", label: "Screenshot window" },
    { id: "screenshot-region", label: "Screenshot area" }
  ];

  function shortcutKeys(accelerator: string): string[] {
    const mac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);
    return accelerator.split("+").map((key) => {
      if (key === "CommandOrControl") return mac ? "⌘" : "Ctrl";
      if (key === "Alt") return mac ? "Option" : "Alt";
      return key;
    });
  }

  function shortcutLabel(accelerator: string): string {
    return shortcutKeys(accelerator).join(" plus ");
  }

  function actionStatus(action: CaptureHotkeyAction): CaptureHotkeyStatus | undefined {
    return captureHotkeyStatus?.find((status) => status.action === action);
  }
</script>

<section class="settings-section" aria-label="Hotkey settings">
  <h3 class="settings-section-title">Keyboard shortcuts</h3>
  <p class="shortcut-note">Default bindings are fixed. These shortcuts work while RecoDex is running; window and area actions open the native picker before capture begins.</p>
  <div class="shortcut-list">
    {#each actions as action (action.id)}
      {@const status = actionStatus(action.id)}
      <div class="shortcut-row">
        <Keyboard aria-hidden="true" class="setting-icon" />
        <span class="setting-label">{action.label}</span>
        {#if status}
          {@const keys = shortcutKeys(status.accelerator)}
          <span class="shortcut-keys" aria-label={shortcutLabel(status.accelerator)}>
            {#each keys as key (`${status.action}-${key}`)}<kbd>{key}</kbd>{/each}
          </span>
          {#if !status.registered}<span class="shortcut-unavailable" title={status.message ?? "Shortcut is unavailable."}><AlertCircle aria-hidden="true" class="size-4" /></span>{/if}
        {:else}
          <span class="shortcut-keys" role="status">Checking…</span>
        {/if}
      </div>
      {#if status && !status.registered && status.message}
        <p class="shortcut-note shortcut-error" role="status">{status.message}</p>
      {/if}
    {/each}
    <div class="shortcut-row">
      <Keyboard aria-hidden="true" class="setting-icon" />
      <span class="setting-label">Open RecoDex / show active session</span>
      {#if shortcutStatus}
        {@const keys = shortcutKeys(shortcutStatus.accelerator)}
        <span class="shortcut-keys" aria-label={shortcutLabel(shortcutStatus.accelerator)}>
          {#each keys as key (`quick-access-${key}`)}<kbd>{key}</kbd>{/each}
        </span>
        {#if !shortcutStatus.registered}<span class="shortcut-unavailable" title={shortcutStatus.message ?? shortcutError ?? "Shortcut is unavailable."}><AlertCircle aria-hidden="true" class="size-4" /></span>{/if}
      {:else}
        <span class="shortcut-keys" role="status">Checking…</span>
      {/if}
    </div>
  </div>
  {#if shortcutError}
    <p class="shortcut-note shortcut-error" role="status">Global quick access status unavailable: {shortcutError}</p>
  {/if}
</section>
