import type { Request, Response } from "express";
import type { CapturePhotoResponse } from "@nocapsnap/shared";
import { capturePhotoSchema } from "./schema";
import { PhotoService } from "./service";

const service = new PhotoService();

export async function capturePhoto(req: Request, res: Response) {
  const payload = capturePhotoSchema.parse(req.body);
  const photo = await service.capture(payload);
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
