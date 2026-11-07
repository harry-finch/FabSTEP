"""Independent synthetic spherical caps and ring-torus fillets."""
import json
import math
from pathlib import Path

import generate_m3
from generate_m2b import Mesh, boundary, transformed
from generate_m2c import profile as notched_profile, triangulate
from generate_m4 import frustum_volume

ROOT = Path(__file__).resolve().parent
PI = math.pi


def ring(mesh, center, z, radius, n, angles=None):
    if radius == 0.0:
        return [mesh.point((*center, z))]
    angles = angles if angles is not None else [math.tau * i / n for i in range(n)]
    return mesh.ring([(center[0] + radius * math.cos(a), center[1] + radius * math.sin(a)) for a in angles], z)


def triangulate_band(mesh, lower, upper):
    if len(upper) == 1:
        for i, vertex in enumerate(lower):
            mesh.triangle(vertex, lower[(i + 1) % len(lower)], upper[0])
        return
    i = j = 0
    n, m = len(lower), len(upper)
    # Advance around both rings, allowing different vertex counts.
    while i < n or j < m:
        if i < n and (j == m or (i + 1) * m <= (j + 1) * n):
            mesh.triangle(lower[i % n], lower[(i + 1) % n], upper[j % m])
            i += 1
        else:
            mesh.triangle(lower[i % n], upper[(j + 1) % m], upper[j % m])
            j += 1


def hemisphere(name, *, rotate=False, sparse=False, warped=False):
    radius, n = 6.0, 8 if sparse else 64
    mesh = Mesh()
    lower = ring(mesh, (0.0, 0.0), 0.0, radius, n)
    mesh.fan(lower, (0.0, 0.0, 0.0), True)
    intervals = 18
    for i in range(1, intervals + 1):
        v = PI * i / (2 * intervals)
        if i == intervals:
            upper = [mesh.point((0.0, 0.0, radius))]
        else:
            count = 8 if sparse else max(8, round(64 * math.cos(v)))
            upper = []
            for j in range(count):
                u = math.tau * j / count
                latitude = v + 0.015 * math.cos(3 * u + i)
                rr = radius * math.cos(latitude)
                if warped and i == 7:
                    rr += 0.15
                upper.append(mesh.point((rr * math.cos(u), rr * math.sin(u), radius * math.sin(latitude))))
        triangulate_band(mesh, lower, upper)
        lower = upper
    mesh.write(name, rotate)
    return mesh


def torus_volume(major, minor):
    return PI * (major**2 * minor + PI * major * minor**2 / 2 + 2 * minor**3 / 3)


def torus_quarter(mesh, center, z, major, minor, lower, angles, *, warped=False):
    for i in range(1, 13):
        v = PI * i / 24
        r = minor + (0.1 if warped and i == 5 else 0.0)
        upper = ring(mesh, center, z + r * math.sin(v), major + r * math.cos(v), len(angles), angles)
        mesh.wall(lower, upper)
        lower = upper
    return lower


def rounded_cylinder(name, *, rotate=False, warped=False, start_height=6.0):
    major, minor = 6.0, 2.0
    base_radius = major + minor
    mesh = Mesh()
    n = 64
    angles = [math.tau * i / n for i in range(n)]
    bottom = ring(mesh, (0.0, 0.0), 0.0, base_radius, n, angles)
    equator = ring(mesh, (0.0, 0.0), start_height, base_radius, n, angles)
    mesh.fan(bottom, (0.0, 0.0, 0.0), True)
    mesh.wall(bottom, equator)
    top = torus_quarter(mesh, (0.0, 0.0), start_height, major, minor, equator, angles, warped=warped)
    mesh.fan(top, (0.0, 0.0, start_height + minor))
    mesh.write(name, rotate)
    return mesh


def non_tangent_torus(name):
    mesh = Mesh()
    n = 64
    angles = [math.tau * i / n for i in range(n)]
    major, minor = 6.0, 2.0
    v0, v1 = PI / 6, PI / 3
    start = minor * math.sin(v0)
    lower = ring(mesh, (0.0, 0.0), start, major + minor * math.cos(v0), n, angles)
    mesh.fan(lower, (0.0, 0.0, start), True)
    for i in range(1, 9):
        v = v0 + (v1 - v0) * i / 8
        z = minor * math.sin(v)
        upper = ring(mesh, (0.0, 0.0), z, major + minor * math.cos(v), n, angles)
        mesh.wall(lower, upper)
        lower = upper
    mesh.fan(lower, (0.0, 0.0, minor * math.sin(v1)))
    mesh.write(name)


