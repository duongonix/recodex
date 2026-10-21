<script lang="ts">
  import Monitor from "@lucide/svelte/icons/monitor";
  import Camera from "@lucide/svelte/icons/camera";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import {
    List as TabsList,
    Trigger as TabsTrigger
  } from "../ui/tabs/index.js";
</script>

<TabsList aria-label="Settings sections" class="settings-tab-list">
  <TabsTrigger value="general" class="settings-tab">
    <Monitor aria-hidden="true" class="size-4" />
    <span>General</span>
  </TabsTrigger>
  <TabsTrigger value="snap" class="settings-tab">
    <Camera aria-hidden="true" class="size-4" />
    <span>Snap</span>
  </TabsTrigger>
  <TabsTrigger value="hotkey" class="settings-tab">
    <Keyboard aria-hidden="true" class="size-4" />
    <span>Hotkey</span>
  </TabsTrigger>
</TabsList>
