import { bridge } from "$lib/bridge";

// Object URLs for processed photos, shared across screens for the session.
// Thumbnails are ~20–40 KB each; full photos are only requested for the hero
// and review-sized views.
const cache = new Map<string, Promise<string>>();

export function mediaUrl(sha256: string, thumb = true): Promise<string> {
  const key = `${sha256}:${thumb ? "t" : "f"}`;
  let url = cache.get(key);
  if (!url) {
    url = bridge.media(sha256, thumb).then((blob) => URL.createObjectURL(blob));
    // A failed read should be retried next time, not cached.
    url.catch(() => cache.delete(key));
    cache.set(key, url);
  }
  return url;
}
