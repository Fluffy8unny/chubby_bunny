import init, { Playground } from "../pkg/chubby_bunny_playground.js";

let playground = null;
const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");
const siteBanner = document.getElementById("site-banner");
const siteBannerClose = document.getElementById("site-banner-close");
let pendingInputEvents = [];
const CLICK_TIME_THRESHOLD_MS = 250;
const lastSelectionByBody = new Map();
const TOUCH_MOUSE_SUPPRESSION_MS = 600;
const MAX_RENDER_DPR = 1.5;
const ENABLE_PROFILER = true;
let lastTouchInputTimestamp = Number.NEGATIVE_INFINITY;
let viewportWidth = 0;
let viewportHeight = 0;
let currentDpr = 1;

const profiler = {
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

const initProfiler = () => {
  if (!ENABLE_PROFILER || profiler.overlay) {
    return;
  }

  const overlay = document.createElement("div");
  overlay.setAttribute("aria-hidden", "true");
  overlay.style.position = "fixed";
  overlay.style.right = "12px";
  overlay.style.bottom = "12px";
  overlay.style.zIndex = "20";
  overlay.style.padding = "8px 10px";
  overlay.style.borderRadius = "8px";
  overlay.style.background = "rgba(20, 20, 20, 0.72)";
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

const updateProfiler = (frameMs, updateMs, renderMs, nowMs) => {
  if (!ENABLE_PROFILER) {
    return;
  }

  profiler.frameMs = smoothSample(profiler.frameMs, frameMs);
  profiler.updateMs = smoothSample(profiler.updateMs, updateMs);
  profiler.renderMs = smoothSample(profiler.renderMs, renderMs);
  profiler.fps = profiler.frameMs > 0 ? 1000 / profiler.frameMs : 0;

  // Throttle text updates to reduce profiler overhead.
  if (!profiler.overlay || nowMs - profiler.lastOverlayUpdate < 100) {
    return;
  }

  profiler.overlay.textContent =
    `fps: ${profiler.fps.toFixed(1)}\n` +
    `frame: ${profiler.frameMs.toFixed(2)} ms\n` +
    `update: ${profiler.updateMs.toFixed(2)} ms\n` +
    `render: ${profiler.renderMs.toFixed(2)} ms\n` +
    `dpr: ${currentDpr.toFixed(2)}`;
  profiler.lastOverlayUpdate = nowMs;
};

const closeBanner = () => {
  if (!siteBanner) {
    return;
  }
  siteBanner.classList.add("site-banner-hidden");
};

const showBanner = () => {
  if (!siteBanner) {
    return;
  }
  siteBanner.classList.remove("site-banner-hidden");
};

if (siteBannerClose) {
  siteBannerClose.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    closeBanner();
  });
}

const selectionKey = (event) => `${event.body_id}:${event.description}`;
const handleOutgoingEvent = (event) => {
  if (event.event_type === "Selection") {
    lastSelectionByBody.set(selectionKey(event), event);
    return;
  }

  if (event.event_type === "Deselection") {
    const key = selectionKey(event);
    const lastSelection = lastSelectionByBody.get(key);
    if (!lastSelection) {
      return;
    }

    const elapsed = event.time_stamp - lastSelection.time_stamp;
    if (elapsed >= 0 && elapsed < CLICK_TIME_THRESHOLD_MS) {
      switch (event.description) {
        case "mail":
          window.location = "mailto:Andreas@Weissenburger.info";
          break;
        case "git":
          window.location = "https://github.com/Fluffy8unny";
          break;
        case "about":
          showBanner();
          break;
      }
      console.log(`Click event: ${event.description}`, event);
    }
    lastSelectionByBody.delete(key);
  }
};

