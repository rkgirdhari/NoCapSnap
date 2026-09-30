import sharp from "sharp";
import { beforeEach, describe, expect, it } from "vitest";
import type { CapturePhotoRequest } from "@nocapsnap/shared";
import { HttpError } from "../../lib/errors";
import type { ObjectStorage, PutObjectInput } from "../../lib/storage";
import { decodeQr, makeJpeg } from "../../test/fixtures";
import { InMemoryPhotoRepository } from "./repository";
import { MAX_IMAGE_BYTES, PhotoService } from "./service";

class FakeStorage implements ObjectStorage {
  readonly objects = new Map<string, PutObjectInput>();
  failOnKey?: RegExp;

  async put(input: PutObjectInput): Promise<string> {
    if (this.failOnKey?.test(input.key)) throw new Error("upload failed");
    this.objects.set(input.key, input);
    return `https://cdn.test/${input.key}`;
  }
}

const LOCATION_ID = "11111111-1111-4111-8111-111111111111";
const MENU_ITEM_ID = "22222222-2222-4222-8222-222222222222";
const STAFF_ID = "33333333-3333-4333-8333-333333333333";
const PHOTO_ID = "abcdef12-3456-4789-8abc-def012345678";
const TOKEN = "3q2-7wE_9mZkP1xYbVtLcA";
const NOW = new Date("2026-09-30T15:40:00Z");

let storage: FakeStorage;
let repo: InMemoryPhotoRepository;
let service: PhotoService;
let jpegBase64: string;

beforeEach(async () => {
  storage = new FakeStorage();
  repo = new InMemoryPhotoRepository();
  service = new PhotoService({
    storage,
    repo,
    appUrl: "https://app.nocapsnap.com",
    now: () => NOW,
    newId: () => PHOTO_ID,
    newToken: () => TOKEN,
  });
  jpegBase64 ??= (await makeJpeg(1600, 1200)).toString("base64");
});

function request(overrides: Partial<CapturePhotoRequest> = {}): CapturePhotoRequest {
  return {
    locationId: LOCATION_ID,
    menuItemId: MENU_ITEM_ID,
    tableNumber: "12",
    imageBase64: jpegBase64,
    ...overrides,
  };
}

async function expectHttpError(promise: Promise<unknown>, status: number, code: string) {
  const err = await promise.then(
    () => undefined,
    (e: unknown) => e
  );
  expect(err).toBeInstanceOf(HttpError);
  expect(err).toMatchObject({ status, code });
}

describe("PhotoService.capture", () => {
  it("uploads original, watermarked, thumbnail and QR, then persists the photo", async () => {
    const photo = await service.capture(request(), { staffId: STAFF_ID });

    const prefix = `locations/${LOCATION_ID}/photos/${PHOTO_ID}`;
    expect(photo).toEqual({
      id: PHOTO_ID,
      locationId: LOCATION_ID,
      staffId: STAFF_ID,
      menuItemId: MENU_ITEM_ID,
      originalUrl: `https://cdn.test/${prefix}/original.jpg`,
      watermarkedUrl: `https://cdn.test/${prefix}/watermarked.jpg`,
      thumbnailUrl: `https://cdn.test/${prefix}/thumbnail.jpg`,
      tableNumber: "12",
      qrCodeToken: TOKEN,
      qrCodeUrl: `https://cdn.test/${prefix}/qr.png`,
      reviewUrl: `https://app.nocapsnap.com/r/${TOKEN}`,
      status: "captured",
      capturedAt: "2026-09-30T15:40:00.000Z",
    });

    expect([...storage.objects.keys()].sort()).toEqual([
      `${prefix}/original.jpg`,
      `${prefix}/qr.png`,
      `${prefix}/thumbnail.jpg`,
      `${prefix}/watermarked.jpg`,
    ]);
    expect(storage.objects.get(`${prefix}/qr.png`)?.contentType).toBe("image/png");
    expect(storage.objects.get(`${prefix}/watermarked.jpg`)?.contentType).toBe("image/jpeg");

    const qr = storage.objects.get(`${prefix}/qr.png`)!.body;
    expect(await decodeQr(qr)).toBe(photo.reviewUrl);

    expect(await service.getByToken(TOKEN)).toEqual(photo);
  });

  it("generates unguessable URL-safe tokens and fresh ids by default", async () => {
    const real = new PhotoService({ storage, repo, appUrl: "https://app.nocapsnap.com" });
    const a = await real.capture(request(), { staffId: STAFF_ID });
    const b = await real.capture(request(), { staffId: STAFF_ID });

    expect(a.qrCodeToken).toMatch(/^[A-Za-z0-9_-]{22}$/);
    expect(a.qrCodeToken).not.toBe(b.qrCodeToken);
    expect(a.id).not.toBe(b.id);
    expect(await repo.findByToken(a.qrCodeToken)).toEqual(a);
    expect(await repo.findByToken(b.qrCodeToken)).toEqual(b);
  });

  it("accepts a data URL and omits an empty table number", async () => {
    const photo = await service.capture(
      request({ imageBase64: `data:image/jpeg;base64,${jpegBase64}`, tableNumber: "" }),
      { staffId: STAFF_ID }
    );
    expect(photo.tableNumber).toBeUndefined();
  });

  it("rejects malformed base64", async () => {
    await expectHttpError(
      service.capture(request({ imageBase64: "not base64!!" }), { staffId: STAFF_ID }),
      400,
      "INVALID_BASE64"
    );
  });

  it("rejects images over the size limit before decoding them", async () => {
    const tooBig = "A".repeat(Math.ceil((MAX_IMAGE_BYTES + 3) / 3) * 4);
    await expectHttpError(
      service.capture(request({ imageBase64: tooBig }), { staffId: STAFF_ID }),
      413,
      "IMAGE_TOO_LARGE"
    );
  });

  it("rejects base64 that does not decode to a supported image", async () => {
    const gif = await sharp({ create: { width: 8, height: 8, channels: 3, background: "#000" } })
      .gif()
      .toBuffer();
    await expectHttpError(
      service.capture(request({ imageBase64: Buffer.from("hello world!").toString("base64") }), {
        staffId: STAFF_ID,
      }),
      400,
      "INVALID_IMAGE"
    );
    await expectHttpError(
      service.capture(request({ imageBase64: gif.toString("base64") }), { staffId: STAFF_ID }),
      400,
      "INVALID_IMAGE"
    );
    expect(storage.objects.size).toBe(0);
  });

  it("does not persist the photo when an upload fails", async () => {
    storage.failOnKey = /thumbnail/;
    await expect(service.capture(request(), { staffId: STAFF_ID })).rejects.toThrow("upload failed");
    expect(await service.getByToken(TOKEN)).toBeNull();
  });
});
