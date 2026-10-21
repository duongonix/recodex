<script lang="ts">
  import Bell from "@lucide/svelte/icons/bell";
  import PanelTop from "@lucide/svelte/icons/panel-top";
  import Power from "@lucide/svelte/icons/power";
  import SettingToggleRow from "./SettingToggleRow.svelte";
  import type { AppSettings, SettingsPatch } from "../../types/settings.js";

  let {
    settings,
    startupLabel,
    startupSupported,
    startupError = null,
    traySupported,
    onUpdate
  }: {
    settings: AppSettings;
    startupLabel: string;
    startupSupported: boolean;
    startupError?: string | null;
    traySupported: boolean;
    onUpdate: (patch: SettingsPatch) => Promise<boolean>;
  } = $props();
</script>

<section class="settings-section" aria-label="General settings">
  <h3 class="settings-section-title">General</h3>
  <div class="settings-rows">
    <SettingToggleRow
      id="start-with-system"
      label={startupLabel}
      description="Launch RecoDex automatically when you sign in."
      icon={Power}
      value={settings.startWithSystem}
      disabled={!startupSupported}
      disabledReason={startupError ?? "Startup registration is unavailable on this platform."}
      onChange={(value) => onUpdate({ startWithSystem: value })}
    />
    <SettingToggleRow
      id="minimize-to-tray"
      label="Minimize to system tray"
      description="Keep RecoDex running when its main window is hidden."
      icon={PanelTop}
      value={settings.minimizeToTray}
      disabled={!traySupported}
      disabledReason="System tray support is not available on this platform."
      onChange={(value) => onUpdate({ minimizeToTray: value })}
    />
    <SettingToggleRow
      id="show-notifications"
      label="Show notifications"
      description="Show capture results and actionable errors."
      icon={Bell}
      value={settings.showNotifications}
      onChange={(value) => onUpdate({ showNotifications: value })}
    />
  </div>
</section>
