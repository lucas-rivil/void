import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

export async function notify(title: string, body: string) {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    if (granted) {
      sendNotification({ title, body });
    }
  } catch {
    void 0;
  }
}

export async function autostartEnabled(): Promise<boolean> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<boolean>("plugin:autostart|is_enabled");
  } catch {
    return false;
  }
}

export async function setAutostart(enabled: boolean): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  if (enabled) {
    await invoke("plugin:autostart|enable");
  } else {
    await invoke("plugin:autostart|disable");
  }
}
