"""Fit and qualify a bounded bicubic height field on an explicitly oriented STL.

Research tool for R-038. The drawing is used only to generate independent
test inputs and to check the reopened STEP. Reconstruction reads the STL.
"""

import subprocess
import argparse
import json
import math
from collections import Counter
from pathlib import Path

import FreeCAD as App
import Mesh
import MeshPart
import numpy as np
import Part


def require(ok, message):
    if not ok:
        raise ValueError(message)


def basis(t):
    return np.stack(
        ((1 - t) ** 3, 3 * t * (1 - t) ** 2, 3 * t**2 * (1 - t), t**3), axis=-1
    )


def solid(bounds, levels):
    x0, y0, bottom, x1, y1 = bounds
    poles = [
        [
            App.Vector(x0 + (x1 - x0) * i / 3, y0 + (y1 - y0) * j / 3, levels[i, j])
            for j in range(4)
        ]
        for i in range(4)
    ]
    surface = Part.BSplineSurface()
    surface.buildFromPolesMultsKnots(
        poles, [4, 4], [4, 4], [0.0, 1.0], [0.0, 1.0], False, False, 3, 3
    )
    corners = [poles[0][0], poles[3][0], poles[3][3], poles[0][3]]
    base = [App.Vector(p.x, p.y, bottom) for p in corners]

    def plane(loop):
        return Part.Face(Part.makePolygon(loop + loop[:1]))

    faces = [surface.toShape(), plane(list(reversed(base)))]
    for i in range(4):
        j = (i + 1) % 4
        faces.append(plane([base[i], base[j], corners[j], corners[i]]))
    shell = Part.makeShell(faces)
    require(shell.isClosed(), "open reconstructed shell")
    obj = Part.makeSolid(shell)
    require(obj.isValid() and obj.Volume > 0, "invalid reconstructed solid")
    return obj


def triangle_error(top, lo, extent, levels):
    """Convex-hull enclosure of graph minus source plane on each triangle."""
    unit = (top[:, :, :2] - lo[:2]) / extent[:2]
    triples = [(i, j, 3 - i - j) for i in range(4) for j in range(4 - i)]

    def coefficient(n, index):
        return math.factorial(n) // math.prod(math.factorial(i) for i in index)

    def blossomed(axis):
        result = []
        for index in triples:
            b = np.ones((len(top), 1))
            for vertex, count in enumerate(index):
                t = unit[:, vertex, axis, None]
                for _ in range(count):
                    grown = np.zeros((len(top), b.shape[1] + 1))
                    grown[:, :-1] += (1 - t) * b
                    grown[:, 1:] += t * b
                    b = grown
            result.append(b)
        return result

    # Compose the two cubic bases with the affine triangle chart and
    # multiply their triangular Bernstein forms. Degree elevation of
    # the linear facet gives its value at each degree-six control point.
    u, v = blossomed(0), blossomed(1)
    control = {}
    for a, basis_u in zip(triples, u):
        rows = basis_u @ levels
        for b, basis_v in zip(triples, v):
            key = tuple(i + j for i, j in zip(a, b))
            weight = coefficient(3, a) * coefficient(3, b) / coefficient(6, key)
            control[key] = control.get(key, 0.0) + weight * np.sum(
                rows * basis_v, axis=1
            )
    bound = max(
        np.max(np.abs(z - top[:, :, 2] @ (np.array(key) / 6)))
        for key, z in control.items()
    )
    scale = max(1.0, np.abs(levels).max(), np.abs(top[:, :, 2]).max())
    return bound + 512 * np.finfo(float).eps * scale


