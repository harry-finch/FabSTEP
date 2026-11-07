"""Measure rounded-enclosure STEP geometry and tangencies with FreeCAD."""

import argparse
from collections import Counter
from itertools import product
import json
import math
from pathlib import Path
import sys

import FreeCAD as App
import Part


FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"
sys.path.insert(0, str(FIXTURES))
from complex_v1.freecad_corpus import tangent_edges
from generate_m6a import box_volume

CASES = json.loads((FIXTURES / "m6a_manifest.json").read_text())["cases"]
TANGENCIES = {"Cylinder/Plane": 24, "Cylinder/Sphere": 24}
LENGTH_TOLERANCE = 2e-6


def vector(point):
    return [point.x, point.y, point.z]


def close(actual, expected):
    return abs(actual - expected) <= LENGTH_TOLERANCE


def bounds(shape):
    box = shape.optimalBoundingBox(False, False)
    return [[box.XMin, box.YMin, box.ZMin], [box.XMax, box.YMax, box.ZMax]]


def quarter(span):
    return math.isclose(span, math.pi / 2, rel_tol=1e-8, abs_tol=1e-8)


def measure(body, ref):
    axes = ref["axes"]
    half = ref["core_half_extents"]
    origin = ref["center_mm"]
    radius = ref["radius_mm"]

    def local(point):
        q = [a - b for a, b in zip(vector(point), origin)]
        return [sum(a * b for a, b in zip(q, axis)) for axis in axes]

    def world(point):
        return App.Vector(*(c + sum(a[i] * h for a, h in zip(axes, point)) for i, c in enumerate(origin)))

    def parallel(direction):
        values = vector(direction)
        for index, axis in enumerate(axes):
            if min(math.dist(values, [sign * a for a in axis]) for sign in (-1, 1)) <= 1e-7:
                return index
        return None

    topology = Counter(type(face.Surface).__name__ for face in body.Faces)
    actual_bounds = bounds(body)
    expected_counts = {"Plane": ref["planes"], "Cylinder": ref["cylinders"], "Sphere": ref["spheres"]}
    checks = {
        "closed_valid_solid": body.isValid() and body.isClosed() and len(body.Solids) == 1,
        "analytic_faces": topology == expected_counts and len(body.Faces) == ref["faces"],
        "bounds": all(close(a, b) for lo, hi in zip(actual_bounds, ref["bounds_mm"]) for a, b in zip(lo, hi)),
    }
    planes, cylinders, spheres = [], [], []
    plane_slots, cylinder_slots, sphere_slots = set(), set(), set()
    supports = [[], [], []]
    for face in body.Faces:
        surface = face.Surface
        u0, u1, v0, v1 = face.ParameterRange
        if isinstance(surface, Part.Plane):
            axis = parallel(surface.Axis)
            offset = local(surface.Position)[axis] if axis is not None else None
            if offset is not None and close(abs(offset), half[axis] + radius):
                plane_slots.add((axis, offset > 0.0))
                direction = vector(surface.Axis)
                alignment = sum(a * b for a, b in zip(direction, axes[axis]))
                relative = [a - b for a, b in zip(vector(surface.Position), origin)]
                supports[axis].append(math.copysign(1.0, alignment) * sum(a * b for a, b in zip(relative, direction)))
            planes.append({"axis": axis, "offset_mm": offset})
        elif isinstance(surface, Part.Cylinder):
            axis = parallel(surface.Axis)
            center = local(surface.Center)
            transverse = [i for i in range(3) if i != axis]
            if (
                axis is not None and close(surface.Radius, radius) and quarter(u1 - u0)
                and close(v1 - v0, 2 * half[axis])
                and all(close(abs(center[i]), half[i]) for i in transverse)
            ):
                cylinder_slots.add((axis, *(center[i] > 0.0 for i in transverse)))
            cylinders.append({
                "axis": axis, "center_local_mm": center, "radius_mm": surface.Radius,
                "angle_deg": math.degrees(u1 - u0), "length_mm": v1 - v0,
            })
        elif isinstance(surface, Part.Sphere):
            center = local(surface.Center)
            if (
                parallel(surface.Axis) is not None and close(surface.Radius, radius)
                and quarter(u1 - u0) and quarter(v1 - v0)
                and min(abs(v0), abs(v1)) <= 1e-8
                and all(close(abs(a), b) for a, b in zip(center, half))
            ):
                sphere_slots.add(tuple(a > 0.0 for a in center))
            spheres.append({
                "center_local_mm": center, "radius_mm": surface.Radius,
                "azimuth_deg": math.degrees(u1 - u0), "latitude_deg": [math.degrees(v0), math.degrees(v1)],
            })
    checks["unique_plane_supports"] = len(plane_slots) == 6
    checks["unique_quarter_cylinders"] = len(cylinder_slots) == 12
    checks["unique_sphere_octants"] = len(sphere_slots) == 8
    nominal_volume = ref["volume_mm3"]
    volume_tolerance = max(1e-5, nominal_volume * 1e-8)
    quantization = ref.get("source_coordinate_quantization_bound_mm")
    fitted = None
    if quantization is not None:
        # Check the fitted geometry independently of the nominal quantization
        # band. Monotonicity propagates the admissible length variation to volume.
        checks["volume_from_reimported_parameters"] = False
        if all(len(pair) == 2 for pair in supports) and spheres and cylinders:
            fitted_radius = spheres[0]["radius_mm"]
            fitted_half = [abs(b - a) / 2.0 - fitted_radius for a, b in supports]
            fitted_volume = box_volume(fitted_half, fitted_radius)
            fitted = {"radius_mm": fitted_radius, "core_half_extents_mm": fitted_half, "volume_mm3": fitted_volume}
            checks["volume_from_reimported_parameters"] = (
                abs(fitted_radius - radius) <= quantization
                and all(abs(a - b) <= quantization for a, b in zip(fitted_half, half))
                and all(abs(face["radius_mm"] - fitted_radius) <= 1e-8 for face in cylinders + spheres)
                and abs(body.Volume - fitted_volume) <= volume_tolerance
            )
        volume_tolerance += max(
            abs(box_volume([h + sign * quantization for h in half], radius + sign * quantization) - nominal_volume)
            for sign in (-1, 1)
        )
    checks["volume"] = abs(body.Volume - nominal_volume) <= volume_tolerance
    tangencies, angle = tangent_edges(body, TANGENCIES)
    checks["tangencies"] = tangencies == TANGENCIES and angle <= 0.01
    # Probe the Minkowski model beyond each corner and edge, inside and
    # outside the radius. These coordinates are independent of the BRep.
    probes = []
    for signs in product((-1, 1), repeat=3):
        for fraction, inside in ((0.5, True), (0.8, False)):
            point = world([sign * (h + fraction * radius) for sign, h in zip(signs, half)])
            probes.append(body.isInside(point, 1e-6, False) == inside)
    for axis in range(3):
        for sj, sk in product((-1, 1), repeat=2):
            j, k = (axis + 1) % 3, (axis + 2) % 3
            for fraction, inside in ((0.5, True), (0.8, False)):
                point = [0.0] * 3
                point[j], point[k] = sj * (half[j] + fraction * radius), sk * (half[k] + fraction * radius)
                probes.append(body.isInside(world(point), 1e-6, False) == inside)
    checks["inside_outside_probes"] = all(probes)
    return {
        "passed": all(checks.values()), "checks": checks, "surfaces": dict(topology),
        "volume_mm3": body.Volume, "bounds_mm": actual_bounds, "tangent_pairs": tangencies,
        "nominal_volume_error_mm3": body.Volume - nominal_volume, "volume_tolerance_mm3": volume_tolerance,
        "reimported_parameters": fitted,
        "maximum_sampled_tangent_angle_deg": angle, "probes": len(probes),
        "planes": planes, "cylinders": cylinders, "spheres": spheres,
    }


