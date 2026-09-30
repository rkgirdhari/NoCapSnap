import type { Bridge, CaptureRecord, MenuItem, Profile } from "./types";

// Browser preview for design review (built only in dev or with
// VITE_CAPSNAP_PREVIEW=1). Mirrors the device contract in memory; nothing is
// persisted, there is no SQLite, and photos are resized with a canvas instead
// of the Rust pipeline.

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

  const profile: Profile = {
    displayName: null,
    locationId: LOCATION_ID,
    locationName: "Atelier No. 8",
    isDemo: true,
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
      profile.displayName = name.trim() || null;
      return { ...profile };
    },
    async menu() {
      return menu.map((m) => ({ ...m }));
    },
    async ingest(photo, details) {
      if (!sniff(photo)) throw new Error("only JPEG, PNG or WebP photos can be saved");
      const dish = details.menuItemId ? menu.find((m) => m.id === details.menuItemId) : undefined;
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
        locationId: dish ? LOCATION_ID : null,
        menuItemId: dish?.id ?? null,
        dishName: dish?.name ?? null,
        tableLabel: details.tableLabel?.trim() || null,
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
    async simulateAck() {
      const oldest = [...records].reverse().find((r) => r.syncState === "pending");
      if (!oldest) return null;
      oldest.syncState = "synced";
      oldest.syncedAt = new Date().toISOString();
      return copy(oldest);
    },
  };
}
