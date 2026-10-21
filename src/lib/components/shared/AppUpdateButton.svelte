<script lang="ts">
  import { onMount } from "svelte";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import Download from "@lucide/svelte/icons/download";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import { Button } from "../ui/button/index.js";
  import { errorMessage } from "../../services/ipc/settings.js";

  const UPDATE_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
  const FOCUS_CHECK_THROTTLE_MS = 5 * 60 * 1000;

  let { desktopApp, visible = true }: { desktopApp: boolean; visible?: boolean } = $props();

  let updateAvailable = $state(false);
  let updateVersion = $state<string | null>(null);
  let installing = $state(false);
  let downloadProgress = $state<number | null>(null);
  let updateError = $state<string | null>(null);

  let pendingUpdate: Update | null = null;
  let disposed = false;
  let checking = false;
  let lastCheckedAt = 0;
  let downloadedBytes = 0;
  let expectedBytes: number | null = null;

  async function checkForUpdate(force = false) {
    const now = Date.now();
    if (
      !desktopApp
      || import.meta.env.DEV
      || disposed
      || checking
      || installing
      || (!force && now - lastCheckedAt < FOCUS_CHECK_THROTTLE_MS)
    ) return;

    checking = true;
    lastCheckedAt = now;

    try {
      const nextUpdate = await check({ timeout: 15_000 });
      if (disposed) {
        await nextUpdate?.close();
        return;
      }

      const previousUpdate = pendingUpdate;
      pendingUpdate = nextUpdate;
      updateAvailable = nextUpdate !== null;
      updateVersion = nextUpdate?.version ?? null;
      updateError = null;

      if (previousUpdate && previousUpdate !== nextUpdate) {
        await previousUpdate.close();
      }
    } catch (error) {
      console.warn("[RecoDex][update] Could not check for updates: " + errorMessage(error));
    } finally {
      checking = false;
    }
  }

  async function installUpdate() {
    const update = pendingUpdate;
    if (!update || installing) return;

    installing = true;
    updateError = null;
    downloadProgress = null;
    downloadedBytes = 0;
    expectedBytes = null;

    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          expectedBytes = event.data.contentLength ?? null;
          downloadedBytes = 0;
          downloadProgress = null;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          downloadProgress = expectedBytes && expectedBytes > 0
            ? Math.min(99, Math.floor((downloadedBytes / expectedBytes) * 100))
            : null;
        } else {
          downloadProgress = 100;
        }
      });

      pendingUpdate = null;
      updateAvailable = false;
      await relaunch();
    } catch (error) {
      updateError = errorMessage(error);
      console.error("[RecoDex][update] Update installation failed: " + updateError);
      installing = false;

      if (pendingUpdate === null) {
        updateVersion = null;
        void checkForUpdate(true);
      }
    }
  }

  onMount(() => {
    disposed = false;
    void checkForUpdate(true);

    const interval = window.setInterval(() => void checkForUpdate(true), UPDATE_CHECK_INTERVAL_MS);
    const handleFocus = () => void checkForUpdate();

    window.addEventListener("focus", handleFocus);

    return () => {
      disposed = true;
      window.clearInterval(interval);
      window.removeEventListener("focus", handleFocus);
      if (pendingUpdate && !installing) {
        void pendingUpdate.close().catch((error: unknown) => {
          console.warn("[RecoDex][update] Could not release update metadata: " + errorMessage(error));
        });
        pendingUpdate = null;
      }
    };
  });
</script>

{#if visible && updateAvailable}
  <Button
    variant="ghost"
    size="icon"
    class="update-button"
    disabled={installing}
    aria-label={installing ? "Installing RecoDex update" : "Update RecoDex to version " + updateVersion}
    title={installing
      ? (downloadProgress === 100
        ? "Installing version " + updateVersion + "…"
        : downloadProgress === null
          ? "Downloading version " + updateVersion + "…"
          : "Downloading update: " + downloadProgress + "%")
      : updateError
        ? "Update failed: " + updateError
        : "Update RecoDex to version " + updateVersion}
    onclick={installUpdate}
  >
    {#if installing}
      <LoaderCircle aria-hidden="true" class={downloadProgress === 100 ? "size-[18px]" : "size-[18px] animate-spin"} />
    {:else}
      <Download aria-hidden="true" class="size-[18px]" />
    {/if}
  </Button>
{/if}
