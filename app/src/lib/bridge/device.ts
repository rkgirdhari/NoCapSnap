import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, Bridge, CaptureRecord, MenuItem, Profile, StoreStatus } from "./types";

// Android's WebView can't hand a request body to the app, so Tauri carries IPC
// over postMessage as text there, and a Uint8Array would travel as a JSON list
// of numbers. The photo goes as base64 instead (~1.33× its size).
const viaText = typeof navigator !== "undefined" && /Android/i.test(navigator.userAgent);

export function toBase64(bytes: Uint8Array): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const url = reader.result as string;
      resolve(url.slice(url.indexOf(",") + 1));
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(new Blob([bytes as BlobPart]));
  });
}

export const deviceBridge: Bridge = {
  mode: "device",
  appInfo: () => invoke<AppInfo>("app_info"),
  status: () => invoke<StoreStatus>("store_status"),
  profile: () => invoke<Profile>("profile_get"),
  setDisplayName: (name) => invoke<Profile>("profile_set_name", { name }),
  menu: () => invoke<MenuItem[]>("menu_list"),
  // The photo is the IPC body (raw bytes, or base64 on Android); the choices
  // ride along as percent-encoded headers, which both transports carry.
  ingest: async (photo, details) =>
    invoke<CaptureRecord>("capture_ingest", viaText ? { photoBase64: await toBase64(photo) } : photo, {
      headers: {
        ...(details.menuItemId ? { "x-capsnap-menu-item": encodeURIComponent(details.menuItemId) } : {}),
        ...(details.tableLabel ? { "x-capsnap-table-label": encodeURIComponent(details.tableLabel) } : {}),
      },
    }),
  list: (limit = 200) => invoke<CaptureRecord[]>("list_captures", { limit }),
  get: (clientId) => invoke<CaptureRecord | null>("capture_get", { clientId }),
  media: async (sha256, thumb) => {
    const bytes = await invoke<ArrayBuffer>("capture_media", { sha256, thumb });
    return new Blob([bytes], { type: "image/jpeg" });
  },
  selftest: () => invoke<string>("run_selftest"),
  simulateAck: () => invoke<CaptureRecord | null>("simulate_ack"),
};
