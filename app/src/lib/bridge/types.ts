export type SyncState = "pending" | "synced";

export interface CaptureRecord {
  clientId: string;
  sha256: string;
  mime: string | null;
  bytes: number | null;
  capturedAt: string; // RFC 3339 UTC
  syncState: SyncState;
  syncedAt: string | null;
}

export interface StoreStatus {
  sqliteVersion: string;
  journalMode: string;
  pending: number;
  synced: number;
}

export interface Bridge {
  /** "device" inside the Tauri app; "preview" in a desktop browser (nothing is saved). */
  readonly mode: "device" | "preview";
  status(): Promise<StoreStatus>;
  ingest(photo: Uint8Array): Promise<CaptureRecord>;
  list(limit?: number): Promise<CaptureRecord[]>;
  selftest(): Promise<string>;
  /** W1 spike only: stands in for the server acknowledging the oldest pending capture. */
  simulateAck(): Promise<CaptureRecord | null>;
}
