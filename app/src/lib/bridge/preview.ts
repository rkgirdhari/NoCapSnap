import type { Bridge, CaptureRecord } from "./types";

// Browser preview for design review. Mirrors the device contract in memory;
// nothing is persisted and there is no SQLite.
const records: CaptureRecord[] = [];

function sniff(b: Uint8Array): string | null {
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return "image/jpeg";
  if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47) return "image/png";
  return null;
}

async function sha256(b: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", b as BufferSource);
  return [...new Uint8Array(digest)].map((x) => x.toString(16).padStart(2, "0")).join("");
}

export const previewBridge: Bridge = {
  mode: "preview",
  async status() {
    return {
      sqliteVersion: "— (browser preview)",
      journalMode: "—",
      pending: records.filter((r) => r.syncState === "pending").length,
      synced: records.filter((r) => r.syncState === "synced").length,
    };
  },
  async ingest(photo) {
    const mime = sniff(photo);
    if (!mime) throw new Error("only JPEG, PNG or WebP photos can be saved");
    const record: CaptureRecord = {
      clientId: crypto.randomUUID(),
      sha256: await sha256(photo),
      mime,
      bytes: photo.byteLength,
      capturedAt: new Date().toISOString(),
      syncState: "pending",
      syncedAt: null,
    };
    records.unshift(record);
    return record;
  },
  async list(limit = 200) {
    return records.slice(0, limit);
  },
  async selftest() {
    return "preview: no SQLite in the browser — run this on the device";
  },
  async simulateAck() {
    const oldest = [...records].reverse().find((r) => r.syncState === "pending");
    if (!oldest) return null;
    oldest.syncState = "synced";
    oldest.syncedAt = new Date().toISOString();
    return oldest;
  },
};
