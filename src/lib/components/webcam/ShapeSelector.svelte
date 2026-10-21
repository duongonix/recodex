<script lang="ts">
  import Circle from "@lucide/svelte/icons/circle";
  import RectangleHorizontal from "@lucide/svelte/icons/rectangle-horizontal";
  import type { WebcamShape } from "../../types/settings.js";

  let { value, onChange }: { value: WebcamShape; onChange: (value: WebcamShape) => void } = $props();
</script>

<section class="webcam-control-section" aria-labelledby="webcam-shape-heading">
  <h3 id="webcam-shape-heading" class="webcam-section-title">Shape</h3>
  <div class="webcam-choice-row" role="group" aria-label="Webcam shape">
    <button type="button" class="webcam-choice" class:webcam-choice-active={value === "rectangle"} aria-pressed={value === "rectangle"} onclick={() => onChange("rectangle")}><RectangleHorizontal aria-hidden="true" class="size-4" />Rectangle</button>
    <button type="button" class="webcam-choice" class:webcam-choice-active={value === "circle"} aria-pressed={value === "circle"} onclick={() => onChange("circle")}><Circle aria-hidden="true" class="size-4" />Circle</button>
  </div>
</section>
