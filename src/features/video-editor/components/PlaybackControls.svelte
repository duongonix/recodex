<script lang="ts">
  import SkipBack from "@lucide/svelte/icons/skip-back";
  import SkipForward from "@lucide/svelte/icons/skip-forward";
  import Play from "@lucide/svelte/icons/play";
  import Pause from "@lucide/svelte/icons/pause";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";

  type Props = {
    currentTimeMs: number;
    durationMs: number;
    playing: boolean;
    volume: number;
    muted: boolean;
    onSeek: (time: number) => void;
    onTogglePlayback: () => void;
    onPrevious: () => void;
    onNext: () => void;
    onVolume: (volume: number) => void;
    onToggleMute: () => void;
  };
  let { currentTimeMs, durationMs, playing, volume, muted, onSeek, onTogglePlayback, onPrevious, onNext, onVolume, onToggleMute }: Props = $props();
  const format = (time: number) => {
    const seconds = Math.floor(Math.max(0, time) / 1000);
    return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
  };
</script>

<div class="playback">
  <div class="seek-row">
    <span class="time">{format(currentTimeMs)}</span>
    <span class="total">/ {format(durationMs)}</span>
    <input aria-label="Seek video" type="range" min="0" max={Math.max(1, durationMs)} value={currentTimeMs} oninput={(event) => onSeek(Number(event.currentTarget.value))} />
  </div>
  <div class="control-row">
    <div class="transport">
      <button type="button" aria-label="Previous clip" title="Previous clip" onclick={onPrevious}><SkipBack size={18} /></button>
      <button class="play" type="button" aria-label={playing ? "Pause" : "Play"} title={playing ? "Pause" : "Play"} onclick={onTogglePlayback}>{#if playing}<Pause size={20} fill="currentColor" />{:else}<Play size={20} fill="currentColor" />{/if}</button>
      <button type="button" aria-label="Next clip" title="Next clip" onclick={onNext}><SkipForward size={18} /></button>
    </div>
    <div class="volume-control">
      <button type="button" aria-label={muted ? "Unmute preview" : "Mute preview"} title={muted ? "Unmute" : "Mute"} onclick={onToggleMute}>{#if muted || volume === 0}<VolumeX size={18} />{:else}<Volume2 size={18} />{/if}</button>
      <input aria-label="Preview volume" type="range" min="0" max="100" value={volume} oninput={(event) => onVolume(Number(event.currentTarget.value))} />
    </div>
  </div>
</div>

<style>
  .playback { display:grid; gap:12px; padding:8px 16px 12px; }
  .seek-row { display:flex; align-items:center; gap:7px; color:#989aa7; font-size:11px; font-variant-numeric:tabular-nums; }
  .seek-row .time { color:#f5f5f7; }
  .seek-row input { flex:1; }
  input[type=range] { height:4px; accent-color:#ff3038; cursor:pointer; }
  .control-row { display:flex; align-items:center; justify-content:center; position:relative; }
  .transport,.volume-control { display:flex; align-items:center; gap:10px; }
  .volume-control { position:absolute; right:0; }
  button { display:grid; width:32px; height:30px; place-items:center; border:0; border-radius:7px; background:transparent; color:#d4d5da; cursor:pointer; }
  button:hover { background:#252730; color:white; }
  button.play { width:38px; color:#fff; }
  .volume-control input { width:88px; }
  @media (max-width:760px) { .volume-control { position:static; margin-left:auto; } .control-row { justify-content:flex-start; } }
</style>
