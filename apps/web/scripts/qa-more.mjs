import { chromium } from "playwright";
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto("http://127.0.0.1:8080/", { waitUntil: "networkidle" });
await page.evaluate(() => document.querySelector("#product")?.scrollIntoView({ block: "start" }));
await page.waitForTimeout(400);
await page.screenshot({ path: "/workspace/screenshots/landing-product.png" });
await page.evaluate(() => document.querySelector("#pricing")?.scrollIntoView({ block: "start" }));
await page.waitForTimeout(400);
await page.screenshot({ path: "/workspace/screenshots/landing-pricing.png" });

const mobile = await browser.newPage({ viewport: { width: 390, height: 844 } });
await mobile.goto("http://127.0.0.1:8080/", { waitUntil: "networkidle" });
const overflow = await mobile.evaluate(() => {
  const doc = document.documentElement;
  return { scrollW: doc.scrollWidth, clientW: doc.clientWidth, overflow: doc.scrollWidth > doc.clientWidth + 1 };
});
console.log("mobile overflow", overflow);
await mobile.screenshot({ path: "/workspace/screenshots/landing-mobile.png" });
await browser.close();
