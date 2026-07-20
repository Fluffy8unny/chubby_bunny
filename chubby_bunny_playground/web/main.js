import init, { Playground } from "../pkg/chubby_bunny_playground.js";
import {
  initProfiler,
  measure,
  toggleProfilerEnabled,
  updateProfiler,
} from "./profiling.js";
import { createRenderer } from "./rendering.js";

// ============================================================================
// Global State
// ============================================================================
let playground = null;
const canvas = document.getElementById("canvas");
const renderer = createRenderer(canvas);
const siteBanner = document.getElementById("site-banner");
const siteBannerClose = document.getElementById("site-banner-close");

let pendingInputEvents = [];
let lastSelection = null;
let latestCompletedPointerGesture = null;
let profilingLogFrameCounter = 0;

const ENABLE_PROFILER = false;
const CLICK_GESTURE_THRESHOLD = Object.freeze({
  maxElapsedMs: 125,
  maxTravelPx: 15,
});

const pointerGesture = {
  isActive: false,
  lastX: 0,
  lastY: 0,
  travelPx: 0,
};

// ============================================================================
// Utility Functions
// ============================================================================
const getProp = (obj, snakeKey, camelKey, defaultVal = "") =>
  (obj?.[snakeKey] ?? obj?.[camelKey] ?? defaultVal);

const readMetric = (node, snakeCaseKey, camelCaseKey) => {
  const value = getProp(node, snakeCaseKey, camelCaseKey);
  return Number.isFinite(value) ? value / 1000.0 : NaN;
};

const toRoundedOrNull = (value, digits = 3) =>
  Number.isFinite(value) ? Number(value.toFixed(digits)) : null;

// ============================================================================
// Profiling
// ============================================================================
const aggregateProfilingScopes = (root) => {
  if (!root || typeof root !== "object") return [];

  const totalsByLabel = new Map();

  const visit = (node) => {
    if (!node || typeof node !== "object") return;

    const label = String(node.name ?? "(unnamed)");
    const calls = Number(getProp(node, "call_count", "callCount", 0));
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
    if (Number.isFinite(minMs)) entry.minMs = Math.min(entry.minMs, minMs);
    if (Number.isFinite(maxMs)) entry.maxMs = Math.max(entry.maxMs, maxMs);

    const children = node.children && Array.isArray(node.children) ? node.children : [];
    for (const child of children) visit(child);
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

const dumpAggregatedConstraintSummary = (stats) => {
  const frameTotalMs = readMetric(stats, "total_time_us", "totalTimeMs");
  if (!Number.isFinite(frameTotalMs)) return;

  const aggregated = aggregateProfilingScopes(stats).filter(
    (entry) => entry.name.endsWith("::solve"),
  );

  if (aggregated.length === 0) return;

  console.log("aggregated constraint timings (frame-wide):");
  console.table(
    aggregated.map((entry) => ({
      constraint: entry.name,
      calls: entry.callCount,
      total_ms: toRoundedOrNull(entry.totalMs),
      frame_pct: toRoundedOrNull((entry.totalMs / frameTotalMs) * 100, 1),
      avg_ms: toRoundedOrNull(entry.avgMs),
      min_ms: toRoundedOrNull(entry.minMs),
      max_ms: toRoundedOrNull(entry.maxMs),
    })),
  );
};

// ============================================================================
// Pointer Gesture Handling
// ============================================================================
const beginPointerGesture = (x, y) => {
  pointerGesture.isActive = true;
  pointerGesture.lastX = x;
  pointerGesture.lastY = y;
  pointerGesture.travelPx = 0;
  latestCompletedPointerGesture = null;
};

const extendPointerGesture = (x, y) => {
  if (!pointerGesture.isActive) return;
  const dx = x - pointerGesture.lastX;
  const dy = y - pointerGesture.lastY;
  pointerGesture.travelPx += Math.hypot(dx, dy);
  pointerGesture.lastX = x;
  pointerGesture.lastY = y;
};

const endPointerGesture = (x, y, timeStamp) => {
  if (pointerGesture.isActive) extendPointerGesture(x, y);
  latestCompletedPointerGesture = {
    timeStamp,
    travelPx: pointerGesture.travelPx,
  };
  pointerGesture.isActive = false;
  pointerGesture.travelPx = 0;
};

// ============================================================================
// Interaction State
// ============================================================================
const resetInteractionState = () => {
  pendingInputEvents = [];
  lastSelection = null;
  pointerGesture.isActive = false;
  pointerGesture.travelPx = 0;
  latestCompletedPointerGesture = null;
};

const isClickWithinThreshold = (elapsedMs, pointerTravelPx, threshold) =>
  elapsedMs >= 0 &&
  elapsedMs < threshold.maxElapsedMs &&
  pointerTravelPx <= threshold.maxTravelPx;

// ============================================================================
// UI & Navigation
// ============================================================================
const toggleBanner = (hidden) => {
  siteBanner?.classList.toggle("site-banner-hidden", hidden);
};

const DESCRIPTION_ACTIONS = Object.freeze({
  mail: () => window.location.assign("mailto:Andreas@Weissenburger.info"),
  git: () => window.location.assign("https://github.com/Fluffy8unny"),
  about: () => toggleBanner(false),
});

if (siteBannerClose) {
  siteBannerClose.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    toggleBanner(true);
  });
}