def validate(path, refs):
    shape = Part.read(str(path))
    bodies = sorted(shape.Solids, key=bounds)
    expected = sorted(refs, key=lambda ref: ref["bounds_mm"])
    reports = [measure(body, ref) for body, ref in zip(bodies, expected)]
    return {
        "path": str(path), "solids": len(bodies), "bodies": reports,
        "passed": shape.isValid() and shape.isClosed() and len(bodies) == len(expected)
        and all(report["passed"] for report in reports),
    }


def write_reference(path, refs):
    bodies = []
    for ref in refs:
        r = ref["radius_mm"]
        outer = [h + r for h in ref["core_half_extents"]]
        box = Part.makeBox(*(2 * h for h in outer), App.Vector(*(-h for h in outer)))
        body = box.makeFillet(r, box.Edges).removeSplitter()
        rotation = App.Rotation(*(App.Vector(*axis) for axis in ref["axes"]), "ZXY")
        body.Placement = App.Placement(App.Vector(*ref["center_mm"]), rotation)
        bodies.append(body)
    shape = bodies[0] if len(bodies) == 1 else Part.makeCompound(bodies)
    shape.exportStep(str(path))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--write-references", action="store_true")
    args = parser.parse_args()
    if args.write_references:
        args.directory.mkdir(parents=True, exist_ok=True)
    reports = {}
    for name, entry in CASES.items():
        refs = entry.get("bodies", [entry])
        path = args.directory / (name + ".step")
        if args.write_references:
            write_reference(path, refs)
        reports[name] = validate(path, refs)
    passed = all(report["passed"] for report in reports.values())
    print(json.dumps({"freecad": ".".join(App.Version()[:3]), "cases": reports, "passed": passed}, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
