"""Measure two adjacent, equal-radius bottom fillets on a FreeCAD blind pocket.

Run with FreeCAD's Python. --cli PATH measures the current STEP in Auto.
--require-analytic additionally requires the actual shared elliptical edge
on both reconstructed cylinders at both mesh resolutions.
"""

import argparse
from collections import Counter
import json
import math
from pathlib import Path
import runpy
import subprocess

import FreeCAD as App
import MeshPart
import Part


checks = runpy.run_path(
    str(Path(__file__).with_name("validate-oblique-pocket-freecad.py"))
)
near = checks["near"]
check_ends = checks["check_ends"]
check_plane = checks["check_plane"]
pcurve_report = checks["ellipse_pcurve_report"]

V = App.Vector
RADIUS = 2.5
FLOOR = 3.0
CORNER = V(-5, 1, FLOOR)
CENTER = CORNER + V(RADIUS, RADIUS, RADIUS)
AXES = [V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)]
SPANS = (17.0, 23.0)
# Each straight fillet adds (1-pi/4)*r² per unit length. Their common
# material is counted twice: its volume is 2*integral(t*h(t), t=0..r)
# with h(t)=r-sqrt(r²-(r-t)²), the retained corner height.
VOLUME = (
    46 * 38 * 14
    - 17 * 23 * (12 - FLOOR)
    + sum(SPANS) * (1 - math.pi / 4) * RADIUS**2
    - (5 / 3 - math.pi / 2) * RADIUS**3
)


def reference():
    blank = Part.makeBox(46, 38, 14, V(-12, -7, -2))
    cutter = Part.makeBox(*SPANS, 15, CORNER)
    edges = []
    for axis, span in zip(AXES, SPANS):
        midpoint = CORNER + axis * (span / 2)
        (edge,) = [e for e in cutter.Edges if (e.CenterOfMass - midpoint).Length < 1e-8]
        edges.append(edge)
    return blank.cut(cutter.makeFillet(RADIUS, edges)).removeSplitter()


def check_material(solid):
    probes = [
        ((0, 12, 2.9), True),
        ((0, 12, 3.1), False),
        ((12.1, 10, 7), True),
        ((11.9, 10, 7), False),
        ((2, 24.1, 7), True),
        ((2, 23.9, 7), False),
    ]
    # Probe both isolated fillets, their common bisector, and each side of
    # that bisector. The material height is the higher of the two fillets.
    for x, y in [(1, 5), (6, 1), (1, 1), (0.65, 1.4), (1.4, 0.65)]:
        t = min(x, y)
        height = RADIUS - math.sqrt(RADIUS**2 - (RADIUS - t) ** 2)
        for offset in (-0.12, 0.12):
            probes.append(
                ((CORNER.x + x, CORNER.y + y, FLOOR + height + offset), offset < 0)
            )
    for xyz, material in probes:
        if solid.isInside(V(*xyz), 1e-7, False) != material:
            raise ValueError(f"Unexpected material at {xyz}")


def inspect_wall(solid, wall, seam, axis_index):
    axis = AXES[axis_index]
    side_normal = AXES[1 - axis_index]
    near(abs(wall.Surface.Axis.dot(axis)), 1, 1e-8)
    near(wall.Surface.Radius, RADIUS)
    near((wall.Surface.Center - CENTER).cross(axis).Length, 0)
    u0, u1, v0, v1 = wall.ParameterRange
    near(u1 - u0, math.pi / 2)
    near(v1 - v0, SPANS[axis_index])
    u, v = (u0 + u1) / 2, (v0 + v1) / 2
    d = wall.valueAt(u, v) - CENTER
    radial = d - axis * d.dot(axis)
    near(wall.normalAt(u, v).dot(radial) / radial.Length, -1.0)
    stops = []
    for e in wall.Edges:
        if e.isSame(seam):
            continue
        (neighbor,) = [
            f for f in solid.ancestorsOfType(e, Part.Face) if not f.isSame(wall)
        ]
        if isinstance(e.Curve, Part.Circle):
            stop = CORNER + axis * SPANS[axis_index]
            check_plane(neighbor, stop, axis)
            near(e.Curve.Radius, RADIUS)
            near(e.Length, RADIUS * math.pi / 2)
            stops.append("end")
        elif isinstance(e.Curve, Part.Line) and abs(e.CenterOfMass.z - FLOOR) < 2e-5:
            check_plane(neighbor, CORNER, AXES[2])
            near(e.Length, SPANS[axis_index] - RADIUS)
            stops.append("floor")
        elif isinstance(e.Curve, Part.Line):
            check_plane(neighbor, CORNER, side_normal)
            near(e.Length, SPANS[axis_index])
            stops.append("side")
        else:
            raise ValueError(f"Unexpected neighbor edge {e.Curve}")
    if sorted(stops) != ["end", "floor", "side"]:
        raise ValueError(f"Incorrect planar neighbors {stops}")
    return pcurve_report(seam, wall)


