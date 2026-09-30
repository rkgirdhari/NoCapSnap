import type { Photo } from "@nocapsnap/shared";

export interface PhotoRepository {
  insert(photo: Photo): Promise<void>;
  findByToken(token: string): Promise<Photo | null>;
}

// STUB: swap for a Postgres-backed repository. Data is lost on restart.
export class InMemoryPhotoRepository implements PhotoRepository {
  private readonly byToken = new Map<string, Photo>();

  async insert(photo: Photo): Promise<void> {
    this.byToken.set(photo.qrCodeToken, photo);
  }

  async findByToken(token: string): Promise<Photo | null> {
    return this.byToken.get(token) ?? null;
  }
}
