#!/usr/bin/env python3
"""Independent tangent-arc profiles, without importing FabSTEP or private meshes."""
import hashlib
import json
import math
from pathlib import Path
import struct

OUT = Path(__file__).parent / "short_arcs"
HEIGHT = 7.0


def cross(a, b, c):
    return (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])


def triangulate(points):
    ids = list(range(len(points)))
    triangles = []
    while len(ids) > 3:
        for i, b in enumerate(ids):
            a, c = ids[i-1], ids[(i+1) % len(ids)]
            if cross(points[a], points[b], points[c]) <= 1e-12:
                continue
            if any(all(x >= -1e-12 for x in (
                cross(points[a], points[b], points[k]),
                cross(points[b], points[c], points[k]),
                cross(points[c], points[a], points[k])))
                for k in ids if k not in (a, b, c)):
                continue
            triangles.append((a, b, c))
            ids.pop(i)
            break
        else:
            raise ValueError("Non-simple profile")
    triangles.append(tuple(ids))
    return triangles


def profile(short_degrees=20, concave=False, polygon=False, anchors=True):
    turns = [math.radians(60), math.radians(short_degrees)*(-1 if concave else 1), math.radians(60)]
    heading = -sum(turns)/2
    p = (0.0, 0.0)
    points, arcs = [], []
    for index, (radius, turn) in enumerate(zip((4., 1., 4.), turns)):
        sign = 1 if turn > 0 else -1
        center = (p[0]-sign*radius*math.sin(heading), p[1]+sign*radius*math.cos(heading))
        angle = heading-sign*math.pi/2
        end = (center[0]+radius*math.cos(angle+turn), center[1]+radius*math.sin(angle+turn))
        if index == 1 and polygon:
            center = (center[0], center[1]+.5)
            radius = math.dist(p, center)
            angle = math.atan2(p[1]-center[1], p[0]-center[0])
            last = math.atan2(end[1]-center[1], end[0]-center[0])
            turn = math.atan2(math.sin(last-angle), math.cos(last-angle))
        count = 3 if index == 1 or not anchors else 12
        start_index = len(points)
        for i in range(count):
            points.append(p if i == 0 else (
                center[0]+radius*math.cos(angle+turn*i/count),
                center[1]+radius*math.sin(angle+turn*i/count)))
        arcs.append(dict(center=center, radius=radius, angle=angle, sweep=turn,
                         start=start_index, count=count, end=end))
        p = end
        heading += turns[index]
    points.append(p)
    return points, arcs


def transform(p, kind):
    x, y, z = p
    if kind == "rotated":
        sx, cx = math.sin(.37), math.cos(.37)
        sy, cy = math.sin(-.51), math.cos(-.51)
        return (4+cy*x+sy*sx*y+sy*cx*z, 8+cx*y-sx*z, 12-sy*x+cy*sx*y+cy*cx*z)
    if kind == "f32":
        return tuple(struct.unpack("<f", struct.pack("<f", v))[0] for v in (73+x, -133+y, z))
    return p


def nominal_bounds(points, arcs, kind, short_recognized):
    # Extremize each coordinate's linear functional on the circle domains.
    rows = [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)]
    translation = [0., 0., 0.]
    if kind == "rotated":
        sx, cx = math.sin(.37), math.cos(.37)
        sy, cy = math.sin(-.51), math.cos(-.51)
        rows = [(cy, sy*sx, sy*cx), (0., cx, -sx), (-sy, cy*sx, cy*cx)]
        translation = [4., 8., 12.]
    elif kind == "f32":
        translation = [73., -133., 0.]
    bounds = [[], []]
    for axis, (a, b, c) in enumerate(rows):
        values = [a*x+b*y for x, y in points]
        for i, arc in enumerate(arcs):
            if i == 1 and not short_recognized:
                continue
            lo, hi = sorted((arc["angle"], arc["angle"]+arc["sweep"]))
            for k in range(-4, 5):
                theta = math.atan2(b, a)+k*math.pi
                if lo <= theta <= hi:
                    x = arc["center"][0]+arc["radius"]*math.cos(theta)
                    y = arc["center"][1]+arc["radius"]*math.sin(theta)
                    values.append(a*x+b*y)
        bounds[0].append(translation[axis]+min(values)+min(0., c*HEIGHT))
        bounds[1].append(translation[axis]+max(values)+max(0., c*HEIGHT))
    return bounds


