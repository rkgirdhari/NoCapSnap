import type { NextFunction, Request, RequestHandler, Response } from "express";

// Express 4 does not forward rejected promises to the error handler; without
// this wrapper a throwing async controller leaves the request hanging and
// crashes the process with an unhandled rejection.
export function asyncHandler(
  fn: (req: Request, res: Response, next: NextFunction) => Promise<unknown>
): RequestHandler {
  return (req, res, next) => {
    fn(req, res, next).catch(next);
  };
}
