import sharp from "sharp";
import jsQR from "jsqr";

export async function makeJpeg(
  width: number,
  height: number,
  options: { orientation?: number } = {}
): Promise<Buffer> {
  let img = sharp({
    create: { width, height, channels: 3, background: { r: 210, g: 120, b: 60 } },
  }).jpeg();
  if (options.orientation) img = img.withMetadata({ orientation: options.orientation });
  return img.toBuffer();
}

export async function decodeQr(png: Buffer): Promise<string | undefined> {
  const { data, info } = await sharp(png)
    .ensureAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  const pixels = new Uint8ClampedArray(data.buffer, data.byteOffset, data.length);
  return jsQR(pixels, info.width, info.height)?.data;
}
