<script lang="ts">
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import type { EditorSettings } from "../types.js";
  type Props = { settings: EditorSettings; onChange: (patch: Partial<EditorSettings>) => void };
  let { settings, onChange }: Props = $props();
</script>

<div class="audio-properties">
  <h3>Audio</h3>
  <button class="mute" type="button" aria-pressed={settings.muted} onclick={() => onChange({ muted: !settings.muted })}>
    {#if settings.muted}<VolumeX size={16} /> Unmute audio{:else}<Volume2 size={16} /> Mute audio{/if}
  </button>
  <label class="volume"><span>Volume</span><input type="range" min="0" max="100" value={settings.volume} oninput={(event) => onChange({ volume: Number(event.currentTarget.value) })} /><output>{settings.volume}%</output></label>
  <p>Changes apply to the recording export.</p>
</div>

<style>
  .audio-properties { display:grid; align-content:start; gap:16px; padding:18px 14px; }
  h3 { margin:0; color:#f5f5f7; font-size:13px; }
  .mute { display:flex; height:34px; align-items:center; justify-content:center; gap:8px; border:1px solid #292b32; border-radius:8px; background:#1b1d24; color:#f5f5f7; font:inherit; font-size:12px; cursor:pointer; }
  .mute:hover { border-color:#ff3038; }
  .volume { display:grid; grid-template-columns:48px minmax(40px,1fr) 44px; align-items:center; gap:8px; color:#c5c6cd; font-size:11px; }
  input[type=range] { min-width:40px; accent-color:#ff3038; }
  output { color:#f5f5f7; text-align:right; }
  p { color:#989aa7; font-size:11px; line-height:1.5; }
</style>
