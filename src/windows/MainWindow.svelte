<script lang="ts">
  import { onMount, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import "./main-window.css";
  import Camera from "@lucide/svelte/icons/camera";
  import CircleDot from "@lucide/svelte/icons/circle-dot";
  import { Button } from "../lib/components/ui/button/index.js";
  import AudioOptions from "../lib/components/recorder/AudioOptions.svelte";
  import CaptureTargetPicker, {
    type CaptureTargetChoice
  } from "../lib/components/recorder/CaptureTargetPicker.svelte";
  import ModeSelector from "../lib/components/recorder/ModeSelector.svelte";
  import ScreenshotOptions from "../lib/components/recorder/ScreenshotOptions.svelte";
  import SettingsView from "../lib/components/settings/SettingsView.svelte";
  import AboutView from "../lib/components/about/AboutView.svelte";
  import UpdateView from "../lib/components/shared/UpdateView.svelte";
  import LibraryView from "../features/library/components/LibraryView.svelte";
  import WindowTitleBar from "../lib/components/recorder/WindowTitleBar.svelte";
  import ActiveRecordingPanel from "../lib/components/recorder/ActiveRecordingPanel.svelte";
  import VideoOptions from "../lib/components/recorder/VideoOptions.svelte";
  import { getCaptureCapabilities, getCapturePermissions } from "../lib/services/ipc/capture.js";
  import {
    getCaptureHotkeyStatus,
    getHoldZoomShortcutStatus,
    getQuickAccessShortcutStatus
  } from "../lib/services/ipc/shortcuts.js";
  import { errorMessage } from "../lib/services/ipc/settings.js";
  import { notifyApp } from "../lib/services/notifications.js";
  import { captureScreenshot } from "../lib/services/ipc/screenshot.js";
  import { getRecordingPreflight, startRecording } from "../lib/services/ipc/recording.js";
  import { preferences } from "../lib/stores/preferences.svelte.js";
  import { recordingSession } from "../lib/stores/recording-session.svelte.js";
  import type { CaptureCapabilities, PermissionInfo } from "../lib/types/capture.js";
  import type {
    CaptureTarget,
    RecordingOptions,
    RecordingPreflightReport
  } from "../lib/types/recording.js";
  import type { SettingsPatch } from "../lib/types/settings.js";
  import type { SettingsTab } from "../lib/types/settings-view.js";
  import type {
    CaptureHotkeyAction,
    CaptureHotkeyRequest,
    CaptureHotkeyStatus,
    HoldZoomShortcutStatus,
    QuickAccessShortcutStatus
  } from "../lib/types/shortcuts.js";

  type CaptureMode = "record" | "screenshot";
  type Quality = "720p" | "1080p" | "2160p";
  type Fps = "30" | "60";
  type ImageFormat = "png" | "jpg";

  let mode = $state<CaptureMode>("record");
  let target = $state<CaptureTargetChoice>("screen");
  let quality = $state<Quality>("1080p");
  let fps = $state<Fps>("30");
  let microphoneEnabled = $state(false);
  let systemAudioEnabled = $state(false);
  let showMouseCursor = $state(true);
  let format = $state<ImageFormat>("png");
  let copyToClipboard = $state(true);
  let saveToFile = $state(true);
  let settingsOpen = $state(false);
  let aboutOpen = $state(false);
  let updateOpen = $state(false);
  let libraryOpen = $state(false);
  let settingsTab = $state<SettingsTab>("snap");
  let startupLabel = $state("Start with system");
  let shortcutStatus = $state<QuickAccessShortcutStatus | null>(null);
  let captureHotkeyStatus = $state<CaptureHotkeyStatus[] | null>(null);
  let holdZoomStatus = $state<HoldZoomShortcutStatus | null>(null);
  let shortcutError = $state<string | null>(null);
  let settingsFormLoaded = $state(false);
  let capabilities = $state<CaptureCapabilities | null>(null);
  let permissions = $state<PermissionInfo[] | null>(null);
  let capabilityError = $state<string | null>(null);
  let actionPending = $state(false);
  let preflightLoading = $state(false);
  let preflightError = $state<string | null>(null);
  let preflightReport = $state<RecordingPreflightReport | null>(null);
  let preflightRequestId = 0;
  let selectedCaptureTarget = $state<CaptureTarget | null>(null);
  let hotkeyCaptureRequest = $state<CaptureHotkeyRequest | null>(null);
  let pendingCaptureHotkey: CaptureHotkeyRequest | null = null;
  let queuedCaptureHotkey: CaptureHotkeyAction | null = null;
  let hotkeyRequestId = 0;
  let formRevision = 0;
  let lastFormSignature = "";
  let lastNotifiedSessionRevision = -1;
  let lastNotifiedSessionError: string | null = null;
  let lastNotifiedRecordingError: string | null = null;

  function openSettings() {
    libraryOpen = false;
    updateOpen = false;
    aboutOpen = false;
    settingsOpen = true;
    shortcutError = null;
    startupLabel = systemStartupLabel();
    if (!preferences.desktopApp) return;
    void getQuickAccessShortcutStatus()
      .then((status) => (shortcutStatus = status))
      .catch((error: unknown) => (shortcutError = errorMessage(error)));
    void getCaptureHotkeyStatus()
      .then((status) => (captureHotkeyStatus = status))
      .catch((error: unknown) => console.error(`[RecoDex] Capture hotkey status unavailable: ${errorMessage(error)}`));
    void getHoldZoomShortcutStatus()
      .then((status) => (holdZoomStatus = status))
      .catch((error: unknown) => console.error(`[RecoDex] Hold-to-zoom status unavailable: ${errorMessage(error)}`));
  }

  function openUpdate() {
    settingsOpen = false;
    aboutOpen = false;
    libraryOpen = false;
    updateOpen = true;
  }

  function openLibrary() {
    settingsOpen = false;
    aboutOpen = false;
    updateOpen = false;
    libraryOpen = true;
  }

  function openAbout() {
    settingsOpen = false;
    updateOpen = false;
    libraryOpen = false;
    aboutOpen = true;
  }

  function systemStartupLabel(): string {
    if (typeof navigator === "undefined") return "Start at login";
    return /windows/i.test(navigator.platform) ? "Start with Windows" : "Start at login";
  }

  function maxHeightForQuality(value: Quality): 720 | 1080 | 2160 {
    if (value === "720p") return 720;
    if (value === "2160p") return 2160;
    return 1080;
  }

  function qualityForHeight(value: number): Quality {
    if (value === 720) return "720p";
    if (value === 2160) return "2160p";
    return "1080p";
  }

  function formPatch(): SettingsPatch {
    return {
      mode,
      captureTarget: target,
      maxHeight: maxHeightForQuality(quality),
      fps: Number(fps) as 30 | 60,
      screenshotFormat: format,
      screenshotCopyToClipboard: copyToClipboard,
      screenshotSaveToFile: saveToFile,
      microphoneEnabled,
      systemAudioEnabled,
      showMouseCursor
    };
  }

  const sessionStatus = $derived(recordingSession.snapshot?.status ?? "idle");
  const sessionActive = $derived(!["idle", "completed", "failed"].includes(sessionStatus));
  const canTakeScreenshot = $derived.by(() => {
    const settings = preferences.settings;
    const statusAllowsCapture = ["idle", "completed", "failed"].includes(sessionStatus);
    const screenshotReady = capabilities?.screenshotCapture.status === "available"
      || capabilities?.screenshotCapture.status === "limited";
    return mode === "screenshot" && statusAllowsCapture && Boolean(selectedCaptureTarget)
      && settings !== null && screenshotReady
      && (settings.showScreenshotPreview || copyToClipboard || saveToFile)
      && !actionPending;
  });

  const canStartRecording = $derived.by(() => {
    const status = recordingSession.snapshot?.status ?? "idle";
    const statusAllowsStart = status === "idle" || status === "completed" || status === "failed";
    const videoReady = capabilities?.videoCapture.status === "available"
      || capabilities?.videoCapture.status === "limited";
    return mode === "record" && statusAllowsStart && Boolean(selectedCaptureTarget)
      && Boolean(preferences.settings) && videoReady && !actionPending;
  });

  function recordingOptions(): RecordingOptions | null {
    const settings = preferences.settings;
    const captureTarget = selectedCaptureTarget;
    if (!settings || !captureTarget) return null;
    return {
      target: captureTarget,
      fps: settings.fps,
      maxHeight: settings.maxHeight,
      microphoneEnabled,
      microphoneDeviceId: settings.microphoneDeviceId || undefined,
      systemAudio: systemAudioEnabled,
      showMouseCursor: settings.showMouseCursor,
      zoomOnHoldEnabled: settings.zoomOnHoldEnabled,
      showPreviewAfterRecording: settings.showRecordingPreview,
      outputDirectory: settings.videoOutputDirectory
    };
  }

  function invalidateRecordingCheck() {
    preflightRequestId += 1;
    preflightLoading = false;
    preflightError = null;
    preflightReport = null;
  }

  async function checkRecordingSetup() {
    if (preflightLoading) return;
    const options = recordingOptions();
    invalidateRecordingCheck();
    if (!options) {
      preflightError = "Select a capture source in the main window before checking recording setup.";
      return;
    }
    const requestId = preflightRequestId;
    preflightLoading = true;
    try {
      const report = await getRecordingPreflight(options);
      if (requestId === preflightRequestId) preflightReport = report;
    } catch (error) {
      if (requestId === preflightRequestId) preflightError = errorMessage(error);
    } finally {
      if (requestId === preflightRequestId) preflightLoading = false;
    }
  }

  async function startCapture() {
    const options = recordingOptions();
    if (!options || !canStartRecording || actionPending) return;
    invalidateRecordingCheck();
    actionPending = true;
    lastNotifiedRecordingError = null;
    try {
      await startRecording(options);
    } catch (error) {
      notifyRecordingFailure(errorMessage(error));
    } finally {
      actionPending = false;
    }
  }

  async function takeScreenshot() {
    if (!canTakeScreenshot || !selectedCaptureTarget) return;
    actionPending = true;
    try {
      const result = await captureScreenshot({
        target: selectedCaptureTarget,
        format,
        copyToClipboard,
        saveToFile
      });
      if (result.editorSessionId) return;
      notifyUser("Screenshot captured", screenshotNotice(result));
    } catch (error) {
      notifyUser("Screenshot failed", errorMessage(error));
    } finally {
      actionPending = false;
    }
  }

  function beginHotkeyCapture(action: CaptureHotkeyAction) {
    if (pendingCaptureHotkey || queuedCaptureHotkey) {
      notifyUser("Capture shortcut unavailable", "Finish or cancel the current source selection first.");
      return;
    }
    if (sessionActive || actionPending) {
      notifyUser("Capture shortcut unavailable", "Finish the current capture before starting another one.");
      return;
    }
    if (!settingsFormLoaded || !preferences.settings) {
      queuedCaptureHotkey = action;
      return;
    }

    settingsOpen = false;
    aboutOpen = false;
    mode = action.startsWith("record-") ? "record" : "screenshot";
    target = action.endsWith("-window") ? "window" : action.endsWith("-region") ? "region" : "screen";
    const request = { id: ++hotkeyRequestId, action };
    pendingCaptureHotkey = request;
    hotkeyCaptureRequest = request;
  }

  function completeHotkeyCapture(requestId: number) {
    if (pendingCaptureHotkey?.id !== requestId) return;
    const { action } = pendingCaptureHotkey;
    pendingCaptureHotkey = null;
    hotkeyCaptureRequest = null;
    void runHotkeyCapture(action);
  }

  function cancelHotkeyCapture(requestId: number) {
    if (pendingCaptureHotkey?.id !== requestId) return;
    pendingCaptureHotkey = null;
    hotkeyCaptureRequest = null;
  }

  async function runHotkeyCapture(action: CaptureHotkeyAction) {
    await tick();
    if (!capabilities) {
      try {
        const [nextCapabilities, nextPermissions] = await Promise.all([
          getCaptureCapabilities(),
          getCapturePermissions()
        ]);
        capabilities = nextCapabilities;
        permissions = nextPermissions;
      } catch (error) {
        notifyUser("Capture failed", errorMessage(error));
        return;
      }
    }
    await tick();
    if (action.startsWith("record-")) {
      if (!canStartRecording) {
        notifyUser("Recording unavailable", captureMessage);
        return;
      }
      await startCapture();
      return;
    }
    if (!canTakeScreenshot) {
      notifyUser("Screenshot unavailable", captureMessage);
      return;
    }
    await takeScreenshot();
  }

  function isCaptureHotkeyAction(value: string): value is CaptureHotkeyAction {
    return [
      "record-screen", "record-window", "record-region",
      "screenshot-screen", "screenshot-window", "screenshot-region"
    ].includes(value);
  }

  function screenshotNotice(result: {
    width: number;
    height: number;
    outputPath: string | null;
    copiedToClipboard: boolean;
    warning: string | null;
  }): string {
    const destinations = [
      result.outputPath ? `Saved to ${result.outputPath}` : null,
      result.copiedToClipboard ? "copied to clipboard" : null
    ].filter(Boolean);
    const warning = result.warning ? ` ${result.warning}` : "";
    return `Captured ${result.width} × ${result.height}px; ${destinations.join(" and ")}.${warning}`;
  }

  function notifyRecordingFailure(message: string) {
    if (lastNotifiedRecordingError === message) return;
    lastNotifiedRecordingError = message;
    notifyUser("Recording failed", message);
  }

  function setFormFromSettings(value: NonNullable<typeof preferences.settings>) {
    mode = value.mode;
    target = value.captureTarget;
    quality = qualityForHeight(value.maxHeight);
    fps = String(value.fps) as Fps;
    microphoneEnabled = value.microphoneEnabled;
    systemAudioEnabled = value.systemAudioEnabled;
    showMouseCursor = value.showMouseCursor;
    format = value.screenshotFormat;
    copyToClipboard = value.screenshotCopyToClipboard;
    saveToFile = value.screenshotSaveToFile;
    lastFormSignature = JSON.stringify(formPatch());
  }

  function applySettingsPatchToForm(patch: SettingsPatch) {
    if (patch.mode !== undefined) mode = patch.mode;
    if (patch.captureTarget !== undefined) target = patch.captureTarget;
    if (patch.maxHeight !== undefined) quality = qualityForHeight(patch.maxHeight);
    if (patch.fps !== undefined) fps = String(patch.fps) as Fps;
    if (patch.microphoneEnabled !== undefined) microphoneEnabled = patch.microphoneEnabled;
    if (patch.systemAudioEnabled !== undefined) systemAudioEnabled = patch.systemAudioEnabled;
    if (patch.showMouseCursor !== undefined) showMouseCursor = patch.showMouseCursor;
    if (patch.screenshotFormat !== undefined) format = patch.screenshotFormat;
    if (patch.screenshotCopyToClipboard !== undefined) copyToClipboard = patch.screenshotCopyToClipboard;
    if (patch.screenshotSaveToFile !== undefined) saveToFile = patch.screenshotSaveToFile;
  }

  async function updateFromSettings(patch: SettingsPatch): Promise<boolean> {
    applySettingsPatchToForm(patch);
    invalidateRecordingCheck();
    formRevision += 1;
    lastFormSignature = JSON.stringify(formPatch());
    const saved = patch.startWithSystem === undefined
      ? await preferences.update(patch)
      : await preferences.updateStartWithSystem(patch.startWithSystem);
    if (preferences.settings) setFormFromSettings(preferences.settings);
    if (saved && (patch.zoomOnHoldEnabled !== undefined || patch.zoomOnHoldShortcut !== undefined)) {
      try {
        holdZoomStatus = await getHoldZoomShortcutStatus();
      } catch (error) {
        console.error(`[RecoDex] Hold-to-zoom status unavailable: ${errorMessage(error)}`);
      }
    }
    return saved;
  }

  function notifyUser(title: string, body: string) {
    if (preferences.settings?.showNotifications === false) return;
    void notifyApp(title, body);
  }

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let unlistenWebcamWarning: (() => void) | undefined;
    let unlistenCaptureHotkey: (() => void) | undefined;

    void preferences.initialize().then(() => {
      if (!disposed && preferences.settings) {
        setFormFromSettings(preferences.settings);
        settingsFormLoaded = true;
        const queued = queuedCaptureHotkey;
        queuedCaptureHotkey = null;
        if (queued) beginHotkeyCapture(queued);
      } else if (!disposed && queuedCaptureHotkey) {
        queuedCaptureHotkey = null;
        notifyUser("Capture shortcut unavailable", preferences.error ?? "RecoDex settings are not ready yet.");
      }
    });
    void recordingSession.connect().then((stopWatching) => {
        if (disposed) stopWatching();
        else unlisten = stopWatching;
      });
    if (preferences.desktopApp) {
      void listen<string>("capture://hotkey", ({ payload }) => {
        if (isCaptureHotkeyAction(payload)) beginHotkeyCapture(payload);
      }).then((stopListening) => {
        if (disposed) stopListening();
        else unlistenCaptureHotkey = stopListening;
      }).catch((error: unknown) => console.error(`[RecoDex] Capture hotkey listener unavailable: ${errorMessage(error)}`));
    }
    if (preferences.desktopApp) {
      void listen<string>("webcam://recording-warning", ({ payload }) => {
        notifyUser("Webcam overlay", payload);
      }).then((stopListening) => {
        if (disposed) stopListening();
        else unlistenWebcamWarning = stopListening;
      }).catch((error: unknown) => console.error(`[RecoDex] Webcam warning listener unavailable: ${errorMessage(error)}`));
    }
    if (preferences.desktopApp) {
      void Promise.all([getCaptureCapabilities(), getCapturePermissions()])
        .then(([nextCapabilities, nextPermissions]) => {
          if (disposed) return;
          capabilities = nextCapabilities;
          permissions = nextPermissions;
        })
        .catch((error: unknown) => {
          if (disposed) return;
          capabilityError = errorMessage(error);
          console.error(`[RecoDex] Capture capabilities unavailable: ${capabilityError}`);
        });
    }

    return () => {
      disposed = true;
      unlisten?.();
      unlistenWebcamWarning?.();
      unlistenCaptureHotkey?.();
    };
  });

  $effect(() => {
    if (!settingsFormLoaded || !preferences.ready || !preferences.desktopApp) return;
    const patch = formPatch();
    const signature = JSON.stringify(patch);
    if (signature === lastFormSignature) return;
    lastFormSignature = signature;
    formRevision += 1;
    const revision = formRevision;
    void preferences.update(patch).then((saved) => {
      if (!saved && revision === formRevision && preferences.settings) {
        setFormFromSettings(preferences.settings);
      }
    });
  });

  $effect(() => {
    const snapshot = recordingSession.snapshot;
    if (snapshot && snapshot.revision > lastNotifiedSessionRevision) {
      lastNotifiedSessionRevision = snapshot.revision;
      if (snapshot.status === "completed") {
        lastNotifiedRecordingError = null;
        if (snapshot.outputPath) {
          notifyUser("Recording saved", `Saved to ${snapshot.outputPath}`);
        } else {
          notifyUser("Recording discarded", "The recording was deleted.");
        }
      } else if (snapshot.status === "failed" && snapshot.error) {
        notifyRecordingFailure(snapshot.error.message);
      }
    }

    const sessionError = recordingSession.error;
    if (!sessionError) {
      lastNotifiedSessionError = null;
      return;
    }
    if (sessionError === lastNotifiedSessionError) return;
    lastNotifiedSessionError = sessionError;
    notifyUser("Recorder status unavailable", sessionError);
  });

  const captureMessage = $derived.by(() => {
    if (capabilityError) return `Capture status is unavailable: ${capabilityError}`;
    if (mode === "screenshot" && !copyToClipboard && !saveToFile) {
      return "Choose at least one screenshot destination.";
    }
    if (!selectedCaptureTarget) {
      if (target === "window") return "Choose a visible window to continue.";
      if (target === "region") return "Select a display and drag a region to continue.";
      return "Choose an available display to continue.";
    }
    const feature = mode === "record"
      ? capabilities?.videoCapture
      : capabilities?.screenshotCapture;
    if (!feature) return preferences.desktopApp ? "Loading native capture capabilities…" : "Capture is available in the desktop app.";

    const permission = permissions?.find((item) => item.kind === "screenRecording");
    return permission ? `${feature.message} ${permission.message}` : feature.message;
  });
