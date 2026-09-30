import type { Config } from "../../config";
import { LocalDiskStorage } from "./local";
import { S3Storage } from "./s3";
import type { ObjectStorage } from "./types";

export type { ObjectStorage, PutObjectInput } from "./types";
export { LocalDiskStorage } from "./local";
export { S3Storage } from "./s3";

export const LOCAL_FILES_ROUTE = "/files";

export function createStorage(config: Config): ObjectStorage {
  if (config.storage.driver === "s3") {
    return new S3Storage(config.storage);
  }
  return new LocalDiskStorage(config.storage.dir, `${config.apiUrl}${LOCAL_FILES_ROUTE}`);
}
