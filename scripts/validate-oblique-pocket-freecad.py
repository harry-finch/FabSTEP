"""Qualify a blind pocket whose cylindrical fillet ends on an oblique plane.

Run with FreeCAD's Python. With no CLI this generates the reference STEP and
two STL files. --cli also converts both meshes in Auto and checks their STEP.
--baseline records the type of each face while requiring a closed solid and the
reference dimensions and material probes, so the original fallback can be measured.
"""

import argparse
from collections import Counter
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part


RADIUS = 2.5
SLOPE = 0.75
CX, CZ = 7.5, 6.5
NEAR, FAR = 6.0, 29.0
V = App.Vector
GEOMETRY_TOL = 2e-5


def stop_y(x):
    return FAR + SLOPE * (x - CX)


def reference_volume():
    # Width times mean length gives the trapezoidal opening. Across the
    # retained fillet, integrate (FAR - NEAR + SLOPE * s) against the height
    # R - sqrt(R^2 - s^2), where 0 <= s = x - CX <= R.
    opening_area = 14 * ((stop_y(-4) + stop_y(10)) / 2 - NEAR)
    fillet_area = RADIUS**2 * (1 - math.pi / 4)
    fillet_moment = RADIUS**3 / 6
    return (
        40 * 50 * 11
        - 7 * opening_area
        + (FAR - NEAR) * fillet_area
        + SLOPE * fillet_moment
    )


def build_reference():
    blank = Part.makeBox(40, 50, 11, V(-12, -5, 0))
    cutter = Part.makeBox(14, 34, 14, V(-4, NEAR, 4))
    (edge,) = [e for e in cutter.Edges if (e.CenterOfMass - V(10, 23, 4)).Length < 1e-8]
    rounded = cutter.makeFillet(RADIUS, [edge])
    corners = [
        V(-15, -8, -2),
        V(20, -8, -2),
        V(20, stop_y(20), -2),
        V(-15, stop_y(-15), -2),
    ]
    limiter = Part.Face(Part.makePolygon(corners + [corners[0]])).extrude(V(0, 0, 25))
    return blank.cut(rounded.common(limiter)).removeSplitter()


def near(value, expected, tolerance=GEOMETRY_TOL):
    if not math.isclose(value, expected, abs_tol=tolerance, rel_tol=0.0):
        raise ValueError(
            f"{value} differs from expected {expected} by more than {tolerance}"
        )


def check_plane(face, point, normal):
    if not isinstance(face.Surface, Part.Plane):
        raise ValueError("Fillet neighbor must be planar")
    near(abs(face.Surface.Axis.dot(normal)), 1.0, 1e-8)
    near((face.CenterOfMass - point).dot(normal), 0.0)


def check_ends(edge, p1, p2):
    a, b = [vertex.Point for vertex in edge.Vertexes]
    near(
        min(
            max((a - p1).Length, (b - p2).Length), max((a - p2).Length, (b - p1).Length)
        ),
        0.0,
    )


def ellipse_pcurve_report(edge, face):
    pcurve, first, last = face.curveOnSurface(edge)
    # Compare at the same edge parameter, after checking that the two domains
    # coincide. A range remapping would hide a correspondence error.
    near(first, edge.FirstParameter, 1e-8)
    near(last, edge.LastParameter, 1e-8)
    deviations = []
    for i in range(257):
        t = first + (last - first) * i / 256
        uv = pcurve.value(t)
        deviations.append((face.valueAt(uv.x, uv.y) - edge.valueAt(t)).Length)
    if any(not math.isfinite(d) for d in deviations):
        raise ValueError("The imported ellipse has a nonfinite position")
    return dict(face_tolerance_mm=face.getTolerance(1), max_sampled_mm=max(deviations))


