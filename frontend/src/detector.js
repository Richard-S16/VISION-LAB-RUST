import { Channel, invoke } from "@tauri-apps/api/core";

const CAPTURE_SIZE = 384;

export function createDetector() {
  const captureCanvas = new OffscreenCanvas(CAPTURE_SIZE, CAPTURE_SIZE);
  const captureContext = captureCanvas.getContext("2d", {
    alpha: false,
    willReadFrequently: true,
  });
  let nextRequestId = 0;
  let threshold = 0.5;
  let busy = false;
  let disposed = false;
  const labelHistory = [];

  function stabilize(detections) {
    if (detections.length !== 1) {
      labelHistory.length = 0;
      return detections;
    }
    const detection = detections[0];
    const label = detection.categories[0].categoryName;
    labelHistory.push(label);
    if (labelHistory.length > 3) labelHistory.shift();
    if (labelHistory.length === 3) {
      const counts = {};
      for (const previousLabel of labelHistory) {
        counts[previousLabel] = (counts[previousLabel] ?? 0) + 1;
      }
      const majority = Object.entries(counts).sort((left, right) => right[1] - left[1])[0];
      if (majority[1] >= 2 && majority[0] !== label) {
        return [{
          ...detection,
          categories: [{ ...detection.categories[0], categoryName: majority[0] }],
        }];
      }
    }
    return [detection];
  }

  return {
    async load(onProgress) {
      if (disposed) throw new Error("detector disposed");
      const onEvent = new Channel();
      onEvent.onmessage = (event) => onProgress?.(event);
      return invoke("initialize_detector", { onEvent });
    },

    setThreshold(value) {
      threshold = value;
    },

    resetHistory() {
      labelHistory.length = 0;
    },

    async detect(canvas, modelGeneration) {
      if (disposed) throw new Error("detector disposed");
      if (busy) return null;
      busy = true;
      const requestId = ++nextRequestId;
      const totalStarted = performance.now();
      try {
        const captureStarted = performance.now();
        captureContext.drawImage(canvas, 0, 0, CAPTURE_SIZE, CAPTURE_SIZE);
        const image = captureContext.getImageData(0, 0, CAPTURE_SIZE, CAPTURE_SIZE);
        const captureMs = performance.now() - captureStarted;
        const ipcStarted = performance.now();
        const response = await invoke("detect_frame", new Uint8Array(image.data.buffer), {
          headers: {
            "x-vision-request-id": String(requestId),
            "x-vision-model-generation": String(modelGeneration),
            "x-vision-frame-width": String(CAPTURE_SIZE),
            "x-vision-frame-height": String(CAPTURE_SIZE),
            "x-vision-threshold": String(threshold),
          },
        });
        const ipcMs = performance.now() - ipcStarted;
        if (
          response.requestId !== requestId
          || response.modelGeneration !== modelGeneration
        ) {
          return null;
        }

        const detections = response.detections.map((detection) => ({
          boundingBox: {
            originX: detection.boundingBox.x * canvas.width,
            originY: detection.boundingBox.y * canvas.height,
            width: detection.boundingBox.width * canvas.width,
            height: detection.boundingBox.height * canvas.height,
          },
          categories: [{ categoryName: detection.label, score: detection.score }],
        }));
        return {
          requestId,
          modelGeneration,
          detections: stabilize(detections),
          latency: response.timing.nativeTotalMs,
          timing: {
            ...response.timing,
            captureMs,
            ipcMs,
            endToEndMs: performance.now() - totalStarted,
          },
        };
      } finally {
        busy = false;
      }
    },

    dispose() {
      disposed = true;
      labelHistory.length = 0;
    },
  };
}
