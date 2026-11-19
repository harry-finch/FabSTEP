"""Crossing, non-orthogonal bores in a skew block with four circular mouths."""

import argparse
from collections import Counter
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part

V = App.Vector
HX, HY, HZ = 16.0, 12.0, 15.0
R, BIG = 2.0, 5.0


def drawing(angle, with_through_bore=True):
    c, s = math.cos(angle), math.sin(angle)
    x, z = V(1, 0, 0), V(c, 0, s)
    # Planes x=±HX, y=±HY and c*x+s*z=±HZ. They meet the bores normally.
    base = [
        V(t, -HY, (v - c * t) / s)
        for t, v in [(-HX, -HZ), (HX, -HZ), (HX, HZ), (-HX, HZ)]
    ]
    block = Part.Face(Part.makePolygon(base + base[:1])).extrude(V(0, 2 * HY, 0))
    big = Part.makeCylinder(BIG, 2 * HX + 2, x * (-HX - 1), x)
    depth = 2 * HZ + 2 if with_through_bore else HZ + 1
    small = Part.makeCylinder(R, depth, z * (-HZ - 1), z)
    return block.cut(big).cut(small).removeSplitter()


def nominal_volume(angle):
    # Integrate the intersection along common-normal chords, y=r*sin(t).
    # The two independent transverse distances have Jacobian sin(angle).
    def intersection(n):
        def integrand(i):
            t = math.pi * i / (2 * n)
            return (
                8 * R**2 * math.cos(t) ** 2 * math.sqrt(BIG**2 - (R * math.sin(t)) ** 2)
            )

        return (
            math.pi
            / (6 * n)
            * (
                integrand(0)
                + integrand(n)
                + sum((4 if i % 2 else 2) * integrand(i) for i in range(1, n))
            )
        )

    v = intersection(512)
    assert abs(v - intersection(1024)) < 1e-9
    return (8 * HX * HY * HZ + v) / math.sin(angle) - 2 * math.pi * (
        HX * BIG**2 + HZ * R**2
    )


def inspect(shape, angle):
    assert len(shape.Solids) == 1 and shape.isClosed() and shape.isValid()
    faces = shape.Faces
    walls = [f for f in faces if isinstance(f.Surface, Part.Cylinder)]
    planar = [f for f in faces if isinstance(f.Surface, Part.Plane)]
    assert len(faces) == 9 and len(walls) == 3 and len(planar) == 6
    c, s = math.cos(angle), math.sin(angle)
    x, y, small = V(1, 0, 0), V(0, 1, 0), V(c, 0, s)
    u = V(s, 0, -c)
    mouths = [e for e in shape.Edges if isinstance(e.Curve, Part.Circle)]
    assert len(mouths) == 4
    gaps = {}
    for axis, r, span in [(x, BIG, HX), (small, R, HZ)]:
        for sign in [-1, 1]:
            (mouth,) = [
                e for e in mouths if (e.Curve.Center - sign * span * axis).Length < 2e-5
            ]
            assert (
                mouth.isClosed()
                and abs(mouth.Curve.Radius - r) < 2e-5
                and abs(abs(axis.dot(mouth.Curve.Axis)) - 1) < 1e-8
            )
            adjacent = shape.ancestorsOfType(mouth, Part.Face)
            assert (
                len(adjacent) == 2
                and sum(isinstance(f.Surface, Part.Plane) for f in adjacent) == 1
            )
    shared = [
        e
        for e in shape.Edges
        if len(owners := shape.ancestorsOfType(e, Part.Face)) == 2
        and all(isinstance(f.Surface, Part.Cylinder) for f in owners)
    ]
    assert shared
    for sign in [-1, 1]:
        edges = [
            e
            for e in shared
            if sign * e.valueAt((e.FirstParameter + e.LastParameter) / 2).z > 0
        ]
        assert edges and Part.Wire(Part.__sortEdges__(edges)).isClosed()
        # Compare the actual trim to points on the independently parametrized
        # ideal branch. Every phase has one point on each of the two branches.
        wire = Part.Compound(edges)
        dist = 0.0
        for i in range(96):
            t = 2 * math.pi * i / 96
            a, b = R * math.cos(t), R * math.sin(t)
            k = sign * math.sqrt(BIG**2 - b * b)
            p = u * a + y * b + small * ((k + c * a) / s)
            dist = max(dist, wire.distToShape(Part.Vertex(p))[0])
        assert dist < 1e-5, dist
        gaps[sign] = dist
    for axis, rad, length in [(x, BIG, HX), (small, R, HZ)]:
        for sign in [-1, 1]:
            center = axis * sign * (length - 0.5)
            assert not shape.isInside(center, 1e-7, False)
            assert not shape.isInside(center + y * (rad - 0.05), 1e-7, False)
            assert shape.isInside(center + y * (rad + 0.05), 1e-7, False)
    assert not shape.isInside(V(0, 0, 0), 1e-7, False)
    assert shape.isInside(V(0, BIG + 0.05, 0), 1e-7, False)
    assert abs(shape.Volume - nominal_volume(angle)) < 0.005, (
        shape.Volume,
        nominal_volume(angle),
    )
    return dict(
        faces=len(faces),
        volume_mm3=shape.Volume,
        ideal_mm3=nominal_volume(angle),
        junction_edges=len(shared),
        sampled_gap_mm=gaps,
    )


