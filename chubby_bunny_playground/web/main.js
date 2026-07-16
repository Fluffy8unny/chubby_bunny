import init, { Playground } from "../pkg/chubby_bunny_playground.js";
import {
  initProfiler,
  measure,
  toggleProfilerEnabled,
  updateProfiler,
} from "./profiling.js";
import { createRenderer } from "./rendering.js";

let playground = null;
const canvas = document.getElementById("canvas");
const renderer = createRenderer(canvas);
const siteBanner = document.getElementById("site-banner");
const siteBannerClose = document.getElementById("site-banner-close");
let pendingInputEvents = [];
const CLICK_GESTURE_THRESHOLD = Object.freeze({
  maxElapsedMs: 125,
  maxTravelPx: 15,
});
const lastSelectionByBody = new Map();
const ENABLE_PROFILER = false;
const pointerGesture = {
  isActive: false,
  lastX: 0,
  lastY: 0,
  travelPx: 0,
};
let latestCompletedPointerGesture = null;
let profilingLogFrameCounter = 0;

const readMetric = (node, snakeCaseKey, camelCaseKey) => {
  // Rust field names use underscores (e.g., total_time_us)
  // Convert from microseconds to milliseconds
  let value = node[snakeCaseKey] !== undefined ? node[snakeCaseKey] : node[camelCaseKey];
  if (Number.isFinite(value)) {
    return value / 1000.0;  // Convert microseconds to milliseconds
  }
  return NaN;
};

const readChildren = (node) => {
  if (!node || !Array.isArray(node.children)) {
    return [];
  }
  return node.children;
};

const toRoundedOrNull = (value, digits = 3) =>
  Number.isFinite(value) ? Number(value.toFixed(digits)) : null;

const aggregateProfilingScopes = (root) => {
  const totalsByLabel = new Map();

  const visit = (node) => {
    if (!node || typeof node !== "object") {
      return;
    }

    const label = String(node.name ?? "(unnamed)");
    const calls = Number(node.call_count ?? node.callCount ?? 0);
    const totalMs = readMetric(node, "total_time_us", "totalTimeMs");
    const minMs = readMetric(node, "min_time_us", "minTimeMs");
    const maxMs = readMetric(node, "max_time_us", "maxTimeMs");

    if (!totalsByLabel.has(label)) {
      totalsByLabel.set(label, {
        name: label,
        callCount: 0,
        totalMs: 0,
        minMs: Number.POSITIVE_INFINITY,
        maxMs: 0,
      });
    }

    const entry = totalsByLabel.get(label);
    entry.callCount += Number.isFinite(calls) ? calls : 0;
    entry.totalMs += Number.isFinite(totalMs) ? totalMs : 0;
    if (Number.isFinite(minMs)) {
      entry.minMs = Math.min(entry.minMs, minMs);
    }
    if (Number.isFinite(maxMs)) {
      entry.maxMs = Math.max(entry.maxMs, maxMs);
    }

    for (const child of readChildren(node)) {
      visit(child);
    }
  };

  visit(root);

  return Array.from(totalsByLabel.values())
    .map((entry) => ({
      ...entry,
      avgMs: entry.callCount > 0 ? entry.totalMs / entry.callCount : NaN,
      minMs: Number.isFinite(entry.minMs) ? entry.minMs : NaN,
    }))
    .sort((a, b) => b.totalMs - a.totalMs);
};

const dumpAggregatedConstraintSummary = (root) => {
  const frameTotalMs = readMetric(root, "total_time_us", "totalTimeMs");
  const aggregated = aggregateProfilingScopes(root).filter(
    (entry) => entry.name.endsWith("::solve"),
  );

  if (aggregated.length === 0) {
    return;
  }

  console.log("aggregated constraint timings (frame-wide):");
  console.table(
    aggregated.map((entry) => {
      const percentOfFrame = frameTotalMs > 0
        ? (entry.totalMs / frameTotalMs) * 100
        : NaN;
      return {
        constraint: entry.name,
        calls: entry.callCount,
        total_ms: toRoundedOrNull(entry.totalMs),
        frame_pct: toRoundedOrNull(percentOfFrame, 1),
        avg_ms: toRoundedOrNull(entry.avgMs),
        min_ms: toRoundedOrNull(entry.minMs),
        max_ms: toRoundedOrNull(entry.maxMs),
      };
    }),
  );


};

const beginPointerGesture = (x, y) => {
  pointerGesture.isActive = true;
  pointerGesture.lastX = x;
  pointerGesture.lastY = y;
  pointerGesture.travelPx = 0;
  latestCompletedPointerGesture = null;
};

const resetInteractionState = () => {
  pendingInputEvents = [];
  lastSelection = null;
  pointerGesture.isActive = false;
  pointerGesture.travelPx = 0;
  latestCompletedPointerGesture = null;
};

const extendPointerGesture = (x, y) => {
  if (!pointerGesture.isActive) {
    return;
  }

  const dx = x - pointerGesture.lastX;
  const dy = y - pointerGesture.lastY;
  pointerGesture.travelPx += Math.hypot(dx, dy);
  pointerGesture.lastX = x;
  pointerGesture.lastY = y;
};

const endPointerGesture = (x, y, timeStamp) => {
  if (pointerGesture.isActive) {
    extendPointerGesture(x, y);
  }

  latestCompletedPointerGesture = {
    timeStamp,
    travelPx: pointerGesture.isActive ? pointerGesture.travelPx : 0,
  };

  pointerGesture.isActive = false;
  pointerGesture.travelPx = 0;
};

const createClickMetrics = (
  selectionTimeStamp,
  deselectionTimeStamp,
  pointerTravelPx,
) => ({
  elapsedMs: deselectionTimeStamp - selectionTimeStamp,
  pointerTravelPx,
});

