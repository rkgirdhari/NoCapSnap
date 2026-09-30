export interface PutObjectInput {
  key: string;
  body: Buffer;
  contentType: string;
}

export interface ObjectStorage {
  /** Stores the object and returns the URL it can be fetched from. */
  put(input: PutObjectInput): Promise<string>;
}
