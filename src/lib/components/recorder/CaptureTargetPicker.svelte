<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import CaptureTargetCards from "./CaptureTargetCards.svelte";
  import CaptureSourceDialog from "./CaptureSourceDialog.svelte";
  import {
    formatCaptureError,
    listDisplays,
    openRegionPicker,
    openWindowPicker
  } from "../../services/ipc/capture.js";
  import type { CaptureTarget } from "../../types/recording.js";
  import type { DisplaySource, WindowSource } from "../../types/capture.js";

  export type CaptureTargetChoice = "screen" | "window" | "region";

  const REGION_SELECTED_EVENT = "capture://region-selected";

  let {
    value,
    desktopApp,
    onSelect,
    onTargetChange
  }: {
    value: CaptureTargetChoice;
    desktopApp: boolean;
    onSelect: (target: CaptureTargetChoice) => void;
    onTargetChange: (target: CaptureTarget | null) => void;
  } = $props();

  let displays = $state<DisplaySource[]>([]);
  let windows = $state<WindowSource[]>([]);
  let selectedDisplayId = $state("");
  let selectedTarget = $state<CaptureTarget | null>(null);
  let selectedWindowTitle = $state("");
  let regionSize = $state<{ width: number; height: number } | null>(null);
  let pickerOpen = $state(false);
  let pickerKind = $state<"display" | "window">("display");
  let windowPickerPending = $state(false);
  let regionPickerPending = $state(false);
  let previousPickerChoice = $state<CaptureTargetChoice>("screen");
  let previousPickerError = $state<string | null>(null);
  let sourceError = $state<string | null>(null);

  $effect(() => {
    if (!desktopApp) return;

    let disposed = false;
    const unlisteners: Array<() => void> = [];
    void listen<CaptureTarget>(REGION_SELECTED_EVENT, ({ payload }) => {
      if (payload.kind !== "region") return;
      selectedTarget = payload;
      regionSize = { width: payload.width, height: payload.height };
      sourceError = null;
      onTargetChange(payload);
    }).then((stop) => {
      if (disposed) stop();
      else unlisteners.push(stop);
    }).catch((error: unknown) => {
      if (!disposed) sourceError = `Could not listen for the region result: ${formatCaptureError(error)}`;
    });

    void listen<WindowSource>("capture://window-selected", ({ payload }) => {
      windowPickerPending = false;
      applyWindowSelection(payload);
    }).then((stop) => {
      if (disposed) stop();
      else unlisteners.push(stop);
    }).catch((error: unknown) => {
      if (!disposed) sourceError = `Could not listen for the window selection: ${formatCaptureError(error)}`;
    });

    void listen("capture://window-picker-cancelled", () => {
      windowPickerPending = false;
      onSelect(previousPickerChoice);
      sourceError = previousPickerError;
    }).then((stop) => {
      if (disposed) stop();
      else unlisteners.push(stop);
    }).catch((error: unknown) => {
      if (!disposed) sourceError = `Could not listen for the window picker: ${formatCaptureError(error)}`;
    });

    void listDisplays().then((sources) => {
      if (disposed) return;
      displays = sources;
      const primary = sources.find((display) => display.primary) ?? sources[0];
      selectedDisplayId = primary?.displayId ?? "";
      if (!primary) sourceError = "No displays are currently available.";
      else if (value === "screen") setDisplayTarget(primary);
    }).catch((error: unknown) => {
      if (!disposed) sourceError = `Could not list displays: ${formatCaptureError(error)}`;
    });

    return () => {
      disposed = true;
      unlisteners.forEach((stop) => stop());
    };
  });

  $effect(() => {
    if (value === "screen" && selectedDisplayId) {
      const display = displays.find((item) => item.displayId === selectedDisplayId);
      if (display && (selectedTarget?.kind !== "display" || selectedTarget.displayId !== display.displayId)) {
        setDisplayTarget(display);
      }
    }
  });

  const sourceMessage = $derived.by(() => {
    if (sourceError) return sourceError;
    if (windowPickerPending && value === "window") return "Opening the native window picker…";
    if (regionPickerPending && value === "region") return "Preparing the region picker…";
    if (selectedTarget?.kind === "display") {
      const displayId = selectedTarget.displayId;
      const display = displays.find((item) => item.displayId === displayId);
      return display
        ? `${display.name} · ${display.width} × ${display.height} · ${Math.round(display.scaleFactor * 100)}%`
        : "The selected display is no longer available.";
    }
    if (selectedTarget?.kind === "window") return `Window: ${selectedWindowTitle}`;
    if (selectedTarget?.kind === "region" && regionSize) {
      return `Region: ${regionSize.width} × ${regionSize.height} px`;
    }
    if (value === "window") return "Choose a visible window.";
    if (value === "region") return "Choose a display, then drag to select a region.";
    if (desktopApp && displays.length > 0) return "Select a display to continue.";
    return desktopApp ? "Loading available displays…" : "Source selection is available in the desktop app.";
  });

  async function chooseScreen() {
    onSelect("screen");
    sourceError = null;
    selectedTarget = null;
    regionSize = null;
    selectedWindowTitle = "";
    if (!desktopApp) {
      onTargetChange(null);
      sourceError = "Native display listing is available only in the desktop app.";
      return;
    }
    try {
      displays = await listDisplays();
    } catch (error) {
      onTargetChange(null);
      sourceError = formatCaptureError(error);
      return;
    }
    const current = displays.find((display) => display.displayId === selectedDisplayId);
    const display = current ?? displays.find((item) => item.primary) ?? displays[0];
    if (!display) {
      onTargetChange(null);
      sourceError = "No displays are currently available.";
      return;
    }
    selectedDisplayId = display.displayId;
    if (displays.length > 1) {
      onTargetChange(null);
      pickerKind = "display";
      pickerOpen = true;
      return;
    }
    setDisplayTarget(display);
  }

  async function chooseWindow() {
    if (windowPickerPending || regionPickerPending) return;
    previousPickerChoice = value;
    previousPickerError = sourceError;
    sourceError = null;
    onSelect("window");
    windowPickerPending = true;
    try {
      await openWindowPicker();
    } catch (error) {
      windowPickerPending = false;
      onSelect(previousPickerChoice);
      sourceError = `Could not open the window picker: ${formatCaptureError(error)}`;
    }
  }

  async function chooseRegion() {
    if (windowPickerPending || regionPickerPending) return;
    onSelect("region");
    onTargetChange(null);
    selectedTarget = null;
    selectedWindowTitle = "";
    regionSize = null;
    sourceError = null;
    regionPickerPending = true;
    try {
      displays = await listDisplays();
      const display = displays.find((item) => item.displayId === selectedDisplayId)
        ?? displays.find((item) => item.primary)
        ?? displays[0];
      if (!display) throw new Error("No displays are currently available.");
      selectedDisplayId = display.displayId;
      await openRegionPicker(display.displayId);
    } catch (error) {
      sourceError = `Could not open the region picker: ${formatCaptureError(error)}`;
    } finally {
      regionPickerPending = false;
    }
  }

  function setDisplayTarget(display: DisplaySource) {
    selectedDisplayId = display.displayId;
    const target: CaptureTarget = { kind: "display", displayId: display.displayId };
    selectedTarget = target;
    sourceError = null;
    onTargetChange(target);
  }

  function selectDisplay(display: DisplaySource) {
    setDisplayTarget(display);
    pickerOpen = false;
  }

  function applyWindowSelection(source: WindowSource) {
    const target: CaptureTarget = { kind: "window", windowId: source.windowId };
    selectedTarget = target;
    selectedWindowTitle = source.title;
    regionSize = null;
    sourceError = null;
    onTargetChange(target);
    onSelect("window");
  }

  function closePicker() {
    pickerOpen = false;
  }

</script>

<CaptureTargetCards
  {value}
  sourceMessage={sourceMessage}
  sourceError={Boolean(sourceError)}
  onChooseScreen={chooseScreen}
  onChooseWindow={chooseWindow}
  onChooseRegion={chooseRegion}
/>

<CaptureSourceDialog
  open={pickerOpen}
  kind={pickerKind}
  {displays}
  {windows}
  loading={false}
  error={sourceError}
  onClose={closePicker}
  onSelectDisplay={selectDisplay}
  onSelectWindow={applyWindowSelection}
/>

