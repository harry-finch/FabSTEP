"""Qualify adjacent concave fillets with unequal radii on an independent FreeCAD solid.

Pass the output directory and optionally --cli PATH. --require-analytic
requires the two cylindrical faces and their shared nonplanar edge in both
STL conversions. The STL meshes are made from the reread reference STEP.
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


helpers = runpy.run_path(
    str(Path(__file__).with_name("validate-oblique-pocket-freecad.py"))
)
near = helpers["near"]
check_ends = helpers["check_ends"]
check_plane = helpers["check_plane"]
pcurve_report = helpers["ellipse_pcurve_report"]

V = App.Vector
CORNER = V(-5, 1, 3)
X, Y, UP = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
SMALL, LARGE = 1.75, 3.25
LENGTH, WIDTH, DEPTH = 17.0, 23.0, 9.0
BOUNDS = (-12, -7, -2, 34, 31, 12)


def offset(radius, height):
    """Horizontal inset at height h on the inward quarter-circle."""
    return (radius - height) ** 2 / (radius + math.sqrt(height * (2 * radius - height)))


def seam_point(h):
    return CORNER + V(offset(LARGE, h), offset(SMALL, h), h)


def volume(intervals):
    # Twice-counted fillet material at height h is a rectangle with one
    # inset for each side. h = s(1-cos(t)) regularizes the square roots at 0.
    def integrand(i):
        t = math.pi * i / (2 * intervals)
        h = 2 * SMALL * math.sin(t / 2) ** 2
        return offset(SMALL, h) * offset(LARGE, h) * SMALL * math.sin(t)

    weights = (integrand(i) * (4 if i % 2 else 2) for i in range(1, intervals))
    overlap = (
        math.pi
        / (6 * intervals)
        * (integrand(0) + integrand(intervals) + math.fsum(weights))
    )
    return (
        46 * 38 * 14
        - LENGTH * WIDTH * DEPTH
        + (1 - math.pi / 4) * (LENGTH * SMALL**2 + WIDTH * LARGE**2)
        - overlap
    )


def source_shape():
    blank = Part.makeBox(46, 38, 14, V(*BOUNDS[:3]))
    cutter = Part.makeBox(LENGTH, WIDTH, 15, CORNER)
    cuts = []
    for radius, axis, span in [(SMALL, X, LENGTH), (LARGE, Y, WIDTH)]:
        mid = CORNER + axis * (span / 2)
        (edge,) = [e for e in cutter.Edges if (e.CenterOfMass - mid).Length < 1e-8]
        cuts.append(cutter.makeFillet(radius, [edge]))
    return blank.cut(cuts[0].common(cuts[1])).removeSplitter()


def check_material(solid):
    probes = [
        (V(0, 12, 2.88), True),
        (V(0, 12, 3.12), False),
        (V(12.12, 10, 7), True),
        (V(11.88, 10, 7), False),
        (V(2, 24.12, 7), True),
        (V(2, 23.88, 7), False),
    ]
    # Either cutter wall retains material. Only their common void is removed.
    for h in (0.27, 0.91, 1.53, 2.4):
        qx = offset(LARGE, h)
        qy = offset(SMALL, h) if h < SMALL else 0.0
        for dx, dy, material in [
            (-0.12, 0.12, True),
            (0.12, -0.12, True),
            (0.12, 0.12, False),
        ]:
            probes.append((CORNER + V(qx + dx, qy + dy, h), material))
    for p, expected in probes:
        if solid.isInside(p, 1e-7, False) != expected:
            raise ValueError(f"Wrong material at {p}")
    return len(probes)


def inspect_wall(solid, wall, seam, small):
    radius, axis, normal, span = (
        (SMALL, X, Y, LENGTH) if small else (LARGE, Y, X, WIDTH)
    )
    center = CORNER + (normal + UP) * radius
    surface = wall.Surface
    near(surface.Radius, radius)
    near(abs(surface.Axis.dot(axis)), 1)
    near((surface.Center - center).cross(axis).Length, 0)
    u0, u1, v0, v1 = wall.ParameterRange
    near(u1 - u0, math.pi / 2)
    u, v = (u0 + u1) / 2, (v0 + v1) / 2
    radial = wall.valueAt(u, v) - center
    radial -= axis * radial.dot(axis)
    near(wall.normalAt(u, v).dot(radial) / radial.Length, -1)
    seen = []
    for e in wall.Edges:
        if e.isSame(seam):
            continue
        (neighbor,) = [
            f for f in solid.ancestorsOfType(e, Part.Face) if not f.isSame(wall)
        ]
        if isinstance(e.Curve, Part.Line):
            if abs(e.CenterOfMass.z - CORNER.z) < 1e-5:
                check_plane(neighbor, CORNER, UP)
                near(e.Length, span - (LARGE if small else SMALL))
                seen.append("floor")
            else:
                check_plane(neighbor, CORNER, normal)
                near(e.Length, span - (offset(LARGE, SMALL) if small else 0.0))
                seen.append("side")
        elif isinstance(e.Curve, Part.Circle):
            near(e.Curve.Radius, radius)
            if abs((e.CenterOfMass - CORNER).dot(axis) - span) < 1e-5:
                check_plane(neighbor, CORNER + axis * span, axis)
                near((e.Curve.Center - center - axis * span).Length, 0)
                near(e.Length, radius * math.pi / 2)
                seen.append("end")
            elif not small:
                check_plane(neighbor, CORNER, axis)
                check_ends(e, seam_point(SMALL), CORNER + UP * LARGE)
                near(e.Length, LARGE * math.asin(1 - SMALL / LARGE))
                seen.append("upper_arc")
            else:
                raise ValueError("Unexpected circular trim on the smaller fillet")
        else:
            raise ValueError(f"Unexpected edge on the fillet: {e.Curve}")
    if Counter(seen) != Counter(
        ["floor", "side", "end"] + ([] if small else ["upper_arc"])
    ):
        raise ValueError(f"Incorrect fillet boundaries: {seen}")
    trace = pcurve_report(seam, wall)
    near(trace["max_sampled_mm"], 0, 1e-5)
    return dict(radius_mm=radius, edges=len(wall.Edges), trace=trace)


def inspect(path, require_analytic, *, exact_transfer=False):
    solid = Part.read(str(path))
    if len(solid.Solids) != 1 or not solid.isValid() or not solid.isClosed():
        raise ValueError(f"{path} is not a single valid closed solid")
    b = solid.BoundBox
    bounds = (b.XMin, b.YMin, b.ZMin, b.XMax, b.YMax, b.ZMax)
    for a, expected in zip(bounds, BOUNDS):
        near(a, expected)
    families = Counter(type(f.Surface).__name__ for f in solid.Faces)
    report = dict(
        faces=families,
        volume_mm3=solid.Volume,
        bounds_mm=bounds,
        probes=check_material(solid),
    )
    if families != {"Plane": 11, "Cylinder": 2}:
        if require_analytic:
            raise ValueError(
                f"{path}: expected 11 planes and 2 cylinders, got {families}"
            )
        return report

    near(solid.Volume, volume(1024), 3e-4)
    small, large = sorted(
        (f for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)),
        key=lambda f: f.Surface.Radius,
    )
    (seam,) = [e for e in small.Edges if any(e.isSame(other) for other in large.Edges)]
    owners = solid.ancestorsOfType(seam, Part.Face)
    if len(owners) != 2 or not all(
        any(w.isSame(o) for o in owners) for w in [small, large]
    ):
        raise ValueError("The two cylinders must share the same edge")
    check_ends(seam, seam_point(0), seam_point(SMALL))
    # The nonplanar intersection is checked geometrically, independently of
    # the particular spline degree or parameterization used during transfer.
    points = [seam_point(SMALL * i / 32) for i in range(33)]
    max_distance = max(seam.distToShape(Part.Vertex(p))[0] for p in points)
    near(max_distance, 0, 1e-5)
    p0, p1, p2 = (points[i] for i in (0, 16, 32))
    n = (p1 - p0).cross(p2 - p0)
    n.normalize()
    off_plane = max(abs((p - p0).dot(n)) for p in points)
    if off_plane < 0.01:
        raise ValueError("Non-planarity of this reference seam was not resolved")
    heights = []
    radial_residual = 0.0
    for i in range(257):
        t = seam.FirstParameter + (seam.LastParameter - seam.FirstParameter) * i / 256
        dx, dy, h = seam.valueAt(t) - CORNER
        if not (-1e-5 <= dx <= LARGE + 1e-5 and -1e-5 <= dy <= SMALL + 1e-5):
            raise ValueError("Seam leaves the inward cylinder branches")
        heights.append(h)
        for distance, radius in ((dx, LARGE), (dy, SMALL)):
            residual = abs(math.hypot(radius - distance, radius - h) - radius)
            radial_residual = max(radial_residual, residual)
    if heights[0] > heights[-1]:
        heights.reverse()
    if any(b <= a for a, b in zip(heights, heights[1:])):
        raise ValueError("Seam doubles back between the floor and smaller tangent rail")
    near(radial_residual, 0, 1e-5)
    report["seam"] = dict(
        curve=type(seam.Curve).__name__,
        length_mm=seam.Length,
        max_locus_distance_mm=max_distance,
        max_sampled_cylinder_residual_mm=radial_residual,
        max_locus_off_three_point_plane_mm=off_plane,
        edge_tolerance_mm=seam.getTolerance(1),
        pcurve_samples_per_face=257,
        walls=[
            inspect_wall(solid, small, seam, True),
            inspect_wall(solid, large, seam, False),
        ],
    )
    if exact_transfer:
        curve = seam.Curve
        if not (
            isinstance(curve, Part.BSplineCurve)
            and curve.isRational()
            and curve.Degree == 4
            and curve.NbPoles == 5
        ):
            raise ValueError(
                "Expected the rational quartic on the reconstructed cylinders"
            )
        for wall in report["seam"]["walls"]:
            if wall["trace"]["max_sampled_mm"] > 1e-7:
                raise ValueError(
                    "A reconstructed seam trace exceeds the native tolerance"
                )
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--require-analytic", action="store_true")
    args = parser.parse_args()
    if args.require_analytic and not args.cli:
        parser.error("--require-analytic requires --cli")
    args.output.mkdir(parents=True, exist_ok=True)
    reference = args.output / "reference.step"
    source_shape().exportStep(str(reference))
    near(volume(512), volume(1024), 1e-9)
    report = dict(
        freecad=".".join(App.Version()[:3]),
        integrated_volume_mm3=volume(1024),
        reference=inspect(reference, True),
        cases={},
    )
    source = Part.read(str(reference))
    for name, angular in [("coarse", 0.25), ("fine", 0.10)]:
        mesh = MeshPart.meshFromShape(
            Shape=source,
            LinearDeflection=0.025,
            AngularDeflection=angular,
            Relative=False,
        )
        stl = args.output / f"{name}.stl"
        mesh.write(str(stl))
        case = dict(triangles=mesh.CountFacets)
        if args.cli:
            step = args.output / f"{name}.step"
            with (args.output / f"{name}-conversion.json").open("w") as stream:
                subprocess.run(
                    [
                        str(args.cli.resolve()),
                        "convert",
                        str(stl),
                        "-o",
                        str(step),
                        "--mode",
                        "auto",
                        "--unit",
                        "mm",
                        "--force",
                        "--json",
                    ],
                    stdout=stream,
                    check=True,
                )
            case.update(
                inspect(
                    step, args.require_analytic, exact_transfer=args.require_analytic
                )
            )
        report["cases"][name] = case
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
