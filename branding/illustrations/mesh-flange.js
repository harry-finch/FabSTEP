/** A drilled flange: the pointer resolves locally faceted rims into circles. */
const {
  Cam, clamp, facing, fillet, fit, hull, open, poly, prism, proj, ringAt,
  rings, run, spring, stepS, unproj, disposer, mk, pointer, put, register, solid,
} = HL;

function mount({ stage, svg, read }, value) {
  const bag = disposer(), C = Cam(45, 0.5, 1.82);
  fit(C, [[-58, -48, 0], [58, -48, 0], [-58, 48, 0], [58, 48, 0], [0, 0, 33]], 200, 166);
  const P = proj(C), front = facing(C), g = mk("g", {}, svg);
  const [base, inset] = rings(-58, -48, 58, 48, 9, 1.6);
  put(solid(g), prism(P, front, base, inset, 0, 8));
  let reach = value, over = null, active = -1;
  // Rounded octagon and circle share sample identities throughout the gesture.
  const corners = Array.from({ length: 8 }, (_, i) => {
    const a = (i + 0.5) * Math.PI / 4;
    return [Math.cos(a), Math.sin(a)];
  });
  const rounded = fillet(corners, Array(8).fill(0.045), 5);
  const facet = rounded.flatMap(([u, v], i) => {
    const [a, b] = rounded[(i + 1) % rounded.length];
    const steps = Math.max(1, Math.ceil(Math.hypot(a - u, b - v) / 0.075));
    return Array.from({ length: steps }, (_, k) => [u + (a - u) * k / steps, v + (b - v) * k / steps]);
  });
  function ring(radius, smooth, x = 0, y = 0) {
    return facet.map(([u, v]) => {
      const d = Math.hypot(u, v), k = radius * (1 + smooth * (1 / d - 1));
      return { u: x + u * k, v: y + v * k, nu: u / d, nv: v / d };
    });
  }
  // Far drills, raised hub, then near drills: opaque faces keep their order.
  const specs = [[-40, -30, 6.5, 8], [40, -30, 6.5, 8],
    [0, 0, 13, 30], [-40, 30, 6.5, 8], [40, 30, 6.5, 8]];
  const parts = [];
  for (const [i, s] of specs.entries()) {
    const [x, y, r, z] = s, group = mk("g", {}, g);
    const hub = i === 2 ? solid(group) : null;
    const aperture = mk("path", { class: "sil" }, group);
    const inside = mk("path", { class: "nf lo" }, group);
    const rest = i === 2 ? 0.10 : [0.08, 0.22, 0, 0.14, 0.32][i];
    parts.push({ x, y, r, z, rest, hub, aperture, inside, sp: spring(rest), drawn: NaN });
  }
  function draw(p) {
    const v = p.sp.x;
    if (v === p.drawn) return;
    p.drawn = v;
    if (p.hub) {
      const outer = ring(26, v), inner = ring(24.5, v);
      put(p.hub, { sil: poly(hull([...ringAt(P, outer, 8), ...ringAt(P, outer, 30)])),
        crease: open(ringAt(P, run(inner, front), 30)) });
    }
    const top = ring(p.r, v, p.x, p.y), edge = poly(ringAt(P, top, p.z));
    p.aperture.setAttribute("d", edge);
    // Keep only the lower lip geometrically inside the aperture; no floor.
    const aperture = ringAt(P, top, p.z);
    function visible(q) {
      const [x, y] = P(q.u, q.v, p.z - 7);
      let inside = false;
      for (let i = 0, j = aperture.length - 1; i < aperture.length; j = i++) {
        const [ax, ay] = aperture[i], [bx, by] = aperture[j];
        if ((ay > y) !== (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax) inside = !inside;
      }
      return inside;
    }
    p.inside.setAttribute("d", open(ringAt(P, run(top, visible), p.z - 7)));
  }
  function accent() {
    for (const [i, p] of parts.entries()) {
      p.aperture.classList.toggle("hi", i === active && !p.hub);
      if (p.hub) p.hub.sil.classList.toggle("hi", active === i || active < 0);
    }
  }
  const loop = register(stage, dt => {
    let moving = false;
    for (const p of parts) { if (stepS(p.sp, dt)) moving = true; draw(p); }
    return moving;
  });
  bag.add(loop.unregister);
  function target(point) {
    over = point;
    if (point) {
      // Immutable rest tops, including the raised bore: no moving hit area.
      const distances = parts.map(p => {
        const [u, v] = unproj(C, point[0], point[1], p.z);
        return Math.hypot(u - p.x, v - p.y);
      });
      active = distances.indexOf(Math.min(...distances));
      if (distances[active] > 35) { over = null; active = -1; }
      for (const [i, p] of parts.entries()) p.sp.t = over
        ? clamp(p.rest + (1 - p.rest) * Math.exp(-((distances[i] / reach) ** 2)), 0, 1) : p.rest;
    } else {
      active = -1;
      for (const p of parts) p.sp.t = p.rest;
    }
    read.textContent = active < 0 ? "rest" : active === 2 ? "bore" : `drill ${active < 2 ? active + 1 : active}`;
    accent(); loop.wake();
  }
  for (const p of parts) draw(p);
  accent(); read.textContent = "rest";
  bag.add(pointer(stage, { move: target, leave: () => target(null) }));
  bag.add(() => svg.replaceChildren());
  return { set: v => { reach = v; target(over); }, destroy: bag.dispose };
}

hairline({
  name: "mesh-flange",
  means: "Une bride percée : le pointeur révèle les courbes derrière les contours facettisés, sans ajouter de matière.",
  rules: [1, 3, 5, 6, 7, 8, 9],
  range: [16, 34, 54],
  tour: [[190, 112], [288, 163], [186, 213], null],
  mount,
});
