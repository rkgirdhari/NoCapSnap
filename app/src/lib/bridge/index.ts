import { deviceBridge } from "./device";
import { createPreviewBridge } from "./preview";
import type { Bridge } from "./types";

export type {
  AppInfo,
  Bridge,
  RemoteLocation,
  Session,
  SyncReport,
  CaptureDetails,
  CaptureRecord,
  MenuItem,
  Profile,
  StoreStatus,
  SyncState,
} from "./types";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// The in-memory preview exists for design review in a desktop browser. Vite
// replaces these flags at build time, so a normal `vite build` (what Tauri
// bundles) folds the branch away and the preview module is not shipped.
const previewAllowed = import.meta.env.DEV || import.meta.env.VITE_CAPSNAP_PREVIEW === "1";

const unavailable = (): never => {
  throw new Error("CapSnap runs inside its Android app. This page has no access to the device store.");
};

const noDevice: Bridge = {
  mode: "none",
  appInfo: unavailable,
  status: unavailable,
  profile: unavailable,
  setDisplayName: unavailable,
  menu: unavailable,
  ingest: unavailable,
  list: unavailable,
  get: unavailable,
  media: unavailable,
  selftest: unavailable,
  simulateAck: unavailable,
  signIn: unavailable,
  signOut: unavailable,
  refreshSession: unavailable,
  chooseLocation: unavailable,
  syncNow: unavailable,
  onSyncUpdated: async () => () => {},
};

export const bridge: Bridge = inTauri ? deviceBridge : previewAllowed ? createPreviewBridge() : noDevice;
