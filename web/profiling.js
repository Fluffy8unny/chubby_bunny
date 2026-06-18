const profiler = {
  enabled: false,
  overlay: null,
  frameMs: 0,
  updateMs: 0,
  renderMs: 0,
  fps: 0,
  lastOverlayUpdate: 0,
};

const smoothSample = (previous, sample, alpha = 0.2) => {
  if (previous <= 0) {
    return sample;
  }
  return previous * (1 - alpha) + sample * alpha;
};

const ensureOverlay = () => {
  if (!profiler.enabled || profiler.overlay) {
    return;
  }

  const overlay = document.createElement("div");
  overlay.setAttribute("aria-hidden", "true");
  overlay.style.position = "fixed";
  overlay.style.left = "12px";
  overlay.style.top = "12px";
  overlay.style.zIndex = "20";
  overlay.style.padding = "8px 10px";
  overlay.style.borderRadius = "8px";
  overlay.style.background = "rgba(200, 152, 108, 0.72)";
  overlay.style.color = "#f6f4ee";
  overlay.style.font =
    "12px/1.35 ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace";
  overlay.style.whiteSpace = "pre";
  overlay.style.pointerEvents = "none";
  overlay.textContent =
    "fps: --\nframe: -- ms\nupdate: -- ms\nrender: -- ms\ndpr: --";
  document.body.appendChild(overlay);
  profiler.overlay = overlay;
};

const removeOverlay = () => {
  if (!profiler.overlay) {
    return;
  }
  profiler.overlay.remove();
  profiler.overlay = null;
};

export const setProfilerEnabled = (enabled) => {
  profiler.enabled = enabled;
  if (enabled) {
    ensureOverlay();
  } else {
    removeOverlay();
  }
};

export const toggleProfilerEnabled = () => {
  setProfilerEnabled(!profiler.enabled);
};

export const initProfiler = (enabled = false) => {
  setProfilerEnabled(enabled);
};

export const measure = (callback) => {
  const start = performance.now();
  const value = callback();
  const durationMs = performance.now() - start;
  return [value, durationMs];
};

export const updateProfiler = (frameMs, updateMs, renderMs, nowMs, dpr) => {
  if (!profiler.enabled) {
    return;
  }

  ensureOverlay();

  profiler.frameMs = smoothSample(profiler.frameMs, frameMs);
  profiler.updateMs = smoothSample(profiler.updateMs, updateMs);
  profiler.renderMs = smoothSample(profiler.renderMs, renderMs);
  profiler.fps = profiler.frameMs > 0 ? 1000 / profiler.frameMs : 0;

  if (!profiler.overlay || nowMs - profiler.lastOverlayUpdate < 100) {
    return;
  }

  profiler.overlay.textContent =
    `fps: ${profiler.fps.toFixed(1)}\n` +
    `frame: ${profiler.frameMs.toFixed(2)} ms\n` +
    `update: ${profiler.updateMs.toFixed(2)} ms\n` +
    `render: ${profiler.renderMs.toFixed(2)} ms\n` +
    `dpr: ${dpr.toFixed(2)}`;
  profiler.lastOverlayUpdate = nowMs;
};
