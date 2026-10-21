<script lang="ts">
  import ToolbarAudioControls from "./ToolbarAudioControls.svelte";
  import ToolbarStatus from "./ToolbarStatus.svelte";
  import ToolbarTransportControls from "./ToolbarTransportControls.svelte";
  import ToolbarWindowActions from "./ToolbarWindowActions.svelte";
  import ToolbarWebcamControl from "./ToolbarWebcamControl.svelte";
  import type { RecordingToolbarProps } from "./types.js";

  let {
    status,
    elapsedMs,
    compact = false,
    microphoneEnabled = null,
    systemAudioEnabled = null,
    microphoneAvailable = false,
    systemAudioAvailable = false,
    webcamEnabled = false,
    webcamAvailable = false,
    audioWarning = null,
    quality,
    message,
    onPauseResume,
    onStop,
    onToggleMicrophone,
    onToggleSystemAudio,
    onToggleWebcam,
    onStartDragging,
    onCompactToggle,
    onHide
  }: RecordingToolbarProps = $props();
</script>

<div
  class="toolbar-frame"
  class:compact
  role="toolbar"
  aria-label="Recording controls"
  aria-describedby="toolbar-status-help"
>
  <div class="toolbar-section status-section">
    <ToolbarStatus {status} {elapsedMs} {message} {audioWarning} {quality} {compact} />
  </div>
  <span id="toolbar-status-help" class="sr-only">
    Recording state and elapsed time are supplied by the native capture service.
  </span>

  {#if !compact}
    <span class="toolbar-divider" aria-hidden="true"></span>
  {/if}

  <div class="toolbar-section controls-section">
    <ToolbarTransportControls
      {status}
      {compact}
      {onPauseResume}
      {onStop}
    />
    {#if !compact}
      <ToolbarAudioControls
        {status}
        {microphoneEnabled}
        {systemAudioEnabled}
        {microphoneAvailable}
        {systemAudioAvailable}
        microphoneLevelPercent={quality?.microphoneLevelPercent ?? 0}
        systemAudioLevelPercent={quality?.systemAudioLevelPercent ?? 0}
        {onToggleMicrophone}
        {onToggleSystemAudio}
      />
    {/if}
    <ToolbarWebcamControl
      {status}
      enabled={webcamEnabled}
      available={webcamAvailable}
      onToggle={onToggleWebcam}
    />
  </div>

  {#if !compact}
    <span class="toolbar-divider" aria-hidden="true"></span>
  {/if}

  <div class="toolbar-section window-section">
    <ToolbarWindowActions {compact} {onStartDragging} {onCompactToggle} {onHide} />
  </div>
</div>

<style>
  .toolbar-frame {
    display: flex;
    width: 620px;
    height: 58px;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    padding: 7px 10px;
    overflow: hidden;
    border: 0;
    border-radius: 15px;
    background: var(--bg);
    box-shadow: 0 8px 28px rgb(0 0 0 / 28%);
  }

  .toolbar-frame.compact {
    width: 270px;
    height: 50px;
    gap: 2px;
    padding: 4px 6px;
    border-radius: 14px;
  }

  .toolbar-frame.compact :global(.status-group) {
    width: 100px;
    padding-inline: 4px;
  }

  .toolbar-frame.compact :global(.state-label) {
    display: none;
  }

  .toolbar-section {
    display: flex;
    min-width: 0;
    align-items: center;
    flex: none;
  }

  .controls-section {
    gap: 6px;
  }

  :global(.toolbar-divider) {
    display: block;
    width: 1px;
    height: 30px;
    flex: none;
    align-self: center;
    background: var(--toolbar-border);
  }

  .compact .controls-section {
    gap: 6px;
  }
</style>
