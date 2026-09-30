import cors from "cors";
import express from "express";
import { photosRouter } from "./modules/photos/routes";
import { reviewsRouter } from "./modules/reviews/routes";
import { healthRouter } from "./modules/health/routes";
import { errorHandler } from "./middleware/errorHandler";

export const app = express();

app.use(cors());
app.use(express.json({ limit: "12mb" }));

app.use("/health", healthRouter);
app.use("/api/photos", photosRouter);
app.use("/api/reviews", reviewsRouter);

app.use(errorHandler);
