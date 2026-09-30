import { STATUS_CODES } from "node:http";
import type { NextFunction, Request, Response } from "express";
import { ZodError } from "zod";
import type { ApiError } from "@nocapsnap/shared";
import { HttpError } from "../lib/errors";

export function errorHandler(
  err: unknown,
  _req: Request,
  res: Response,
  _next: NextFunction
) {
  if (err instanceof ZodError) {
    const body: ApiError = {
      error: err.issues.map((i) => `${i.path.join(".") || "body"}: ${i.message}`).join("; "),
      code: "VALIDATION_ERROR",
    };
    res.status(400).json(body);
    return;
  }

  if (err instanceof HttpError) {
    const body: ApiError = { error: err.message, code: err.code };
    res.status(err.status).json(body);
    return;
  }

  // body-parser (malformed JSON, payload too large) and express.static (missing
  // file) errors carry a status. Only `expose`d messages are safe to return;
  // static-file errors include server filesystem paths.
  if (err instanceof Error && "status" in err) {
    const status = Number(err.status);
    if (status >= 400 && status < 500) {
      const message = "expose" in err && err.expose === true ? err.message : STATUS_CODES[status];
      res.status(status).json({ error: message ?? "Request error" } satisfies ApiError);
      return;
    }
  }

  console.error(err);
  res.status(500).json({ error: "Internal server error" } satisfies ApiError);
}