def inspect(path, require_analytic):
    s = Part.read(str(path))
    if len(s.Solids) != 1 or not s.isValid() or not s.isClosed():
        raise ValueError(f"{path} is not a single valid closed solid")
    b = s.BoundBox
    bounds = [b.XMin, b.YMin, b.ZMin, b.XMax, b.YMax, b.ZMax]
    for value, expected in zip(bounds, (-12, -7, -2, 34, 31, 12)):
        near(value, expected)
    check_material(s)
    faces = Counter(type(f.Surface).__name__ for f in s.Faces)
    result = dict(
        faces=faces,
        bounds_mm=bounds,
        volume_mm3=s.Volume,
        volume_error_mm3=s.Volume - VOLUME,
    )
    if faces != {"Plane": 11, "Cylinder": 2}:
        if require_analytic:
            raise ValueError(f"{path}: expected 11 planes and 2 cylinders, got {faces}")
        return result

    near(s.Volume, VOLUME, 3e-4)
    walls = sorted(
        (f for f in s.Faces if isinstance(f.Surface, Part.Cylinder)),
        key=lambda f: abs(f.Surface.Axis.y),
    )
    (edge,) = [a for a in walls[0].Edges if any(a.isSame(b) for b in walls[1].Edges)]
    owners = s.ancestorsOfType(edge, Part.Face)
    if len(owners) != 2 or not all(any(w.isSame(o) for o in owners) for w in walls):
        raise ValueError("The intersection edge must belong to both cylinders")
    curve = edge.Curve
    if not isinstance(curve, Part.Ellipse):
        raise ValueError(f"Expected an elliptic intersection, got {curve}")
    near(curve.MajorRadius, RADIUS * math.sqrt(2))
    near(curve.MinorRadius, RADIUS)
    near((curve.Center - CENTER).Length, 0)
    near(abs(curve.Axis.dot(V(1, -1, 0))) / math.sqrt(2), 1.0)
    near(edge.LastParameter - edge.FirstParameter, math.pi / 2)
    check_ends(edge, CENTER - AXES[2] * RADIUS, CORNER + AXES[2] * RADIUS)
    traces = [inspect_wall(s, wall, edge, i) for i, wall in enumerate(walls)]
    # Sampling at identical edge parameters is an independent geometric
    # observation, not an OCCT certification bound or an export tolerance.
    for trace in traces:
        near(trace["max_sampled_mm"], 0, tolerance=5e-6)
    result["seam"] = dict(
        major_mm=curve.MajorRadius,
        minor_mm=curve.MinorRadius,
        length_mm=edge.Length,
        edge_tolerance_mm=edge.getTolerance(1),
        samples_per_wall=257,
        traces=traces,
    )
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--require-analytic", action="store_true")
    args = parser.parse_args()
    if args.require_analytic and not args.cli:
        parser.error("--require-analytic requires --cli")
    args.output.mkdir(exist_ok=True, parents=True)
    source = reference()
    step = args.output / "reference.step"
    source.exportStep(str(step))
    report = dict(
        freecad=".".join(App.Version()[:3]),
        exact_volume_mm3=VOLUME,
        reference=inspect(step, True),
        cases={},
    )
    for tag, angular in [("coarse", 0.25), ("fine", 0.10)]:
        mesh = MeshPart.meshFromShape(
            Shape=source,
            LinearDeflection=0.025,
            AngularDeflection=angular,
            Relative=False,
        )
        stl = args.output / f"{tag}.stl"
        mesh.write(str(stl))
        case = dict(triangles=mesh.CountFacets)
        if args.cli:
            step = args.output / f"{tag}.step"
            with (args.output / f"{tag}-conversion.json").open("w") as log:
                subprocess.run(
                    [
                        str(args.cli.resolve()),
                        "convert",
                        str(stl),
                        "--unit",
                        "mm",
                        "--mode",
                        "auto",
                        "--force",
                        "--json",
                        "-o",
                        str(step),
                    ],
                    stdout=log,
                    check=True,
                )
            case.update(inspect(step, args.require_analytic))
        report["cases"][tag] = case
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
