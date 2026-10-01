import { chromium } from "playwright";
const [out, fix, fonts] = process.argv.slice(2);
const base = "http://127.0.0.1:1420";
const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome" });
const errors = [];
const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 3, timezoneId: "America/Chicago", locale: "en-US" });
await ctx.route("**/__fonts/*", (r) => r.fulfill({ path: `${fonts}/${r.request().url().split("/").pop()}`, contentType: "font/woff2" }));
await ctx.addInitScript(() => {
  const css = `@font-face{font-family:"Roboto";src:url(/__fonts/roboto.woff2) format("woff2");font-weight:100 900}
@font-face{font-family:"Noto Serif";src:url(/__fonts/noto-serif.woff2) format("woff2");font-weight:100 900}
:root{--sans:"Roboto",sans-serif!important;--serif:"Noto Serif",serif!important}`;
  const add = () => { const s = document.createElement("style"); s.textContent = css; document.head.append(s); };
  if (document.head) add(); else document.addEventListener("DOMContentLoaded", add);
});
const page = await ctx.newPage();
page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
page.on("console", (m) => m.type() === "error" && errors.push(`console: ${m.text()}`));
const shot = (name, fullPage = false) => page.screenshot({ path: `${out}/${name}.png`, fullPage });
const tab = (label) => page.getByRole("navigation").getByText(label, { exact: true }).click();

async function capture(dish, table, file) {
  await tab("Home");
  await page.getByRole("link", { name: "Capture a dish" }).click();
  await page.getByText("Which dish?").waitFor();
  await page.locator("label.dish", { hasText: dish }).click();
  if (table) await page.getByPlaceholder("e.g. 12B").fill(table);
  await page.getByRole("button", { name: "Continue to camera" }).click();
  const phoneLink = page.getByRole("button", { name: "Use the phone camera app instead" });
  if (await phoneLink.isVisible()) await phoneLink.click();
  await page.getByText("Open camera app").waitFor();
  await page.locator('input[type="file"]').setInputFiles(`${fix}/${file}`);
  await page.getByRole("button", { name: "Save this capture" }).click();
  await page.getByText("Saved on this device.").first().waitFor();
}

await page.goto(`${base}/`, { waitUntil: "networkidle" });
// 1. A demo plate before signing in.
await capture("Wild mushroom risotto", null, "risotto.jpg");
await page.waitForTimeout(300);
await shot("w3-01-invite-demo");
await page.getByRole("link", { name: "Done" }).click();

// 2. Sign in (preview accepts any credentials).
await tab("Settings");
await page.getByText("Server address").waitFor();
await shot("w3-02-settings-signed-out", true);
await page.getByLabel("Server address").fill("https://capsnap.atelier8.example");
await page.getByLabel("Sign-in name").fill("maya@atelier");
await page.getByLabel("Password").fill("correct horse battery");
await page.getByRole("button", { name: "Sign in" }).click();
await page.getByText("Signed in. Plates now sync").waitFor();
await shot("w3-03-settings-signed-in", true);

// 3. A real plate: pending until synced.
await capture("Saffron butter cod", "12B", "cod.jpg");
await page.waitForTimeout(300);
await shot("w3-04-invite-pending");
await page.getByRole("link", { name: "Done" }).click();

// 4. History: retry = sync now.
await tab("History");
await page.getByRole("button", { name: "Retry when connected" }).click();
await page.getByText("1 synced.").waitFor();
await page.waitForTimeout(300);
await shot("w3-05-history-after-sync", true);

// 5. The synced plate's real QR.
await page.locator("a.row", { hasText: "Saffron butter cod" }).click();
await page.getByText("Ready to share.").waitFor();
await page.waitForTimeout(300);
await shot("w3-06-invite-qr");
await page.locator(".qr").screenshot({ path: `${out}/qr-real.png` });
// Also a phone-sized render (about 2.5 cm wide at 3x), to check small scans.
console.log(errors.length ? `ERRORS:\n${errors.join("\n")}` : "no page errors");
await browser.close();
