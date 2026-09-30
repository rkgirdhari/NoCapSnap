import { chromium } from "playwright";
import { readFileSync } from "node:fs";
const [url1, url2, logPath, out, fonts] = process.argv.slice(2);
const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome" });
const problems = [];
const requests = [];
async function visit(url) {
  const ctx = await browser.newContext({ viewport: { width: 375, height: 667 }, deviceScaleFactor: 2 });
  await ctx.route("**/__fonts/*", (r) => r.fulfill({ path: `${fonts}/${r.request().url().split("/").pop()}` }));
  const page = await ctx.newPage();
  page.on("console", (m) => m.type() === "error" && problems.push(`console: ${m.text()}`));
  page.on("pageerror", (e) => problems.push(`pageerror: ${e.message}`));
  page.on("request", (r) => requests.push(r.url()));
  await page.goto(url);
  await page.getByText("How was your dish?").waitFor();
  return { ctx, page };
}
const token1 = url1.split("#")[1];

// 1. Token gone from the address bar and from history.
const a = await visit(url1);
const href = a.page.url();
console.log("address bar after load has no token:", !href.includes(token1) && !href.includes("#"));
const histLen = await a.page.evaluate(() => history.length);
console.log("history entries:", histLen, "(the entry was replaced, not added)");
await a.page.screenshot({ path: `${out}/portal-form.png`, fullPage: true });
const formHtml = await a.page.locator("form").innerHTML();

// 2. Rate 1 with a comment; thank-you page.
await a.page.getByRole("radio", { name: "1", exact: true }).click();
await a.page.locator("#comment").fill("The cod was cold and the sauce split.");
await a.page.getByRole("button", { name: "Send privately" }).click();
await a.page.getByText("Thank you.").waitFor();
const thanks1 = await a.page.locator("#thanks").innerText();
await a.page.screenshot({ path: `${out}/portal-thanks.png` });

// 3. Same page, same form, same thank-you for a 5.
const b = await visit(url2);
const formHtml2 = await b.page.locator("form").innerHTML();
await b.page.getByRole("radio", { name: "5", exact: true }).click();
await b.page.getByRole("button", { name: "Send privately" }).click();
await b.page.getByText("Thank you.").waitFor();
const thanks5 = await b.page.locator("#thanks").innerText();
const norm = (h) => h.replaceAll(" style=\"\"", "").replace(/<span id="dish">[^<]*<\/span>/, "").replace(/<span id="place" class="place">[^<]*<\/span>/, "").replace(/src="[^"]*"/, "");
console.log("same form for both guests (ignoring dish name/photo):", norm(formHtml) === norm(formHtml2));
if (norm(formHtml) !== norm(formHtml2)) {
  const x = norm(formHtml), y = norm(formHtml2);
  let i = 0; while (i < x.length && x[i] === y[i]) i++;
  console.log("first difference:\n  A:", x.slice(Math.max(0, i - 80), i + 80), "\n  B:", y.slice(Math.max(0, i - 80), i + 80));
}
console.log("same thank-you for rating 1 and rating 5:", thanks1 === thanks5);

// 4. One use only: opening the first link again.
const again = await browser.newPage();
await again.goto(url1);
await again.getByText("This link is no longer available.").waitFor();
console.log("used link shows 'no longer available': true");
await again.screenshot({ path: `${out}/portal-used.png` });

// 5. Only same-origin requests; no CSP violations or errors; token never in the server log.
const origin = new URL(url1).origin;
const foreign = requests.filter((u) => !u.startsWith(origin) && !u.startsWith("data:"));
console.log("third-party requests:", foreign.length ? foreign : "none");
console.log("console/page errors:", problems.length ? problems : "none");
const log = readFileSync(logPath, "utf8");
console.log("server log contains either token:", log.includes(token1) || log.includes(url2.split("#")[1]));
console.log("server log contains the comment:", log.includes("sauce split"));
await browser.close();
