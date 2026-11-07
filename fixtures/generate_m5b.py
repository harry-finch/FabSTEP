"""Independent analytical spherical-cap meshes, including nonuniform triangles."""
import json
import math
from pathlib import Path

import generate_m3
from generate_m2b import Mesh, boundary, transformed
from generate_m5 import triangulate_band

ROOT = Path(__file__).resolve().parent
RADIUS = 6.0


def latitude(height):
    return math.asin((RADIUS - height) / RADIUS)


def cap_volume(height):
    return math.pi * height**2 * (RADIUS - height / 3.0)


def sample_band(mesh, lower, angles, lo, hi, center_z, direction, *, sparse=False):
    intervals = max(1, math.ceil((hi - lo) / math.radians(5)))
    for j in range(1, intervals + 1):
        v = lo + (hi - lo) * j / intervals
        if abs(v - math.pi / 2) < 1e-14:
            upper = [mesh.point((0.0, 0.0, center_z + direction * RADIUS))]
        else:
            count = len(angles) if sparse else max(8, round(len(angles) * math.cos(v)))
            upper = []
            for i in range(count):
                u = angles[i] if sparse else angles[0] + math.tau * i / count
                # Keep band endpoints exact. Interior rows have no shared z.
                local_v = v if j == intervals else v + 0.009 * math.cos(3 * u + j)
                radial = RADIUS * math.cos(local_v)
                upper.append(mesh.point((radial * math.cos(u), radial * math.sin(u),
                                         center_z + direction * RADIUS * math.sin(local_v))))
        triangulate_band(mesh, lower, upper)
        lower = upper
    return lower


def append_cap(mesh, lower, angles, height, z0=0.0, direction=1.0, *, sparse=False):
    start = latitude(height)
    center_z = z0 + direction * (height - RADIUS)
    if start < 0.0:
        lower = sample_band(mesh, lower, angles, start, 0.0, center_z, direction, sparse=sparse)
        start = 0.0
    return sample_band(mesh, lower, angles, start, math.pi / 2, center_z, direction, sparse=sparse)[0]


def standalone(name, height, *, rotate=False, sparse=False, spheroid=False, shifted=False):
    mesh = Mesh()
    count = 8 if sparse else 64
    angles = [math.tau * i / count for i in range(count)]
    radial = math.sqrt(height * (2.0 * RADIUS - height))
    lower = mesh.ring([(radial * math.cos(u), radial * math.sin(u)) for u in angles], 0.0)
    mesh.fan(lower, (0.0, 0.0, 0.0), True)
    tip = append_cap(mesh, lower, angles, height, sparse=sparse)
    if spheroid:
        mesh.vertices = [(x, y, 0.8 * z) for x, y, z in mesh.vertices]
    if shifted:
        mesh.vertices[tip] = (0.015, 0.0, height)
    mesh.write(name, rotate)
    return mesh


def concave(name, height):
    mesh = Mesh()
    outer = boundary(-10.0, 10.0, -10.0, 10.0, 16)
    angles = [math.atan2(y, x) for x, y in outer]
    low = mesh.ring(outer, 0.0)
    high = mesh.ring(outer, 10.0)
    radial = math.sqrt(height * (2.0 * RADIUS - height))
    rim = mesh.ring([(radial * math.cos(u), radial * math.sin(u)) for u in angles], 10.0)
    mesh.fan(low, (0.0, 0.0, 0.0), True)
    mesh.wall(low, high)
    mesh.annulus(high, rim)
    # Reflection reverses the surface, as does placing it on the void side
    # of the block. The two reversals cancel in the source winding.
    append_cap(mesh, rim, angles, height, 10.0, -1.0)
    mesh.write(name)
    return mesh


def spherical_zone(name):
    mesh = Mesh()
    angles = [math.tau * i / 64 for i in range(64)]
    radial = math.sqrt(RADIUS**2 - 4.0)
    low = mesh.ring([(radial * math.cos(u), radial * math.sin(u)) for u in angles], -2.0)
    mesh.fan(low, (0.0, 0.0, -2.0), True)
    top = sample_band(mesh, low, angles, -math.asin(2.0 / RADIUS), math.asin(2.0 / RADIUS), 0.0, 1.0)
    mesh.fan(top, (0.0, 0.0, 2.0))
    mesh.write(name)
    return mesh


