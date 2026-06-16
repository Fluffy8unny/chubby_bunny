import init, { Playground } from "../pkg/chubby_bunny_playground.js";

let playground = null;
const canvas = document.getElementById("canvas");
let pendingInputEvents = [];
const CLICK_TIME_THRESHOLD_MS = 250;
const lastSelectionByBody = new Map();

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
          event.description = alert("todo");
          break;
      }
      console.log(`Click event: ${event.description}`, event);
    }
    lastSelectionByBody.delete(key);
  }
};

const resizeCanvas = () => {
  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;
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

document.addEventListener("mousemove", (event) => {
  pendingInputEvents.push(getEvent("move", event));
});

document.addEventListener("mousedown", (event) => {
  pendingInputEvents.push(getEvent("down", event));
});

document.addEventListener("mouseup", (event) => {
  pendingInputEvents.push(getEvent("up", event));
});

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

  playground = new Playground();
  playground.init(width, height);
  playground.last_timestamp = performance.now();
  requestAnimationFrame(loop);
};

const loop = (timestamp) => {
  let dt = timestamp - (playground.last_timestamp || timestamp);
  playground.last_timestamp = timestamp;
  flushInputEvents();
  let outgoingEvents = playground.update(dt);
  if (Array.isArray(outgoingEvents) && outgoingEvents.length > 0) {
    for (const event of outgoingEvents) {
      handleOutgoingEvent(event);
    }
  }
  render();
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
  ctx.lineWidth = polygon_arrays.meta.line_weight;
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
  /*
  for (let p of polygon_arrays.vertices) {
    ctx.fillStyle = `rgba(${polygon_arrays.meta.line_color.r}, ${polygon_arrays.meta.line_color.g}, ${polygon_arrays.meta.line_color.b}, ${1.0})`;
    ctx.beginPath();
    ctx.arc(p[0], p[1], 4, 0, Math.PI * 2);
    ctx.fill();
  }
  */
  for (let child of polygon_arrays.children) {
    render_polygon_arrays(child, ctx);
  }
};
const render = () => {
  const polygon_arrays = playground.get_polygon_arrays();
  const canvas = document.getElementById("canvas");
  const ctx = canvas.getContext("2d");

  ctx.clearRect(0, 0, canvas.width, canvas.height);
  for (let p of polygon_arrays) {
    render_polygon_arrays(p, ctx);
  }
};

start();
