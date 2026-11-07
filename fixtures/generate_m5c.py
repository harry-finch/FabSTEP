"""Independent sphere meshes and analytical reference values."""
import json
import math
from pathlib import Path

import generate_m3
from generate_m2b import Mesh, transformed
from generate_m5b import append_cap, cap_reference

ROOT = Path(__file__).resolve().parent
RADIUS = 6.0


def normalized(p):
    length = math.hypot(*p)
    return tuple(x / length for x in p)


def icosphere(levels=3):
    phi = (1.0 + math.sqrt(5.0)) / 2.0
    points = [normalized(p) for p in [
        (-1, phi, 0), (1, phi, 0), (-1, -phi, 0), (1, -phi, 0),
        (0, -1, phi), (0, 1, phi), (0, -1, -phi), (0, 1, -phi),
        (phi, 0, -1), (phi, 0, 1), (-phi, 0, -1), (-phi, 0, 1),
    ]]
    triangles = [
        (0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11),
        (1, 5, 9), (5, 11, 4), (11, 10, 2), (10, 7, 6), (7, 1, 8),
        (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9),
        (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1),
    ]
    for _ in range(levels):
        midpoints = {}

        def midpoint(a, b):
            key = tuple(sorted([a, b]))
            if key not in midpoints:
                midpoints[key] = len(points)
                points.append(normalized(tuple(x + y for x, y in zip(points[a], points[b]))))
            return midpoints[key]

        refined = []
        for a, b, c in triangles:
            ab, bc, ca = midpoint(a, b), midpoint(b, c), midpoint(c, a)
            refined.extend([(a, ab, ca), (b, bc, ab), (c, ca, bc), (ab, bc, ca)])
        triangles = refined
    # A small, asymmetric displacement before projection preserves the exact
    # radius and eliminates the need for any pre-existing antipodal vertices.
    shift = (0.0011, -0.0023, 0.0017)
    mesh = Mesh()
    mesh.vertices = [
        tuple(RADIUS * x for x in normalized(tuple(a + b for a, b in zip(p, shift))))
        for p in points
    ]
    mesh.faces = triangles
    return mesh


def uv_sphere(count=64, bands=32):
    mesh = Mesh()
    angles = [math.tau * i / count for i in range(count)]
    south = mesh.point((0.0, 0.0, -RADIUS))
    north = mesh.point((0.0, 0.0, RADIUS))
    lower = None
    for j in range(1, bands):
        latitude = -math.pi / 2 + math.pi * j / bands
        radial = RADIUS * math.cos(latitude)
        upper = mesh.ring([(radial * math.cos(u), radial * math.sin(u)) for u in angles],
                          RADIUS * math.sin(latitude))
        if lower is None:
            for i, vertex in enumerate(upper):
                mesh.triangle(south, upper[(i + 1) % count], vertex)
        else:
            mesh.wall(lower, upper)
        lower = upper
    for i, vertex in enumerate(lower):
        mesh.triangle(vertex, lower[(i + 1) % count], north)
    return mesh


def truncated_sphere():
    mesh = Mesh()
    height = 2.0
    radius = math.sqrt(height * (2 * RADIUS - height))
    angles = [math.tau * i / 64 for i in range(64)]
    rim = mesh.ring([(radius * math.cos(u), radius * math.sin(u)) for u in angles], 0.0)
    append_cap(mesh, rim, angles, height)
    # Triangulate the flat closure with rim vertices only. Every source
    # vertex is cospherical, yet the existing planar closure is meaningful.
    for i in range(1, len(rim) - 1):
        mesh.triangle(rim[0], rim[i + 1], rim[i])
    return mesh


