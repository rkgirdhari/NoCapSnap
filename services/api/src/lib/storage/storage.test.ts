import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { mockClient } from "aws-sdk-client-mock";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { loadConfig } from "../../config";
import { createStorage, LocalDiskStorage, S3Storage } from ".";

describe("S3Storage", () => {
  const s3 = mockClient(S3Client);
  beforeEach(() => {
    s3.reset();
    s3.on(PutObjectCommand).resolves({});
  });

  it("puts the object with its content type and returns the S3 URL", async () => {
    const storage = new S3Storage({ bucket: "nocapsnap-photos", region: "us-east-1" });
    const url = await storage.put({
      key: "locations/l/photos/p/watermarked.jpg",
      body: Buffer.from("jpeg"),
      contentType: "image/jpeg",
    });

    expect(url).toBe("https://nocapsnap-photos.s3.us-east-1.amazonaws.com/locations/l/photos/p/watermarked.jpg");
    const calls = s3.commandCalls(PutObjectCommand);
    expect(calls).toHaveLength(1);
    expect(calls[0].args[0].input).toMatchObject({
      Bucket: "nocapsnap-photos",
      Key: "locations/l/photos/p/watermarked.jpg",
      ContentType: "image/jpeg",
      CacheControl: "public, max-age=31536000, immutable",
    });
  });

  it("serves from CloudFront when a domain is configured", () => {
    const storage = new S3Storage({
      bucket: "b",
      region: "us-east-1",
      cloudfrontDomain: "https://d123.cloudfront.net/",
    });
    expect(storage.urlFor("a/b.jpg")).toBe("https://d123.cloudfront.net/a/b.jpg");
  });

  it("uses path-style URLs for a custom endpoint", () => {
    const storage = new S3Storage({ bucket: "b", region: "us-east-1", endpoint: "http://localhost:9000/" });
    expect(storage.urlFor("a/b.jpg")).toBe("http://localhost:9000/b/a/b.jpg");
  });

  it("propagates upload failures", async () => {
    s3.on(PutObjectCommand).rejects(new Error("AccessDenied"));
    const storage = new S3Storage({ bucket: "b", region: "us-east-1" });
    await expect(storage.put({ key: "k", body: Buffer.alloc(1), contentType: "image/png" })).rejects.toThrow(
      "AccessDenied"
    );
  });
});

describe("LocalDiskStorage", () => {
  let dir: string;
  beforeAll(async () => {
    dir = await mkdtemp(path.join(os.tmpdir(), "capsnap-storage-"));
  });
  afterAll(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("writes under the root and returns a URL under the public base", async () => {
    const storage = new LocalDiskStorage(dir, "http://localhost:3000/files/");
    const url = await storage.put({ key: "a/b/c.png", body: Buffer.from("png"), contentType: "image/png" });

    expect(url).toBe("http://localhost:3000/files/a/b/c.png");
    expect(await readFile(path.join(dir, "a/b/c.png"), "utf8")).toBe("png");
  });

  it("refuses keys that escape the root", async () => {
    const storage = new LocalDiskStorage(dir, "http://localhost:3000/files");
    await expect(
      storage.put({ key: "../escape.txt", body: Buffer.from("x"), contentType: "text/plain" })
    ).rejects.toThrow(/outside storage root/);
  });
});

describe("createStorage / loadConfig", () => {
  it("defaults to local storage, treating empty env values as unset", () => {
    const config = loadConfig({ STORAGE_DRIVER: "", LOCAL_STORAGE_DIR: "", PORT: "", API_URL: "" });
    expect(config.port).toBe(3000);
    expect(config.storage).toEqual({ driver: "local", dir: path.resolve(".data/uploads") });
    expect(createStorage(config)).toBeInstanceOf(LocalDiskStorage);
  });

  it("selects S3 when configured", () => {
    const config = loadConfig({ STORAGE_DRIVER: "s3", S3_BUCKET: "b", AWS_REGION: "eu-west-1" });
    expect(config.storage).toEqual({
      driver: "s3",
      bucket: "b",
      region: "eu-west-1",
      cloudfrontDomain: undefined,
      endpoint: undefined,
    });
    expect(createStorage(config)).toBeInstanceOf(S3Storage);
  });

  it("fails fast on a missing bucket or unknown driver", () => {
    expect(() => loadConfig({ STORAGE_DRIVER: "s3" })).toThrow(/requires S3_BUCKET/);
    expect(() => loadConfig({ STORAGE_DRIVER: "gcs" })).toThrow(/Unknown STORAGE_DRIVER "gcs"/);
  });
});
