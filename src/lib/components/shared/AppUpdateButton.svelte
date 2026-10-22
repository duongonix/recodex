<script lang="ts">
  import { untrack } from "svelte";
  import Download from "@lucide/svelte/icons/download";
  import { Button } from "../ui/button/index.js";
  import { appUpdate } from "../../stores/app-update.svelte.js";

  let {
    desktopApp,
    visible = true,
    onOpen
  }: {
    desktopApp: boolean;
    visible?: boolean;
    onOpen: () => void;
  } = $props();

  $effect(() => {
    const enabled = desktopApp;
    return untrack(() => appUpdate.start(enabled));
  });
</script>

{#if visible && appUpdate.updateAvailable}
  <Button
    variant="ghost"
    size="icon"
    class="update-button"
    aria-label={`View RecoDex update version ${appUpdate.version}`}
    title={`View update details for version ${appUpdate.version}`}
    onclick={onOpen}
  >
    <Download aria-hidden="true" class="size-[18px]" />
  </Button>
{/if}
