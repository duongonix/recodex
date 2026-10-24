<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import Info from "@lucide/svelte/icons/info";
  import Library from "@lucide/svelte/icons/library";
  import AppUpdateButton from "../shared/AppUpdateButton.svelte";
  import Minus from "@lucide/svelte/icons/minus";
  import Settings from "@lucide/svelte/icons/settings";
  import X from "@lucide/svelte/icons/x";
  import { Button } from "../ui/button/index.js";
  import { errorMessage } from "../../services/ipc/settings.js";

  type WindowAction = "minimize" | "close";

  let {
    desktopApp,
    settingsActive = false,
    libraryActive = false,
    minimizeToTray = true,
    settingsDisabled,
    libraryDisabled = false,
    settingsTitle,
    onSettings,
    onLibrary,
    onAbout,
    onUpdate,
    onBack = () => {},
    onWindowError
  }: {
    desktopApp: boolean;
    settingsActive?: boolean;
    libraryActive?: boolean;
    minimizeToTray?: boolean;
    settingsDisabled: boolean;
    libraryDisabled?: boolean;
    settingsTitle: string;
    onSettings: () => void;
    onLibrary: () => void;
    onAbout: () => void;
    onUpdate: () => void;
    onBack?: () => void;
    onWindowError: (message: string) => void;
  } = $props();

  async function performWindowAction(action: WindowAction) {
    if (!desktopApp) return;
    try {
      const window = getCurrentWindow();
      if (action === "minimize" && minimizeToTray) await window.hide();
      else if (action === "minimize") await window.minimize();
      else await window.close();
    } catch (error) {
      onWindowError(`Could not ${action} the RecoDex window: ${errorMessage(error)}`);
    }
  }
</script>

<header class="brand-row" data-tauri-drag-region>
  <div class="brand">
    {#if settingsActive}
      <Button
        variant="ghost"
        size="icon"
        aria-label="Back to recorder"
        title="Back"
        class="back-button"
        onclick={onBack}
      >
        <ArrowLeft aria-hidden="true" class="size-[18px]" />
      </Button>
    {/if}
    <span class="brand-mark" aria-hidden="true"><img src="/icon.png" alt="" draggable="false" /></span>
    <h1 id="main-title">RecoDex</h1>
  </div>
  <div class="titlebar-actions">
    <AppUpdateButton desktopApp visible={!settingsActive} onOpen={onUpdate} />
    {#if !settingsActive}
    <Button
      variant="ghost"
      size="icon"
      aria-label="About RecoDex"
      title="About RecoDex"
      class="about-button"
      onclick={onAbout}
    >
      <Info aria-hidden="true" class="size-[18px]" />
    </Button>
    <Button
      variant="ghost"
      size="icon"
      disabled={settingsDisabled}
      aria-label="Settings"
      title={settingsTitle}
      class="settings-button"
      onclick={onSettings}
    >
      <Settings aria-hidden="true" class="size-[18px]" />
    </Button>
    <Button
      variant="ghost"
      size="icon"
      disabled={libraryDisabled}
      aria-label="Capture library"
      aria-pressed={libraryActive}
      title="Capture library"
      class={libraryActive ? "library-button active" : "library-button"}
      onclick={onLibrary}
    >
      <Library aria-hidden="true" class="size-[18px]" />
    </Button>
    {/if}
    {#if desktopApp}
      <span class="titlebar-divider" aria-hidden="true"></span>
      <div class="window-controls">
        <Button
          variant="ghost"
          size="icon"
          aria-label="Minimize window"
          title="Minimize"
          class="window-control"
          onclick={() => performWindowAction("minimize")}
        >
          <Minus aria-hidden="true" class="size-[18px]" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Close RecoDex"
          title="Close"
          class="window-control close-control"
          onclick={() => performWindowAction("close")}
        >
          <X aria-hidden="true" class="size-[18px]" />
        </Button>
      </div>
    {/if}
  </div>
</header>
