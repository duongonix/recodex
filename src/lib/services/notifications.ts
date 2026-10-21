import { isTauri } from "@tauri-apps/api/core";

export async function notifyApp(title: string, body: string, enabled = true): Promise<void> {
  if (!enabled) return;
  if (import.meta.env.DEV || !isTauri()) {
    console.info(`[RecoDex] ${title}: ${body}`);
    return;
  }

  try {
    const {
      isPermissionGranted,
      requestPermission,
      sendNotification
    } = await import("@tauri-apps/plugin-notification");

    let permissionGranted = await isPermissionGranted();
    if (!permissionGranted) {
      permissionGranted = (await requestPermission()) === "granted";
    }
    if (!permissionGranted) {
      console.warn(`[RecoDex] System notification permission denied: ${title}`);
      return;
    }

    sendNotification({ title: `RecoDex · ${title}`, body });
  } catch (error) {
    console.error(`[RecoDex] Could not show system notification: ${title}`, error);
  }
}
