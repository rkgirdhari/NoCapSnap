import type { UUID } from "./types";

/** Short code stamped on the watermark and shown to staff to match a photo. */
export function photoVerificationCode(photoId: UUID): string {
  return photoId.replace(/-/g, "").slice(0, 8).toUpperCase();
}
