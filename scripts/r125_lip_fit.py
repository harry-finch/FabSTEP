"""Fit the five supports of an obliquely clipped, round-ended blank from STL alone.

Laboratory domain for R-125. All dimensions, the axis and the cutting plane
come from the input. The two trimmed curved faces have independent watertight
UV certificates; this module neither substitutes a STEP nor changes Auto.
"""

import math
from collections import defaultdict
from dataclasses import dataclass

from r113_shaft_fit import (
    FIT,
    circle,
    cross,
    cycles,
    dot,
    mean,
    radial,
    require,
    sub,
    unit,
)

BUDGET = 0.05
QUARTER = math.pi / 2


def plane_regions(vertices, triangles):
    edges, normals = defaultdict(list), []
    for i, face in enumerate(triangles):
        p, q, r = (vertices[v] for v in face)
        normals.append(cross(sub(q, p), sub(r, p)))
        for a, b in zip(face, face[1:] + face[:1]):
            edges[min(a, b), max(a, b)].append((i, a, b))
    directions = [unit(n) for n in normals]
    neighbors = [[] for _ in triangles]
    for uses in edges.values():
        require(len(uses) == 2, "open or non-manifold edge")
        (i, a, b), (j, c, d) = uses
        require((a, b) == (d, c), "reversed incidence")
        if dot(directions[i], directions[j]) > 1 - 1e-8:
            neighbors[i].append(j)
            neighbors[j].append(i)
    patches, seen = [], set()
    for i in range(len(triangles)):
        if i in seen:
            continue
        seen.add(i)
        ids, pending = {i}, [i]
        while pending:
            for j in neighbors[pending.pop()]:
                if j not in seen:
                    seen.add(j)
                    ids.add(j)
                    pending.append(j)
        if len(ids) < 6:
            continue
        borders = []
        for j in ids:
            t = triangles[j]
            for a, b in zip(t, t[1:] + t[:1]):
                if any(k not in ids for k, _, _ in edges[min(a, b), max(a, b)]):
                    borders.append((a, b))
        n = unit(mean([normals[j] for j in sorted(ids)]))
        p = vertices[triangles[i][0]]
        if (
            max(abs(dot(sub(vertices[v], p), n)) for j in ids for v in triangles[j])
            <= FIT
        ):
            (loop,) = cycles(borders)
            patches.append((ids, n, p, loop))
    require(len(patches) == 3, "expected three planar support disks")
    return patches, edges


@dataclass(frozen=True)
class Lip:
    origin: tuple
    axis: tuple
    radial: tuple
    outer: float
    major: float
    height: float
    cut_offset: float
    slope: float

    @property
    def minor(self):
        return self.outer - self.major

    @property
    def shoulder(self):
        return self.height - self.minor

    def local(self, point):
        p = sub(point, self.origin)
        return (
            dot(p, self.radial),
            dot(p, cross(self.axis, self.radial)),
            dot(p, self.axis),
        )

    def profile(self, family, v):
        if family == "cylinder":
            return self.outer, v
        return self.major + self.minor * math.cos(
            v
        ), self.shoulder + self.minor * math.sin(v)

    def span(self, family, v):
        r, z = self.profile(family, v)
        return math.acos(-(self.cut_offset + self.slope * z) / r)

    def point(self, family, s, v):
        r, z = self.profile(family, v)
        u = s * self.span(family, v)
        return -r * math.cos(u), -r * math.sin(u), z

    def curvature(self, family):
        # q=(offset+slope*z)/rho; derive from q*rho=offset+slope*z. The
        # lower bound on sqrt(1-q²) covers BOTH branches over the whole face.
        fillet = family == "torus"
        lo, hi = (self.shoulder, self.height) if fillet else (0, self.shoulder)
        min_radius = self.major if fillet else self.outer
        q = max(abs(self.cut_offset + self.slope * z) for z in (lo, hi)) / min_radius
        require(q < 0.97, "cut near tangent or missing branch")
        rho1 = self.minor if fillet else 0
        z1 = self.minor if fillet else 1
        q1 = (abs(self.slope) * z1 + q * rho1) / min_radius
        q2 = ((abs(self.slope) + q + 2 * q1) * rho1) / min_radius
        root = math.sqrt(1 - q * q)
        a, a1 = math.acos(-q), q1 / root
        a2 = q2 / root + q * q1 * q1 / root**3
        return (
            self.outer * a * a,
            rho1 * a + self.outer * (a + 1) * a1,
            rho1 + 2 * rho1 * a1 + self.outer * (a1 * a1 + a2),
        )


