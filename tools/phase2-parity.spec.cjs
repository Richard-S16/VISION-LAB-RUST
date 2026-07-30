const { test, expect } = require("@playwright/test");
const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.resolve(__dirname, "..");
const sourceRoot = path.resolve(projectRoot, "..", "VISION-LAB");
const resultRoot = path.join(projectRoot, "tests", "results", "phase-2");

test("Phase 2 browser parity shell uses local visual assets", async ({ page }) => {
  fs.mkdirSync(resultRoot, { recursive: true });
  const requests = [];
  const errors = [];
  page.on("request", (request) => requests.push(request.url()));
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#statusText")).toHaveText("READY", { timeout: 120_000 });
  await page.waitForTimeout(1000);

  await expect(page.locator("#detectBtn")).toHaveText("DETECT");
  await expect(page.locator("#thresholdValue")).toHaveText("50%");
  expect(await page.evaluate(() => document.fonts.check('16px "Michroma"'))).toBe(true);
  expect(await page.evaluate(() => document.fonts.check('16px "Sora"'))).toBe(true);
  expect(await page.evaluate(() => document.fonts.check('16px "JetBrains Mono"'))).toBe(true);

  await page.screenshot({
    path: path.join(resultRoot, "browser-default-car-1440x900.png"),
    fullPage: true,
  });

  const laptopFiles = [
    "l15.gltf",
    "L15.bin",
    "NormalMap_L15.png",
    "Lenovo_NormalMap.png",
    "thinkpad-logo_alpha_thinkpad-logo.png",
    "THINKPAD_T490_14INCH_Alpha_alpha_THINKPAD_T490_14INCH_Alpha.png",
  ].map((name) => path.join(sourceRoot, "public", "l15", name));
  await page.locator("#modelFileInput").setInputFiles(laptopFiles);
  await expect(page.locator("#statusText")).toHaveText("READY", { timeout: 120_000 });
  await page.waitForTimeout(500);
  await page.screenshot({
    path: path.join(resultRoot, "browser-laptop-1440x900.png"),
    fullPage: true,
  });

  await page.evaluate(async () => {
    const response = await fetch("/CarConcept.glb");
    const model = new File([await response.blob()], "dropped-car.glb", {
      type: "model/gltf-binary",
    });
    const transfer = new DataTransfer();
    transfer.items.add(model);
    window.dispatchEvent(new DragEvent("dragenter", { dataTransfer: transfer, bubbles: true }));
    window.dispatchEvent(new DragEvent("drop", { dataTransfer: transfer, bubbles: true }));
  });
  await expect(page.locator("#statusText")).toHaveText("READY", { timeout: 120_000 });
  await expect(page.locator("#dropzone")).not.toHaveClass(/visible/);

  const externalVisualRequests = requests.filter((url) =>
    /fonts\.googleapis|fonts\.gstatic|assets\.babylonjs/i.test(url),
  );
  expect(externalVisualRequests).toEqual([]);
  expect(requests.some((url) => url.endsWith("/environmentSpecular.env"))).toBe(true);
  expect(requests.some((url) => url.endsWith("/CarConcept.glb"))).toBe(true);
  expect(errors).toEqual([]);

  fs.writeFileSync(
    path.join(resultRoot, "browser-smoke.json"),
    `${JSON.stringify(
      {
        capturedAt: new Date().toISOString(),
        viewport: { width: 1440, height: 900 },
        status: "READY",
        localVisualAssets: true,
        externalVisualRequests,
        requestCount: requests.length,
        errors,
      },
      null,
      2,
    )}\n`,
  );
});
