<script lang="ts">
  import AudioLines from "@lucide/svelte/icons/audio-lines";
  import { errorMessage } from "../../services/ipc/settings.js";
  import { testAudioInputs } from "../../services/ipc/audio.js";
  import type { AudioInputTestResult } from "../../types/recording.js";

  let {
    microphoneEnabled,
    systemAudioEnabled,
    microphoneDeviceId,
    microphoneSupported,
    systemAudioSupported,
    disabled = false
  }: {
    microphoneEnabled: boolean;
    systemAudioEnabled: boolean;
    microphoneDeviceId: string | null;
    microphoneSupported: boolean;
    systemAudioSupported: boolean;
    disabled?: boolean;
  } = $props();

  let running = $state(false);
  let result = $state<AudioInputTestResult | null>(null);
  let error = $state<string | null>(null);
  let lastInputs = "";

  const hasSelectedInput = $derived(microphoneEnabled || systemAudioEnabled);
  const selectedInputsAvailable = $derived(
    (!microphoneEnabled || microphoneSupported) && (!systemAudioEnabled || systemAudioSupported)
  );

  function inputSignature(microphone: boolean, system: boolean, deviceId: string | null) {
    return `${microphone}:${system}:${deviceId ?? ""}`;
  }

  $effect(() => {
    const inputs = inputSignature(microphoneEnabled, systemAudioEnabled, microphoneDeviceId);
    if (lastInputs && lastInputs !== inputs) {
      result = null;
      error = null;
    }
    lastInputs = inputs;
  });

  async function runTest() {
    if (disabled || running || !hasSelectedInput || !selectedInputsAvailable) return;
    const microphone = microphoneEnabled;
    const system = systemAudioEnabled;
    const deviceId = microphoneDeviceId;
    const testedInputs = inputSignature(microphone, system, deviceId);
    running = true;
    result = null;
    error = null;
    try {
      const testResult = await testAudioInputs(microphone, system, deviceId || undefined);
      if (testedInputs === inputSignature(microphoneEnabled, systemAudioEnabled, microphoneDeviceId)) {
        result = testResult;
      }
    } catch (cause) {
      if (testedInputs === inputSignature(microphoneEnabled, systemAudioEnabled, microphoneDeviceId)) {
        error = errorMessage(cause);
      }
    } finally {
      running = false;
    }
  }

  function meterLabel(testResult: AudioInputTestResult, source: "microphone" | "system") {
    const level = source === "microphone" ? testResult.microphone : testResult.system;
    const packets = source === "microphone" ? testResult.microphonePackets : testResult.systemPackets;
    if (packets === 0) return "No audio packets received";
    if (!level || level.rmsPercent === 0) return "No signal detected";
    return `Signal ${level.rmsPercent}% · peak ${level.peakPercent}%`;
  }
</script>

<section class="audio-test" aria-label="Audio input signal test" aria-busy={running}>
  <div class="audio-test-heading">
    <div class="audio-test-title">
      <AudioLines aria-hidden="true" class="audio-test-icon" />
      <div>
        <strong>Input level test</strong>
        <span>Check the selected microphone and system audio.</span>
      </div>
    </div>
    <button type="button" class="test-button" disabled={disabled || running || !hasSelectedInput || !selectedInputsAvailable} onclick={runTest}>
      {running ? "Listening…" : "Test audio"}
    </button>
  </div>

  {#if running}
    <p class="test-message" role="status">Speak or play sound during the 2-second check…</p>
  {:else if disabled && hasSelectedInput}
    <p class="test-message" role="status">Audio testing is unavailable while recording.</p>
  {:else if !hasSelectedInput}
    <p class="test-message">Enable microphone or system audio above to test an input.</p>
  {:else if !selectedInputsAvailable}
    <p class="test-message" role="status">A selected audio input is unavailable on this system.</p>
  {:else if error}
    <p class="test-message error-text" role="alert">{error}</p>
  {:else if result}
    {#if microphoneEnabled}
      <div class="meter-row">
        <span>Mic</span>
        <div class="meter-track" role="meter" aria-label={`Microphone input ${result.microphone?.rmsPercent ?? 0}%`} aria-valuemin="0" aria-valuemax="100" aria-valuenow={result.microphone?.rmsPercent ?? 0}>
          <span class="meter-fill" style:width={`${result.microphone?.rmsPercent ?? 0}%`}></span>
        </div>
        <small>{meterLabel(result, "microphone")}</small>
      </div>
    {/if}
    {#if systemAudioEnabled}
      <div class="meter-row">
        <span>System</span>
        <div class="meter-track" role="meter" aria-label={`System audio input ${result.system?.rmsPercent ?? 0}%`} aria-valuemin="0" aria-valuemax="100" aria-valuenow={result.system?.rmsPercent ?? 0}>
          <span class="meter-fill" style:width={`${result.system?.rmsPercent ?? 0}%`}></span>
        </div>
        <small>{meterLabel(result, "system")}</small>
      </div>
    {/if}
  {/if}
</section>

<style>
  .audio-test {
    display: grid;
    min-width: 0;
    gap: 7px;
    margin: 5px 0 7px 29px;
    padding: 9px 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
  }

  .audio-test-heading,
  .audio-test-title {
    display: flex;
    min-width: 0;
    align-items: center;
  }

  .audio-test-heading {
    justify-content: space-between;
    gap: 8px;
  }

  .audio-test-title {
    gap: 7px;
  }

  .audio-test-title > div {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  :global(.audio-test-icon) {
    width: 15px;
    height: 15px;
    flex: none;
    color: var(--accent);
  }

  .audio-test-title strong {
    color: var(--text);
    font-size: 11px;
    font-weight: 600;
  }

  .audio-test-title span,
  .test-message {
    margin: 0;
    color: var(--muted);
    font-size: 10px;
    line-height: 1.35;
  }

  .test-button {
    min-height: 28px;
    flex: none;
    padding: 0 9px;
    border: 1px solid color-mix(in srgb, var(--accent) 44%, var(--border));
    border-radius: 8px;
    background: color-mix(in srgb, var(--accent) 16%, var(--surface));
    color: var(--text);
    font-size: 10px;
    font-weight: 600;
  }

  .test-button:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 24%, var(--surface));
  }

  .test-button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .meter-row {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr);
    align-items: center;
    gap: 4px 8px;
    color: var(--text);
    font-size: 10px;
  }

  .meter-track {
    height: 5px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--surface-hover);
  }

  .meter-fill {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width 120ms linear;
  }

  .meter-row small {
    grid-column: 2;
    color: var(--muted);
    font-size: 9px;
  }

  .error-text {
    color: var(--accent-hover);
  }

  @media (prefers-reduced-motion: reduce) {
    .meter-fill { transition: none; }
  }
</style>
