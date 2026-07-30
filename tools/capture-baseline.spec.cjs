const { test, expect } = require("@playwright/test");
const fs = require("node:fs");
const path = require("node:path");

const rustRoot = path.resolve(__dirname, "..");
const sourceRoot = path.resolve(rustRoot, "..", "VISION-LAB");
const fixtureRoot = path.join(rustRoot, "tests", "fixtures");
const frameRoot = path.join(fixtureRoot, "frames");
const expectedRoot = path.join(fixtureRoot, "expected");
const visualRoot = path.join(fixtureRoot, "visual");

function saveDataUrl(filePath, dataUrl) {
  const encoded = dataUrl.replace(/^data:image\/png;base64,/, "");
  fs.writeFileSync(filePath, Buffer.from(encoded, "base64"));
}

test("capture VISION/LAB baseline", async ({ page, browserName }) => {
  fs.mkdirSync(frameRoot, { recursive: true });
  fs.mkdirSync(expectedRoot, { recursive: true });
  fs.mkdirSync(visualRoot, { recursive: true });

  const consoleMessages = [];
  page.on("console", (message) => {
    const entry = `[${message.type()}] ${message.text()}`;
    consoleMessages.push(entry);
    console.log(entry);
  });
  page.on("pageerror", (error) => consoleMessages.push(`[pageerror] ${error.stack || error}`));

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#statusText")).toHaveText("READY", { timeout: 120_000 });
  await page.waitForTimeout(5000);

  await page.screenshot({
    path: path.join(visualRoot, "default-car-1440x900.png"),
    fullPage: true,
  });

  const environment = await page.evaluate(async () => {
    const gl = document.getElementById("renderCanvas").getContext("webgl2");
    const debugInfo = gl?.getExtension("WEBGL_debug_renderer_info");
    let webgpu = { exposed: !!navigator.gpu, adapter: null };
    if (navigator.gpu) {
      const adapter = await navigator.gpu.requestAdapter();
      webgpu.adapter = adapter
        ? {
            architecture: adapter.info?.architecture || null,
            description: adapter.info?.description || null,
            device: adapter.info?.device || null,
            vendor: adapter.info?.vendor || null,
          }
        : null;
    }
    const start = performance.now();
    let frames = 0;
    await new Promise((resolve) => {
      const tick = (now) => {
        frames += 1;
        if (now - start >= 2000) resolve();
        else requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    });
    return {
      userAgent: navigator.userAgent,
      browserLanguage: navigator.language,
      devicePixelRatio: window.devicePixelRatio,
      viewport: { width: innerWidth, height: innerHeight },
      renderCanvas: {
        width: document.getElementById("renderCanvas").width,
        height: document.getElementById("renderCanvas").height,
      },
      webgl: {
        renderer: debugInfo ? gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) : null,
        vendor: debugInfo ? gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) : null,
      },
      webgpu,
      measuredRenderFps: (frames * 1000) / (performance.now() - start),
    };
  });

  let loadMetrics;
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    try {
      loadMetrics = await page.evaluate(async () => {
        const { createDetector } = await import("/src/detector.js");
        const detector = createDetector();
        const progress = [];
        const startedAt = performance.now();
        await detector.load((value) => progress.push(value));
        window.__baselineDetector = detector;
        return {
          loadMs: performance.now() - startedAt,
          progressSamples: progress,
        };
      });
      break;
    } catch (error) {
      if (attempt === 3 || !String(error).includes("Execution context was destroyed")) throw error;
      await expect(page.locator("#statusText")).toHaveText("READY", { timeout: 120_000 });
      await page.waitForTimeout(3000);
    }
  }

  async function capture(name) {
    const captured = await page.evaluate(async () => {
      const source = document.getElementById("renderCanvas");
      const frozen = document.createElement("canvas");
      frozen.width = 384;
      frozen.height = 384;
      const context = frozen.getContext("2d", { willReadFrequently: true });
      context.drawImage(source, 0, 0, frozen.width, frozen.height);
      const png = frozen.toDataURL("image/png");
      const wallStart = performance.now();
      const result = await window.__baselineDetector.detect(frozen);
      return {
        png,
        detections: result.detections,
        inferenceMs: result.latency,
        wallMs: performance.now() - wallStart,
      };
    });

    saveDataUrl(path.join(frameRoot, `${name}.png`), captured.png);
    fs.writeFileSync(
      path.join(expectedRoot, `${name}.json`),
      `${JSON.stringify(
        {
          fixture: `frames/${name}.png`,
          width: 384,
          height: 384,
          threshold: 0.5,
          model: "onnx-community/rfdetr_nano-ONNX",
          precision: "fp32-webgpu-or-q8-wasm-see-baseline-metadata",
          inferenceMs: captured.inferenceMs,
          wallMs: captured.wallMs,
          detections: captured.detections,
        },
        null,
        2
      )}\n`
    );
  }

  await capture("car-front-three-quarter");
  await page.mouse.move(720, 450);
  await page.mouse.down();
  await page.mouse.move(1000, 450, { steps: 12 });
  await page.mouse.up();
  await capture("car-side");
  await page.mouse.move(720, 450);
  await page.mouse.down();
  await page.mouse.move(1120, 450, { steps: 12 });
  await page.mouse.up();
  await capture("car-rear-three-quarter");

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
  await page.waitForTimeout(1000);
  await page.screenshot({
    path: path.join(visualRoot, "laptop-1440x900.png"),
    fullPage: true,
  });
  await capture("laptop-front-three-quarter");
  await page.mouse.move(720, 450);
  await page.mouse.down();
  await page.mouse.move(980, 450, { steps: 12 });
  await page.mouse.up();
  await capture("laptop-side");

  await page.evaluate(() => window.__baselineDetector?.dispose());

  const fallbackUsed = consoleMessages.some((entry) => entry.includes("falling back to wasm"));
  const metadata = {
    capturedAt: new Date().toISOString(),
    browserName,
    sourceCommit: "9653e15",
    sourcePackageVersion: "1.0.0",
    dependencies: {
      "@babylonjs/core": "9.17.0",
      "@babylonjs/loaders": "9.17.0",
      "@huggingface/transformers": "4.2.0",
      vite: "8.1.5",
    },
    detectorBackend: fallbackUsed ? "wasm-q8" : "webgpu-fp32",
    environment,
    detectorLoad: {
      loadMs: loadMetrics.loadMs,
      progressSampleCount: loadMetrics.progressSamples.length,
      progressMin: Math.min(...loadMetrics.progressSamples),
      progressMax: Math.max(...loadMetrics.progressSamples),
    },
    consoleMessages,
  };
  fs.writeFileSync(
    path.join(fixtureRoot, "baseline-metadata.json"),
    `${JSON.stringify(metadata, null, 2)}\n`
  );
});
