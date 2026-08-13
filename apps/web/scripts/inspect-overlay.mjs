import { chromium } from "playwright";
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto("http://127.0.0.1:8080/", { waitUntil: "networkidle" });
await page.waitForTimeout(500);
const css = await page.evaluate(() => {
  const section = document.querySelector("section");
  const kids = [...section.querySelectorAll(":scope > div")];
  return kids.map((el) => {
    const cs = getComputedStyle(el);
    return {
      cls: el.className.slice(0, 80),
      z: cs.zIndex,
      op: cs.opacity,
      mix: cs.mixBlendMode,
      bg: cs.backgroundImage.slice(0, 180),
      bgc: cs.backgroundColor,
      pos: cs.position,
    };
  });
});
console.log(JSON.stringify(css, null, 2));
const img = page.locator('img[src="/heroes/green-hills.jpg"]');
await img.screenshot({ path: "/workspace/screenshots/hero-img-el.png" });
await browser.close();
