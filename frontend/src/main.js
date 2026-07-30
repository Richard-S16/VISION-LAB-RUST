import { createDetector } from "./detector.js";
import { hud } from "./hud.js";
import { DetectionOverlay } from "./overlay.js";
import { createBabylonScene } from "./scene.js";

const renderCanvas = document.getElementById("renderCanvas");
const overlay = new DetectionOverlay(document.getElementById("overlayCanvas"));
const MIN_INTERVAL = 300;

let engine = null;
let sceneHandle = null;
let detector = null;
let detecting = false;
let lastRun = 0;
let inFlight = false;
let threshold = 0.5;
let modelGeneration = 0;
let modelReady = false;
let detectionEpoch = 0;

async function boot() {
  try {
    hud.setLoaderStage("loading 3d model...", 0.15);
    sceneHandle = await createBabylonScene(renderCanvas);
    engine = sceneHandle.engine;
    hud.setLoaderStage("compiling shaders...", 0.55);
    await sceneHandle.scene.whenReadyAsync();
    modelGeneration = 1;
    modelReady = true;
    engine.runRenderLoop(() => sceneHandle.scene.render());

    const syncOverlay = () => {
      overlay.resize(
        renderCanvas.clientWidth,
        renderCanvas.clientHeight,
        window.devicePixelRatio || 1,
      );
    };
    syncOverlay();
    window.addEventListener("resize", () => {
      engine.resize();
      syncOverlay();
    });

    hud.bindControls({
      onThreshold: (value) => {
        threshold = value;
        detector?.setThreshold(value);
      },
      onDetectToggle: toggleDetect,
      onUpload: handleUpload,
      onReset: handleReset,
    });

    hud.setLoaderStage("ready", 1);
    hud.hideLoader();
    hud.setStatus("READY");
    hud.setDetectState("idle");
  } catch (error) {
    console.error(error);
    hud.setLoaderStage(`error: ${error.message}`, 0);
    hud.setStatus("ERROR", "error");
  }
}

async function toggleDetect() {
  if (detecting) {
    detecting = false;
    detectionEpoch += 1;
    overlay.clear();
    hud.setDetectState("idle");
    hud.setStatus("READY");
    return;
  }

  try {
    if (!detector) {
      hud.setDetectState("loading");
      detector = createDetector();
      detector.setThreshold(threshold);
      const info = await detector.load((event) => hud.setDetectProgress(event));
      console.info(`[detector] native ${info.provider}`, info);
    }
    detecting = true;
    const epoch = ++detectionEpoch;
    lastRun = 0;
    hud.setDetectState("live");
    hud.setStatus("LIVE", "live");
    requestAnimationFrame(() => detectionTick(epoch));
  } catch (error) {
    console.error(error);
    detector?.dispose();
    detector = null;
    hud.setDetectState("idle");
    hud.setStatus("ERROR", "error");
  }
}

async function detectionTick(epoch) {
  if (!detecting || epoch !== detectionEpoch) return;
  requestAnimationFrame(() => detectionTick(epoch));
  const now = performance.now();
  if (inFlight || !detector || !modelReady || now - lastRun < MIN_INTERVAL) return;
  lastRun = now;
  inFlight = true;
  const requestGeneration = modelGeneration;
  try {
    const result = await detector.detect(renderCanvas, requestGeneration);
    if (
      result
      && detecting
      && epoch === detectionEpoch
      && modelReady
      && result.modelGeneration === modelGeneration
    ) {
      window.__lastDetections = result.detections;
      overlay.draw(result.detections, renderCanvas.width, renderCanvas.height);
      hud.updateStats({
        latency: result.latency,
        fps: engine.getFps(),
        count: result.detections.length,
      });
    }
  } catch (error) {
    console.error("inference failed:", error);
  } finally {
    inFlight = false;
  }
}

async function handleUpload(files) {
  const generation = ++modelGeneration;
  modelReady = false;
  overlay.clear();
  detector?.resetHistory();
  try {
    hud.setStatus("LOADING", "paused");
    await sceneHandle.loadFiles(files);
    if (generation !== modelGeneration) return;
    modelReady = true;
    hud.setStatus(detecting ? "LIVE" : "READY", detecting ? "live" : undefined);
  } catch (error) {
    console.error(error);
    if (generation === modelGeneration) {
      modelReady = true;
      hud.setStatus("ERROR", "error");
    }
  }
}

async function handleReset() {
  const generation = ++modelGeneration;
  modelReady = false;
  overlay.clear();
  detector?.resetHistory();
  try {
    hud.setStatus("LOADING", "paused");
    await sceneHandle.loadDefault();
    if (generation !== modelGeneration) return;
    modelReady = true;
    hud.setStatus(detecting ? "LIVE" : "READY", detecting ? "live" : undefined);
  } catch (error) {
    console.error(error);
    if (generation === modelGeneration) {
      modelReady = true;
      hud.setStatus("ERROR", "error");
    }
  }
}

window.addEventListener("beforeunload", () => {
  detecting = false;
  detectionEpoch += 1;
  detector?.dispose();
  sceneHandle?.dispose();
});

boot();
