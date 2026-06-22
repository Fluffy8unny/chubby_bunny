const DEFAULT_MAX_RENDER_DPR = 1.5;
const DEFAULT_LINE_WEIGHT_BASELINE_MIN_DIMENSION = 1280;
const DEFAULT_LINE_WEIGHT_MIN = 0.75;
const DEFAULT_LINE_WEIGHT_MAX = 6;

const clamp = (value, min, max) => {
  return Math.max(min, Math.min(max, value));
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

export const createRenderer = (
  canvas,
  {
    maxRenderDpr = DEFAULT_MAX_RENDER_DPR,
    lineWeightBaselineMinDimension = DEFAULT_LINE_WEIGHT_BASELINE_MIN_DIMENSION,
    lineWeightMin = DEFAULT_LINE_WEIGHT_MIN,
    lineWeightMax = DEFAULT_LINE_WEIGHT_MAX,
  } = {},
) => {
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("2D canvas context is not available");
  }

  let viewportWidth = 0;
  let viewportHeight = 0;
  let currentDpr = 1;

  const getLineWeightScale = () => {
    const minDim = Math.max(1, Math.min(viewportWidth, viewportHeight));
    return minDim / lineWeightBaselineMinDimension;
  };

  const renderPolygonArrays = (polygonArrays) => {
    ctx.strokeStyle = `rgba(${polygonArrays.meta.line_color.r},
                     ${polygonArrays.meta.line_color.g},
                     ${polygonArrays.meta.line_color.b},
                     ${polygonArrays.meta.line_color.a})`;
    const scaledWeight = polygonArrays.meta.line_weight * getLineWeightScale();
    ctx.lineWidth = clamp(scaledWeight, lineWeightMin, lineWeightMax);
    ctx.fillStyle = `rgba(${polygonArrays.meta.fill_color.r}, ${polygonArrays.meta.fill_color.g}, ${polygonArrays.meta.fill_color.b}, ${polygonArrays.meta.fill_color.a})`;
    if (polygonArrays.meta.smooth_edges) {
      drawSmoothClosedPath(ctx, polygonArrays.vertices);
      ctx.fill();
      ctx.stroke();
    } else {
      ctx.beginPath();
      ctx.moveTo(polygonArrays.vertices[0][0], polygonArrays.vertices[0][1]);
      for (let i = 1; i < polygonArrays.vertices.length; i++) {
        ctx.lineTo(polygonArrays.vertices[i][0], polygonArrays.vertices[i][1]);
      }
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
    }

    for (const child of polygonArrays.children) {
      renderPolygonArrays(child);
    }
  };

  const resize = (width = window.innerWidth, height = window.innerHeight) => {
    viewportWidth = width;
    viewportHeight = height;
    currentDpr = Math.min(window.devicePixelRatio || 1, maxRenderDpr);

    canvas.width = Math.floor(viewportWidth * currentDpr);
    canvas.height = Math.floor(viewportHeight * currentDpr);
    canvas.style.width = `${viewportWidth}px`;
    canvas.style.height = `${viewportHeight}px`;

    ctx.setTransform(currentDpr, 0, 0, currentDpr, 0, 0);

    return {
      width: viewportWidth,
      height: viewportHeight,
      dpr: currentDpr,
    };
  };

  const render = (polygonArrays) => {
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.setTransform(currentDpr, 0, 0, currentDpr, 0, 0);

    for (const polygon of polygonArrays) {
      renderPolygonArrays(polygon);
    }
  };

  const getCurrentDpr = () => currentDpr;
  const getViewport = () => ({ width: viewportWidth, height: viewportHeight });

  return {
    resize,
    render,
    getCurrentDpr,
    getViewport,
  };
};