const resizeCanvas = () => {
  viewportWidth = window.innerWidth;
  viewportHeight = window.innerHeight;
  const dpr = Math.min(window.devicePixelRatio || 1, MAX_RENDER_DPR);
  currentDpr = dpr;

  canvas.width = Math.floor(viewportWidth * dpr);
  canvas.height = Math.floor(viewportHeight * dpr);
  canvas.style.width = `${viewportWidth}px`;
  canvas.style.height = `${viewportHeight}px`;

  // Keep draw code in CSS pixels while still using a high-resolution backing store.
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  if (playground) {
    pendingInputEvents = [];
    lastSelectionByBody.clear();
    playground.reset(viewportWidth, viewportHeight);
    playground.last_timestamp = performance.now();
  }
};
window.addEventListener("resize", resizeCanvas);
resizeCanvas();
const getEvent = (eventName, event) => {
  return {
    kind: eventName,
    x: event.clientX,
    y: event.clientY,
    button: event.button,
    time_stamp: performance.now(),
  };
};

const shouldIgnoreMouseEvent = () => {
  return (
    performance.now() - lastTouchInputTimestamp < TOUCH_MOUSE_SUPPRESSION_MS
  );
};

const getPrimaryTouch = (touchEvent) => {
  if (touchEvent.changedTouches && touchEvent.changedTouches.length > 0) {
    return touchEvent.changedTouches[0];
  }

  if (touchEvent.touches && touchEvent.touches.length > 0) {
    return touchEvent.touches[0];
  }

  return null;
};

const isTouchInsideBanner = (touchEvent) => {
  const targetElement = touchEvent.target;
  if (!(targetElement instanceof Element) || !siteBanner) {
    return false;
  }
  return siteBanner.contains(targetElement);
};

const enqueueTouchEvent = (eventName, touchEvent) => {
  if (isTouchInsideBanner(touchEvent)) {
    // Let links/buttons in the banner use native touch behavior.
    lastTouchInputTimestamp = performance.now();
    return;
  }

  const touch = getPrimaryTouch(touchEvent);
  if (!touch) {
    return;
  }

  touchEvent.preventDefault();
  lastTouchInputTimestamp = performance.now();
  const nextEvent = {
    kind: eventName,
    x: touch.clientX,
    y: touch.clientY,
    button: 0,
    time_stamp: performance.now(),
  };

  if (eventName === "move" && pendingInputEvents.length > 0) {
    const lastIdx = pendingInputEvents.length - 1;
    if (pendingInputEvents[lastIdx].kind === "move") {
      pendingInputEvents[lastIdx] = nextEvent;
      return;
    }
  }

  pendingInputEvents.push(nextEvent);
};

document.addEventListener("mousemove", (event) => {
  if (shouldIgnoreMouseEvent()) {
    return;
  }
  const moveEvent = getEvent("move", event);
  if (
    pendingInputEvents.length > 0 &&
    pendingInputEvents[pendingInputEvents.length - 1].kind === "move"
  ) {
    pendingInputEvents[pendingInputEvents.length - 1] = moveEvent;
    return;
  }
  pendingInputEvents.push(moveEvent);
});

document.addEventListener("mousedown", (event) => {
  if (shouldIgnoreMouseEvent()) {
    return;
  }
  pendingInputEvents.push(getEvent("down", event));
});

document.addEventListener("mouseup", (event) => {
  if (shouldIgnoreMouseEvent()) {
    return;
  }
  pendingInputEvents.push(getEvent("up", event));
});

document.addEventListener(
  "touchstart",
  (event) => {
    enqueueTouchEvent("down", event);
  },
  { passive: false },
);

document.addEventListener(
  "touchmove",
  (event) => {
    enqueueTouchEvent("move", event);
  },
  { passive: false },
);

document.addEventListener(
  "touchend",
  (event) => {
    enqueueTouchEvent("up", event);
  },
  { passive: false },
);

document.addEventListener(
  "touchcancel",
  (event) => {
    enqueueTouchEvent("up", event);
  },
  { passive: false },
);

