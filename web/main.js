import init, { Playground } from "../pkg/chubby_bunny_playground.js";

let playground = null;
const canvas = document.getElementById("canvas");

const resizeCanvas = () => {
  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;
};
window.addEventListener("resize", resizeCanvas);
resizeCanvas();

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
  playground.update(dt);
  render();
  requestAnimationFrame(loop);
};

const render_polygon_arrays = (polygon_arrays, ctx) => {
  ctx.strokeStyle = `rgba(${polygon_arrays.meta.line_color.r},
                     ${polygon_arrays.meta.line_color.g},
                     ${polygon_arrays.meta.line_color.b},
                     ${polygon_arrays.meta.line_color.a})`;
  ctx.lineWidth = polygon_arrays.meta.line_weight;
  ctx.fillStyle = `rgba(${polygon_arrays.meta.fill_color.r}, ${polygon_arrays.meta.fill_color.g}, ${polygon_arrays.meta.fill_color.b}, ${polygon_arrays.meta.fill_color.a})`;
  if (polygon_arrays.vertices.length >= 3) {
    ctx.beginPath();
    ctx.moveTo(polygon_arrays.vertices[0][0], polygon_arrays.vertices[0][1]);
    for (let i = 1; i < polygon_arrays.vertices.length; i++) {
      ctx.lineTo(polygon_arrays.vertices[i][0], polygon_arrays.vertices[i][1]);
    }
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
  }
  for (let p of polygon_arrays.vertices) {
    ctx.fillStyle = `rgba(${polygon_arrays.meta.line_color.r}, ${polygon_arrays.meta.line_color.g}, ${polygon_arrays.meta.line_color.b}, ${1.0})`;

    ctx.beginPath();
    ctx.arc(p[0], p[1], 4, 0, Math.PI * 2);
    ctx.fill();
  }
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
