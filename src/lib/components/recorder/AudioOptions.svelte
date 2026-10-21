<script lang="ts">
  import Mic from "@lucide/svelte/icons/mic";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import { Switch } from "../ui/switch/index.js";

  let {
    systemAudioEnabled = $bindable(false),
    microphoneEnabled = $bindable(false),
    systemAudioAvailable = false,
    microphoneAvailable = false,
    systemAudioMessage = "System-audio capture is not currently available.",
    microphoneMessage = "Microphone capture is not currently available."
  }: {
    systemAudioEnabled?: boolean;
    microphoneEnabled?: boolean;
    systemAudioAvailable?: boolean;
    microphoneAvailable?: boolean;
    systemAudioMessage?: string;
    microphoneMessage?: string;
  } = $props();
</script>

<section aria-label="Audio options" class="audio-section">
  <div class="audio-option" class:disabled={!systemAudioAvailable && !systemAudioEnabled} title={systemAudioMessage}>
    <Volume2 aria-hidden="true" class="size-5" />
    <span id="system-audio-label">System Audio</span>
    <Switch
      disabled={!systemAudioAvailable && !systemAudioEnabled}
       bind:checked={systemAudioEnabled}
      aria-labelledby="system-audio-label"
      aria-describedby="audio-capability-note"
    />
  </div>
  <div class="audio-option" class:disabled={!microphoneAvailable && !microphoneEnabled} title={microphoneMessage}>
    <Mic aria-hidden="true" class="size-5" />
    <span id="microphone-label">Microphone</span>
    <Switch
      disabled={!microphoneAvailable && !microphoneEnabled}
       bind:checked={microphoneEnabled}
      aria-labelledby="microphone-label"
      aria-describedby="audio-capability-note"
    />
  </div>
  <p id="audio-capability-note" class="sr-only">
    Saved preferences are shown. Disabled controls include the reason reported by the native capability service.
  </p>
</section>

<style>
  .audio-section {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .audio-option {
    display: flex;
    min-width: 0;
    height: 44px;
    align-items: center;
    gap: 9px;
    padding: 0 10px;
    border: 1px solid transparent;
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12px;
  }

  .audio-option :global(svg) {
    flex: none;
  }

  .audio-option span {
    flex: 1;
    white-space: nowrap;
  }

  .audio-option :global([data-slot="switch"]) {
    flex: none;
  }

  .audio-option.disabled {
    color: var(--disabled);
    cursor: not-allowed;
  }
</style>
