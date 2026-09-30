import { Router } from "express";
import { asyncHandler } from "../../lib/asyncHandler";
import { capturePhoto, getPhotoByQr } from "./controller";

export const photosRouter = Router();

photosRouter.post("/capture", asyncHandler(capturePhoto));
photosRouter.get("/qr/:token", asyncHandler(getPhotoByQr));