def detect(vertices, triangles):
    patches, edges = plane_regions(vertices, triangles)
    candidates = [
        (a, b)
        for i, a in enumerate(patches)
        for b in patches[i + 1 :]
        if dot(a[1], b[1]) < -1 + 1e-10
    ]
    require(len(candidates) == 1, "ambiguous cap axis")
    a, b = candidates[0]
    (cut,) = [p for p in patches if p is not a and p is not b]

    def cap(p):
        rim = [
            vertices[v]
            for v in p[3]
            if abs(dot(sub(vertices[v], cut[2]), cut[1])) > FIT
        ]
        require(len(rim) >= 6, "missing circular cap arc")
        c, r = circle(rim, p[1])
        return r, c, p

    (major, top, top_face), (outer, bottom, bot_face) = sorted(
        (cap(a), cap(b)), key=lambda x: x[0]
    )
    axis = top_face[1]
    delta = sub(top, bottom)
    height = dot(delta, axis)
    require(
        math.hypot(*radial(delta, axis)) <= FIT
        and 10 * FIT < outer - major < height - FIT,
        "cap circles do not support a common filleted cylinder",
    )
    xdir = unit(radial(cut[1], axis))
    nx = dot(cut[1], xdir)
    require(
        abs(dot(cut[1], axis)) > 1e-3 and nx > 0.25, "expected a transverse oblique cut"
    )
    lip = Lip(
        bottom,
        axis,
        xdir,
        outer,
        major,
        height,
        dot(sub(cut[2], bottom), cut[1]) / nx,
        -dot(cut[1], axis) / nx,
    )
    owners = {
        name: set(p[0])
        for name, p in (("bottom", bot_face), ("top", top_face), ("cut", cut))
    }
    return lip, owners, edges


def bilinear_bound(lip, family, uv, xyz, coeff, depth=0):
    ds, dv = [max(row) - min(row) for row in zip(*uv)]
    drift = max(math.dist(lip.point(family, *p), q) for p, q in zip(uv, xyz))
    ss, sv, vv = coeff
    bound = drift + (ss * ds * ds + 2 * sv * ds * dv + vv * dv * dv) / 8 + 1e-10
    if bound <= BUDGET or depth == 4:
        return bound

    def subdivide(t):
        return [
            *t,
            *(tuple((x + y) / 2 for x, y in zip(t[i - 1], t[i])) for i in range(3)),
        ]

    u, q = subdivide(uv), subdivide(xyz)
    return max(
        bilinear_bound(
            lip, family, [u[i] for i in t], [q[i] for i in t], coeff, depth + 1
        )
        for t in ((0, 3, 4), (1, 4, 5), (2, 5, 3), (3, 4, 5))
    )