def missing_mouth(root, cli):
    angle = math.radians(60)
    shape = drawing(angle, with_through_bore=False)
    assert shape.isClosed() and shape.isValid()
    assert sum(isinstance(e.Curve, Part.Circle) for e in shape.Edges) == 3
    mesh = MeshPart.meshFromShape(
        Shape=shape, LinearDeflection=0.015, AngularDeflection=0.16, Relative=False
    )
    stl = root / "three-mouths.stl"
    mesh.write(str(stl))
    if cli:
        dest = stl.with_suffix(".step")
        command = [
            cli.resolve(),
            "convert",
            stl,
            "--mode",
            "auto",
            "--unit",
            "mm",
            "--force",
            "--json",
            "-o",
            dest,
        ]
        output = subprocess.run(command, capture_output=True, text=True, check=True)
        report = json.loads(output.stdout)
        rebuilt = Part.read(str(dest))
        assert rebuilt.isClosed() and rebuilt.isValid() and len(rebuilt.Solids) == 1
        assert (
            not (report.get("rounded_recognition") or {})
            .get("model", {})
            .get("cross_drill")
        )
        small_axis = V(math.cos(angle), 0, math.sin(angle))
        for direction in [-1, 1]:
            interior = small_axis * (direction * (HZ - 0.1))
            assert rebuilt.isInside(interior, 1e-7, False) == (direction > 0)
        print(
            json.dumps(
                dict(missing_mouth="retained material", triangles=mesh.CountFacets)
            ),
            flush=True,
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--require-analytic", action="store_true")
    args = parser.parse_args()
    root = args.output
    root.mkdir(parents=True, exist_ok=True)
    missing_mouth(root, args.cli)
    for degrees in [60, 75]:
        angle = math.radians(degrees)
        step = root / f"angle-{degrees}-design.step"
        drawing(angle).exportStep(str(step))
        ref = Part.read(str(step))
        print(json.dumps(dict(degrees=degrees, design=inspect(ref, angle))), flush=True)
        for label, deflection in [("coarse", 0.16), ("fine", 0.11)]:
            mesh = MeshPart.meshFromShape(
                Shape=ref,
                LinearDeflection=0.015,
                AngularDeflection=deflection,
                Relative=False,
            )
            stl = root / f"angle-{degrees}-{label}.stl"
            mesh.write(str(stl))
            if args.cli:
                dest = stl.with_suffix(".step")
                with dest.with_suffix(".json").open("w") as f:
                    rc = subprocess.run(
                        [
                            args.cli.resolve(),
                            "convert",
                            stl,
                            "--mode",
                            "auto",
                            "--unit",
                            "mm",
                            "--force",
                            "--json",
                            "-o",
                            dest,
                        ],
                        stdout=f,
                        check=False,
                    ).returncode
                if rc:
                    assert not args.require_analytic
                    print(
                        json.dumps(
                            dict(degrees=degrees, triangles=mesh.CountFacets, error=rc)
                        ),
                        flush=True,
                    )
                    continue
                result = Part.read(str(dest))
                assert result.isClosed() and result.isValid()
                types = Counter(f.Surface.TypeId for f in result.Faces)
                data = dict(
                    degrees=degrees, mesh=label, triangles=mesh.CountFacets, types=types
                )
                if args.require_analytic or "Part::GeomCylinder" in types:
                    data["analytic"] = inspect(result, angle)
                print(json.dumps(data), flush=True)


if __name__ == "__main__":
    main()
