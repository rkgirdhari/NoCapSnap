import path from "node:path";

export type StorageDriver = "s3" | "local";

export interface Config {
  port: number;
  /** Base URL of the guest-facing web app; review QR codes point here. */
  appUrl: string;
  /** Public base URL of this API; local storage serves files under it. */
  apiUrl: string;
  storage:
    | {
        driver: "s3";
        bucket: string;
        region: string;
        /** Serve objects from this CloudFront domain instead of the S3 URL. */
        cloudfrontDomain?: string;
        /** S3-compatible endpoint (MinIO, R2, LocalStack). */
        endpoint?: string;
      }
    | {
        driver: "local";
        dir: string;
      };
}

function readStorage(env: NodeJS.ProcessEnv): Config["storage"] {
  const driver = (env.STORAGE_DRIVER || "local") as StorageDriver;
  if (driver === "s3") {
    const bucket = env.S3_BUCKET;
    if (!bucket) throw new Error("STORAGE_DRIVER=s3 requires S3_BUCKET");
    return {
      driver,
      bucket,
      region: env.AWS_REGION || "us-east-1",
      cloudfrontDomain: env.CLOUDFRONT_DOMAIN || undefined,
      endpoint: env.S3_ENDPOINT || undefined,
    };
  }
  if (driver === "local") {
    return {
      driver,
      dir: path.resolve(env.LOCAL_STORAGE_DIR || ".data/uploads"),
    };
  }
  throw new Error(`Unknown STORAGE_DRIVER "${driver}" (expected "s3" or "local")`);
}

export function loadConfig(env: NodeJS.ProcessEnv = process.env): Config {
  const port = Number(env.PORT || 3000);
  return {
    port,
    appUrl: (env.APP_URL || "https://app.nocapsnap.com").replace(/\/+$/, ""),
    apiUrl: (env.API_URL || `http://localhost:${port}`).replace(/\/+$/, ""),
    storage: readStorage(env),
  };
}

export const config = loadConfig();
