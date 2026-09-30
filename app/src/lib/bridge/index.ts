import { deviceBridge } from "./device";
import { previewBridge } from "./preview";
import type { Bridge } from "./types";

export type { Bridge, CaptureRecord, StoreStatus, SyncState } from "./types";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const bridge: Bridge = inTauri ? deviceBridge : previewBridge;
