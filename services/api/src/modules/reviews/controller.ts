import type { Request, Response } from "express";
import { submitReviewSchema } from "./schema";
import { ReviewService } from "./service";

const service = new ReviewService();

export async function submitReview(req: Request, res: Response) {
  const payload = submitReviewSchema.parse(req.body);
  const review = await service.submit(payload);
  res.status(201).json({ review });
}
