const $ = (id) => document.getElementById(id);

export const hud = {
  setLoaderStage(text, progress) {
    $("loaderLog").textContent = text;
    $("loaderBar").style.width = `${Math.round(progress * 100)}%`;
  },

  hideLoader() {
    const loader = $("loader");
    loader.classList.add("done");
    setTimeout(() => loader.remove(), 700);
  },

  setStatus(text, state) {
    $("statusText").textContent = text;
    const chip = $("statusChip");
    chip.classList.remove("live", "paused", "error");
    if (state) chip.classList.add(state);
  },

  updateStats({ latency, fps, count }) {
    $("statLatency").innerHTML = `${Math.round(latency)}<small>ms</small>`;
    $("statFps").innerHTML = `${Math.round(fps)}<small>fps</small>`;
    $("statCount").textContent = count;
  },

  setDetectState(state) {
    const button = $("detectBtn");
    button.classList.remove("loading", "live");
    button.disabled = false;
    if (state === "idle") {
      button.textContent = "DETECT";
    } else if (state === "loading") {
      button.classList.add("loading");
      button.textContent = "LOAD 0%";
      button.disabled = true;
    } else {
      button.classList.add("live");
      button.textContent = "STOP";
    }
  },

  setDetectProgress(event) {
    const labels = {
      validatingModel: "VALIDATE",
      loadingOnnxRuntime: "RUNTIME",
      registeringDirectMl: "DIRECTML",
      fallingBackToCpu: "CPU FALLBACK",
      optimizingGraph: "OPTIMIZE",
      warmingDetector: "WARMUP",
      ready: "READY",
    };
    $("detectBtn").textContent = labels[event.stage] ?? "LOADING";
  },

  bindControls({ onThreshold, onDetectToggle, onUpload, onReset }) {
    const slider = $("threshold");
    slider.addEventListener("input", () => {
      const value = slider.value / 100;
      $("thresholdValue").textContent = `${slider.value}%`;
      onThreshold(value);
    });

    $("detectBtn").addEventListener("click", onDetectToggle);

    const fileInput = $("modelFileInput");
    $("uploadBtn").addEventListener("click", () => fileInput.click());
    fileInput.addEventListener("change", () => {
      if (fileInput.files.length) onUpload(fileInput.files);
      fileInput.value = "";
    });

    $("resetBtn").addEventListener("click", onReset);

    const dropzone = $("dropzone");
    let dragDepth = 0;
    window.addEventListener("dragenter", (event) => {
      event.preventDefault();
      dragDepth += 1;
      dropzone.classList.add("visible");
    });
    window.addEventListener("dragover", (event) => event.preventDefault());
    window.addEventListener("dragleave", (event) => {
      event.preventDefault();
      dragDepth -= 1;
      if (dragDepth <= 0) {
        dragDepth = 0;
        dropzone.classList.remove("visible");
      }
    });
    window.addEventListener("drop", (event) => {
      event.preventDefault();
      dragDepth = 0;
      dropzone.classList.remove("visible");
      if (event.dataTransfer.files.length) onUpload(event.dataTransfer.files);
    });
  },
};
