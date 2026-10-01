import { chromium } from "playwright";
const [out, fix, y4m, fonts] = process.argv.slice(2);
const base = "http://127.0.0.1:1420";
const browser = await chromium.launch({
  executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome",
  args: ["--use-fake-device-for-media-stream", "--use-fake-ui-for-media-stream", `--use-file-for-fake-video-capture=${y4m}`],
});
const errors = [];
const at = (hhmm) => new Date(`2026-09-29T${hhmm}:00-05:00`); // a Tuesday, Chicago

async function newPage() {
  const ctx = await browser.newContext({
    viewport: { width: 375, height: 667 }, deviceScaleFactor: 3, timezoneId: "America/Chicago", locale: "en-US",
    permissions: ["camera"],
  });
  // Android's WebView renders system-ui as Roboto and serif as Noto Serif; this
  // container has neither, so the harness supplies them (test-only, not the app).
  await ctx.route("**/__fonts/*", (route) =>
    route.fulfill({ path: `${fonts}/${route.request().url().split("/").pop()}`, contentType: "font/woff2" }),
  );
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
  page.on("requestfailed", (r) => errors.push(`requestfailed: ${r.url().replace(base, "")} ${r.failure()?.errorText}`));
  return { ctx, page };
}

const { ctx, page } = await newPage();
const shot = (name, fullPage = false) => page.screenshot({ path: `${out}/${name}.png`, fullPage });
const tab = (label) => page.getByRole("navigation").getByText(label, { exact: true }).click();

await page.clock.setFixedTime(at("17:55"));
await page.goto(`${base}/`, { waitUntil: "networkidle" });
await page.getByText("No captures yet").waitFor();
await shot("00-home-empty");

await tab("Settings");
await page.getByPlaceholder("e.g. Maya").fill("Maya");
await page.getByRole("button", { name: "Save name" }).click();
await page.getByText("Saved on this device.").waitFor();

async function capture(dish, table, file, time, o = {}) {
  await page.clock.setFixedTime(at(time));
  await tab("Home");
  await page.getByRole("link", { name: "Capture a dish" }).click();
  await page.getByText("Which dish?").waitFor();
  if (o.chip) await page.getByRole("button", { name: o.chip, exact: true }).click();
  await page.locator("label.dish", { hasText: dish }).click();
  if (table) await page.getByPlaceholder("e.g. 12B").fill(table);
  if (o.shotPrepare) {
    await page.waitForTimeout(300);
    await shot("02-prepare");
    await shot("02-prepare-full", true);
  }
  await page.getByRole("button", { name: "Continue to camera" }).click();
  const phoneLink = page.getByRole("button", { name: "Use the phone camera app instead" });
  if (await phoneLink.isVisible()) await phoneLink.click();
  await page.getByText("Open camera app").waitFor();
  await page.locator('input[type="file"]').setInputFiles(`${fix}/${file}`);
  await page.getByText("The plate, as served.").waitFor();
  await page.waitForTimeout(300);
  if (o.shotReview) await shot("03-review");
  await page.getByRole("button", { name: "Save this capture" }).click();
  await page.getByText("Saved on this device.").first().waitFor();
  await page.waitForTimeout(400);
  if (o.shotInvite) await shot("05b-invite-pending");
  await page.getByRole("link", { name: "Done" }).click();
  await page.getByText("Capture a dish").first().waitFor();
}

await capture("Charred heritage carrots", "4", "carrots.jpg", "18:38");
await capture("Wild mushroom risotto", "7", "risotto.jpg", "19:11");
await capture("Saffron butter cod", "12B", "cod.jpg", "19:24", { chip: "Mains", shotPrepare: true, shotReview: true, shotInvite: true });

await page.clock.setFixedTime(at("19:30"));
await tab("Settings").catch(async () => { await page.goto(`${base}/settings`); });
await page.getByRole("button", { name: "Simulate server acknowledgement" }).click();
await page.getByText(/Marked synced/).waitFor();
await page.getByRole("button", { name: "Run device check" }).click();
await page.getByText(/preview: no SQLite/).waitFor();
await shot("07-settings-full", true);

await tab("Home");
await page.getByText("3 captures").waitFor();
await page.waitForTimeout(500);
await shot("01-home");
await shot("01-home-full", true);

// Load History's route code while online (the browser preview fetches route
// chunks lazily; the Android app serves them locally), then drop the network.
await tab("History");
await page.getByText("2 saved offline").waitFor();
await tab("Insights");
await page.getByText("Most captured").waitFor();
await tab("Home");
await ctx.setOffline(true);
await tab("History");
await page.getByText("2 saved offline").waitFor();
await page.waitForTimeout(400);
await shot("04-history-offline");
await shot("04-history-full", true);

await page.locator("a.row", { hasText: "Charred heritage carrots" }).click();
await page.getByText("Ready to share.").waitFor();
await page.waitForTimeout(300);
await shot("05-invite-synced");
await page.locator(".qr").screenshot({ path: `${out}/qr-only.png` });
await page.getByRole("link", { name: "Done" }).click();

await tab("Insights");
await page.getByText("Most captured").waitFor();
await shot("06-insights");

// The in-app viewfinder route, in its own context (Chromium's fake camera).
const cam = await newPage();
await cam.page.goto(`${base}/capture`, { waitUntil: "networkidle" });
await cam.page.locator("label.dish", { hasText: "Pork belly bao" }).click();
await cam.page.getByRole("button", { name: "Continue to camera" }).click();
await cam.page.waitForFunction(() => { const v = document.querySelector("video"); return v && v.readyState >= 2 && v.videoWidth > 0; });
await cam.page.waitForTimeout(600);
await cam.page.screenshot({ path: `${out}/08-camera-viewfinder.png` });
await cam.page.getByRole("button", { name: "Take photo" }).click();
await cam.page.getByText("The plate, as served.").waitFor();
await cam.page.goBack();
await cam.page.getByText("Start camera").waitFor(); // back from review = retake; stream stopped
const tracksLive = await cam.page.evaluate(() => !!document.querySelector("video"));
console.log("viewfinder: back-from-review leaves no live video element:", !tracksLive);
// Retake from review restarts the viewfinder once the camera step is back.
await cam.page.getByRole("button", { name: "Start camera" }).click();
await cam.page.waitForFunction(() => { const v = document.querySelector("video"); return v && v.readyState >= 2; });
await cam.page.getByRole("button", { name: "Take photo" }).click();
await cam.page.getByRole("button", { name: "Retake" }).click();
await cam.page.waitForFunction(() => { const v = document.querySelector("video"); return v && v.readyState >= 2 && v.videoWidth > 0; }, null, { timeout: 10000 });
console.log("viewfinder: Retake restarts the live camera: true");

console.log(errors.length ? `ERRORS:\n${errors.join("\n")}` : "no page errors");
await browser.close();
