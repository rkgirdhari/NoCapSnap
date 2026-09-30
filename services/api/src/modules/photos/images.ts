import sharp from "sharp";

const ALLOWED_FORMATS = new Set(["jpeg", "png", "webp"]);
/** Guards against decompression bombs; ~100 MP covers current phone sensors. */
const MAX_INPUT_PIXELS = 100_000_000;
/** Long edge of the guest-facing watermarked image. */
export const WATERMARK_MAX_EDGE = 2048;
export const THUMBNAIL_WIDTH = 480;

const BAND_COLOR = "#0B0F0C";
const ACCENT_COLOR = "#C8F5C0";
const TEXT_COLOR = "#FFFFFF";
const FONT_FAMILY = "DejaVu Sans, Liberation Sans, Arial, Helvetica, sans-serif";

export class InvalidImageError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidImageError";
  }
}

export interface Stamp {
  capturedAt: Date;
  /** IANA zone for the printed time; the location's timezone once it is known. */
  timeZone?: string;
  /** Short code printed on the image so staff/guests can match it to the record. */
  verificationCode: string;
  tableNumber?: string;
  dishName?: string;
  locationName?: string;
}

export interface ProcessedCapture {
  /** Auto-rotated, metadata (incl. any GPS EXIF) stripped, full resolution. */
  original: Buffer;
  watermarked: Buffer;
  thumbnail: Buffer;
  width: number;
  height: number;
}

export async function processCapture(input: Buffer, stamp: Stamp): Promise<ProcessedCapture> {
  await assertSupportedImage(input);

  const original = await sharp(input, { limitInputPixels: MAX_INPUT_PIXELS })
    .rotate()
    .jpeg({ quality: 92, mozjpeg: true })
    .toBuffer({ resolveWithObject: true });

  // Resize to raw pixels first so the overlay is built for the exact output size.
  const resized = await sharp(original.data)
    .resize({
      width: WATERMARK_MAX_EDGE,
      height: WATERMARK_MAX_EDGE,
      fit: "inside",
      withoutEnlargement: true,
    })
    .raw()
    .toBuffer({ resolveWithObject: true });
  const { width, height, channels } = resized.info;

  const watermarked = await sharp(resized.data, { raw: { width, height, channels } })
    .composite([{ input: Buffer.from(stampSvg(width, height, stamp)), left: 0, top: 0 }])
    .jpeg({ quality: 85, mozjpeg: true })
    .toBuffer();

  const thumbnail = await sharp(watermarked)
    .resize({ width: THUMBNAIL_WIDTH, withoutEnlargement: true })
    .jpeg({ quality: 75, mozjpeg: true })
    .toBuffer();

  return {
    original: original.data,
    watermarked,
    thumbnail,
    width: original.info.width,
    height: original.info.height,
  };
}

async function assertSupportedImage(input: Buffer) {
  let format: string | undefined;
  try {
    ({ format } = await sharp(input, { limitInputPixels: MAX_INPUT_PIXELS }).metadata());
  } catch {
    throw new InvalidImageError("imageBase64 is not a readable image");
  }
  if (!format || !ALLOWED_FORMATS.has(format)) {
    throw new InvalidImageError(`Unsupported image format "${format ?? "unknown"}"; use JPEG, PNG or WebP`);
  }
}

export function formatStampTime(date: Date, timeZone = "UTC"): string {
  return new Intl.DateTimeFormat("en-US", {
    timeZone,
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
    timeZoneName: "short",
  }).format(date);
}

/** Bottom band: "CapSnap" + #code on the first line, details on the second. */
export function stampSvg(width: number, height: number, stamp: Stamp): string {
  const pad = Math.round(width / 48);
  const titleSize = Math.round(width / 22);
  const detailSize = Math.round(width / 38);
  const bandHeight = Math.round(pad * 2 + titleSize + detailSize * 1.4);
  const bandTop = height - bandHeight;
  const titleY = bandTop + pad + Math.round(titleSize * 0.85);
  const detailY = titleY + Math.round(detailSize * 1.45);

  const details = [
    stamp.locationName,
    stamp.dishName,
    stamp.tableNumber ? `Table ${stamp.tableNumber}` : undefined,
    formatStampTime(stamp.capturedAt, stamp.timeZone),
  ]
    .filter((part): part is string => Boolean(part))
    .map((part) => truncate(part, 40))
    .join(" · ");

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">
  <rect x="0" y="${bandTop}" width="${width}" height="${bandHeight}" fill="${BAND_COLOR}" fill-opacity="0.78"/>
  <text x="${pad}" y="${titleY}" font-family="${FONT_FAMILY}" font-size="${titleSize}" font-weight="700" fill="${ACCENT_COLOR}">CapSnap</text>
  <text x="${width - pad}" y="${titleY}" text-anchor="end" font-family="${FONT_FAMILY}" font-size="${detailSize}" font-weight="700" fill="${ACCENT_COLOR}">#${escapeXml(stamp.verificationCode)}</text>
  <text x="${pad}" y="${detailY}" font-family="${FONT_FAMILY}" font-size="${detailSize}" fill="${TEXT_COLOR}">${escapeXml(details)}</text>
</svg>`;
}

function truncate(value: string, max: number): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}

export function escapeXml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}
