<script lang="ts">
  import { tick } from "svelte";
  import SettingsIcon from "@lucide/svelte/icons/settings-2";
  import {
    Content as TabsContent,
    Root as TabsRoot
  } from "../ui/tabs/index.js";
  import GeneralSettings from "./GeneralSettings.svelte";
  import HotkeySettings from "./HotkeySettings.svelte";
  import SettingsTabs from "./SettingsTabs.svelte";
  import SnapSettings from "./SnapSettings.svelte";
  import WebcamConfiguration from "../webcam/WebcamConfiguration.svelte";
  import type { RecordingPreflightReport } from "../../types/recording.js";
  import type { AppSettings, SettingsPatch } from "../../types/settings.js";
  import type { HoldZoomShortcutStatus, QuickAccessShortcutStatus } from "../../types/shortcuts.js";
  import type { SettingsTab } from "../../types/settings-view.js";
  import "./settings.css";

  let {
    tab = $bindable<SettingsTab>("snap"),
    settings,
    saving,
    error,
    shortcutStatus,
    shortcutError,
    startupLabel,
    startupSupported,
    startupError,
    traySupported,
    maxResolutionHeight,
    availableFps,
    captureActive,
    cursorSupported,
    microphoneSupported,
    microphoneMessage,
    systemAudioSupported,
    systemAudioMessage,
    zoomFollowCursorSupported,
    zoomFollowCursorMessage,
    holdZoomStatus,
    preflightLoading,
    preflightError,
    preflightReport,
    onUpdate,
    onChooseFolder,
    onCheckRecording
  }: {
    tab?: SettingsTab;
    settings: AppSettings | null;
    saving: boolean;
    error: string | null;
    shortcutStatus: QuickAccessShortcutStatus | null;
    shortcutError: string | null;
    startupLabel: string;
    startupSupported: boolean;
    startupError: string | null;
    traySupported: boolean;
    maxResolutionHeight: number | null;
    availableFps: number[];
    captureActive: boolean;
    cursorSupported: boolean;
    microphoneSupported: boolean;
    microphoneMessage: string;
    systemAudioSupported: boolean;
    systemAudioMessage: string;
    zoomFollowCursorSupported: boolean;
    zoomFollowCursorMessage: string;
    holdZoomStatus: HoldZoomShortcutStatus | null;
    preflightLoading: boolean;
    preflightError: string | null;
    preflightReport: RecordingPreflightReport | null;
    onUpdate: (patch: SettingsPatch) => Promise<boolean>;
    onChooseFolder: (kind: "video" | "screenshot") => void;
    onCheckRecording: () => void;
  } = $props();

  let webcamOpen = $state(false);
  let settingsScroll = $state<HTMLDivElement>();
  let snapScrollTop = 0;

  function openWebcam() {
    snapScrollTop = settingsScroll?.scrollTop ?? 0;
    webcamOpen = true;
  }

  function backFromWebcam() {
    webcamOpen = false;
    void tick().then(() => {
      if (settingsScroll) settingsScroll.scrollTop = snapScrollTop;
    });
  }
</script>

{#if webcamOpen && settings}
  <WebcamConfiguration
    settings={settings.webcam}
    saving={saving}
    {captureActive}
    onBack={backFromWebcam}
    onChange={(webcam) => onUpdate({ webcam })}
  />
{:else}
  <TabsRoot bind:value={tab} class="settings-tabs-root">
    <section class="settings-heading" aria-labelledby="settings-heading">
      <span class="settings-heading-icon" aria-hidden="true">
        <SettingsIcon class="size-5" />
      </span>
      <div class="settings-heading-copy">
        <h2 id="settings-heading">Settings</h2>
        <p>Recording &amp; screenshot preferences</p>
      </div>
    </section>

    <SettingsTabs />

    <div bind:this={settingsScroll} class="settings-scroll" aria-label="Settings content">
      {#if error}
        <p class="settings-error" role="alert">{error}</p>
      {:else if saving}
        <p class="settings-saving" role="status">Saving preferences…</p>
      {/if}

      {#if settings}
        <TabsContent value="general" class="settings-panel">
          <GeneralSettings
            {settings}
            {startupLabel}
            {startupSupported}
            {startupError}
            {traySupported}
            {onUpdate}
          />
        </TabsContent>
        <TabsContent value="snap" class="settings-panel">
          <SnapSettings
            {settings}
            {saving}
            {maxResolutionHeight}
            {availableFps}
            {captureActive}
            {cursorSupported}
            {microphoneSupported}
            {microphoneMessage}
            {systemAudioSupported}
            {systemAudioMessage}
            {zoomFollowCursorSupported}
            {zoomFollowCursorMessage}
            {holdZoomStatus}
            {preflightLoading}
            {preflightError}
            {preflightReport}
            {onUpdate}
            {onChooseFolder}
            {onCheckRecording}
            onOpenWebcam={openWebcam}
          />
        </TabsContent>
        <TabsContent value="hotkey" class="settings-panel">
          <HotkeySettings {shortcutStatus} {shortcutError} />
        </TabsContent>
      {:else}
        <p class="settings-loading" role="status">Loading saved preferences…</p>
      {/if}
    </div>
  </TabsRoot>
{/if}
