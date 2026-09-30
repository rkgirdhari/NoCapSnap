import sharp from "sharp";
import { describe, expect, it } from "vitest";
import { decodeQr } from "../../test/fixtures";
import { renderQrPng } from "./qr";

describe("renderQrPng", () => {
  it("renders a scannable PNG that encodes the review URL", async () => {
    const url = "https://app.nocapsnap.com/r/3q2-7wE_9mZkP1xYbVtLcA";
    const png = await renderQrPng(url);

    expect(await sharp(png).metadata()).toMatchObject({ format: "png", width: 512, height: 512 });
    expect(await decodeQr(png)).toBe(url);
  });
});
