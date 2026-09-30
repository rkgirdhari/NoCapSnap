export type SyncState = "pending" | "synced";

export interface CaptureRecord {
  clientId: string;
  sha256: string;
  mime: string | null;
  bytes: number | null;
  width: number | null;
  height: number | null;
  capturedAt: string; // RFC 3339 UTC
  syncState: SyncState;
  syncedAt: string | null;
  locationId: string | null;
  menuItemId: string | null;
  dishName: string | null;
  /** Staff reference kept on this device only (owner default M6). */
  tableLabel: string | null;
}

export interface MenuItem {
  id: string;
  name: string;
  category: string;
  /** "demo" until the server provides the menu (W3). */
  source: "demo" | "server" | "import" | string;
}

export interface Profile {
  displayName: string | null;
  locationId: string | null;
  locationName: string | null;
  isDemo: boolean;
}

export interface StoreStatus {
  sqliteVersion: string;
  journalMode: string;
  pending: number;
  synced: number;
}

export interface AppInfo {
  version: string;
  /** Debug build of the Rust core: adds developer-only tools such as simulateAck. */
  debug: boolean;
}

export interface CaptureDetails {
  menuItemId: string | null;
  tableLabel: string | null;
}

export interface Bridge {
  /**
   * "device" inside the Tauri app; "preview" in a browser build made for design
   * review (in memory, nothing is saved); "none" in any other browser.
   */
  readonly mode: "device" | "preview" | "none";
  appInfo(): Promise<AppInfo>;
  status(): Promise<StoreStatus>;
  profile(): Promise<Profile>;
  setDisplayName(name: string): Promise<Profile>;
  menu(): Promise<MenuItem[]>;
  /** Processes the photo on the device (2048 px, metadata stripped) and queues it. */
  ingest(photo: Uint8Array, details: CaptureDetails): Promise<CaptureRecord>;
  list(limit?: number): Promise<CaptureRecord[]>;
  get(clientId: string): Promise<CaptureRecord | null>;
  /** The processed photo (or its thumbnail) as a JPEG blob. */
  media(sha256: string, thumb: boolean): Promise<Blob>;
  selftest(): Promise<string>;
  /** Debug builds only: stands in for the server acknowledging the oldest pending capture. */
  simulateAck(): Promise<CaptureRecord | null>;
}
