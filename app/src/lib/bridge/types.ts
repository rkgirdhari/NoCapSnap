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
  /** Made at the demo location: stays on this phone, never synced. */
  isDemo: boolean;
  /** `<server>/g/#<token>` once the server acknowledged the capture (W3a). */
  guestUrl: string | null;
  guestExpiresAt: string | null;
  lastSyncError: string | null;
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
  signedIn: boolean;
  serverUrl: string | null;
  organizationName: string | null;
  role: string | null;
}

export interface RemoteLocation {
  id: string;
  name: string;
  timezone: string;
}

/** A dish found on the restaurant's website. Price and description are for the reviewer only. */
export interface ImportedDish {
  name: string;
  category: string;
  description: string | null;
  price: string | null;
}

/** The server's draft menu: nothing is saved until the owner confirms it. */
export interface ImportDraft {
  businessName: string | null;
  menu: ImportedDish[];
  pagesRead: string[];
  /** Plain-language notes: what was skipped and why. */
  notes: string[];
}

/** One guest answer. The server keeps nothing about the guest. */
export interface FeedbackItem {
  id: string;
  rating: number;
  comment: string | null;
  createdAt: string;
  dishName: string | null;
}

export interface FeedbackSummary {
  count: number;
  /** null when nobody has answered in the period. */
  average: number | null;
  /** How many guests gave 1, 2, 3, 4 and 5. */
  distribution: [number, number, number, number, number];
}

export interface FeedbackPage {
  days: number;
  summary: FeedbackSummary;
  /** Newest first. Never sorted or filtered by rating (Spec §4, neutrality). */
  items: FeedbackItem[];
  /** Pass back to `feedback` for the next, older page; null on the last one. */
  nextBefore: string | null;
}

/** A dish as saved: a name and a course. */
export interface MenuChoice {
  name: string;
  category: string;
}

export interface Session {
  profile: Profile;
  locations: RemoteLocation[];
}

export interface SyncReport {
  synced: number;
  refused: number;
  remaining: number;
  /** Why the run stopped early, if it did. */
  stopped: string | null;
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
  /** Reads the restaurant's own website through the server and drafts a menu. Admin or manager only. Saves nothing. */
  previewMenuImport(siteUrl: string): Promise<ImportDraft>;
  /** What guests said about this location's dishes. Admin or manager only. */
  feedback(days: number, before?: string | null): Promise<FeedbackPage>;
  /** Saves the reviewed menu for this location and returns the phone's refreshed menu. */
  saveMenuImport(items: MenuChoice[]): Promise<MenuItem[]>;
  /** Processes the photo on the device (2048 px, metadata stripped) and queues it. */
  ingest(photo: Uint8Array, details: CaptureDetails): Promise<CaptureRecord>;
  list(limit?: number): Promise<CaptureRecord[]>;
  get(clientId: string): Promise<CaptureRecord | null>;
  /** The processed photo (or its thumbnail) as a JPEG blob. */
  media(sha256: string, thumb: boolean): Promise<Blob>;
  selftest(): Promise<string>;
  signIn(serverUrl: string, login: string, password: string): Promise<Session>;
  /** Opens the server's /privacy page in the phone's browser (Rust builds the address; needs a server). */
  openPrivacyPolicy(): Promise<void>;
  signOut(): Promise<Profile>;
  refreshSession(): Promise<Session>;
  chooseLocation(locationId: string): Promise<Profile>;
  syncNow(): Promise<SyncReport>;
  /** Called after every sync run (background or manual); returns an unsubscribe function. */
  onSyncUpdated(callback: () => void): Promise<() => void>;
  /** Debug builds only: stands in for the server acknowledging the oldest pending capture. */
  simulateAck(): Promise<CaptureRecord | null>;
}
