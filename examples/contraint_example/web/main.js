import init, { ConstraintsExample } from "../pkg/constraint_example.js";
import { createRenderer } from "../../../chubby_bunny_playground/web/rendering.js";

const canvas = document.getElementById("canvas");
const renderer = createRenderer(canvas);
let app = null;

const resize = () => {
  const { width, height } = renderer.resize();
  if (app) {
    app.reset(width, height);
  }
};

for (const [domType, method] of [
  ["pointerdown", "mouse_down"],
  ["pointerup", "mouse_up"],
  ["pointermove", "mouse_move"],
]) {
  document.addEventListener(domType, (event) => {
    if (!app) {
      return;
    }

    if (method === "mouse_move") {
      app.mouse_move(event.clientX, event.clientY, performance.now());
      return;
    }

    app[method](event.clientX, event.clientY, event.button, performance.now());
  });
}

const loop = (timestamp) => {
  if (!app) {
    return;
  }

  const dt = timestamp - (app.lastTimestamp || timestamp);
  app.lastTimestamp = timestamp;
  app.update(dt);

  const polygonArrays = app.get_polygon_arrays();
  renderer.render(polygonArrays);
  requestAnimationFrame(loop);
};

const start = async () => {
  await init();
  app = new ConstraintsExample();
  resize();
  app.init(window.innerWidth, window.innerHeight);
  app.lastTimestamp = performance.now();
  requestAnimationFrame(loop);
};

window.addEventListener("resize", resize);
start();
