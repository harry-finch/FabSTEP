"""Reimport widened cross-drill ratios in FreeCAD and check their two communicating bores.

The reference solids are built with two Boolean cuts. With --cli, also run FabSTEP
on each public STL and inspect its exported STEP through the same geometric tests.
"""

import argparse
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part


CASES = ("f_ratio", "f_ratio_mid", "f_ratio_near")
FIXTURES = Path(__file__).resolve().parents[1] / "fixtures" / "cross_drill"
V = App.Vector


def near(measured, expected, tolerance, label):
    if not math.isfinite(measured) or abs(measured - expected) > tolerance:
        raise ValueError(f"{label}: expected {expected}, got {measured}")


def reference(case, *, local=False, subdivided=False):
    p = case["parameters"]
    lx, ly, lz = p["dimensions_mm"]
    r, big = p["small_radius_mm"], p["large_radius_mm"]
    block = Part.makeBox(lx, ly, lz, V(-lx / 2, -ly / 2, -lz / 2))

    def drill(axis, stops, radius):
        segments = [
            Part.makeCylinder(radius, end - start, axis * start, axis)
            for start, end in zip(stops, stops[1:])
        ]
        return segments[0].fuse(segments[1:]) if len(segments) > 1 else segments[0]

    # Optional axial subdivisions preserve the solid and insert circles which
    # OCCT's independent mesher samples in the interior of the cylindrical faces.
    x_stops = [-lx / 2 - 1, *([-6, 6] if subdivided else []), lx / 2 + 1]
    z_stops = [-lz / 2 - 1, *([-7, 7] if subdivided else []), lz / 2 + 1]
    shape = block.cut(drill(V(0, 0, 1), z_stops, r)).cut(
        drill(V(1, 0, 0), x_stops, big)
    )
    if not subdivided:
        shape = shape.removeSplitter()
    if local:
        return shape
    x, z = V(*p["large_axis"]), V(*p["small_axis"])
    # The centered block and bores are symmetric about their local XZ plane.
    shape.Placement = App.Placement(
        V(*p["center_mm"]), App.Rotation(x, z.cross(x), z, "ZXY")
    )
    return shape


def inspect(path, case, *, local=False, boolean=False, to_local=None):
    p = case["parameters"]
    shape = Part.read(str(path))
    if to_local is not None:
        shape.Placement = to_local.multiply(shape.Placement)
    if len(shape.Solids) != 1 or len(shape.Faces) != 9:
        raise ValueError(f"{path}: expected a single nine-face solid")
    solid = shape.Solids[0]
    if not solid.isValid() or not solid.isClosed():
        raise ValueError(f"{path}: invalid solid")
    r, big = p["small_radius_mm"], p["large_radius_mm"]
    x, z, origin = V(*p["large_axis"]), V(*p["small_axis"]), V(*p["center_mm"])
    probe_frame = [x, z.cross(x), z]
    probe_origin = origin
    if local:
        x, z, origin = V(1, 0, 0), V(0, 0, 1), V(0, 0, 0)
    walls = sorted(
        [f.Surface for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)],
        key=lambda surface: surface.Radius,
    )
    if (
        len(walls) != 3
        or sum(isinstance(f.Surface, Part.Plane) for f in solid.Faces) != 6
    ):
        raise ValueError(f"{path}: unexpected supports")
    for wall, radius, axis in zip(walls, (r, r, big), (z, z, x)):
        near(wall.Radius, radius, 2e-5, "bore radius in mm")
        near(abs(wall.Axis.dot(axis)), 1, 1e-8, "bore direction")
        near(
            (wall.Center - origin).cross(axis).Length, 0, 2e-5, "bore centerline in mm"
        )
    bounds = solid.optimalBoundingBox(False)
    actual_bounds = [
        [bounds.XMin, bounds.YMin, bounds.ZMin],
        [bounds.XMax, bounds.YMax, bounds.ZMax],
    ]
    expected_bounds = (
        [[sign * d / 2 for d in p["dimensions_mm"]] for sign in (-1, 1)]
        if local
        else case["analytic_bounds_mm"]
    )
    for actual, expected in zip(actual_bounds, expected_bounds):
        for a, b in zip(actual, expected):
            near(a, b, 2e-5, "world bounds in mm")
    # FreeCAD's default BRep quadrature is less accurate here than the
    # independently integrated sections and FabSTEP's adaptive OCCT volume.
    near(
        solid.Volume,
        p["volume_mm3"],
        p["volume_mm3"] * (2e-6 if boolean else 1e-7),
        "FreeCAD volume in mm3",
    )
    probe_count = 0
    for probe in case["material_probes"]:
        point = V(*probe["point_mm"])
        if local:
            point = V(
                *[(point - probe_origin).dot(direction) for direction in probe_frame]
            )
        if solid.isInside(point, 1e-7, False) != probe["material"]:
            raise ValueError(f"{path}: material mismatch at {probe}")
        probe_count += 1

    branches = {-1: [], 1: []}
    max_residual = 0.0
    for edge in solid.Edges:
        owners = solid.ancestorsOfType(edge, Part.Face)
        if len(owners) != 2 or not all(
            isinstance(f.Surface, Part.Cylinder) for f in owners
        ):
            continue
        near(min(f.Surface.Radius for f in owners), r, 2e-5, "small owner")
        near(max(f.Surface.Radius for f in owners), big, 2e-5, "large owner")
        if not isinstance(edge.Curve, Part.BSplineCurve):
            raise ValueError(f"{path}: unexpected seam curve")
        sign = 1 if (edge.CenterOfMass - origin).dot(z) > 0 else -1
        branches[sign].append(edge)
        for i in range(65):
            u = (
                edge.FirstParameter
                + (edge.LastParameter - edge.FirstParameter) * i / 64
            )
            q = edge.valueAt(u) - origin
            if q.dot(z) * sign <= 0:
                raise ValueError(f"{path}: seam crosses the middle of the block")
            max_residual = max(
                max_residual, abs(q.cross(z).Length - r), abs(q.cross(x).Length - big)
            )
    for edges in branches.values():
        wires = [Part.Wire(curves) for curves in Part.sortEdges(edges)] if edges else []
        if len(wires) != 1 or not wires[0].isClosed():
            raise ValueError(f"{path}: expected a closed shared boundary on each side")
    near(max_residual, 0, 2e-5 if boolean else 1e-6, "sampled seam residual in mm")
    return dict(
        volume_mm3=solid.Volume,
        reference_volume_mm3=p["volume_mm3"],
        bounds_mm=actual_bounds,
        radii_mm=[w.Radius for w in walls],
        material_probes=probe_count,
        shared_edges_per_branch=[len(edges) for edges in branches.values()],
        max_sampled_seam_residual_mm=max_residual,
    )


