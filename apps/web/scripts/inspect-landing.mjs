import { chromium } from "playwright";
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox", "--disable-dev-shm-usage"] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
page.on("pageerror", (e) => errors.push(String(e)));
const failed = [];
page.on("requestfailed", (r) => failed.push(r.url() + " " + r.failure()?.errorText));
await page.goto("http://127.0.0.1:8080/", { waitUntil: "networkidle", timeout: 45000 });
await page.waitForTimeout(1500);
const info = await page.evaluate(() => {
  const imgs = [...document.querySelectorAll("img")].map((img) => ({
    src: img.getAttribute("src"),
    w: img.naturalWidth,
    h: img.naturalHeight,
    complete: img.complete,
    op: getComputedStyle(img).opacity,
    display: getComputedStyle(img).display,
  }));
  const h1 = document.querySelector("h1")?.innerText ?? null;
  const sections = [...document.querySelectorAll("section")].map((s) => ({
    id: s.id,
    h: Math.round(s.getBoundingClientRect().height),
    cls: s.className.slice(0, 90),
  }));
  return {
    h1,
    title: document.title,
    bodyStart: document.body.innerText.slice(0, 500),
    imgs,
    sections,
    htmlLen: document.body.innerHTML.length,
  };
});
console.log(JSON.stringify({ info, errors, failed }, null, 2));
await page.screenshot({ path: "/workspace/screenshots/landing-wide.png", fullPage: false });
await page.screenshot({ path: "/workspace/screenshots/landing-full.png", fullPage: true });
await browser.close();
