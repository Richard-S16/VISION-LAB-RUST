import { Channel, invoke } from "@tauri-apps/api/core";

const CAPTURE_SIZE = 384;

export function initializeDetector(onEvent) {
  const channel = new Channel();
  channel.onmessage = onEvent;
  return invoke("initialize_detector", { onEvent: channel });
}

export function detectFrame(frame, requestId, modelGeneration, threshold) {
  return invoke("detect_frame", frame, {
    headers: {
      "x-vision-request-id": String(requestId),
      "x-vision-model-generation": String(modelGeneration),
      "x-vision-frame-width": String(CAPTURE_SIZE),
      "x-vision-frame-height": String(CAPTURE_SIZE),
      "x-vision-threshold": String(threshold),
    },
  });
}
