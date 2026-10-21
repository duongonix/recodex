<script lang="ts">
  import Camera from "@lucide/svelte/icons/camera";
  import Eye from "@lucide/svelte/icons/eye";
  import Gauge from "@lucide/svelte/icons/gauge";
  import Mic from "@lucide/svelte/icons/mic";
  import Monitor from "@lucide/svelte/icons/monitor";
  import MousePointer2 from "@lucide/svelte/icons/mouse-pointer-2";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import ZoomIn from "@lucide/svelte/icons/zoom-in";
  import FolderPickerRow from "./FolderPickerRow.svelte";
  import AudioInputTestPanel from "./AudioInputTestPanel.svelte";
  import MicrophoneDeviceSelect from "./MicrophoneDeviceSelect.svelte";
  import RecordingPreflightPanel from "./RecordingPreflightPanel.svelte";
  import SettingSelectRow from "./SettingSelectRow.svelte";
  import SettingToggleRow from "./SettingToggleRow.svelte";
  import WebcamSettingsRow from "./WebcamSettingsRow.svelte";
  import ZoomShortcutField from "./ZoomShortcutField.svelte";
  import type { AppSettings, SettingsPatch } from "../../types/settings.js";
  import type { RecordingPreflightReport } from "../../types/recording.js";
  import type { HoldZoomShortcutStatus } from "../../types/shortcuts.js";

  let {
    settings,
    saving,
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
    onCheckRecording,
    onOpenWebcam
  }: {
    settings: AppSettings;
    saving: boolean;
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
    onOpenWebcam: () => void;
  } = $props();

  const qualityChoices = $derived([
    { value: "720", label: "720p (HD)", disabled: maxResolutionHeight === null || maxResolutionHeight < 720 },
    { value: "1080", label: "1080p (FHD)", disabled: maxResolutionHeight === null || maxResolutionHeight < 1080 },
    { value: "2160", label: "2160p (UHD)", disabled: maxResolutionHeight === null || maxResolutionHeight < 2160 }
  ]);
  const fpsChoices = $derived([
    { value: "30", label: "30 FPS", disabled: !availableFps.includes(30) },
    { value: "60", label: "60 FPS", disabled: !availableFps.includes(60) }
  ]);
</script>

