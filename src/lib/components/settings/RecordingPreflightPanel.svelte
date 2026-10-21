<script lang="ts">
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleHelp from "@lucide/svelte/icons/circle-help";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import type { PreflightCheck, RecordingPreflightReport } from "../../types/recording.js";
  import "./recording-preflight-panel.css";

  let {
    loading = false,
    report = null,
    error = null,
    onCheck
  }: {
    loading?: boolean;
    report?: RecordingPreflightReport | null;
    error?: string | null;
    onCheck: () => void;
  } = $props();

  function statusIcon(check: PreflightCheck) {
    if (check.status === "ready") return CircleCheck;
    if (check.status === "warning" || check.status === "error") return CircleAlert;
    return CircleHelp;
  }

  function formatBytes(bytes: number | null): string {
    if (bytes === null) return "Unavailable";
    if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
    return `${Math.floor(bytes / 1_048_576)} MB`;
  }
</script>

<section class="recording-preflight-panel" aria-labelledby="recording-preflight-title" aria-busy={loading}>
  <header class="preflight-panel-header">
    <div>
      <h4 id="recording-preflight-title">Recording check</h4>
      <p>Review capture source, audio devices, and available storage.</p>
    </div>
    <button type="button" class="preflight-check-button" onclick={onCheck} disabled={loading}>
      {#if loading}
        <LoaderCircle aria-hidden="true" class="size-4 spin" />
        <span>Checking…</span>
      {:else}
        <RefreshCw aria-hidden="true" class="size-4" />
        <span>{report ? "Refresh" : "Check setup"}</span>
      {/if}
    </button>
  </header>

  {#if loading}
    <div class="preflight-panel-message" role="status">
      Checking capture source, devices, and disk space…
    </div>
  {:else if error}
    <p class="preflight-panel-error" role="alert">{error}</p>
  {:else if report}
    <div class="preflight-panel-summary">
      <span>{report.outputWidth ?? "—"} × {report.outputHeight ?? "—"}</span>
      <span>{report.fps} FPS</span>
      <span>{formatBytes(report.freeDiskBytes)} free</span>
    </div>
    <p class="preflight-readiness" class:blocked={!report.canStart} role="status">
      {report.canStart
        ? (report.checks.some((check) => check.status === "warning") ? "Setup check passed with warnings" : "Setup check passed")
        : "Preflight found issues to review"}
    </p>
    <div class="preflight-panel-checks" aria-live="polite">
      {#each report.checks as check (check.key)}
        {@const Icon = statusIcon(check)}
        <div class="preflight-panel-check" class:error={check.status === "error"} class:warning={check.status === "warning"}>
          <Icon aria-hidden="true" class="preflight-check-icon size-4" />
          <div class="preflight-check-copy">
            <strong>{check.label}</strong>
            <span>{check.detail}</span>
          </div>
        </div>
      {/each}
    </div>
    {#if report.estimatedMinutesRemaining !== null}
      <p class="preflight-estimate-note">Estimated recording time uses the current MP4 bitrate and free space.</p>
    {/if}
  {:else}
    <p class="preflight-panel-empty">Run a check here when you want to review your current recording setup.</p>
  {/if}
</section>