const flushInputEvents = () => {
  for (const event of pendingInputEvents) {
    if (event.kind === "move") {
      playground.mouse_move(event.x, event.y, event.time_stamp);
      continue;
    }

    if (event.kind === "down") {
      playground.mouse_down(event.x, event.y, event.button, event.time_stamp);
      continue;
    }

    if (event.kind === "up") {
      playground.mouse_up(event.x, event.y, event.button, event.time_stamp);
    }
  }

  pendingInputEvents = [];
};

const start = async () => {
  const width = window.innerWidth;
  const height = window.innerHeight;
  await init();

  initProfiler();

  playground = new Playground();
  playground.init(width, height);
  playground.last_timestamp = performance.now();
  requestAnimationFrame(loop);
};

const loop = (timestamp) => {
  const frameStart = performance.now();
  let dt = timestamp - (playground.last_timestamp || timestamp);
  playground.last_timestamp = timestamp;
  flushInputEvents();

  const updateStart = performance.now();
  let outgoingEvents = playground.update(dt);
  const updateEnd = performance.now();

  if (Array.isArray(outgoingEvents) && outgoingEvents.length > 0) {
    for (const event of outgoingEvents) {
      handleOutgoingEvent(event);
    }
  }

  const renderStart = performance.now();
  render();
  const frameEnd = performance.now();
  updateProfiler(
    frameEnd - frameStart,
    updateEnd - updateStart,
    frameEnd - renderStart,
    frameEnd,
  );

  requestAnimationFrame(loop);
};

const drawSmoothClosedPath = (ctx, vertices) => {
  if (vertices.length === 0) {
    return;
  }
  if (vertices.length < 3) {
    ctx.beginPath();
    ctx.moveTo(vertices[0][0], vertices[0][1]);
    for (let i = 1; i < vertices.length; i++) {
      ctx.lineTo(vertices[i][0], vertices[i][1]);
    }
    if (vertices.length > 2) {
      ctx.closePath();
    }
    return;
  }

  const midpoint = (a, b) => [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
  const last = vertices[vertices.length - 1];
  const first = vertices[0];
  const startMid = midpoint(last, first);

  // Smooth closed curve by connecting edge midpoints with quadratic segments.
  ctx.beginPath();
  ctx.moveTo(startMid[0], startMid[1]);
  for (let i = 0; i < vertices.length; i++) {
    const current = vertices[i];
    const next = vertices[(i + 1) % vertices.length];
    const endMid = midpoint(current, next);
    ctx.quadraticCurveTo(current[0], current[1], endMid[0], endMid[1]);
  }
  ctx.closePath();
};

const render_polygon_arrays = (polygon_arrays, ctx) => {
  ctx.strokeStyle = `rgba(${polygon_arrays.meta.line_color.r},
                     ${polygon_arrays.meta.line_color.g},
                     ${polygon_arrays.meta.line_color.b},
                     ${polygon_arrays.meta.line_color.a})`;
  ctx.lineWidth = polygon_arrays.meta.line_weight / currentDpr;
  ctx.fillStyle = `rgba(${polygon_arrays.meta.fill_color.r}, ${polygon_arrays.meta.fill_color.g}, ${polygon_arrays.meta.fill_color.b}, ${polygon_arrays.meta.fill_color.a})`;
  if (polygon_arrays.meta.smooth_edges) {
    drawSmoothClosedPath(ctx, polygon_arrays.vertices);
    ctx.fill();
    ctx.stroke();
  } else {
    ctx.beginPath();
    ctx.moveTo(polygon_arrays.vertices[0][0], polygon_arrays.vertices[0][1]);
    for (let i = 1; i < polygon_arrays.vertices.length; i++) {
      ctx.lineTo(polygon_arrays.vertices[i][0], polygon_arrays.vertices[i][1]);
    }
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
  }

  for (let child of polygon_arrays.children) {
    render_polygon_arrays(child, ctx);
  }
};
const render = () => {
  const polygon_arrays = playground.get_polygon_arrays();

  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.setTransform(currentDpr, 0, 0, currentDpr, 0, 0);
  for (let p of polygon_arrays) {
    render_polygon_arrays(p, ctx);
  }
};

start();
