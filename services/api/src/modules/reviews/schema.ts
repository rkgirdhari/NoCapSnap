import { z } from "zod";

export const submitReviewSchema = z.object({
  token: z.string().min(1),
  rating: z.union([z.literal(1), z.literal(2), z.literal(3), z.literal(4), z.literal(5)]),
  comment: z.string().trim().max(2000).optional(),
  customerName: z.string().trim().max(80).optional(),
});
