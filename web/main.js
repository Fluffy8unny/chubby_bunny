import init, { Playground } from "../pkg/chubby_bunny_playground.js";

let playground = null;

async function start() {
    console.log("Initializing...");
  await init();

  playground = new Playground();
  playground.init();
  playground.last_timestamp = performance.now();
  requestAnimationFrame(loop);
}

function loop(timestamp) {
    console.log("Updating...");
  let dt = timestamp - (playground.last_timestamp || timestamp);
  playground.last_timestamp = timestamp;
  playground.update(dt);
  render();
  requestAnimationFrame(loop);
}

function render_polygon_arrays( polygon_arrays , ctx) {
    for (let p of polygon_arrays.vertices) {
            ctx.beginPath();
            ctx.arc(p[0], p[1], 4, 0, Math.PI * 2);
            ctx.fill();
    }
    for (let e of polygon_arrays.edges) {
        ctx.beginPath();
        ctx.moveTo(polygon_arrays.vertices[e[0]][0], polygon_arrays.vertices[e[0]][1]);
        ctx.lineTo(polygon_arrays.vertices[e[1]][0], polygon_arrays.vertices[e[1]][1]);
        ctx.stroke();
    }

    for (let child of polygon_arrays.children) {
        render_polygon_arrays(child, ctx);
    }
}
function render() {
  const polygon_arrays = playground.get_polygon_arrays();
  const canvas = document.getElementById("canvas");
  const ctx = canvas.getContext("2d");

  ctx.clearRect(0, 0, canvas.width, canvas.height);
    for (let p of polygon_arrays) {
    render_polygon_arrays(p, ctx);
    }
}

start();