</script>

<main
  class="main-window"
  class:session-active={sessionActive}
  class:settings-open={settingsOpen}
  class:about-open={aboutOpen}
  class:update-open={updateOpen}
  class:library-open={libraryOpen}
  data-tauri-drag-region="false"
  aria-labelledby="main-title"
>
  <WindowTitleBar
    desktopApp={preferences.desktopApp}
    settingsActive={settingsOpen || aboutOpen || updateOpen}
    libraryActive={libraryOpen}
    minimizeToTray={preferences.settings?.minimizeToTray ?? true}
    settingsDisabled={!preferences.desktopApp || !preferences.settings}
    libraryDisabled={!preferences.desktopApp || sessionActive}
    settingsTitle={preferences.error ?? "Recording and screenshot preferences."}
    onSettings={openSettings}
    onAbout={openAbout}
    onLibrary={openLibrary}
    onUpdate={openUpdate}
    onBack={() => {
      settingsOpen = false;
      aboutOpen = false;
      updateOpen = false;
      libraryOpen = false;
    }}
    onWindowError={(message) => console.error(`[RecoDex] Window control error: ${message}`)}
  />

  {#if updateOpen}
    <UpdateView />
  {:else if libraryOpen}
    <LibraryView onBack={() => (libraryOpen = false)} />
  {:else if aboutOpen}
    <AboutView />
  {:else if settingsOpen}
    <SettingsView
      bind:tab={settingsTab}
      settings={preferences.settings}
      saving={preferences.pendingWrites > 0}
      error={preferences.error}
      {shortcutStatus}
      {captureHotkeyStatus}
      {shortcutError}
      {startupLabel}
      startupSupported={preferences.autoStartAvailable}
      startupError={preferences.autoStartError}
      traySupported={preferences.desktopApp}
      maxResolutionHeight={capabilities?.maxResolutionHeight ?? null}
      availableFps={capabilities?.availableFps ?? []}
      captureActive={sessionActive}
      cursorSupported={capabilities?.videoCapture.status === "available" || capabilities?.videoCapture.status === "limited"}
      zoomFollowCursorSupported={capabilities?.zoomFollowCursor.status === "available" || capabilities?.zoomFollowCursor.status === "limited"}
      zoomFollowCursorMessage={capabilities?.zoomFollowCursor.message ?? "Waiting for native zoom capability information."}
      {holdZoomStatus}
      microphoneSupported={capabilities?.microphoneCapture.status === "available" || capabilities?.microphoneCapture.status === "limited"}
      microphoneMessage={capabilities?.microphoneCapture.message ?? "Waiting for native audio capabilities."}
      systemAudioSupported={capabilities?.systemAudioCapture.status === "available" || capabilities?.systemAudioCapture.status === "limited"}
      systemAudioMessage={capabilities?.systemAudioCapture.message ?? "Waiting for native audio capabilities."}
      {preflightLoading}
      {preflightError}
      {preflightReport}
      onUpdate={updateFromSettings}
      onCheckRecording={checkRecordingSetup}
      onChooseFolder={(kind) => { void preferences.chooseOutputFolder(kind); }}
    />
  {:else if sessionActive}
    <ActiveRecordingPanel />
  {:else}
    <ModeSelector bind:value={mode} />

    <CaptureTargetPicker
      value={target}
      hotkeyRequest={hotkeyCaptureRequest}
      onSelect={(next) => (target = next)}
      desktopApp={preferences.desktopApp}
      onTargetChange={(next) => {
        selectedCaptureTarget = next;
        invalidateRecordingCheck();
      }}
      onHotkeyTargetSelected={completeHotkeyCapture}
      onHotkeyRequestCancelled={cancelHotkeyCapture}
    />

    {#if mode === "record"}
      <div class="record-options">
        <AudioOptions
          bind:systemAudioEnabled
          bind:microphoneEnabled
          systemAudioAvailable={capabilities?.systemAudioCapture.status === "available" || capabilities?.systemAudioCapture.status === "limited"}
          microphoneAvailable={capabilities?.microphoneCapture.status === "available" || capabilities?.microphoneCapture.status === "limited"}
          systemAudioMessage={capabilities?.systemAudioCapture.message ?? "Waiting for native audio capabilities."}
          microphoneMessage={capabilities?.microphoneCapture.message ?? "Waiting for native audio capabilities."}
        />
        <VideoOptions
          bind:quality
          bind:fps
          maxResolutionHeight={capabilities?.maxResolutionHeight ?? null}
          availableFps={capabilities?.availableFps ?? []}
        />
      </div>
    {:else}
      <ScreenshotOptions bind:format bind:copyToClipboard bind:saveToFile />
    {/if}

    <Button
      type="button"
      disabled={mode === "record" ? !canStartRecording : !canTakeScreenshot}
      aria-describedby="capture-unavailable"
      title={actionPending ? (mode === "record" ? "Starting recording…" : "Capturing screenshot…") : captureMessage}
      class="primary-action"
      onclick={mode === "record" ? startCapture : takeScreenshot}
    >
      {#if mode === "record"}
        <CircleDot aria-hidden="true" class="size-5" />
        <span>{actionPending ? "Starting…" : "Start Recording"}</span>
      {:else}
        <Camera aria-hidden="true" class="size-5" />
        <span>{actionPending ? "Capturing…" : "Take Screenshot"}</span>
      {/if}
    </Button>
    <p id="capture-unavailable" class="sr-only">{captureMessage}</p>
  {/if}

</main>