def fit(vertices, triangles, budget):
    require(0 < len(triangles) <= 32000, "triangle budget")
    require(
        np.isfinite(vertices).all() and math.isfinite(budget) and budget > 0,
        "non-finite input or invalid budget",
    )
    edges = Counter()
    for a, b, c in triangles:
        for edge in ((a, b), (b, c), (c, a)):
            edges[edge] += 1
    require(
        all(n == 1 and edges[b, a] == 1 for (a, b), n in edges.items()),
        "open or inconsistent source edges",
    )

    lo, hi = vertices.min(axis=0), vertices.max(axis=0)
    width, depth = (hi - lo)[:2]
    eps = 2e-5
    require(min(width, depth, hi[2] - lo[2]) > 10 * eps, "degenerate frame")
    bounds = (lo[0], lo[1], lo[2], hi[0], hi[1])
    tri = vertices[triangles]
    normals = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
    require(np.all(np.linalg.norm(normals, axis=1) > 0), "degenerate triangle")

    # A closed manifold with these six connected disk charts, positive
    # projected orientation, and single rectangular boundaries covers the
    # rectangle exactly once. Count edge incidence per projected chart.
    groups = []
    for ax, side, sign in (
        (0, lo[0], -1),
        (0, hi[0], 1),
        (1, lo[1], -1),
        (1, hi[1], 1),
        (2, lo[2], -1),
    ):
        mask = (tri[:, :, ax] == side).all(axis=1)
        require(
            mask.any() and np.all(sign * normals[mask, ax] > 0),
            "unproved planar support",
        )
        groups.append(mask)
    assigned = np.sum(groups, axis=0)
    require(assigned.max() <= 1, "ambiguous side")
    roof = assigned == 0
    require(roof.any() and np.all(normals[roof, 2] > 0), "fold in height chart")
    groups.append(roof)
    for group in groups:
        cells = triangles[group].tolist()
        incidents = {}
        for n, (a, b, c) in enumerate(cells):
            for i, j in ((a, b), (b, c), (c, a)):
                incidents.setdefault(tuple(sorted((i, j))), []).append(n)
        adjacent = [set() for _ in cells]
        boundary = []
        for edge, owners in incidents.items():
            if len(owners) == 1:
                boundary.append(edge)
            else:
                i, j = owners
                adjacent[i].add(j)
                adjacent[j].add(i)
        pending, seen = [0], {0}
        while pending:
            neighbors = adjacent[pending.pop()] - seen
            pending.extend(neighbors)
            seen.update(neighbors)
        nodes = set(v for cell in cells for v in cell)
        require(
            len(seen) == len(cells)
            and len(nodes) - len(incidents) + len(cells) == 1
            and all(d == 2 for d in Counter(v for e in boundary for v in e).values()),
            "source chart is not a disk",
        )
    require(
        abs(normals[roof, 2].sum() / 2 - width * depth) <= width * depth * 1e-9,
        "incomplete projected coverage",
    )
    top_ids = np.unique(triangles[roof])
    top = vertices[top_ids]
    uv = (top[:, :2] - lo[:2]) / (hi - lo)[:2]
    rim = ((uv == 0) | (uv == 1)).any(axis=1)
    require(rim.any(), "no source rim")
    h = float(top[rim, 2].mean())
    require(np.max(np.abs(top[rim, 2] - h)) <= eps, "nonplanar rim")
    b, c = basis(uv[:, 0]), basis(uv[:, 1])
    design = (b[:, 1:3, None] * c[:, None, 1:3]).reshape((-1, 4))
    interior, _, rank, singular = np.linalg.lstsq(design, top[:, 2] - h, rcond=None)
    require(rank == 4 and singular[-1] > singular[0] * 1e-5, "unresolved height field")
    levels = np.full((4, 4), h)
    levels[1:3, 1:3] += interior.reshape((2, 2))
    residual = np.max(np.abs(design @ interior + h - top[:, 2]))
    require(
        levels.min() > lo[2] + eps and residual <= eps,
        "source residual or ground clearance",
    )
    bound = triangle_error(tri[roof], lo, hi - lo, levels)
    require(bound <= budget, "continuous deviation budget")
    return bounds, levels, float(bound)


def experiment(out, name, interior, cli):
    bounds = (-6.0, 1.0, -2.0, 14.0, 17.0)
    levels = np.full((4, 4), 4.0)
    levels[1:3, 1:3] += interior
    reference = solid(bounds, levels)
    reference.exportStep(str(out / (name + "-design.step")))
    for scale in [1.0, 0.65]:
        label = f"{name}-{scale}"
        mesh = MeshPart.meshFromShape(
            Shape=Part.read(str(out / (name + "-design.step"))),
            LinearDeflection=0.001 * scale,
            AngularDeflection=0.035 * scale,
            Relative=False,
        )
        stl = out / (label + ".stl")
        mesh.write(str(stl))
        v, t = Mesh.Mesh(str(stl)).Topology
        v, t = np.array([[p.x, p.y, p.z] for p in v]), np.array(t)
        measured, fitted, bound = fit(v, t, 0.05)
        step = out / (label + ".step")
        if cli is None:
            solid(measured, fitted).exportStep(str(step))
        else:
            subprocess.run([str(cli), "convert", str(stl), "--mode", "height-patch",
                            "-o", str(step), "--force"], check=True)
        reread = Part.read(str(step))
        require(
            len(reread.Solids) == 1
            and reread.isValid()
            and reread.Solids[0].isClosed()
            and len(reread.Faces) == 6,
            "invalid STEP readback",
        )
        require(
            sum(isinstance(f.Surface, Part.BSplineSurface) for f in reread.Faces) == 1
            and sum(isinstance(f.Surface, Part.Plane) for f in reread.Faces) == 5,
            "missing analytical surface",
        )
        require(
            abs(reread.Volume - reference.Volume) < 0.001
            and abs(reread.Volume - 320 * (fitted.mean() + 2)) < 0.001,
            "volume mismatch",
        )
        for u, w in ((0.2, 0.2), (0.5, 0.4), (0.8, 0.4), (0.6, 0.8)):
            h = basis(np.array(u)) @ levels @ basis(np.array(w))
            for offset in (-0.01, 0.01):
                point = App.Vector(-6 + 20 * u, 1 + 16 * w, h + offset)
                require(
                    reread.isInside(point, 1e-7, False) == (offset < 0),
                    "material mismatch",
                )
        damaged = v.copy()
        center = np.array([4.0, 9.0, 4.0])
        nearest = np.argmin(np.linalg.norm(v - center, axis=1))
        damaged[nearest, 2] += 0.15
        for pts, facets, limit, reason in (
            (v, t[:-1], 0.05, "open or inconsistent source edges"),
            (damaged, t, 0.05, "source residual or ground clearance"),
            (v, t, bound / 2, "continuous deviation budget"),
        ):
            try:
                fit(pts, facets, limit)
            except ValueError as err:
                require(str(err) == reason, f"unexpected refusal: {err}")
            else:
                raise ValueError("invalid patch accepted")
        print(
            json.dumps(
                dict(
                    case=label,
                    triangles=len(t),
                    bound_mm=bound,
                    volume_mm3=reread.Volume,
                    negative_checks=3,
                )
            ),
            flush=True,
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path, help="Reopen the actual FabSTEP export for each STL.")
    args = parser.parse_args()
    output = args.output
    output.mkdir(parents=True, exist_ok=True)
    experiment(output, "boss", [[2.0, 3.0], [1.0, 2.5]], args.cli)
    experiment(output, "saddle", [[1.5, -2.0], [-1.5, 2.0]], args.cli)
