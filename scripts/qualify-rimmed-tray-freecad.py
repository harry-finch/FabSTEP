"""Reopen an independent two-compartment tray and qualify the compact curved faces."""

import argparse
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part

V = App.Vector
# Outside of the base, outside of overhanging rim, and the two cavities.
# (width, depth, z origin, height, vertical corner radius, center x)
BASE = (58, 32, 0, 9, 4, 0)
RIM = (62, 36, 9, 3, 5.5, 0)
POCKETS = [(22, 24, 2, 10, 2, -13.5), (22, 24, 4, 8, 2.5, 13.5)]
BORE = (-12.5, 1.0, 1.6)


def rounded_box(spec):
    w, d, z, h, r, x = spec
    box = Part.makeBox(w, d, h, V(x - w / 2, -d / 2, z))
    rails = [
        e
        for e in box.Edges
        if abs(e.Vertexes[0].Point.z - e.Vertexes[1].Point.z) > h - 1e-5
    ]
    return box.makeFillet(r, rails)


def drawing():
    tray = rounded_box(BASE).fuse(rounded_box(RIM))
    for p in POCKETS:
        tray = tray.cut(rounded_box(p))
    x, y, r = BORE
    return tray.cut(Part.makeCylinder(r, 14, V(x, y, -1))).removeSplitter()


def inspect(path):
    tray = Part.read(str(path))
    assert len(tray.Solids) == 1 and tray.isClosed() and tray.isValid()
    walls = [f for f in tray.Faces if isinstance(f.Surface, Part.Cylinder)]
    planes = [f for f in tray.Faces if isinstance(f.Surface, Part.Plane)]
    assert len(walls) == 17 and len(planes) + len(walls) == len(tray.Faces), (
        len(tray.Faces),
        len(walls),
        len(planes),
    )

    def volume(spec):
        w, d, _, h, r, _ = spec
        return (w * d - (4 - math.pi) * r * r) * h

    x, y, r = BORE
    expected = (
        volume(BASE)
        + volume(RIM)
        - sum(volume(p) for p in POCKETS)
        - 2 * math.pi * r * r
    )
    assert abs(tray.Volume - expected) < 0.001, (tray.Volume, expected)

    def has_matter(p):
        return tray.isInside(V(*p), 1e-7, False)

    def check_wall(center, r, z0, z1):
        (wall,) = [
            f
            for f in walls
            if abs(f.Surface.Radius - r) < 2e-5
            and abs(abs(f.Surface.Axis.z) - 1.0) < 1e-8
            and math.hypot(
                f.Surface.Center.x - center[0], f.Surface.Center.y - center[1]
            )
            < 2e-5
        ]
        arcs = [e for e in wall.Edges if isinstance(e.Curve, Part.Circle)]
        assert len(arcs) == 2
        assert all(len(tray.ancestorsOfType(e, Part.Face)) == 2 for e in arcs)
        for e, z in zip(sorted(arcs, key=lambda e: e.Curve.Center.z), [z0, z1]):
            assert abs(e.Curve.Center.z - z) < 2e-5
        return wall

    for spec in (BASE, RIM, *POCKETS):
        w, d, z, h, rad, cx = spec
        for sx in [-1, 1]:
            for sy in [-1, 1]:
                wall = check_wall(
                    (cx + sx * (w / 2 - rad), sy * (d / 2 - rad)), rad, z, z + h
                )
                assert abs(wall.Area - math.pi * rad * h / 2) < 0.001
    bore = check_wall((x, y), r, 0, 2)
    assert abs(bore.Area - 4 * math.pi * r) < 0.001
    for cx, level in [(-13.5, 2), (13.5, 4)]:
        assert has_matter((cx, 6, level - 0.02)) and not has_matter(
            (cx, 6, level + 0.02)
        )
    for z in [1, 5, 11.9]:
        assert has_matter((0, 0, z)) and not has_matter((x, y, z))
    assert has_matter((x + r + 0.1, y, 1))
    for sx in [-1, 1]:
        for sy in [-1, 1]:
            assert not has_matter((sx * 30.0, sy * 5.0, 8.98)) and has_matter(
                (sx * 30.0, sy * 5.0, 9.02)
            )
            assert not has_matter((sx * 5.0, sy * 17.0, 8.98)) and has_matter(
                (sx * 5.0, sy * 17.0, 9.02)
            )
    return dict(
        faces=len(tray.Faces),
        planes=len(planes),
        cylinders=len(walls),
        volume_mm3=tray.Volume,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--cli", type=Path)
    args = parser.parse_args()
    root = args.directory
    root.mkdir(parents=True, exist_ok=True)
    reference = root / "design.step"
    drawing().exportStep(str(reference))
    report = {"design": inspect(reference)}
    for label, angle in [("coarse", 0.22), ("fine", 0.13)]:
        mesh = MeshPart.meshFromShape(
            Shape=Part.read(str(reference)),
            LinearDeflection=0.03,
            AngularDeflection=angle,
            Relative=False,
        )
        stl = root / (label + ".stl")
        mesh.write(str(stl))
        if args.cli:
            out = stl.with_suffix(".step")
            with out.with_suffix(".json").open("w") as f:
                subprocess.run(
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
                        out,
                    ],
                    stdout=f,
                    check=True,
                )
            report[label] = dict(triangles=mesh.CountFacets, **inspect(out))
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
