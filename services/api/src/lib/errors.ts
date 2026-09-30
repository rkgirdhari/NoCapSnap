export class HttpError extends Error {
  constructor(
    public readonly status: number,
    message: string,
    public readonly code?: string
  ) {
    super(message);
    this.name = "HttpError";
  }
}

export class NotImplementedError extends HttpError {
  constructor(what: string) {
    super(501, `${what} not implemented`, "NOT_IMPLEMENTED");
    this.name = "NotImplementedError";
  }
}
