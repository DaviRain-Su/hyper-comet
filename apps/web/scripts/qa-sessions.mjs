import { chromium } from "playwright";

const base = process.argv[2] ?? "http://127.0.0.1:8080";
const shot = (name) => `/workspace/screenshots/${name}`;

const browser = await chromium.launch({ args: ["--no-sandbox"] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
page.on("pageerror", (e) => console.log("PAGEERROR", e.message));
page.on("console", (m) => {
  if (m.type() === "error") console.log("CONSOLE", m.text());
});

await page.goto(base + "/", { waitUntil: "networkidle" });
await page.waitForTimeout(400);
await page.screenshot({ path: shot("landing.png"), fullPage: false });

await page.goto(base + "/login", { waitUntil: "networkidle" });
await page.getByRole("button", { name: /New here|创建账户/ }).click();
const email = `qa+${Date.now()}@proofship.dev`;
await page.getByPlaceholder("you@example.com").fill(email);
await page.getByPlaceholder(/Password|密码/).fill("gatecheck99");
await page.getByPlaceholder(/Name|名字/).fill("QA Pilot");
await page.getByRole("button", { name: /Create account|创建账户/ }).click();
await page.waitForURL(/\/sessions/, { timeout: 20000 });
await page.waitForTimeout(800);
await page.screenshot({ path: shot("sessions-empty.png") });

const bodyEmpty = await page.locator("body").innerText();
console.log("HAS_PAIRING", /remote panel|远程面板|Attach desktop|连接桌面/.test(bodyEmpty));
console.log("NO_CLOUD_COPY", /will not draft in the cloud|不会在云端|do not call a cloud/.test(bodyEmpty));

await page.getByRole("button", { name: /RWA share registry|RWA 份额登记/ }).click();
await page.waitForTimeout(2500);
await page.screenshot({ path: shot("sessions-rwa.png") });

const body = await page.locator("body").innerText();
console.log("HAS_GATE", /passed|通过|fail-closed/.test(body));
console.log("HAS_LEAN", /import ProofForgeV2/.test(body));
console.log("HAS_STARTER_NOTE", /Read-only starter|只读模板/.test(body));
console.log("COMPOSER_LOCKED", /Attach desktop to send|先连接桌面/.test(body));
console.log("URL", page.url());

const mobile = await browser.newPage({ viewport: { width: 390, height: 844 } });
await mobile.goto(base + "/", { waitUntil: "networkidle" });
await mobile.waitForTimeout(400);
await mobile.screenshot({ path: shot("landing-mobile.png") });
const overflow = await mobile.evaluate(
  () => document.documentElement.scrollWidth > document.documentElement.clientWidth + 2,
);
console.log("MOBILE_OVERFLOW", overflow);

await browser.close();