def certify_chart(lip, family, xyz, triangles, owners, labels, touching, edges, coeff):
    lo, hi = ("bottom", "torus") if family == "cylinder" else ("cylinder", "top")
    vmax = lip.shoulder if family == "cylinder" else QUARTER
    uv, sides = {}, defaultdict(list)
    for i, neighbors in touching.items():
        if family not in neighbors:
            continue
        x, y, z = xyz[i]
        v = (
            z
            if family == "cylinder"
            else math.atan2(z - lip.shoulder, math.hypot(x, y) - lip.major)
        )
        if lo in neighbors:
            v = 0.0
        if hi in neighbors:
            v = vmax
        angle = math.atan2(-y, -x)
        s = (
            math.copysign(1, angle)
            if "cut" in neighbors
            else angle / lip.span(family, v)
        )
        require(-1 <= s <= 1 and 0 <= v <= vmax, "point outside trimmed chart")
        require(
            math.dist(lip.point(family, s, v), xyz[i]) <= FIT,
            "source leaves fitted support",
        )
        uv[i] = s, v
    max_span, trim_error = 0.0, 0.0
    neighbor_bounds = defaultdict(float)
    for t in owners[family]:
        tri = triangles[t]
        q = [uv[i] for i in tri]
        (s, v), (s1, v1), (s2, v2) = q
        require(
            (s1 - s) * (v2 - v) - (v1 - v) * (s2 - s) > 1e-12,
            "folded parameter triangle",
        )
        points = [xyz[i] for i in tri]
        if family == "cylinder":
            # Develop the cylinder in unwrapped angle/height. A long vertical
            # generatrix adds no curvature. Only the two cut sides differ
            # between this polygonal UV boundary and the analytic trim.
            a, b, c = (s * lip.span(family, v) for s, v in q)
            require(
                (b - a) * (v2 - v) - (v1 - v) * (c - a) > 1e-12,
                "folded cylinder development",
            )
            drift = max(math.dist(lip.point(family, *p), x) for p, x in zip(q, points))
            bound = drift + lip.outer * (max(a, b, c) - min(a, b, c)) ** 2 / 8 + 1e-10
        else:
            bound = bilinear_bound(lip, family, q, points, coeff)
        max_span = max(max_span, bound)
    for uses in edges.values():
        (i, a, b), (j, _, _) = uses
        if labels[i] == labels[j] or family not in (labels[i], labels[j]):
            continue
        if labels[i] != family:
            i, j, a, b = j, i, b, a
        neighbor = labels[j]
        (s, v), (t, w) = uv[a], uv[b]
        if neighbor == "cut":
            require(s == t and abs(s) == 1, "cut branch mismatch")
            key = "+" if s > 0 else "-"
            start, end = s * v, s * w
            if family == "cylinder":
                ratio = (
                    max(abs(lip.cut_offset + lip.slope * z) for z in (v, w)) / lip.outer
                )
                second = ratio * (lip.slope / lip.outer) ** 2 / (1 - ratio**2) ** 1.5
                trim_error = max(trim_error, lip.outer * second * (w - v) ** 2 / 8)
        else:
            require(
                neighbor in (lo, hi) and v == w == (0.0 if neighbor == lo else vmax),
                "wrong rim neighbor",
            )
            key = neighbor
            start, end = (s, t) if neighbor == lo else (-s, -t)
        if neighbor == "cut":
            remainder = coeff[2] * (w - v) ** 2 / 8
        else:
            radius, _ = lip.profile(family, v)
            remainder = radius * (lip.span(family, v) * (t - s)) ** 2 / 8
        drift = max(math.dist(lip.point(family, *uv[i]), xyz[i]) for i in (a, b))
        neighbor_bounds[neighbor] = max(
            neighbor_bounds[neighbor], drift + remainder + 1e-10
        )
        require(start < end, "reversed boundary")
        sides[key].append((start, end, a, b))
    keys = (lo, "+", hi, "-")
    corner = []
    for key, extent in zip(keys, ((-1, 1), (0, vmax), (-1, 1), (-vmax, 0))):
        chain = sorted(sides[key])
        require(chain and (chain[0][0], chain[-1][1]) == extent, "incomplete branch")
        for left, right in zip(chain, chain[1:]):
            require(
                left[1] == right[0] and left[3] == right[2],
                "disconnected or overlapping curve",
            )
        corner.append((chain[0][2], chain[-1][3]))
    require(
        all(corner[i - 1][1] == pair[0] for i, pair in enumerate(corner)),
        "branches end at different vertices",
    )
    max_span += trim_error
    require(
        max_span <= BUDGET,
        f"{family} continuous deviation bound {max_span:.8g} mm exceeds budget",
    )
    return {
        "bound_mm": max_span,
        "boundary_mm": dict(neighbor_bounds),
        "cut_segments": [len(sides[k]) for k in ("-", "+")],
    }


