import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import type { ObjectStorage, PutObjectInput } from "./types";

/** Development storage: writes under `dir`, served by the API at `/files`. */
export class LocalDiskStorage implements ObjectStorage {
  private readonly root: string;

  constructor(
    dir: string,
    private readonly publicBaseUrl: string
  ) {
    this.root = path.resolve(dir);
  }

  async put({ key, body }: PutObjectInput): Promise<string> {
    const target = path.resolve(this.root, key);
    if (!target.startsWith(this.root + path.sep)) {
      throw new Error(`Refusing to write outside storage root: ${key}`);
    }
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(target, body);
    return `${this.publicBaseUrl.replace(/\/+$/, "")}/${key}`;
  }
}