const isClickWithinThreshold = (metrics, threshold) =>
  metrics.elapsedMs >= 0 &&
  metrics.elapsedMs < threshold.maxElapsedMs &&
  metrics.pointerTravelPx <= threshold.maxTravelPx;

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

let lastSelection = null;
const DESCRIPTION_ACTIONS = Object.freeze({
  mail: () => window.location.assign("mailto:Andreas@Weissenburger.info"),
  git: () => window.location.assign("https://github.com/Fluffy8unny"),
  about: () => showBanner(),
});

const handleOutgoingEvent = (rawEvent) => {
  if (!rawEvent || typeof rawEvent !== "object") {
    return;
  }

  const eventType = String(
    rawEvent.event_type ?? rawEvent.eventType ?? "",
  ).toLowerCase();
  const bodyId = String(rawEvent.body_id ?? rawEvent.bodyId ?? "");
  const description = String(rawEvent.description ?? rawEvent.name ?? "");
  const timeStamp = Number(rawEvent.time_stamp ?? rawEvent.timeStamp);

  if (!eventType || !bodyId || !description || Number.isNaN(timeStamp)) {
    return;
  }

  if (eventType === "selection") {
    lastSelection = { bodyId, description, timeStamp };
    return;
  }

  if (eventType === "deselection") {
    if (
      !lastSelection ||
      lastSelection.bodyId !== bodyId ||
      lastSelection.description !== description
    ) {
      return;
    }

    const clickMetrics = createClickMetrics(
      lastSelection.timeStamp,
      timeStamp,
      latestCompletedPointerGesture?.travelPx ?? Number.POSITIVE_INFINITY,
    );

    if (isClickWithinThreshold(clickMetrics, CLICK_GESTURE_THRESHOLD)) {
      DESCRIPTION_ACTIONS[description]?.();
    }

    resetInteractionState();
  }
};

const resizeCanvas = () => {
  const { width, height } = renderer.resize();

  if (playground) {
    resetInteractionState();
    playground.reset(width, height);
    playground.last_timestamp = performance.now();
  }
};
window.addEventListener("resize", resizeCanvas);
resizeCanvas();
const enqueueInputEvent = (kind, event) => {
  if (
    siteBanner &&
    event.target instanceof Node &&
    siteBanner.contains(event.target)
  ) {
    return;
  }

  const nextEvent = {
    kind,
    x: event.clientX,
    y: event.clientY,
    button: event.button,
    time_stamp: performance.now(),
  };

  const pointerGestureHandlers = {
    down: () => beginPointerGesture(nextEvent.x, nextEvent.y),
    move: () => extendPointerGesture(nextEvent.x, nextEvent.y),
    up: () => endPointerGesture(nextEvent.x, nextEvent.y, nextEvent.time_stamp),
  };
  pointerGestureHandlers[kind]?.();

  if (kind === "move" && pendingInputEvents.length > 0) {
    const lastIdx = pendingInputEvents.length - 1;
    if (pendingInputEvents[lastIdx].kind === "move") {
      pendingInputEvents[lastIdx] = nextEvent;
      return;
    }
  }

  pendingInputEvents.push(nextEvent);
};

for (const [domType, kind] of [
  ["pointermove", "move"],
  ["pointerdown", "down"],
  ["pointerup", "up"],
  ["pointercancel", "up"],
]) {
  document.addEventListener(domType, (event) => {
    enqueueInputEvent(kind, event);
  });
}

window.addEventListener(
  "keydown",
  (event) => {
    if (
      event.repeat ||
      !(event.code === "KeyD" || event.key === "d" || event.key === "D")
    ) {
      return;
    }
    event.preventDefault();
    toggleProfilerEnabled();
  },
  true,
);

const flushInputEvents = () => {
  const inputEventHandlers = {
    move: (event) => playground.mouse_move(event.x, event.y, event.time_stamp),
    down: (event) =>
      playground.mouse_down(event.x, event.y, event.button, event.time_stamp),
    up: (event) => playground.mouse_up(event.x, event.y, event.button, event.time_stamp),
  };

  for (const event of pendingInputEvents) {
    inputEventHandlers[event.kind]?.(event);
  }

  pendingInputEvents = [];
};

const start = async () => {
  const width = window.innerWidth;
  const height = window.innerHeight;
  await init();

  initProfiler(ENABLE_PROFILER);

  playground = new Playground();
  playground.init(width, height);
  playground.last_timestamp = performance.now();
  requestAnimationFrame(loop);
};

const loop = (timestamp) => {
  let dt = timestamp - (playground.last_timestamp || timestamp);
  playground.last_timestamp = timestamp;
  flushInputEvents();

  const [frameResult, frameMs] = measure(() => {
    const [outgoingEvents, updateMs] = measure(() => playground.update(dt));
    const [, renderMs] = measure(() => {
      const polygonArrays = playground.get_polygon_arrays();
      renderer.render(polygonArrays);
    });
    return { outgoingEvents, updateMs, renderMs };
  });

  let outgoingEvents = frameResult.outgoingEvents;

  if (Array.isArray(outgoingEvents) && outgoingEvents.length > 0) {
    for (const event of outgoingEvents) {
      handleOutgoingEvent(event);
    }
  }

  const nowMs = performance.now();
  updateProfiler(
    frameMs,
    frameResult.updateMs,
    frameResult.renderMs,
    nowMs,
    renderer.getCurrentDpr(),
  );

  profilingLogFrameCounter += 1;
  if (profilingLogFrameCounter % 30 === 0) {
    const stats = playground.get_profiling_stats();
    dumpAggregatedConstraintSummary(stats);
  }

  requestAnimationFrame(loop);
};

start();