def planar_bound(lip, name, xyz, triangles, ids, labels, edges, reports):
    # Orthogonal projection is linear. For a disk with a simple projected
    # boundary and positive triangles, the projected triangle interiors cover
    # it once (winding number). If matched Jordan boundaries move by at most e,
    # straight-line homotopy preserves winding outside their e-neighborhoods.
    # Thus interior Hausdorff distance in either direction is at most e too.
    def coordinates(p):
        x, y, z = p
        if name == "cut":
            length = math.hypot(1, lip.slope)
            return (y, (lip.slope * x + z) / length), (
                x - lip.cut_offset - lip.slope * z
            ) / length
        return ((x, y) if name == "top" else (y, x)), z - (
            lip.height if name == "top" else 0.0
        )

    projected = {i: coordinates(xyz[i]) for t in ids for i in triangles[t]}

    def signed(a, b, c):
        return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])

    for t in ids:
        require(
            signed(*(projected[i][0] for i in triangles[t])) > 1e-12,
            "folded planar face",
        )
    drift = max(abs(h) for _, h in projected.values())
    require(drift <= FIT, "source leaves fitted plane")
    boundary, straight = [], defaultdict(list)
    for uses in edges.values():
        (i, a, b), (j, _, _) = uses
        if (i in ids) == (j in ids):
            continue
        if j in ids:
            i, j, a, b = j, i, b, a
        boundary.append((a, b))
        if labels[j] in ("bottom", "top", "cut"):
            cap = labels[j] if name == "cut" else name
            require(
                cap in ("bottom", "top") and "cut" in (name, labels[j]),
                "unexpected plane interface",
            )
            straight[cap].append((a, b))
    (ring,) = cycles(boundary)
    require(len(ring) <= 512, "planar loop budget")
    q = [projected[i][0] for i in ring]
    # Reject nonadjacent crossings or uncertain orientations; axis intervals
    # disjoint by a physical allowance need no orientation computation.
    for i, a in enumerate(q):
        b = q[(i + 1) % len(q)]
        for j in range(i + 2, len(q) - (i == 0)):
            c, d = q[j], q[(j + 1) % len(q)]
            if any(
                max(a[k], b[k]) + 1e-10 < min(c[k], d[k])
                or max(c[k], d[k]) + 1e-10 < min(a[k], b[k])
                for k in (0, 1)
            ):
                continue
            require(
                signed(a, b, c) * signed(a, b, d) > 1e-20
                or signed(c, d, a) * signed(c, d, b) > 1e-20,
                "crossed or uncertain planar loop",
            )
    rim = max(report["boundary_mm"].get(name, 0.0) for report in reports.values())
    require(
        set(straight) == ({"top", "bottom"} if name == "cut" else {name}),
        "missing straight trim",
    )
    for cap, segments in straight.items():
        z = 0.0 if cap == "bottom" else lip.height
        r = lip.outer if cap == "bottom" else lip.major
        x = lip.cut_offset + lip.slope * z
        y = math.sqrt(r * r - x * x)
        # The remaining two plane corners are the measured circular endpoints.
        # Intermediate vertices may freely subdivide the same straight segment.
        segments.sort(key=lambda e: min(xyz[e[0]][1], xyz[e[1]][1]))
        a, b = min(
            (p for edge in segments for p in edge), key=lambda p: xyz[p][1]
        ), max((p for edge in segments for p in edge), key=lambda p: xyz[p][1])
        require(
            math.dist(xyz[a], (x, -y, z)) <= FIT
            and math.dist(xyz[b], (x, y, z)) <= FIT,
            "cut chord endpoints disagree",
        )
        last, forward = a, None
        for i, j in segments:
            direct = xyz[i][1] < xyz[j][1]
            require(forward is None or direct == forward, "reversed chord")
            forward = direct
            left, right = (i, j) if direct else (j, i)
            require(left == last and xyz[left][1] < xyz[right][1], "cut chord gap")
            for k in (left, right):
                rim = max(rim, math.dist(xyz[k], (x, max(-y, min(y, xyz[k][1])), z)))
            last = right
        require(last == b, "incomplete chord")
    require(drift + rim <= BUDGET, "plane exceeds continuous deviation budget")
    return {"bound_mm": drift + rim, "edges": len(ring)}


def fit(vertices, triangles):
    require(4 <= len(triangles) <= 10_000, "expected small lab fixture")
    lip, owners, edges = detect(vertices, triangles)
    families = ("cylinder", "torus")
    coeff = {f: lip.curvature(f) for f in families}
    owners.update({f: set() for f in families})
    xyz = [lip.local(p) for p in vertices]
    labels = {t: name for name, ids in owners.items() for t in ids}
    touching = defaultdict(set)
    for i, tri in enumerate(triangles):
        if i not in labels:
            points = [xyz[v] for v in tri]
            residual = max(abs(math.hypot(x, y) - lip.outer) for x, y, _ in points)
            name = (
                "cylinder"
                if residual < FIT and max(z for _, _, z in points) <= lip.shoulder + FIT
                else "torus"
            )
            labels[i] = name
            owners[name].add(i)
        for v in tri:
            touching[v].add(labels[i])
    require(all(owners.values()), "missing measured support")
    reports = {
        f: certify_chart(
            lip, f, xyz, triangles, owners, labels, touching, edges, coeff[f]
        )
        for f in families
    }
    planes = {
        name: planar_bound(
            lip, name, xyz, triangles, owners[name], labels, edges, reports
        )
        for name in ("top", "bottom", "cut")
    }
    reports.update(planes)
    return lip, {name: sorted(ids) for name, ids in owners.items()}, reports
