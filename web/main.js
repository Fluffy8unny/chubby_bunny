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
const TOUCH_MOUSE_SUPPRESSION_MS = 768;
const ENABLE_PROFILER = false;

let lastTouchInputTimestamp = Number.NEGATIVE_INFINITY;

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

const normalizeOutgoingEvent = (event) => {
  if (!event || typeof event !== "object") {
    return null;
  }

  const eventType =
    event.event_type ?? event.eventType ?? event.type ?? event.kind ?? null;
  const bodyId = event.body_id ?? event.bodyId ?? null;
  const description = event.description ?? event.name ?? null;
  const timeStamp = event.time_stamp ?? event.timeStamp ?? null;

  if (!eventType || bodyId === null || !description || timeStamp === null) {
    return null;
  }

  return {
    eventType: String(eventType).toLowerCase(),
    bodyId: String(bodyId),
    description: String(description),
    timeStamp: Number(timeStamp),
    raw: event,
  };
};

const selectionKey = (event) => `${event.bodyId}:${event.description}`;
const handleOutgoingEvent = (rawEvent) => {
  const event = normalizeOutgoingEvent(rawEvent);
  if (!event) {
    return;
  }

  if (event.eventType === "selection") {
    lastSelectionByBody.set(selectionKey(event), event);
    return;
  }

  if (event.eventType === "deselection") {
    const key = selectionKey(event);
    const lastSelection = lastSelectionByBody.get(key);
    if (!lastSelection) {
      return;
    }

    const elapsed = event.timeStamp - lastSelection.timeStamp;
    if (elapsed >= 0 && elapsed < CLICK_TIME_THRESHOLD_MS) {
      switch (event.description) {
        case "mail":
          window.location.assign("mailto:Andreas@Weissenburger.info");
          break;
        case "git":
          window.location.assign("https://github.com/Fluffy8unny");
          break;
        case "about":
          showBanner();
          break;
      }
      console.log(`Click event: ${event.description}`, event.raw);
    }
    lastSelectionByBody.delete(key);
  }
};

const resizeCanvas = () => {
  const { width, height } = renderer.resize();

  if (playground) {
    pendingInputEvents = [];
    lastSelectionByBody.clear();
    playground.reset(width, height);
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

const handleDebugWindowToggleKeydown = (event) => {
  const isDebugKey = event.code === "KeyD" || event.key === "d" || event.key === "D";

  if (event.repeat || !isDebugKey) {
    return;
  }

  event.preventDefault();
  toggleProfilerEnabled();
};

window.addEventListener("keydown", handleDebugWindowToggleKeydown, true);

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
  if (!document.hasFocus()) {
    playground.last_timestamp = timestamp;
    requestAnimationFrame(loop);
    return;
  }

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
