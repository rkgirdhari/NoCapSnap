import type { Review, SubmitReviewRequest } from "@nocapsnap/shared";
import { NotImplementedError } from "../../lib/errors";

export class ReviewService {
  async submit(_payload: SubmitReviewRequest): Promise<Review> {
    // STUB: resolve photo by token, persist review, mark photo "reviewed"
    throw new NotImplementedError("ReviewService.submit");
  }
}
