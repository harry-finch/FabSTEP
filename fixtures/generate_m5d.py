"""Build periodic torus STL and 3MF witnesses from independent equations."""

import json
import math
from pathlib import Path
import struct

import generate_m3
from generate_m2b import transformed


ROOT = Path(__file__).resolve().parent
MAJOR = 6.0
MINOR = 2.0


def torus(nu, nv, *, irregular=False, phase=0.0):
    points = []
    faces = []
    for i in range(nu):
        for j in range(nv):
            u, v = math.tau * (i + phase) / nu, math.tau * j / nv
            if irregular:
                u += 0.006 * math.sin(3 * u + 2 * v)
                v += 0.009 * math.cos(4 * u - 3 * v)
            rho = MAJOR + MINOR * math.cos(v)
            points.append((rho * math.cos(u), rho * math.sin(u), MINOR * math.sin(v)))
    for i in range(nu):
        for j in range(nv):
            a = i * nv + j
            b = ((i + 1) % nu) * nv + j
            c = ((i + 1) % nu) * nv + (j + 1) % nv
            d = i * nv + (j + 1) % nv
            faces.extend(((a, b, d), (b, c, d)) if (i + j) % 2 else ((a, b, c), (a, c, d)))
    return points, faces


def write_stl(name, points, faces):
    with (ROOT / (name + ".stl")).open("wb") as stream:
        stream.write(name.encode("ascii").ljust(80, b" ") + struct.pack("<I", len(faces)))
        for face in faces:
            coords = tuple(value for index in face for value in points[index])
            stream.write(struct.pack("<12fH", 0.0, 0.0, 0.0, *coords, 0))


def write_instances(name, points, faces, second):
    resource = generate_m3.mesh_object(1, points, faces, "Tore R6 r2")
    generate_m3.write(name, generate_m3.model([resource], [(1, None), (1, second)]))


def reference(faces, center=(0.0, 0.0, 0.0), axis=(0.0, 0.0, 1.0)):
    half_extents = [MINOR + MAJOR * math.sqrt(1.0 - a * a) for a in axis]
    return {
        "triangles": len(faces),
        "faces": 1,
        "bounds_mm": [[c + sign * d for c, d in zip(center, half_extents)] for sign in (-1, 1)],
        "volume_mm3": 2.0 * math.pi**2 * MAJOR * MINOR**2,
        "tori": [{
            "center_mm": center,
            "axis": axis,
            "major_radius_mm": MAJOR,
            "minor_radius_mm": MINOR,
            "angular_span_rad": math.tau,
            "meridian_span_rad": math.tau,
        }],
    }


def main():
    cases = {}
    points, faces = torus(60, 40)
    write_stl("m5d_ring_torus", points, faces)
    cases["m5d_ring_torus"] = reference(faces)

    irregular, irregular_faces = torus(60, 40, irregular=True)
    center = transformed((0.0, 0.0, 0.0))
    axis = tuple(a - c for a, c in zip(transformed((0.0, 0.0, 1.0)), center))
    write_stl("m5d_torus_transformed", [transformed(p) for p in irregular], irregular_faces)
    cases["m5d_torus_transformed"] = reference(irregular_faces, center, axis)

    write_instances("m5d_torus_instances", irregular, irregular_faces, "-1 0 0 0 1 0 0 0 1 24 4 2")
    cases["m5d_torus_instances"] = {
        "format": "3mf", "triangles": 2 * len(irregular_faces),
        "bodies": [reference(irregular_faces), reference(irregular_faces, (24.0, 4.0, 2.0))],
    }

    # Both sides of the geometric contact are independently tested. The second
    # witness offsets azimuth samples so its input meshes have a positive gap.
    for stage, phase in (("source", 0.0), ("result", 0.5)):
        name = "m5d_tori_contact_" + stage
        sampled, sampled_faces = torus(44, 32, phase=phase)
        write_instances(name, sampled, sampled_faces, generate_m3.translate(16, 0, 0))
        cases[name] = {
            "format": "3mf", "triangles": 2 * len(sampled_faces), "contact_stage": stage,
            "expected_refusal": "contact des corps",
        }

    dent = points.copy()
    x, y, z = dent[len(points) // 3]
    dent[len(points) // 3] = (x, y, z + 0.06)
    sparse_points, sparse_faces = torus(10, 7)
    for name, source, triangles, reason in (
        ("m5d_open_torus", points, faces[1:], "maillage ouvert"),
        ("m5d_torus_dent", dent, faces, "sommet déplacé de 0,06 mm"),
        ("m5d_elliptic_torus", [(1.08 * x, y, z) for x, y, z in points], faces, "mise à l’échelle non uniforme"),
        ("m5d_sparse_torus", sparse_points, sparse_faces, "échantillonnage insuffisant"),
    ):
        write_stl(name, source, triangles)
        cases[name] = {"triangles": len(triangles), "expected_refusal": reason}
    (ROOT / "m5d_manifest.json").write_text(
        json.dumps({"unit": "mm", "cases": cases}, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
