<script lang="ts">
  import PlaybackControls from "./PlaybackControls.svelte";

  type Props = {
    source: string | null;
    videoElement: HTMLVideoElement | undefined;
    durationMs: number;
    currentTimeMs: number;
    playing: boolean;
    volume: number;
    muted: boolean;
    error: string | null;
    canvasStyle: string;
    videoStyle: string;
    onMetadata: () => void;
    onTimeUpdate: () => void;
    onPlay: () => void;
    onPause: () => void;
    onSeek: (time: number) => void;
    onTogglePlayback: () => void;
    onPrevious: () => void;
    onNext: () => void;
    onVolume: (value: number) => void;
    onToggleMute: () => void;
  };
  let { source, videoElement = $bindable(), durationMs, currentTimeMs, playing, volume, muted, error, canvasStyle, videoStyle, onMetadata, onTimeUpdate, onPlay, onPause, onSeek, onTogglePlayback, onPrevious, onNext, onVolume, onToggleMute }: Props = $props();
</script>

<section class="preview-panel" aria-label="Video preview">
  <div class="video-stage">
    {#if source}
      <div class="canvas" style={canvasStyle}>
        <!-- Recorded sources can lack caption metadata; an empty caption track would be misleading. -->
        <!-- svelte-ignore a11y_media_has_caption -->
        <video bind:this={videoElement} src={source} playsinline preload="metadata" style={videoStyle} aria-label="Video preview" onloadedmetadata={onMetadata} ontimeupdate={onTimeUpdate} onplay={onPlay} onpause={onPause}></video>
      </div>
    {:else if error}
      <p class="stage-message" role="alert">{error}</p>
    {:else}
      <p class="stage-message">Loading recording…</p>
    {/if}
  </div>
  <PlaybackControls {currentTimeMs} {durationMs} {playing} {volume} {muted} onSeek={onSeek} onTogglePlayback={onTogglePlayback} onPrevious={onPrevious} onNext={onNext} onVolume={onVolume} onToggleMute={onToggleMute} />
</section>

<style>
  .preview-panel { display:grid; min-width:0; min-height:0; grid-template-rows:minmax(0,1fr) auto; background:#101116; }
  .video-stage { display:flex; min-width:0; min-height:0; align-items:center; justify-content:center; overflow:hidden; background:#08090c; container-type:size; }
  .canvas { position:relative; width:min(100cqw,calc(100cqh * var(--ratio))); height:min(100cqh,calc(100cqw / var(--ratio))); aspect-ratio:var(--ratio); overflow:hidden; }
  video { position:absolute; inset:0; display:block; width:100%; height:100%; object-fit:var(--fit,contain); }
  .stage-message { max-width:600px; padding:24px; color:#989aa7; text-align:center; font-size:13px; }
</style>