def rounded_mixed(name):
    mesh = Mesh()
    xy = notched_profile(18.0, 36)
    z = 18.0
    lower = mesh.ring(xy, 0.0)
    upper = mesh.ring(xy, z)
    for a, b, c in triangulate(xy):
        mesh.triangle(lower[a], lower[b], lower[c], True)
        mesh.triangle(upper[a], upper[b], upper[c])
    for i, p in enumerate(xy):
        j = (i + 1) % len(xy)
        if p[0] != 30.0 or xy[j][0] != 30.0:
            mesh.triangle(lower[i], lower[j], upper[j])
            mesh.triangle(lower[i], upper[j], upper[i])

    xy = boundary(30.0, 60.0, 0.0, 60.0, 32)
    lower = mesh.ring(xy, 0.0)
    upper = mesh.ring(xy, z)
    mesh.fan(lower, (45.0, 30.0, 0.0), True)
    for i, p in enumerate(xy):
        j = (i + 1) % len(xy)
        if p[0] != 30.0 or xy[j][0] != 30.0:
            mesh.triangle(lower[i], lower[j], upper[j])
            mesh.triangle(lower[i], upper[j], upper[i])
    center = (45.0, 30.0)
    angles = [math.atan2(y - center[1], x - center[0]) for x, y in xy]
    profiles = [ring(mesh, center, h, r, len(xy), angles) for h, r in [(18., 10.), (20., 8.), (26., 8.)]]
    mesh.annulus(upper, profiles[0])
    mesh.wall(profiles[0], profiles[1])
    mesh.wall(profiles[1], profiles[2])
    top = torus_quarter(mesh, center, 26.0, 6.0, 2.0, profiles[2], angles)
    mesh.fan(top, (*center, 28.0))
    mesh.write(name)
    return mesh


def revolved_bounds(sphere, rotated):
    center = transformed((0, 0, 0)) if rotated else (0., 0., 0.)
    axis = tuple(p - t for p, t in zip(transformed((0, 0, 1)), center)) if rotated else (0., 0., 1.)
    limits = []
    for sign in (-1., 1.):
        corner = []
        for c, a in zip(center, axis):
            q = math.sqrt(max(0.0, 1.0 - a * a))
            axial = sign * a
            if sphere:
                support = 6.0 * (1.0 if axial >= 0.0 else q)
            else:
                phi = max(0.0, min(PI / 2.0, math.atan2(axial, q)))
                support = max(
                    8.0 * q,
                    axial * (6.0 + 2.0 * math.sin(phi)) + q * (6.0 + 2.0 * math.cos(phi)),
                )
            corner.append(c + sign * support)
        limits.append(corner)
    return limits, list(axis)


def main():
    cases = {}
    for name, rotate in [("m5_hemisphere", False), ("m5_hemisphere_rotated", True)]:
        mesh = hemisphere(name, rotate=rotate)
        bounds, axis = revolved_bounds(True, rotate)
        cases[name] = {
            "triangles": len(mesh.faces), "faces": 2, "planes": 1, "cylinders": 0, "cones": 0,
            "spheres": [{"radius_mm": 6.0, "latitudes_deg": [0.0, 90.0]}], "tori": [],
            "volume_mm3": 144.0 * PI, "rotated": rotate, "axis": axis, "bounds_mm": bounds,
        }
    for name, rotate in [("m5_toroidal_fillet", False), ("m5_toroidal_fillet_rotated", True)]:
        mesh = rounded_cylinder(name, rotate=rotate)
        bounds, axis = revolved_bounds(False, rotate)
        cases[name] = {
            "triangles": len(mesh.faces), "faces": 4, "planes": 2, "cylinders": 1, "cones": 0,
            "spheres": [], "tori": [{"major_radius_mm": 6.0, "minor_radius_mm": 2.0, "meridian_deg": [0.0, 90.0]}],
            "toroidal_fillets": 1, "volume_mm3": 384.0 * PI + torus_volume(6., 2.), "rotated": rotate,
            "axis": axis, "bounds_mm": bounds,
        }
        if not rotate:
            resource = generate_m3.mesh_object(1, mesh.vertices, mesh.faces, "Congé torique")
            for case, distance in [("m5_fillet_instances", 30.0), ("m5_fillet_touching", 16.0)]:
                items = [(1, None), (1, generate_m3.translate(distance, 0, 0))]
                generate_m3.write(case, generate_m3.model([resource], items))
                cases[case] = {
                    "format": "3mf", "bodies": [name, name], "translations_mm": [[0.0, 0, 0], [distance, 0, 0]],
                }
            cases["m5_fillet_touching"]["expected_refusal"] = "corps en contact"
    mesh = rounded_mixed("m5_rounded_mixed")
    m2c = json.loads((ROOT / "m2c_manifest.json").read_text())["cases"]["m2c_open_arc"]
    cases["m5_rounded_mixed"] = {
        "triangles": len(mesh.faces), "faces": 11, "planes": 7, "cylinders": 2, "cones": 1,
        "spheres": [], "tori": cases["m5_toroidal_fillet"]["tori"], "toroidal_fillets": 1,
        "volume_mm3": m2c["nominal_volume_mm3"] + frustum_volume(10., 8., 2.) + 384.0 * PI + torus_volume(6., 2.),
        "bounds_mm": [[0., 0., 0.], [60., 60., 28.]], "axis": [0., 0., 1.],
    }
    hemisphere("m5_warped_sphere", warped=True)
    hemisphere("m5_sparse_sphere", sparse=True)
    rounded_cylinder("m5_warped_torus", warped=True)
    non_tangent_torus("m5_non_tangent_torus")
    for name, reason in [
        ("m5_warped_sphere", "latitudes déformées"),
        ("m5_sparse_sphere", "anneau limite insuffisamment échantillonné"),
        ("m5_warped_torus", "méridien déformé"),
        ("m5_non_tangent_torus", "bords de tore sans tangence aux plans adjacents"),
    ]:
        cases[name] = {"expected_refusal": reason}
    (ROOT / "m5_manifest.json").write_text(json.dumps({"unit": "mm", "cases": cases}, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