def cap_reference(height, *, rotated=False):
    center = (0.0, 0.0, height - RADIUS)
    axis = (0.0, 0.0, 1.0)
    if rotated:
        target = transformed((0.0, 0.0, height - RADIUS + 1.0))
        center = transformed(center)
        axis = tuple(b - a for a, b in zip(center, target))
    low = latitude(height)
    limits = []
    for sign in (-1.0, 1.0):
        corner = []
        for coordinate, a in zip(center, axis):
            axial = sign * a
            radial = math.sqrt(max(0.0, 1.0 - a * a))
            v = max(low, math.atan2(axial, radial))
            corner.append(coordinate + sign * RADIUS * (axial * math.sin(v) + radial * math.cos(v)))
        limits.append(corner)
    return {
        "faces": 2, "planes": 1, "cylinders": 0, "cones": 0, "tori": [],
        "spheres": [{
            "radius_mm": RADIUS, "height_mm": height,
            "latitudes_deg": [math.degrees(low), 90.0], "center_mm": center,
        }],
        "axis": axis, "bounds_mm": limits, "volume_mm3": cap_volume(height),
    }


def main():
    cases = {}
    for name, height, rotate in [
        ("m5b_small_cap", 2.0, False),
        ("m5b_large_cap", 10.0, False),
        ("m5b_large_cap_rotated", 10.0, True),
    ]:
        mesh = standalone(name, height, rotate=rotate)
        cases[name] = {**cap_reference(height, rotated=rotate), "triangles": len(mesh.faces)}
        if name == "m5b_small_cap":
            resource = generate_m3.mesh_object(1, mesh.vertices, mesh.faces, "Calotte")
            mirrored = "1 0 0 0 1 0 0 0 -1 20 0 2"
            generate_m3.write("m5b_cap_instances", generate_m3.model([resource], [(1, None), (1, mirrored)]))
            lo, hi = cases[name]["bounds_mm"]
            reflected = [[lo[0] + 20.0, lo[1], 2.0 - hi[2]], [hi[0] + 20.0, hi[1], 2.0 - lo[2]]]
            cases["m5b_cap_instances"] = {
                "format": "3mf", "triangles": 2 * len(mesh.faces), "bodies": [name, name],
                "body_bounds_mm": [[lo, hi], reflected], "mirrored_z": [False, True],
                "translations_mm": [[0.0, 0.0, 0.0], [20.0, 0.0, 2.0]],
            }
            touching = generate_m3.translate(2.0 * math.sqrt(20.0), 0.0, 0.0)
            generate_m3.write("m5b_cap_touching", generate_m3.model([resource], [(1, None), (1, touching)]))
            cases["m5b_cap_touching"] = {"format": "3mf", "expected_refusal": "anneaux des bases tangents"}
    mesh = concave("m5b_concave_cap", 2.0)
    cases["m5b_concave_cap"] = {
        "triangles": len(mesh.faces), "faces": 7, "planes": 6, "cylinders": 0, "cones": 0, "tori": [],
        "spheres": [{
            "radius_mm": RADIUS, "height_mm": 2.0,
            "latitudes_deg": [-90.0, -math.degrees(latitude(2.0))], "center_mm": [0.0, 0.0, 14.0],
        }],
        "axis": [0.0, 0.0, 1.0], "bounds_mm": [[-10.0, -10.0, 0.0], [10.0, 10.0, 10.0]],
        "volume_mm3": 4000.0 - cap_volume(2.0), "concave": True,
    }
    for name, mesh, reason in [
        ("m5b_spheroid", standalone("m5b_spheroid", 2.0, spheroid=True), "sphéroïde aplati"),
        ("m5b_shifted_pole", standalone("m5b_shifted_pole", 10.0, shifted=True), "pôle source hors de l’axe"),
        ("m5b_spherical_zone", spherical_zone("m5b_spherical_zone"), "zone sphérique à deux plans et sans pôle"),
        ("m5b_sparse_cap", standalone("m5b_sparse_cap", 2.0, sparse=True), "huit sommets sur l’anneau"),
    ]:
        cases[name] = {"triangles": len(mesh.faces), "expected_refusal": reason}
    (ROOT / "m5b_manifest.json").write_text(json.dumps({"unit": "mm", "cases": cases}, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
