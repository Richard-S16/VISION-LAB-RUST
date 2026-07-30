const { test, expect } = require("@playwright/test");
const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.resolve(__dirname, "..");
const resultRoot = path.join(projectRoot, "tests", "results", "phase-4");

test("Phase 4 frontend sends exact raw frames to native detector", async ({ page }) => {
  fs.mkdirSync(resultRoot, { recursive: true });
  const schedulerSource = fs.readFileSync(
    path.join(projectRoot, "frontend", "src", "main.js"),
    "utf8",
  );
  expect(schedulerSource).toContain("const MIN_INTERVAL = 300;");
  expect(schedulerSource).toContain("now - lastRun < MIN_INTERVAL");
  const requests = [];
  const errors = [];
  page.on("request", (request) => requests.push(request.url()));
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.addInitScript(() => {
    const callbacks = new Map();
    let nextCallbackId = 1;
    window.__nativeDetectorCalls = [];
    window.__TAURI_INTERNALS__ = {
      transformCallback(callback) {
        const id = nextCallbackId++;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback(id) {
        callbacks.delete(id);
      },
      async invoke(command, args, options) {
        if (command === "initialize_detector") {
          const callback = callbacks.get(args.onEvent.id);
          callback({ index: 0, message: { stage: "validatingModel", message: "Validating" } });
          callback({ index: 1, message: { stage: "warmingDetector", message: "Warming" } });
          callback({ index: 2, message: { stage: "ready", message: "Ready" } });
          return {
            provider: "cpu",
            providerFallback: "test fallback",
            modelRepository: "onnx-community/rfdetr_nano-ONNX",
            modelRevision: "test",
            modelChecksumPrefix: "9cbac6b11ce3",
            inputName: "pixel_values",
            inputShape: [1, 3, 384, 384],
            outputNames: ["logits", "pred_boxes"],
            warmupMs: 1,
          };
        }
        if (command === "detect_frame") {
          const headers = options.headers;
          window.__nativeDetectorCalls.push({
            at: performance.now(),
            bytes: args.byteLength,
            headers: { ...headers },
          });
          await new Promise((resolve) => setTimeout(resolve, 20));
          return {
            requestId: Number(headers["x-vision-request-id"]),
            modelGeneration: Number(headers["x-vision-model-generation"]),
            detections: [{
              labelId: 3,
              label: "car",
              score: 0.95,
              boundingBox: { x: 0.25, y: 0.2, width: 0.5, height: 0.4 },
            }],
            timing: {
              preprocessMs: 1,
              inferenceMs: 10,
              postprocessMs: 1,
              nativeTotalMs: 12,
            },
          };
        }
        throw new Error(`unexpected command: ${command}`);
      },
    };
  });

  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#statusText")).toHaveText("READY", { timeout: 120_000 });
  await page.locator("#threshold").evaluate((slider) => {
    slider.value = "65";
    slider.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await page.locator("#detectBtn").click();
  await expect(page.locator("#statusText")).toHaveText("LIVE");
  await expect.poll(() => page.evaluate(() => window.__nativeDetectorCalls.length)).toBeGreaterThan(1);

  const state = await page.evaluate(() => ({
    calls: window.__nativeDetectorCalls,
    detections: window.__lastDetections,
  }));
  await page.locator("#detectBtn").click();
  await expect(page.locator("#detectBtn")).toHaveText("DETECT");

  expect(state.calls[0].bytes).toBe(384 * 384 * 4);
  expect(state.calls[0].headers["x-vision-frame-width"]).toBe("384");
  expect(state.calls[0].headers["x-vision-frame-height"]).toBe("384");
  expect(state.calls[0].headers["x-vision-threshold"]).toBe("0.65");
  expect(Number(state.calls[0].headers["x-vision-model-generation"])).toBeGreaterThan(0);
  // Scheduler timestamps capture start; first WebGL readback can delay its invoke substantially.
  expect(state.calls[1].at - state.calls[0].at).toBeGreaterThanOrEqual(200);
  expect(state.detections[0].categories[0].categoryName).toBe("car");
  expect(state.detections[0].boundingBox.originX).toBeGreaterThan(
    state.detections[0].boundingBox.originY,
  );

  const detectorRequests = requests.filter((url) => /huggingface|\.hf\.co|onnx|ort-wasm/i.test(url));
  expect(detectorRequests).toEqual([]);
  expect(errors).toEqual([]);

  fs.writeFileSync(
    path.join(resultRoot, "browser-native-ipc-smoke.json"),
    `${JSON.stringify(
      {
        capturedAt: new Date().toISOString(),
        calls: state.calls,
        detections: state.detections,
        detectorRequests,
        errors,
      },
      null,
      2,
    )}\n`,
  );
});