def inspect(path, analytic):
    solid = Part.read(str(path))
    if len(solid.Solids) != 1 or not solid.isClosed() or not solid.isValid():
        raise ValueError(f"{path}: expected one valid closed solid")
    bounds = solid.BoundBox
    for actual, expected in zip(
        (bounds.XMin, bounds.YMin, bounds.ZMin, bounds.XMax, bounds.YMax, bounds.ZMax),
        (-12, -5, 0, 28, 45, 11),
    ):
        near(actual, expected)
    for point, material in [
        ((9.9, 18, 4.1), True),
        ((9, 18, 6), False),
        ((9, 5.9, 6), True),
        ((9, 6.1, 6), False),
        ((9, stop_y(9) + 0.1, 6), True),
        ((9, stop_y(9) - 0.1, 6), False),
        ((0, 18, 4.1), False),
        ((0, 18, 3.9), True),
    ]:
        if solid.isInside(V(*point), 1e-6, False) != material:
            raise ValueError(f"{path}: unexpected material at {point}")
    faces = Counter(type(f.Surface).__name__ for f in solid.Faces)
    report = dict(faces=dict(faces), volume_mm3=solid.Volume)
    if not analytic:
        return report

    near(solid.Volume, reference_volume(), 0.0003)
    if faces != {"Plane": 11, "Cylinder": 1}:
        raise ValueError(f"{path}: unexpected faces {faces}")
    (wall,) = [f for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)]
    near(wall.Surface.Radius, RADIUS)
    near(abs(wall.Surface.Axis.y), 1.0, 1e-8)
    if wall.Orientation != "Reversed":
        raise ValueError(f"{path}: expected a concave wall")
    near(wall.ParameterRange[1] - wall.ParameterRange[0], math.pi / 2)
    seen = []
    for edge in wall.Edges:
        (neighbor,) = [
            f for f in solid.ancestorsOfType(edge, Part.Face) if not f.isSame(wall)
        ]
        curve = edge.Curve
        if isinstance(curve, Part.Ellipse):
            check_plane(neighbor, V(CX, FAR, CZ), V(0.6, -0.8, 0))
            check_ends(edge, V(CX, FAR, 4), V(10, stop_y(10), CZ))
            near((curve.Center - V(CX, FAR, CZ)).Length, 0)
            near(curve.MajorRadius, RADIUS * math.hypot(1, SLOPE))
            near(curve.MinorRadius, RADIUS)
            near(abs(curve.XAxis.dot(V(0.8, 0.6, 0))), 1, 1e-8)
            near(edge.LastParameter - edge.FirstParameter, math.pi / 2)
            report.update(
                ellipse_major_mm=curve.MajorRadius,
                ellipse_minor_mm=curve.MinorRadius,
                ellipse_edge_tolerance_mm=edge.getTolerance(1),
                ellipse_pcurves=dict(
                    samples_per_face=257,
                    cylinder=ellipse_pcurve_report(edge, wall),
                    stop=ellipse_pcurve_report(edge, neighbor),
                ),
            )
            seen.append("oblique")
        elif isinstance(curve, Part.Circle):
            check_plane(neighbor, V(CX, NEAR, CZ), V(0, 1, 0))
            check_ends(edge, V(CX, NEAR, 4), V(10, NEAR, CZ))
            near(curve.Radius, RADIUS)
            near((curve.Center - V(CX, NEAR, CZ)).Length, 0)
            near(edge.LastParameter - edge.FirstParameter, math.pi / 2)
            seen.append("straight")
        elif isinstance(curve, Part.Line):
            if abs(edge.CenterOfMass.x - 10) < GEOMETRY_TOL:
                check_plane(neighbor, V(10, NEAR, CZ), V(1, 0, 0))
                check_ends(edge, V(10, NEAR, CZ), V(10, stop_y(10), CZ))
                seen.append("side")
            else:
                check_plane(neighbor, V(CX, NEAR, 4), V(0, 0, 1))
                check_ends(edge, V(CX, NEAR, 4), V(CX, FAR, 4))
                seen.append("floor")
        else:
            raise ValueError(f"{path}: unexpected wall edge {curve}")
    if sorted(seen) != ["floor", "oblique", "side", "straight"]:
        raise ValueError(f"{path}: incorrect fillet boundaries {seen}")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--baseline", action="store_true")
    args = parser.parse_args()
    if args.baseline and not args.cli:
        parser.error("--baseline requires --cli")
    args.output.mkdir(exist_ok=True, parents=True)
    source = build_reference()
    reference = args.output / "reference.step"
    source.exportStep(str(reference))
    report = dict(
        freecad=".".join(App.Version()[:3]),
        exact_volume_mm3=reference_volume(),
        reference=inspect(reference, analytic=True),
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
            case.update(inspect(step, analytic=not args.baseline))
        report["cases"][tag] = case
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
