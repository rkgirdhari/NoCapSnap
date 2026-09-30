import { randomBytes, randomUUID } from "node:crypto";
import { photoVerificationCode, type CapturePhotoRequest, type Photo, type UUID } from "@nocapsnap/shared";
import { HttpError } from "../../lib/errors";
import type { ObjectStorage } from "../../lib/storage";
import { InvalidImageError, processCapture } from "./images";
import { renderQrPng } from "./qr";
import type { PhotoRepository } from "./repository";

/** Decoded size cap; its base64 form stays under the API's 12 MB JSON limit. */
export const MAX_IMAGE_BYTES = 8 * 1024 * 1024;

const DATA_URL_PREFIX = /^data:image\/[a-z0-9.+-]+;base64,/i;
const BASE64 = /^[A-Za-z0-9+/]+={0,2}$/;

export interface PhotoServiceDeps {
  storage: ObjectStorage;
  repo: PhotoRepository;
  /** Guest web app base URL; the QR code encodes `${appUrl}/r/${token}`. */
  appUrl: string;
  now?: () => Date;
  newId?: () => UUID;
  newToken?: () => string;
}

export interface CaptureContext {
  staffId: UUID;
}

export class PhotoService {
  private readonly now: () => Date;
  private readonly newId: () => UUID;
  private readonly newToken: () => string;

  constructor(private readonly deps: PhotoServiceDeps) {
    this.now = deps.now ?? (() => new Date());
    this.newId = deps.newId ?? randomUUID;
    // 128 bits, URL-safe: guests can't enumerate other tables' photos.
    this.newToken = deps.newToken ?? (() => randomBytes(16).toString("base64url"));
  }

  async capture(payload: CapturePhotoRequest, ctx: CaptureContext): Promise<Photo> {
    // STUB: verify ctx.staffId belongs to payload.locationId and look up the
    // dish/location names for the stamp once those tables exist.
    const image = decodeImage(payload.imageBase64);
    const id = this.newId();
    const token = this.newToken();
    const capturedAt = this.now();

    let processed;
    try {
      processed = await processCapture(image, {
        capturedAt,
        verificationCode: photoVerificationCode(id),
        tableNumber: payload.tableNumber || undefined,
      });
    } catch (err) {
      if (err instanceof InvalidImageError) {
        throw new HttpError(400, err.message, "INVALID_IMAGE");
      }
      throw err;
    }

    const reviewUrl = `${this.deps.appUrl}/r/${token}`;
    const qrPng = await renderQrPng(reviewUrl);

    const prefix = `locations/${payload.locationId}/photos/${id}`;
    const { storage } = this.deps;
    const [originalUrl, watermarkedUrl, thumbnailUrl, qrCodeUrl] = await Promise.all([
      storage.put({ key: `${prefix}/original.jpg`, body: processed.original, contentType: "image/jpeg" }),
      storage.put({ key: `${prefix}/watermarked.jpg`, body: processed.watermarked, contentType: "image/jpeg" }),
      storage.put({ key: `${prefix}/thumbnail.jpg`, body: processed.thumbnail, contentType: "image/jpeg" }),
      storage.put({ key: `${prefix}/qr.png`, body: qrPng, contentType: "image/png" }),
    ]);

    const photo: Photo = {
      id,
      locationId: payload.locationId,
      staffId: ctx.staffId,
      menuItemId: payload.menuItemId,
      originalUrl,
      watermarkedUrl,
      thumbnailUrl,
      tableNumber: payload.tableNumber || undefined,
      qrCodeToken: token,
      qrCodeUrl,
      reviewUrl,
      status: "captured",
      capturedAt: capturedAt.toISOString(),
    };
    await this.deps.repo.insert(photo);
    return photo;
  }

  async getByToken(token: string): Promise<Photo | null> {
    return this.deps.repo.findByToken(token);
  }
}

function decodeImage(imageBase64: string): Buffer {
  const cleaned = imageBase64.replace(DATA_URL_PREFIX, "").replace(/\s+/g, "");
  if (!cleaned || cleaned.length % 4 !== 0 || !BASE64.test(cleaned)) {
    throw new HttpError(400, "imageBase64 is not valid base64", "INVALID_BASE64");
  }
  // Check the decoded length before allocating it.
  const padding = cleaned.endsWith("==") ? 2 : cleaned.endsWith("=") ? 1 : 0;
  if ((cleaned.length / 4) * 3 - padding > MAX_IMAGE_BYTES) {
    throw new HttpError(413, `Image exceeds ${MAX_IMAGE_BYTES / (1024 * 1024)} MB`, "IMAGE_TOO_LARGE");
  }
  return Buffer.from(cleaned, "base64");
}
