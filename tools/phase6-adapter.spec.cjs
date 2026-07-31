const { test, expect } = require("@playwright/test");
const path = require("node:path");

const projectRoot = path.resolve(__dirname, "..");

function multiRootFixture(bufferUri) {
  return {
    asset: { version: "2.0" },
    scene: 0,
    scenes: [{ nodes: [0, 1] }],
    nodes: [
      { mesh: 0, translation: [-2, 0, 0] },
      { mesh: 1, translation: [2, 1, 0] },
    ],
    meshes: [
      { primitives: [{ attributes: { POSITION: 0 } }] },
      { primitives: [{ attributes: { POSITION: 1 } }] },
    ],
    buffers: [{ uri: bufferUri, byteLength: 72 }],
    bufferViews: [
      { buffer: 0, byteOffset: 0, byteLength: 36 },
      { buffer: 0, byteOffset: 36, byteLength: 36 },
    ],
    accessors: [
      {
        bufferView: 0,
        componentType: 5126,
        count: 3,
        type: "VEC3",
        min: [0, 0, 0],
        max: [1, 1, 0],
      },
      {
        bufferView: 1,
        componentType: 5126,
        count: 3,
        type: "VEC3",
        min: [0, 0, 0],
        max: [1, 1, 0],
      },
    ],
  };
}

function triangleBuffer() {
  const buffer = Buffer.alloc(72);
  const values = [
    0, 0, 0, 1, 0, 0, 0, 1, 0,
    0, 0, 0, 1, 0, 0, 0, 1, 0,
  ];
  values.forEach((value, index) => buffer.writeFloatLE(value, index * 4));
  return buffer;
}

test("Phase 6 adapter serializes and disposes model replacements", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(String(error)));
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#statusText")).toHaveText("READY", { timeout: 120_000 });

  const initial = await page.evaluate(() => window.__visionLabSceneDiagnostics);
  expect(initial.renderableMeshes).toBeGreaterThan(0);

  const binary = triangleBuffer();
  const gltf = multiRootFixture("geometry/data.bin");
  await page.locator("#modelFileInput").setInputFiles([
    {
      name: "multi-root.gltf",
      mimeType: "model/gltf+json",
      buffer: Buffer.from(JSON.stringify(gltf)),
    },
    { name: "data.bin", mimeType: "application/octet-stream", buffer: binary },
  ]);
  await expect.poll(
    () => page.evaluate(() => window.__visionLabSceneDiagnostics.replacementCount),
    { timeout: 120_000 },
  ).toBe(initial.replacementCount + 1);
  await expect(page.locator("#statusText")).toHaveText("READY");
  const multiRoot = await page.evaluate(() => window.__visionLabSceneDiagnostics);
  expect(multiRoot.renderableMeshes).toBe(2);
  expect(Math.max(
    multiRoot.normalizedSize.x,
    multiRoot.normalizedSize.y,
    multiRoot.normalizedSize.z,
  )).toBeCloseTo(4, 4);
  expect(multiRoot.minimumY).toBeCloseTo(0, 4);

  const broken = multiRootFixture("missing.bin");
  await page.locator("#modelFileInput").setInputFiles({
    name: "broken.gltf",
    mimeType: "model/gltf+json",
    buffer: Buffer.from(JSON.stringify(broken)),
  });
  await expect(page.locator("#statusText")).toHaveText("ERROR");
  const afterFailure = await page.evaluate(() => window.__visionLabSceneDiagnostics);
  expect(afterFailure.activeGeneration).toBe(multiRoot.activeGeneration);
  expect(afterFailure.replacementCount).toBe(multiRoot.replacementCount);

  await page.locator("#modelFileInput").setInputFiles([
    {
      name: "duplicate.gltf",
      mimeType: "model/gltf+json",
      buffer: Buffer.from(JSON.stringify(multiRootFixture("data.bin"))),
    },
    { name: "data.bin", mimeType: "application/octet-stream", buffer: binary },
    { name: "data.bin", mimeType: "application/octet-stream", buffer: binary },
  ]);
  await expect(page.locator("#statusText")).toHaveText("ERROR");
  const afterDuplicate = await page.evaluate(() => window.__visionLabSceneDiagnostics);
  expect(afterDuplicate.activeGeneration).toBe(multiRoot.activeGeneration);
  expect(afterDuplicate.replacementCount).toBe(multiRoot.replacementCount);

  const carPath = path.join(projectRoot, "public", "CarConcept.glb");
  await page.locator("#modelFileInput").setInputFiles(carPath);
  await page.locator("#resetBtn").click();
  await expect.poll(
    () => page.evaluate(() => {
      const value = window.__visionLabSceneDiagnostics;
      return value.activeGeneration === value.latestGeneration;
    }),
    { timeout: 120_000 },
  ).toBe(true);
  await expect(page.locator("#statusText")).toHaveText("READY");

  let previous = await page.evaluate(() => window.__visionLabSceneDiagnostics);
  for (let index = 0; index < 3; index += 1) {
    await page.locator("#resetBtn").click();
    const expectedGeneration = previous.latestGeneration + 1;
    await expect.poll(
      () => page.evaluate(() => window.__visionLabSceneDiagnostics.activeGeneration),
      { timeout: 120_000 },
    ).toBe(expectedGeneration);
    const current = await page.evaluate(() => window.__visionLabSceneDiagnostics);
    expect(current.replacementCount).toBe(previous.replacementCount + 1);
    expect(current.disposalCount).toBe(previous.disposalCount + 1);
    previous = current;
  }

  expect(pageErrors).toEqual([]);
});
