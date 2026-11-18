"""Mesh a drilled plate in two poses and verify its exported STEP in FreeCAD."""

import argparse
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part


V = App.Vector
Z = V(0, 0, 1)
VOLUME = 24 * 20 * 6 - math.pi * 3**2 * 6
TOLERANCE = 2e-5  # mm for the independent STEP readback


def plate():
    block = Part.makeBox(24, 20, 6, V(-12, -10, -3))
    tool = Part.makeCylinder(3, 8, V(0, 0, -4))
    return block.cut(tool).removeSplitter()


def near(value, expected, tolerance, label):
    if not math.isfinite(value) or abs(value - expected) > tolerance:
        raise ValueError(f"{label}: expected {expected}, got {value}")


def inspect(path, pose, *, faceted=False):
    shape = Part.read(str(path))
    shape.Placement = pose.inverse().multiply(shape.Placement)
    if len(shape.Solids) != 1 or not shape.isValid() or not shape.Solids[0].isClosed():
        raise ValueError(f"{path}: expected one valid, closed solid")
    solid = shape.Solids[0]
    bounds = solid.optimalBoundingBox(False)
    for value, expected in zip(
        (bounds.XMin, bounds.YMin, bounds.ZMin, bounds.XMax, bounds.YMax, bounds.ZMax),
        (-12, -10, -3, 12, 10, 3),
    ):
        near(value, expected, TOLERANCE, "local bounds in mm")
    probes = [
        (V(x, y, z), material)
        for z in (-2.95, 0, 2.95)
        for x, y, material in [
            (0, 0, False),
            (2.9, 0, False),
            (3.1, 0, True),
            (0, -3.1, True),
            (8, 6, True),
        ]
    ]
    for point, material in probes:
        if solid.isInside(point, 1e-7, False) != material:
            raise ValueError(f"{path}: material mismatch at {point}")

    walls = [f for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)]
    planes = [f for f in solid.Faces if isinstance(f.Surface, Part.Plane)]
    result = {
        "faces": len(shape.Faces),
        "cylinders": len(walls),
        "planes": len(planes),
        "volume_mm3": solid.Volume,
        "probes": len(probes),
    }
    if faceted:
        if len(planes) != len(shape.Faces) or walls:
            raise ValueError(f"{path}: unproved analytical surface at tight tolerance")
        return result

    if (len(shape.Faces), len(planes), len(walls)) != (7, 6, 1):
        raise ValueError(f"{path}: expected six planes and one cylinder, got {result}")
    wall = walls[0].Surface
    near(wall.Center.cross(Z).Length, 0, TOLERANCE, "bore centerline in mm")
    near(wall.Axis.cross(Z).Length, 0, 1e-6, "bore axis")
    near(wall.Radius, 3, TOLERANCE, "radius in mm")
    near(solid.Volume, VOLUME, 0.001, "volume in mm³")
    # Each mouth shares a closed circle with one of the two end planes.
    mouths = {-1: [], 1: []}
    for edge in solid.Edges:
        owners = solid.ancestorsOfType(edge, Part.Face)
        if not any(f.isSame(walls[0]) for f in owners) or not any(
            isinstance(f.Surface, Part.Plane) for f in owners
        ):
            continue
        circle = edge.Curve
        if not isinstance(circle, Part.Circle) or len(owners) != 2:
            raise ValueError(f"{path}: invalid boundary of cylindrical wall")
        sign = 1 if circle.Center.z > 0 else -1
        near(
            (circle.Center - Z * (3 * sign)).Length, 0, TOLERANCE, "mouth center in mm"
        )
        near(circle.Radius, 3, TOLERANCE, "mouth radius in mm")
        mouths[sign].append(edge)
    for edges in mouths.values():
        wires = Part.sortEdges(edges) if edges else []
        if len(wires) != 1 or not Part.Wire(wires[0]).isClosed():
            raise ValueError(f"{path}: opening is incomplete")
        near(
            sum(e.Length for e in edges),
            6 * math.pi,
            2 * math.pi * TOLERANCE,
            "mouth circumference in mm",
        )
    result["radius_mm"] = wall.Radius
    result["openings"] = len(mouths)
    return result


def convert(cli, source, path, mode, *, tight=False):
    command = [
        str(cli.resolve()),
        "convert",
        str(source),
        "-o",
        str(path),
        "--mode",
        mode,
        "--unit",
        "mm",
        "--force",
        "--json",
    ]
    if tight:
        # The outermost chord of the coarse hole falls ~0.00373 mm inside the circle.
        command.extend(["--max-deviation", "0.001"])
    with path.with_suffix(".json").open("w") as stream:
        subprocess.run(command, stdout=stream, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--cli", type=Path, help="FabSTEP CLI to test on the generated STL files."
    )
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    x, z = V(1, 8, -4), V(4, -4, -7)
    poses = {
        "main": App.Placement(V(9, -5, 11), App.Rotation(x, z.cross(x), z, "ZXY")),
        "other": App.Placement(V(3, -2, 4), App.Rotation(V(1, 2, 3), 24)),
    }
    report = {
        "freecad_version": ".".join(App.Version()[:3]),
        "reference_volume_mm3": VOLUME,
    }
    for name, pose in poses.items():
        reference_path = args.output / f"{name}-reference.step"
        shape = plate()
        shape.Placement = pose
        shape.exportStep(str(reference_path))
        report[f"{name}-reference"] = inspect(reference_path, pose)
        for density, angle in [("coarse", 0.20), ("fine", 0.10)]:
            stem = f"{name}-{density}"
            source = args.output / f"{stem}.stl"
            mesh = MeshPart.meshFromShape(
                Shape=Part.read(str(reference_path)),
                LinearDeflection=0.025,
                AngularDeflection=angle,
                Relative=False,
            )
            mesh.write(str(source))
            entry = {"triangles": mesh.CountFacets}
            if args.cli:
                modes = ("auto", "cylinders") if stem == "main-coarse" else ("auto",)
                for mode in modes:
                    path = args.output / f"{stem}-{mode}.step"
                    convert(args.cli, source, path, mode)
                    entry[mode] = inspect(path, pose)
                if name == "main":
                    path = args.output / f"{stem}-tight.step"
                    convert(args.cli, source, path, "auto", tight=True)
                    entry["tight"] = inspect(path, pose, faceted=density == "coarse")
                    if density == "coarse":
                        points, triangles = mesh.Topology
                        source_volume = abs(
                            math.fsum(
                                points[a].dot(points[b].cross(points[c]))
                                for a, b, c in triangles
                            )
                            / 6
                        )
                        near(
                            entry["tight"]["volume_mm3"],
                            source_volume,
                            0.001,
                            "faceted STL volume in mm³",
                        )
            report[stem] = entry
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
