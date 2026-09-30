import type { CapturePhotoRequest, CapturePhotoResponse } from "@nocapsnap/shared";
import { api } from "./client";

export function capturePhotoStub(payload: CapturePhotoRequest) {
  return api<CapturePhotoResponse>("/api/photos/capture", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}
