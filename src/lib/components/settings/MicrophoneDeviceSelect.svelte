<script lang="ts">
  import { onMount } from "svelte";
  import Mic from "@lucide/svelte/icons/mic";
  import SettingSelectRow from "./SettingSelectRow.svelte";
  import type { AudioDevice } from "../../types/capture.js";
  import { listAudioDevices } from "../../services/ipc/capture.js";

  const WINDOWS_DEFAULT = "windows-default";

  let {
    selectedDeviceId,
    disabled = false,
    onChange
  }: {
    selectedDeviceId: string;
    disabled?: boolean;
    onChange: (deviceId: string) => Promise<boolean>;
  } = $props();

  let devices = $state<AudioDevice[]>([]);
  let devicesLoaded = $state(false);
  let loadError = $state<string | null>(null);

  onMount(() => {
    void listAudioDevices()
      .then((available) => {
        devices = available.filter((device) => device.source === "microphone");
      })
      .catch((error: unknown) => {
        loadError = error instanceof Error ? error.message : String(error);
      })
      .finally(() => {
        devicesLoaded = true;
      });
  });

  const selectedMicrophone = $derived(
    devices.find((device) => device.deviceId === selectedDeviceId) ??
      devices.find((device) => device.isDefault) ??
      null
  );
  const selectedInputMayContainPlayback = $derived(
    /stereo mix|what u hear|wave out mix|loopback/i.test(selectedMicrophone?.name ?? "")
  );

  const choices = $derived.by(() => {
    const result: { value: string; label: string; disabled?: boolean }[] = [
      { value: WINDOWS_DEFAULT, label: "Windows default input" }
    ];
    if (selectedDeviceId && !devices.some((device) => device.deviceId === selectedDeviceId)) {
      result.push({
        value: selectedDeviceId,
        label: "Saved microphone unavailable",
        disabled: true
      });
    }
    for (const device of devices) {
      result.push({
        value: device.deviceId,
        label: device.isDefault ? `${device.name} · Default` : device.name
      });
    }
    return result;
  });

  function selectDevice(deviceId: string): Promise<boolean> {
    return onChange(deviceId === WINDOWS_DEFAULT ? "" : deviceId);
  }
</script>

<SettingSelectRow
  id="microphone-device"
  label="Microphone input"
  icon={Mic}
  value={selectedDeviceId || WINDOWS_DEFAULT}
  {choices}
  placeholder="Windows default input"
  disabled={disabled || !devicesLoaded || (!loadError && devices.length === 0)}
  onChange={selectDevice}
/>
{#if loadError}
  <p class="settings-info" role="status">Could not list microphone inputs: {loadError}</p>
{:else if devicesLoaded && devices.length === 0}
  <p class="settings-info" role="status">No microphone input is currently available.</p>
{:else if selectedInputMayContainPlayback && selectedMicrophone}
  <p class="settings-info playback-mix-warning" role="alert">
    “{selectedMicrophone.name}” captures Windows playback audio as a microphone input. System audio can still be heard when its separate switch is off; choose a physical microphone to isolate voice.
  </p>
{:else}
  <p class="settings-info">Use a physical microphone; “Stereo Mix” or “What U Hear” inputs can carry system playback.</p>
{/if}
