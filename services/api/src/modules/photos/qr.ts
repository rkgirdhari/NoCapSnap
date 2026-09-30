import QRCode from "qrcode";

/** Dark-on-white PNG; scanners handle inverted/low-contrast codes poorly. */
export function renderQrPng(text: string): Promise<Buffer> {
  return QRCode.toBuffer(text, {
    type: "png",
    width: 512,
    margin: 2,
    errorCorrectionLevel: "M",
    color: { dark: "#0B0F0C", light: "#FFFFFF" },
  });
}
