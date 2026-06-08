import init, { Playground } from "../pkg/chubby_bunny_playground.js";

let playground = null;

async function start() {
  await init();

  playground = new Playground();
  playground.init();

  requestAnimationFrame(loop);
}

function loop(timestamp) {
  const dt = 1 / 60;
  playground.update(dt);
  render();
  requestAnimationFrame(loop);
}

function render() {
  const canvas = document.getElementById("canvas");
  const ctx = canvas.getContext("2d");

  ctx.clearRect(0, 0, canvas.width, canvas.height);

  for (let i = 0; i < playground.point_count(); i++) {
    const x = playground.point_x(i);
    const y = playground.point_y(i);

    ctx.beginPath();
    ctx.arc(x, y, 4, 0, Math.PI * 2);
    ctx.fill();
  }

  for (let i = 0; i < playground.line_count(); i++) {
    const a = playground.line_a(i);
    const b = playground.line_b(i);

    const ax = playground.point_x(a);
    const ay = playground.point_y(a);
    const bx = playground.point_x(b);
    const by = playground.point_y(b);

    ctx.beginPath();
    ctx.moveTo(ax, ay);
    ctx.lineTo(bx, by);
    ctx.stroke();
  }
}

start();
