import sharp from "sharp";
import jsQR from "jsqr";
for (const file of process.argv.slice(2)) {
  for (const scale of [1, 0.5, 0.25]) {
    const meta = await sharp(file).metadata();
    const { data, info } = await sharp(file)
      .resize({ width: Math.round(meta.width * scale) })
      .ensureAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });
    const hit = jsQR(new Uint8ClampedArray(data), info.width, info.height, { inversionAttempts: "attemptBoth" });
    console.log(`${file.split("/").slice(-2).join("/")} @${scale}: ${hit ? `DECODED "${hit.data.slice(0, 60)}"` : "no QR found"}`);
  }
}