// ============================================================================
// Event Handling
// ============================================================================
const handleOutgoingEvent = (rawEvent) => {
  if (!rawEvent || typeof rawEvent !== "object") return;

  const eventType = String(getProp(rawEvent, "event_type", "eventType")).toLowerCase();
  const bodyId = String(getProp(rawEvent, "body_id", "bodyId"));
  const description = String(getProp(rawEvent, "description", "name"));
  const timeStamp = Number(getProp(rawEvent, "time_stamp", "timeStamp"));

  if (!eventType || !bodyId || !description || !Number.isFinite(timeStamp)) return;

  if (eventType === "selection") {
    lastSelection = { bodyId, description, timeStamp };
    return;
  }

  if (eventType === "deselection") {
    if (!lastSelection || lastSelection.bodyId !== bodyId || lastSelection.description !== description) return;

    const elapsedMs = timeStamp - lastSelection.timeStamp;
    const pointerTravelPx = latestCompletedPointerGesture?.travelPx ?? Number.POSITIVE_INFINITY;

    if (isClickWithinThreshold(elapsedMs, pointerTravelPx, CLICK_GESTURE_THRESHOLD)) {
      DESCRIPTION_ACTIONS[description]?.();
    }

    resetInteractionState();
  }
};

// ============================================================================
// Canvas & Input
// ============================================================================
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
  // Don't process events targeted at UI
  if (siteBanner?.contains(event.target)) return;

  const nextEvent = {
    kind,
    x: event.clientX,
    y: event.clientY,
    button: event.button,
    time_stamp: performance.now(),
  };

  if (kind === "down") beginPointerGesture(nextEvent.x, nextEvent.y);
  else if (kind === "move") extendPointerGesture(nextEvent.x, nextEvent.y);
  else if (kind === "up") endPointerGesture(nextEvent.x, nextEvent.y, nextEvent.time_stamp);

  // Coalesce move events
  if (kind === "move" && pendingInputEvents.length > 0) {
    const lastIdx = pendingInputEvents.length - 1;
    if (pendingInputEvents[lastIdx].kind === "move") {
      pendingInputEvents[lastIdx] = nextEvent;
      return;
    }
  }

  pendingInputEvents.push(nextEvent);
};

// Register pointer events
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

// Toggle profiler with D key
window.addEventListener(
  "keydown",
  (event) => {
    if (event.repeat || !/^KeyD$|^d$/i.test(event.code || event.key)) return;
    event.preventDefault();
    toggleProfilerEnabled();
  },
  true,
);

const flushInputEvents = () => {
  for (const event of pendingInputEvents) {
    if (event.kind === "move") {
      playground.mouse_move(event.x, event.y, event.time_stamp);
    } else if (event.kind === "down") {
      playground.mouse_down(event.x, event.y, event.button, event.time_stamp);
    } else if (event.kind === "up") {
      playground.mouse_up(event.x, event.y, event.button, event.time_stamp);
    }
  }
  pendingInputEvents = [];
};

// ============================================================================
// Game Loop
// ============================================================================
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
  const dt = timestamp - (playground.last_timestamp || timestamp);
  playground.last_timestamp = timestamp;

  flushInputEvents();

  const [frameResult, frameMs] = measure(() => {
    const [outgoingEvents, updateMs] = measure(() => playground.update(dt));
    const [, renderMs] = measure(() => {
      renderer.render(playground.get_polygon_arrays());
    });
    return { outgoingEvents, updateMs, renderMs };
  });

  // Handle gameplay events
  if (Array.isArray(frameResult.outgoingEvents)) {
    for (const event of frameResult.outgoingEvents) {
      handleOutgoingEvent(event);
    }
  }

  // Update profiler
  updateProfiler(
    frameMs,
    frameResult.updateMs,
    frameResult.renderMs,
    performance.now(),
    renderer.getCurrentDpr(),
  );

  // Periodically dump profiling stats
  if ((++profilingLogFrameCounter) % 30 === 0) {
    dumpAggregatedConstraintSummary(playground.get_profiling_stats());
  }

  requestAnimationFrame(loop);
};

start();