def convert(cli, source, output):
    with output.with_suffix(".json").open("w") as stream:
        subprocess.run(
            [
                str(cli.resolve()),
                "convert",
                str(source),
                "-o",
                str(output),
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


def independent_meshes(directory, cli, case):
    results = {}
    second_pose = App.Placement(V(3, -2, 4), App.Rotation(V(1, 2, 3), 24))
    for variant, local, subdivided, pose in [
        ("plain", True, False, None),
        ("subdivided", True, True, None),
        ("rotated", False, False, None),
        ("rotated_24", True, False, second_pose),
    ]:
        reference_path = directory / f"freecad_{variant}.step"
        shape = reference(case, local=local, subdivided=subdivided)
        if pose is not None:
            shape.Placement = pose.multiply(shape.Placement)
        shape.exportStep(str(reference_path))
        # Mesh the reread CAD file, independently from the analytical STL generator.
        for name, angle in [("coarse", 0.20), ("fine", 0.10)]:
            stem = f"freecad_{variant}_{name}"
            source = directory / f"{stem}.stl"
            mesh = MeshPart.meshFromShape(
                Shape=Part.read(str(reference_path)),
                LinearDeflection=0.025,
                AngularDeflection=angle,
                Relative=False,
            )
            mesh.write(str(source))
            entry = {"triangles": mesh.CountFacets}
            if cli:
                output = directory / f"{stem}-auto.step"
                convert(cli, source, output)
                to_local = pose.inverse() if pose is not None else None
                entry.update(inspect(output, case, local=local, to_local=to_local))
                conversion = json.loads(output.with_suffix(".json").read_text())
                entry["deviation_bound_mm"] = conversion["rounded_recognition"][
                    "report"
                ]["max_deviation_bound_mm"]
            results[stem] = entry
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path)
    parser.add_argument(
        "--mesh",
        action="store_true",
        help="Also generate and check FreeCAD tessellations.",
    )
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((FIXTURES / "manifest.json").read_text())["cases"]
    report = {"freecad_version": ".".join(App.Version()[:3])}
    for name in CASES:
        case = manifest[name]
        path = args.output / f"{name}-reference.step"
        reference(case).exportStep(str(path))
        report[name] = {"boolean": inspect(path, case, boolean=True)}
        if args.cli:
            path = args.output / f"{name}-auto.step"
            convert(args.cli, FIXTURES / f"{name}.stl", path)
            report[name]["fabstep"] = inspect(path, case)
    if args.mesh:
        report["independent_meshes"] = independent_meshes(
            args.output, args.cli, manifest["f_ratio_mid"]
        )
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
