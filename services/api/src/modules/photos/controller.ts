import type { Request, Response } from "express";
import type { CapturePhotoResponse } from "@nocapsnap/shared";
import { config } from "../../config";
import { createStorage } from "../../lib/storage";
import { InMemoryPhotoRepository } from "./repository";
import { capturePhotoSchema } from "./schema";
import { PhotoService } from "./service";

const service = new PhotoService({
  storage: createStorage(config),
  repo: new InMemoryPhotoRepository(),
  appUrl: config.appUrl,
});

// STUB: comes from the authenticated staff session once auth lands.
const STUB_STAFF_ID = "00000000-0000-0000-0000-000000000000";

export async function capturePhoto(req: Request, res: Response) {
  const payload = capturePhotoSchema.parse(req.body);
  const photo = await service.capture(payload, { staffId: STUB_STAFF_ID });
  res.status(201).json({ photo } satisfies CapturePhotoResponse);
}

export async function getPhotoByQr(req: Request, res: Response) {
  const photo = await service.getByToken(req.params.token);
  if (!photo) {
    res.status(404).json({ error: "Photo not found" });
    return;
  }
  res.json({ photo });
}
