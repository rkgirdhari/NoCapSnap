import type { Bridge, CaptureRecord, ImportDraft, MenuItem, Profile, RemoteLocation } from "./types";

// Browser preview for design review (built only in dev or with
// VITE_CAPSNAP_PREVIEW=1). Mirrors the device contract in memory; nothing is
// persisted, there is no SQLite, and photos are resized with a canvas instead
// of the Rust pipeline. Sign-in and sync are simulated (any credentials; the
// "server" issues links on guests.preview.invalid) so the signed-in screens and
// the real guest QR can be reviewed and decode-tested in a browser.

// Everything lives inside the factory so that a build without the preview
// flag drops this module entirely (top-level work would pin it in the bundle).
export function createPreviewBridge(): Bridge {
  const LOCATION_ID = "demo-atelier-no-8";

  // Same labelled demo seed as crates/capsnap-store/src/menu.rs.
  const menu: MenuItem[] = [
    ["demo-saffron-butter-cod", "Saffron butter cod", "Mains"],
    ["demo-wild-mushroom-risotto", "Wild mushroom risotto", "Mains"],
    ["demo-charred-heritage-carrots", "Charred heritage carrots", "Mains"],
    ["demo-five-spice-duck", "Five-spice duck breast", "Mains"],
    ["demo-pork-belly-bao", "Pork belly bao", "Starters"],
    ["demo-sweet-corn-miso", "Sweet corn & white miso soup", "Starters"],
    ["demo-tuna-crudo", "Tuna crudo, yuzu kosho", "Starters"],
  ].map(([id, name, category]) => ({ id, name, category, source: "demo" }));
  const serverLocation: RemoteLocation = { id: "preview-atelier", name: "Atelier No. 8", timezone: "America/Chicago" };
  let serverMenu: MenuItem[] = menu.map((m) => ({ ...m, id: m.id.replace("demo-", "srv-"), source: "server" }));

  let displayName: string | null = null;
  const signedOut = (): Profile => ({
    displayName,
    locationId: LOCATION_ID,
    locationName: "Atelier No. 8",
    isDemo: true,
    signedIn: false,
    serverUrl: null,
    organizationName: null,
    role: null,
  });
  let profile: Profile = signedOut();
  const listeners = new Set<() => void>();
  const notify = () => listeners.forEach((l) => l());
  const currentMenu = () => (profile.isDemo ? menu : serverMenu);
  const token43 = () => {
    const bytes = crypto.getRandomValues(new Uint8Array(32));
    return btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  };

  const records: CaptureRecord[] = [];
  const blobs = new Map<string, { photo: Blob; thumb: Blob }>();

  function sniff(b: Uint8Array): string | null {
    if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return "image/jpeg";
    if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47) return "image/png";
    if (b[0] === 0x52 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x46) return "image/webp";
    return null;
  }

  async function sha256(b: Blob): Promise<string> {
    const digest = await crypto.subtle.digest("SHA-256", await b.arrayBuffer());
    return [...new Uint8Array(digest)].map((x) => x.toString(16).padStart(2, "0")).join("");
  }

  async function fit(bitmap: ImageBitmap, longEdge: number): Promise<{ blob: Blob; w: number; h: number }> {
    const scale = Math.min(1, longEdge / Math.max(bitmap.width, bitmap.height));
    const w = Math.round(bitmap.width * scale);
    const h = Math.round(bitmap.height * scale);
    const canvas = new OffscreenCanvas(w, h);
    const ctx = canvas.getContext("2d")!;
    ctx.fillStyle = "#fff";
    ctx.fillRect(0, 0, w, h);
    ctx.drawImage(bitmap, 0, 0, w, h);
    return { blob: await canvas.convertToBlob({ type: "image/jpeg", quality: 0.85 }), w, h };
  }

  const copy = (r: CaptureRecord): CaptureRecord => ({ ...r });

  return {
    mode: "preview",
    async appInfo() {
      return { version: "preview", debug: true };
    },
    async status() {
      return {
        sqliteVersion: "— (browser preview)",
        journalMode: "—",
        pending: records.filter((r) => r.syncState === "pending").length,
        synced: records.filter((r) => r.syncState === "synced").length,
      };
    },
    async profile() {
      return { ...profile };
    },
    async setDisplayName(name) {
      displayName = name.trim() || null;
      profile.displayName = displayName;
      return { ...profile };
    },
    async menu() {
      return currentMenu().map((m) => ({ ...m }));
    },
    // Preview only: no website is read. Sample dishes show how the review step looks.
    async previewMenuImport(siteUrl) {
      if (!profile.signedIn) throw new Error("sign in again to sync");
      if (profile.role !== "admin" && profile.role !== "manager") throw new Error("your role can't do that");
      if (!/^https?:\/\/[^/\s]+/i.test(siteUrl.trim())) throw new Error("that isn't a web address");
      const draft: ImportDraft = {
        businessName: "Sample Bistro",
        menu: [
          { name: "Olive oil cake", category: "Desserts", description: null, price: "9 USD" },
          { name: "Short rib", category: "Mains", description: null, price: "32 USD" },
          { name: "Seared salmon", category: "Mains", description: null, price: "29 USD" },
          { name: "Burrata", category: "Starters", description: null, price: "14 USD" },
        ],
        pagesRead: [siteUrl.trim()],
        notes: ["Preview: these are sample dishes. No website was read."],
      };
      return draft;
    },
    async saveMenuImport(items) {
      if (!items.length) throw new Error("send at least one dish; an empty menu would wipe the current one");
      serverMenu = items.map((m, i) => ({ id: `srv-import-${i}`, name: m.name, category: m.category, source: "server" }));
      return serverMenu.map((m) => ({ ...m }));
    },
    async ingest(photo, details) {
      if (!sniff(photo)) throw new Error("only JPEG, PNG or WebP photos can be saved");
      const dish = details.menuItemId ? currentMenu().find((m) => m.id === details.menuItemId) : undefined;
      if (details.menuItemId && !dish) throw new Error("that dish is not on this menu");
      const bitmap = await createImageBitmap(new Blob([photo as BlobPart]), { imageOrientation: "from-image" });
      const full = await fit(bitmap, 2048);
      const thumb = await fit(bitmap, 480);
      bitmap.close();
      const digest = await sha256(full.blob);
      blobs.set(digest, { photo: full.blob, thumb: thumb.blob });
      const record: CaptureRecord = {
        clientId: crypto.randomUUID(),
        sha256: digest,
        mime: "image/jpeg",
        bytes: full.blob.size,
        width: full.w,
        height: full.h,
        capturedAt: new Date().toISOString(),
        syncState: "pending",
        syncedAt: null,
        locationId: profile.locationId,
        menuItemId: dish?.id ?? null,
        dishName: dish?.name ?? null,
        tableLabel: details.tableLabel?.trim() || null,
        isDemo: profile.isDemo,
        guestUrl: null,
        guestExpiresAt: null,
        lastSyncError: null,
      };
      records.unshift(record);
      return copy(record);
    },
    async list(limit = 200) {
      return records.slice(0, limit).map(copy);
    },
    async get(clientId) {
      const r = records.find((x) => x.clientId === clientId);
      return r ? copy(r) : null;
    },
    async media(digest, thumb) {
      const entry = blobs.get(digest);
      if (!entry) throw new Error("this photo is no longer on the device");
      return thumb ? entry.thumb : entry.photo;
    },
    async selftest() {
      return "preview: no SQLite in the browser — run this on the device";
    },
    async signIn(serverUrl, login, password) {
      if (!serverUrl.trim() || !login.trim() || !password) throw new Error("server address, sign-in name and password are required");
      // Preview only: a sign-in name starting "ada" or "admin" is an admin, anything else is a server.
      const manager = /^(ada|admin)/i.test(login.trim());
      profile = {
        displayName: manager ? "Ada" : "Maya",
        locationId: serverLocation.id,
        locationName: serverLocation.name,
        isDemo: false,
        signedIn: true,
        serverUrl: serverUrl.trim(),
        organizationName: "Atelier No. 8",
        role: manager ? "admin" : "server",
      };
      displayName = manager ? "Ada" : "Maya";
      return { profile: { ...profile }, locations: [serverLocation] };
    },
    async openPrivacyPolicy() {
      if (!profile.serverUrl) throw new Error("sign in to a server first; the privacy policy is on its web address");
      window.open(`${profile.serverUrl.replace(/\/+$/, "")}/privacy`, "_blank", "noopener");
    },
    async signOut() {
      profile = signedOut();
      notify();
      return { ...profile };
    },
    async refreshSession() {
      if (!profile.signedIn) throw new Error("sign in again to sync");
      return { profile: { ...profile }, locations: [serverLocation] };
    },
    async chooseLocation() {
      return { ...profile };
    },
    async syncNow() {
      if (!profile.signedIn) throw new Error("sign in again to sync");
      let synced = 0;
      for (const r of [...records].reverse()) {
        if (r.syncState !== "pending" || r.isDemo) continue;
        r.syncState = "synced";
        r.syncedAt = new Date().toISOString();
        r.guestUrl = `https://guests.preview.invalid/g/#${token43()}`;
        r.guestExpiresAt = new Date(Date.now() + 30 * 24 * 3600 * 1000).toISOString();
        synced++;
      }
      notify();
      return { synced, refused: 0, remaining: 0, stopped: null };
    },
    async onSyncUpdated(callback) {
      listeners.add(callback);
      return () => listeners.delete(callback);
    },
    async simulateAck() {
      const oldest = [...records].reverse().find((r) => r.syncState === "pending");
      if (!oldest) return null;
      oldest.syncState = "synced";
      oldest.syncedAt = new Date().toISOString();
      return copy(oldest);
    },
  };
}
