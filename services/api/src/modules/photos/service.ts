import type { CapturePhotoRequest, Photo } from "@nocapsnap/shared";
import { NotImplementedError } from "../../lib/errors";

export class PhotoService {
  async capture(_payload: CapturePhotoRequest): Promise<Photo> {
    // STUB: validate staff, write S3, watermark, persist Postgres
    throw new NotImplementedError("PhotoService.capture");
  }

  async getByToken(_token: string): Promise<Photo | null> {
    // STUB: lookup by qr_code_token
    return null;
  }
}
