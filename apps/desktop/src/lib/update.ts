import { check } from "@tauri-apps/plugin-updater";
import type { Update } from "@tauri-apps/plugin-updater";

export async function checkForUpdates(): Promise<Update | null> {
  return check();
}
