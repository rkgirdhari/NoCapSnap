import { invoke } from "@tauri-apps/api/core";
import type { Bridge, CaptureRecord, StoreStatus } from "./types";

export const deviceBridge: Bridge = {
  mode: "device",
  status: () => invoke<StoreStatus>("store_status"),
  // Raw bytes travel as the IPC body — no base64 round trip.
  ingest: (photo) => invoke<CaptureRecord>("capture_ingest", photo),
  list: (limit = 200) => invoke<CaptureRecord[]>("list_captures", { limit }),
  selftest: () => invoke<string>("run_selftest"),
  simulateAck: () => invoke<CaptureRecord | null>("simulate_ack"),
};
