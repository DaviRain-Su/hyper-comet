import { chromium } from "playwright";
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto("http://127.0.0.1:8080/", { waitUntil: "networkidle" });
await page.waitForTimeout(800);
const info = await page.evaluate(() => {
  const section = document.querySelector("section");
  const wraps = [...section.querySelectorAll(":scope > div:first-child > div")].map((el, i) => {
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    const img = el.querySelector("img");
    const ir = img?.getBoundingClientRect();
    return {
      i,
      tag: el.tagName,
      cls: el.className.slice(0, 120),
      op: cs.opacity,
      z: cs.zIndex,
      vis: cs.visibility,
      overflow: cs.overflow,
      w: Math.round(r.width),
      h: Math.round(r.height),
      t: Math.round(r.top),
      img: img && {
        src: img.getAttribute("src"),
        w: Math.round(ir.width),
        h: Math.round(ir.height),
        nw: img.naturalWidth,
        op: getComputedStyle(img).opacity,
        pos: getComputedStyle(img).position,
        obj: getComputedStyle(img).objectFit,
      },
    };
  });
  const overlay = getComputedStyle(section);
  return {
    section: {
      h: Math.round(section.getBoundingClientRect().height),
      bg: getComputedStyle(section).backgroundColor,
      overflow: overlay.overflow,
    },
    wraps,
  };
});
console.log(JSON.stringify(info, null, 2));
await browser.close();