def reference(mesh, radius=RADIUS, center=(0.0, 0.0, 0.0)):
    return {
        "triangles": len(mesh.faces), "faces": 1, "planes": 0, "cylinders": 0, "cones": 0,
        "complete_spheres": 1, "tori": [],
        "spheres": [{"radius_mm": radius, "center_mm": center, "latitudes_deg": [-90.0, 90.0]}],
        "bounds_mm": [[c + sign * radius for c in center] for sign in (-1.0, 1.0)],
        "volume_mm3": 4.0 * math.pi * radius**3 / 3.0,
    }


def main():
    cases = {}
    mesh = icosphere()
    mesh.write("m5c_icosphere")
    cases["m5c_icosphere"] = reference(mesh)
    mesh.write("m5c_icosphere_transformed", rotate=True)
    cases["m5c_icosphere_transformed"] = reference(mesh, center=transformed((0.0, 0.0, 0.0)))
    resource = generate_m3.mesh_object(1, mesh.vertices, mesh.faces, "Sphère")
    uv = uv_sphere()
    uv.write("m5c_uv_sphere")
    cases["m5c_uv_sphere"] = reference(uv)
    # The source contact witness uses a lighter UV mesh with exact cardinal
    # vertices so the native separation test can run within its face budget.
    contact_mesh = uv_sphere(32, 24)
    uv_resource = generate_m3.mesh_object(1, contact_mesh.vertices, contact_mesh.faces, "Sphère UV")
    for name, selected in [("m5c_spheres_contact_source", uv_resource), ("m5c_spheres_contact_result", resource)]:
        generate_m3.write(name, generate_m3.model([selected], [(1, None), (1, generate_m3.translate(12, 0, 0))]))
        cases[name] = {
            "format": "3mf", "expected_refusal": "contact des corps",
            "contact_stage": "source" if selected == uv_resource else "result",
            "triangles": 2 * len(contact_mesh.faces if selected == uv_resource else mesh.faces),
        }
    mirror = "-1 0 0 0 1 0 0 0 1 20 2 1"
    generate_m3.write("m5c_sphere_instances", generate_m3.model([resource], [(1, None), (1, mirror)]))
    cases["m5c_sphere_instances"] = {
        "format": "3mf",
        "bodies": [reference(mesh), reference(mesh, center=(20.0, 2.0, 1.0))],
        "triangles": 2 * len(mesh.faces),
    }
    scale = "1.5 0 0 0 1.5 0 0 0 1.5 3 -4 5"
    generate_m3.write("m5c_sphere_scaled", generate_m3.model([resource], [(1, scale)]))
    cases["m5c_sphere_scaled"] = {"format": "3mf", **reference(mesh, radius=9.0, center=(3.0, -4.0, 5.0))}
    truncated = truncated_sphere()
    truncated.write("m5c_truncated_cospherical")
    cases["m5c_truncated_cospherical"] = {
        **cap_reference(2.0), "triangles": len(truncated.faces), "complete_spheres": 0,
    }
    sparse = icosphere(0)
    sparse.write("m5c_sparse_sphere")
    cases["m5c_sparse_sphere"] = {"triangles": len(sparse.faces), "expected_refusal": "arêtes angulaires trop grandes"}
    deformed = icosphere()
    deformed.vertices = [(x, y, z * 1.05) for x, y, z in deformed.vertices]
    deformed.write("m5c_spheroid")
    cases["m5c_spheroid"] = {"triangles": len(deformed.faces), "expected_refusal": "sphéroïde allongé"}
    dented = icosphere()
    dented.vertices[0] = tuple(x * (1.0 - 0.04 / RADIUS) for x in dented.vertices[0])
    dented.write("m5c_sphere_dent")
    cases["m5c_sphere_dent"] = {"triangles": len(dented.faces), "expected_refusal": "un sommet en retrait de 0,04 mm"}
    opened = icosphere()
    opened.faces.pop()
    opened.write("m5c_open_sphere")
    cases["m5c_open_sphere"] = {"triangles": len(opened.faces), "expected_refusal": "maillage ouvert"}
    (ROOT / "m5c_manifest.json").write_text(json.dumps({"unit": "mm", "cases": cases}, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
