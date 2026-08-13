import { chromium } from "playwright";

const browser = await chromium.launch({
  headless: true,
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});
const errors = [];
const failed = [];

async function shot(page, path) {
  await page.screenshot({ path, fullPage: false });
}

const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
page.on("console", (m) => {
  if (m.type() === "error") errors.push(m.text());
});
page.on("pageerror", (e) => errors.push(String(e)));
page.on("requestfailed", (r) => failed.push(r.url() + " " + r.failure()?.errorText));

await page.goto("http://127.0.0.1:8080/", { waitUntil: "networkidle", timeout: 45000 });
await page.waitForTimeout(1200);
await shot(page, "/workspace/screenshots/landing-wide.png");

const showcase = page.locator("#showcase");
await showcase.scrollIntoViewIfNeeded();
await page.waitForTimeout(400);
await shot(page, "/workspace/screenshots/landing-product.png");

const webTab = page.getByRole("button", { name: /Web 开箱|Web empty/ });
if (await webTab.count()) {
  await webTab.click();
  await page.waitForTimeout(300);
  await shot(page, "/workspace/screenshots/landing-product-empty.png");
}
const sessionTab = page.getByRole("button", { name: /Web 会话中|Web in session/ });
if (await sessionTab.count()) {
  await sessionTab.click();
  await page.waitForTimeout(300);
  await shot(page, "/workspace/screenshots/landing-product-session.png");
}

const mobile = await browser.newPage({ viewport: { width: 390, height: 844 } });
await mobile.goto("http://127.0.0.1:8080/", { waitUntil: "networkidle", timeout: 45000 });
await mobile.waitForTimeout(800);
await mobile.screenshot({ path: "/workspace/screenshots/landing-mobile.png" });
const overflow = await mobile.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 2);

await page.goto("http://127.0.0.1:8080/login?redirect=/sessions", { waitUntil: "networkidle" });
await page.waitForTimeout(400);
const email = `qa-${Date.now()}@proofship.dev`;
if (!(await page.locator('input[autocomplete="name"]').count())) {
  await page.getByRole("button", { name: /新用户|New here/ }).click();
  await page.waitForTimeout(200);
}
await page.locator('input[type="email"]').fill(email);
await page.locator('input[type="password"]').fill("password1234");
await page.locator('input[autocomplete="name"]').fill("QA");
await page.locator('form button[type="submit"]').click();
await page.waitForTimeout(1500);
await shot(page, "/workspace/screenshots/sessions-empty.png");

const starter = page.getByRole("button", { name: /RWA/ }).first();
if (await starter.count()) {
  await starter.click();
  await page.waitForTimeout(800);
  await shot(page, "/workspace/screenshots/sessions-rwa.png");
}

const ops = page.getByRole("button", { name: /^Ops$|^运维$/ });
if (await ops.count()) {
  await ops.click();
  await page.waitForTimeout(300);
  await shot(page, "/workspace/screenshots/sessions-ops.png");
}

console.log(JSON.stringify({ errors, failed, mobileOverflow: overflow, url: page.url() }, null, 2));
await browser.close();
process.exit(errors.length ? 2 : 0);
