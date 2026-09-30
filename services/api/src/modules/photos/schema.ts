import { z } from "zod";

export const capturePhotoSchema = z.object({
  locationId: z.string().uuid(),
  menuItemId: z.string().uuid(),
  tableNumber: z.string().trim().max(16).optional(),
  imageBase64: z.string().min(1, "image is required"),
});
