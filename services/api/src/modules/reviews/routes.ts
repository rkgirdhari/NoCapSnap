import { Router } from "express";
import { asyncHandler } from "../../lib/asyncHandler";
import { submitReview } from "./controller";

export const reviewsRouter = Router();

reviewsRouter.post("/", asyncHandler(submitReview));
