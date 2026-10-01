import { chromium } from "playwright";
import { randomBytes } from "node:crypto";
const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome" });
const page = await browser.newPage();
for (const n of [0, 1, 2, 3, 1000, 6 * 1024 * 1024 + 1]) {
  const buf = randomBytes(n);
  const got = await page.evaluate(async (b64in) => {
    const bytes = Uint8Array.from(atob(b64in), (c) => c.charCodeAt(0));
    // Same body as toBase64 in src/lib/bridge/device.ts
    return await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => { const url = reader.result; resolve(url.slice(url.indexOf(",") + 1)); };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(new Blob([bytes]));
    });
  }, buf.toString("base64"));
  console.log(`${n} bytes: ${got === buf.toString("base64") ? "match" : "MISMATCH"}`);
}
await browser.close();
