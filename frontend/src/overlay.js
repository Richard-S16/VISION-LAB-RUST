export class DetectionOverlay {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
  }

  resize(cssWidth, cssHeight, dpr) {
    this.canvas.width = Math.round(cssWidth * dpr);
    this.canvas.height = Math.round(cssHeight * dpr);
    this.canvas.style.width = `${cssWidth}px`;
    this.canvas.style.height = `${cssHeight}px`;
  }

  clear() {
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  draw(detections, srcWidth, srcHeight) {
    const { ctx, canvas } = this;
    this.clear();
    if (!detections || detections.length === 0) return;

    const scaleX = canvas.width / srcWidth;
    const scaleY = canvas.height / srcHeight;
    const dpr = canvas.width / Number.parseFloat(canvas.style.width);

    ctx.lineJoin = "round";
    ctx.font = `${11 * dpr}px "JetBrains Mono", monospace`;
    ctx.textBaseline = "middle";

    for (const detection of detections) {
      const box = detection.boundingBox;
      if (!box) continue;
      const category = detection.categories[0];
      const label = `${category.categoryName} ${Math.round(category.score * 100)}%`;
      const x = box.originX * scaleX;
      const y = box.originY * scaleY;
      const width = box.width * scaleX;
      const height = box.height * scaleY;

      ctx.strokeStyle = "rgba(0, 229, 255, 0.35)";
      ctx.lineWidth = dpr;
      ctx.strokeRect(x, y, width, height);

      const arm = Math.min(14 * dpr, width / 4, height / 4);
      ctx.strokeStyle = "#00e5ff";
      ctx.lineWidth = 2.5 * dpr;
      ctx.beginPath();
      ctx.moveTo(x, y + arm); ctx.lineTo(x, y); ctx.lineTo(x + arm, y);
      ctx.moveTo(x + width - arm, y); ctx.lineTo(x + width, y); ctx.lineTo(x + width, y + arm);
      ctx.moveTo(x + width, y + height - arm); ctx.lineTo(x + width, y + height); ctx.lineTo(x + width - arm, y + height);
      ctx.moveTo(x + arm, y + height); ctx.lineTo(x, y + height); ctx.lineTo(x, y + height - arm);
      ctx.stroke();

      const paddingX = 6 * dpr;
      const chipHeight = 20 * dpr;
      const chipWidth = ctx.measureText(label).width + paddingX * 2;
      let chipY = y - chipHeight;
      if (chipY < 0) chipY = y;
      ctx.fillStyle = "rgba(0, 229, 255, 0.92)";
      ctx.fillRect(x, chipY, chipWidth, chipHeight);
      ctx.fillStyle = "#04161a";
      ctx.fillText(label.toUpperCase(), x + paddingX, chipY + chipHeight / 2 + dpr);
    }
  }
}