<section class="settings-section" aria-label="Recording preferences">
  {#if captureActive}
    <p class="settings-info" role="status">A recording is active. Changes to recording preferences apply to the next session.</p>
  {/if}
  <h3 class="settings-section-title">Recording</h3>
  <div class="snap-folder-row">
    <FolderPickerRow
      id="recording-output-directory"
      label="Default save folder"
      path={settings.videoOutputDirectory}
      disabled={saving}
      onChoose={() => onChooseFolder("video")}
    />
  </div>
  <RecordingPreflightPanel
    loading={preflightLoading}
    error={preflightError}
    report={preflightReport}
    onCheck={onCheckRecording}
  />
  <div class="settings-rows">
    <SettingToggleRow
      id="show-mouse-cursor"
      label="Show mouse cursor"
      icon={MousePointer2}
      value={settings.showMouseCursor}
      disabled={!cursorSupported}
      disabledReason="Cursor capture is unavailable with the current recording backend."
      onChange={(value) => onUpdate({ showMouseCursor: value })}
    />
    <SettingToggleRow
      id="show-recording-preview"
      label="Preview recording"
      description="Review each recording before saving it."
      icon={Eye}
      value={settings.showRecordingPreview}
      disabled={saving || captureActive}
      disabledReason={captureActive ? "Recording preferences apply to the next session." : undefined}
      onChange={(value) => onUpdate({ showRecordingPreview: value })}
    />
    <WebcamSettingsRow
      enabled={settings.webcam.enabled}
      disabled={captureActive || (!settings.webcam.cameraId && !settings.webcam.enabled)}
      disabledReason={captureActive ? "Use the recording toolbar to toggle webcam while recording." : "Choose a camera in Webcam Overlay settings first."}
      onToggle={(enabled) => onUpdate({ webcam: {
        ...settings.webcam,
        enabled,
        ...(enabled ? { position: "bottom-right", x: 1, y: 1 } : {})
      } })}
      onOpen={onOpenWebcam}
    />
    <SettingToggleRow
      id="hold-to-zoom"
      label="Hold-to-zoom"
      description="Zoom to the cursor while the shortcut is held; available for screen and window recordings."
      icon={ZoomIn}
      value={settings.zoomOnHoldEnabled}
      disabled={!zoomFollowCursorSupported || captureActive}
      disabledReason={captureActive ? "Recording preferences apply to the next session." : zoomFollowCursorMessage}
      onChange={(value) => onUpdate({ zoomOnHoldEnabled: value })}
    />
    <ZoomShortcutField
      accelerator={settings.zoomOnHoldShortcut}
      status={holdZoomStatus}
      disabled={!settings.zoomOnHoldEnabled || !zoomFollowCursorSupported || captureActive}
      onChange={(value) => onUpdate({ zoomOnHoldShortcut: value })}
    />
    <SettingToggleRow
      id="snap-system-audio"
      label="System audio"
      icon={Volume2}
      value={settings.systemAudioEnabled}
      disabled={!systemAudioSupported}
      disabledReason={systemAudioMessage}
      onChange={(value) => onUpdate({ systemAudioEnabled: value })}
    />
    <SettingToggleRow
      id="snap-microphone"
      label="Microphone"
      icon={Mic}
      value={settings.microphoneEnabled}
      disabled={!microphoneSupported}
      disabledReason={microphoneMessage}
      onChange={(value) => onUpdate({ microphoneEnabled: value })}
    />
    <MicrophoneDeviceSelect
      selectedDeviceId={settings.microphoneDeviceId}
      disabled={saving || captureActive || !microphoneSupported}
      onChange={(microphoneDeviceId) => onUpdate({ microphoneDeviceId })}
    />
    <p class="settings-info">
      Microphone and system audio are mixed into one track in the saved video. Use headphones if your microphone picks up sound from speakers to prevent echo.
    </p>
    <AudioInputTestPanel
      microphoneEnabled={settings.microphoneEnabled}
      systemAudioEnabled={settings.systemAudioEnabled}
      microphoneDeviceId={settings.microphoneDeviceId}
      {microphoneSupported}
      {systemAudioSupported}
      disabled={saving || captureActive}
    />
    <SettingSelectRow
      id="default-quality"
      label="Default quality"
      icon={Monitor}
      value={String(settings.maxHeight)}
      choices={qualityChoices}
      placeholder="1080p (FHD)"
      disabled={maxResolutionHeight === null}
      onChange={(value) => onUpdate({ maxHeight: Number(value) as 720 | 1080 | 2160 })}
    />
    <SettingSelectRow
      id="default-fps"
      label="Default FPS"
      icon={Gauge}
      value={String(settings.fps)}
      choices={fpsChoices}
      placeholder="30 FPS"
      disabled={availableFps.length === 0}
      onChange={(value) => onUpdate({ fps: Number(value) as 30 | 60 })}
    />
  </div>

  <div class="settings-divider" aria-hidden="true"></div>
  <section class="settings-section screenshot-folder-section" aria-label="Screenshot preferences">
    <h3 class="settings-section-title">Screenshot</h3>
    <div class="settings-rows">
      <SettingToggleRow
        id="show-screenshot-preview"
        label="Show screenshot preview"
        description="Open the screenshot editor after each capture."
        icon={Camera}
        value={settings.showScreenshotPreview}
        disabled={saving}
        onChange={(value) => onUpdate({ showScreenshotPreview: value })}
      />
    </div>
    <FolderPickerRow
      id="screenshot-output-directory"
      label="Default save folder"
      path={settings.screenshotOutputDirectory}
      disabled={saving}
      onChoose={() => onChooseFolder("screenshot")}
    />
  </section>
</section>
