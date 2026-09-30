import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import type { ObjectStorage, PutObjectInput } from "./types";

export interface S3StorageOptions {
  bucket: string;
  region: string;
  cloudfrontDomain?: string;
  endpoint?: string;
  client?: S3Client;
}

// Credentials come from the AWS SDK default chain (AWS_ACCESS_KEY_ID /
// AWS_SECRET_ACCESS_KEY, an instance role, etc.).
export class S3Storage implements ObjectStorage {
  private readonly client: S3Client;

  constructor(private readonly options: S3StorageOptions) {
    this.client =
      options.client ??
      new S3Client({
        region: options.region,
        endpoint: options.endpoint,
        // MinIO and LocalStack need path-style URLs.
        forcePathStyle: Boolean(options.endpoint),
      });
  }

  async put({ key, body, contentType }: PutObjectInput): Promise<string> {
    await this.client.send(
      new PutObjectCommand({
        Bucket: this.options.bucket,
        Key: key,
        Body: body,
        ContentType: contentType,
        // Keys are unique per photo and never overwritten.
        CacheControl: "public, max-age=31536000, immutable",
      })
    );
    return this.urlFor(key);
  }

  urlFor(key: string): string {
    const { bucket, region, cloudfrontDomain, endpoint } = this.options;
    if (cloudfrontDomain) {
      return `https://${cloudfrontDomain.replace(/^https?:\/\//, "").replace(/\/+$/, "")}/${key}`;
    }
    if (endpoint) {
      return `${endpoint.replace(/\/+$/, "")}/${bucket}/${key}`;
    }
    return `https://${bucket}.s3.${region}.amazonaws.com/${key}`;
  }
}
