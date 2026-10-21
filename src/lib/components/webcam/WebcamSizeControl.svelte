<script lang="ts">
  let { value, onInput }: { value: number; onInput: (value: number) => void } = $props();
</script>

<section class="webcam-control-section" aria-labelledby="webcam-size-heading">
  <div class="webcam-control-heading">
    <h3 id="webcam-size-heading" class="webcam-section-title">Size</h3>
    <output for="webcam-size" class="webcam-value">{value}px</output>
  </div>
  <input id="webcam-size" class="webcam-range" type="range" min="80" max="640" step="10" value={value} oninput={(event) => onInput(Number(event.currentTarget.value))} />
  <div class="webcam-range-labels"><span>80px</span><span>640px</span></div>
</section>