def make(name, *, concave=False, short_degrees=20, polygon=False, anchors=True,
         kind="identity", defect=None):
    points, arcs = profile(short_degrees, concave, polygon, anchors)
    n = len(points)
    vertices = [(x, y, z) for z in (0., 2.8, HEIGHT) for x, y in points]
    triangles = []
    caps = triangulate(points)
    triangles.extend((c, b, a) for a, b, c in caps)
    triangles.extend((a+2*n, b+2*n, c+2*n) for a, b, c in caps)
    for layer in range(2):
        for i in range(n):
            j = (i+1) % n
            pair = [(layer*n+i, layer*n+j, (layer+1)*n+j),
                    (layer*n+i, (layer+1)*n+j, (layer+1)*n+i)]
            if defect and layer == 0 and i == arcs[1]["start"]+1:
                tri = pair[0]
                middle = [sum(vertices[v][k] for v in tri)/3 for k in range(3)]
                center, radius = arcs[1]["center"], arcs[1]["radius"]
                theta = math.atan2(middle[1]-center[1], middle[0]-center[0])
                if defect == "fold":
                    theta = arcs[1]["angle"]-0.1
                else:
                    radius += .03
                middle[:2] = [center[0]+radius*math.cos(theta), center[1]+radius*math.sin(theta)]
                v = len(vertices)
                vertices.append(tuple(middle))
                triangles.extend((tri[k], tri[(k+1) % 3], v) for k in range(3))
                triangles.append(pair[1])
            else:
                triangles.extend(pair)
    vertices = [transform(p, kind) for p in vertices]
    lines = ["solid independent_short_arcs"]
    for tri in triangles:
        lines.extend(("facet normal 0 0 0", "outer loop"))
        lines.extend("vertex "+" ".join(format(x, ".17g") for x in vertices[i]) for i in tri)
        lines.extend(("endloop", "endfacet"))
    lines.append("endsolid independent_short_arcs")
    payload = ("\n".join(lines)+"\n").encode()
    (OUT/(name+".stl")).write_bytes(payload)
    recognized = anchors and not polygon and short_degrees >= 20
    area = sum(points[i][0]*points[(i+1) % n][1]-points[(i+1) % n][0]*points[i][1] for i in range(n))/2
    # Replace each inscribed polygon sector by its independently known arc.
    for index, arc in enumerate(arcs):
        if (index == 1 and not recognized) or not anchors:
            continue
        delta = arc["sweep"]/arc["count"]
        area += arc["count"]*arc["radius"]**2*(delta-math.sin(delta))/2
    result = dict(sha256=hashlib.sha256(payload).hexdigest(), triangles=len(triangles),
                  fit_mm=2e-5 if kind == "f32" else 1e-5, short_recognized=recognized,
                  cylinders=3 if recognized else 2, planes=3 if recognized else 6,
                  volume_mm3=HEIGHT*area, bounds_mm=nominal_bounds(points, arcs, kind, recognized))
    if defect:
        result["expected_error"] = "recognition.partial_unproven"
    elif not anchors:
        result["expected_error"] = "recognition.unsupported"
    return result


def main():
    OUT.mkdir(exist_ok=True)
    cases = {}
    for name, options in [
        ("convex", {}), ("concave", {"concave": True}),
        ("rotated", {"kind": "rotated"}), ("f32_translated", {"kind": "f32"}),
        ("unresolved", {"short_degrees": 2}), ("polygonal_gap", {"polygon": True}),
        ("no_anchors", {"anchors": False}),
        ("bulge", {"defect": "bulge"}), ("fold", {"defect": "fold"}),
    ]:
        cases[name] = make(name, **options)
    (OUT/"expected.json").write_text(json.dumps({"height_mm": HEIGHT,
        "description": "Three tangent circular arcs R4/R1/R4, then the closing chord; source-only defects are explicit.",
        "cases": cases}, indent=2)+"\n")


if __name__ == "__main__":
    main()
