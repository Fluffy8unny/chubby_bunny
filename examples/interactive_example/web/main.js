import init, { InteractiveExample } from "../pkg/interactive_example.js";
import { createRenderer } from "../../../web/rendering.js";

const canvas = document.getElementById("canvas");
const renderer = createRenderer(canvas);
let app = null;
let pendingInputEvents = [];

const resize = () => {
  const { width, height } = renderer.resize();
  if (app) {
    app.reset(width, height);
  }
};
const flushInputEvents = () => {
  for (const event of pendingInputEvents) {
    if (!app) {
      continue;
    }

    if (event.kind === "move") {
      app.mouse_move(event.x, event.y, event.time_stamp);
      continue;
    }

    if (event.kind === "down") {
      app.mouse_down(event.x, event.y, event.button, event.time_stamp);
      continue;
    }

    if (event.kind === "up") {
      app.mouse_up(event.x, event.y, event.button, event.time_stamp);
    }
  }

  pendingInputEvents = [];
};

const enqueueInputEvent = (kind, event) => {
  if (!app) {
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
  ["mousemove", "move"],
  ["mousedown", "down"],
]) {
  document.addEventListener(domType, (event) => {
    enqueueInputEvent(kind, event);
  });
}

window.addEventListener("mouseup", (event) => {
  enqueueInputEvent("up", event);
});

const loop = (timestamp) => {
  if (!app) {
    return;
  }
  flushInputEvents();
  const dt = timestamp - (app.lastTimestamp || timestamp);
  app.lastTimestamp = timestamp;
  app.update(dt);

  const polygonArrays = app.get_polygon_arrays();
  renderer.render(polygonArrays);
  requestAnimationFrame(loop);
};

const start = async () => {
  await init();
  app = new InteractiveExample();
  resize();
  app.init(window.innerWidth, window.innerHeight);
  app.lastTimestamp = performance.now();
  requestAnimationFrame(loop);
};

window.addEventListener("resize", resize);
start();
