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
const CLICK_TIME_THRESHOLD_MS = 375;
const lastSelectionByBody = new Map();
const ENABLE_PROFILER = false;

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

    const elapsed = timeStamp - lastSelection.timeStamp;
    if (elapsed >= 0 && elapsed < CLICK_TIME_THRESHOLD_MS) {
      if (description === "mail") {
        window.location.assign("mailto:Andreas@Weissenburger.info");
      } else if (description === "git") {
        window.location.assign("https://github.com/Fluffy8unny");
      } else if (description === "about") {
        showBanner();
      }
    }
    lastSelection = null;
  }
};

const resizeCanvas = () => {
  const { width, height } = renderer.resize();

  if (playground) {
    pendingInputEvents = [];
    lastSelection = null;
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

  requestAnimationFrame(loop);
};

start();
