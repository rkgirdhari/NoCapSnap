import sharp, { type Region } from "sharp";
import { describe, expect, it } from "vitest";
import { makeJpeg } from "../../test/fixtures";
import {
  InvalidImageError,
  WATERMARK_MAX_EDGE,
  THUMBNAIL_WIDTH,
  escapeXml,
  formatStampTime,
  processCapture,
  stampSvg,
} from "./images";

// stats() ignores extract() in the same pipeline, so crop to a buffer first.
async function regionStats(image: Buffer, region: Region) {
  return sharp(await sharp(image).extract(region).toBuffer()).stats();
}

const stamp = {
  capturedAt: new Date("2026-09-30T15:40:00Z"),
  verificationCode: "AB12CD34",
  tableNumber: "12",
};

describe("processCapture", () => {
  it("stamps a readable band across the bottom of the image", async () => {
    const out = await processCapture(await makeJpeg(1200, 900), stamp);

    const meta = await sharp(out.watermarked).metadata();
    expect(meta).toMatchObject({ format: "jpeg", width: 1200, height: 900 });

    // Above the band the photo is untouched (orange); the band (bottom 150px
    // at this width) is dark, with light text drawn on its left side.
    const photo = await regionStats(out.watermarked, { left: 0, top: 0, width: 1200, height: 700 });
    expect(photo.channels[0].mean).toBeGreaterThan(190);
    const emptyBand = await regionStats(out.watermarked, { left: 820, top: 760, width: 120, height: 140 });
    expect(emptyBand.channels[0].mean).toBeLessThan(70);
    const textBand = await regionStats(out.watermarked, { left: 0, top: 760, width: 600, height: 140 });
    expect(textBand.channels[1].max).toBeGreaterThan(200);
  });

  it("caps the watermarked long edge but keeps the original at full size", async () => {
    const out = await processCapture(await makeJpeg(4000, 3000), stamp);

    expect(out).toMatchObject({ width: 4000, height: 3000 });
    const original = await sharp(out.original).metadata();
    expect(original).toMatchObject({ width: 4000, height: 3000 });
    const watermarked = await sharp(out.watermarked).metadata();
    expect(watermarked).toMatchObject({ width: WATERMARK_MAX_EDGE, height: 1536 });
    const thumbnail = await sharp(out.thumbnail).metadata();
    expect(thumbnail).toMatchObject({ format: "jpeg", width: THUMBNAIL_WIDTH, height: 360 });
  });

  it("applies EXIF orientation and strips metadata", async () => {
    const out = await processCapture(await makeJpeg(1200, 800, { orientation: 6 }), stamp);

    expect(out).toMatchObject({ width: 800, height: 1200 });
    for (const buf of [out.original, out.watermarked, out.thumbnail]) {
      const meta = await sharp(buf).metadata();
      expect(meta.orientation).toBeUndefined();
      expect(meta.exif).toBeUndefined();
    }
  });

  it("rejects bytes that are not an image", async () => {
    await expect(processCapture(Buffer.from("definitely not a jpeg"), stamp)).rejects.toBeInstanceOf(
      InvalidImageError
    );
  });

  it("rejects image formats other than JPEG, PNG and WebP", async () => {
    const gif = await sharp({
      create: { width: 20, height: 20, channels: 3, background: "#ff0000" },
    })
      .gif()
      .toBuffer();
    await expect(processCapture(gif, stamp)).rejects.toThrow(/Unsupported image format "gif"/);
  });

  it("accepts PNG input", async () => {
    const png = await sharp({
      create: { width: 640, height: 480, channels: 4, background: "#336699" },
    })
      .png()
      .toBuffer();
    const out = await processCapture(png, stamp);
    expect((await sharp(out.watermarked).metadata()).format).toBe("jpeg");
  });
});

describe("stampSvg", () => {
  it("escapes staff-entered text so it cannot break out of the SVG", async () => {
    const svg = stampSvg(1200, 900, { ...stamp, tableNumber: `</text><image href="x"/>&` });
    expect(svg).not.toContain("<image");
    expect(svg).toContain("Table &lt;/text&gt;&lt;image href=&quot;x&quot;/&gt;&amp;");
    // And it still renders.
    const out = await processCapture(await makeJpeg(1200, 900), {
      ...stamp,
      tableNumber: `</text><image href="x"/>&`,
    });
    expect((await sharp(out.watermarked).metadata()).width).toBe(1200);
  });

  it("includes location, dish, table and time when known", () => {
    const svg = stampSvg(1200, 900, {
      ...stamp,
      locationName: "Hammurabi Grill",
      dishName: "Lamb Kofta",
    });
    expect(svg).toContain("Hammurabi Grill · Lamb Kofta · Table 12 · Sep 30, 2026, 15:40 UTC");
    expect(svg).toContain("#AB12CD34");
  });

  it("truncates very long dish names", () => {
    const svg = stampSvg(1200, 900, { ...stamp, dishName: "x".repeat(80) });
    expect(svg).toContain(`${"x".repeat(39)}…`);
    expect(svg).not.toContain("x".repeat(40));
  });
});

describe("formatStampTime", () => {
  it("renders in the given time zone", () => {
    const date = new Date("2026-09-30T15:40:00Z");
    expect(formatStampTime(date)).toBe("Sep 30, 2026, 15:40 UTC");
    expect(formatStampTime(date, "America/New_York")).toBe("Sep 30, 2026, 11:40 EDT");
  });
});

describe("escapeXml", () => {
  it("escapes all XML special characters", () => {
    expect(escapeXml(`<a href="x">'&'</a>`)).toBe("&lt;a href=&quot;x&quot;&gt;&apos;&amp;&apos;&lt;/a&gt;");
  });
});
