#!/usr/bin/env python3
"""Generate a closed blind pocket with two independently sampled floor fillets."""
import argparse
from dataclasses import replace
import hashlib
import json
import math
from pathlib import Path
import runpy

BASE = runpy.run_path(
    str(Path(__file__).with_name("generate-pocket-fillet-fixtures.py"))
)
OUT = Path(__file__).resolve().parents[1] / "fixtures" / "opposed_pocket"


def generate(d, other_radius, bands, intervals, sample, warped):
    """Combine the curved sides of two pocket sections with a common flat floor."""
    sides = []
    for i, radius in enumerate((d.radius, other_radius)):
        source = "warped_stop" if warped and i == 1 else sample
        faces, ranges = BASE["generate"](
            source, replace(d, radius=radius), bands, intervals[i]
        )
        sides.append({key: faces[a:b] for key, (a, b) in ranges.items()})
    left, right = sides
    ymax = d.pocket_width / 2

    def reflect(tri):
        a, b, c = tri
        return tuple((x, -y, z) for x, y, z in (a, c, b))

    def floor_end(p):
        x, y, z = p
        return (x, ymax - other_radius, z) if y == ymax else p

    groups = {g: left[g] for g in BASE["GROUPS"][:6]}
    groups["floor"] = [tuple(map(floor_end, t)) for t in left["floor"]]
    for label, side, transform in [("left", left, tuple), ("right", right, reflect)]:
        groups[label + "_fillet"] = [transform(t) for t in side["fillet"]]
        groups[label + "_wall"] = [transform(t) for t in side["tangent_wall"]]

    for end in ("stop_min", "stop_max"):
        # The end of each section is a fan. The center is shared, the top and
        # floor are included once, and each arc contributes only its own fan.
        stop = []
        for side in (left, right):
            for tri in side[end]:
                center, a, b = tri
                if a[1] == b[1] == ymax:
                    continue
                if a[2] == b[2] == d.floor:
                    if side is left:
                        stop.append((center, floor_end(a), floor_end(b)))
                elif a[2] == b[2] == d.height:
                    if side is left:
                        stop.append(tri)
                else:
                    stop.append(tri if side is left else reflect(tri))
        groups[end] = stop
    return groups


def write(
    out, name, d, other_radius, bands, intervals, sample, rotated=False, warped=False
):
    def place(p):
        x, y, z = p
        # A rigid permutation with translation preserves all coordinate planes.
        return (z + 7, x - 4, y + 2) if rotated else p

    faces = []
    ranges = {}
    for group, source in generate(
        d, other_radius, bands, intervals, sample, warped
    ).items():
        start = len(faces)
        faces.extend(tuple(map(place, tri)) for tri in source)
        ranges[group] = [start, len(faces)]

    data, triangles = BASE["binary_mesh"](faces)
    mesh_volume, boundary = BASE["mesh_measurements"](triangles)
    assert boundary == 0 and mesh_volume > 0
    (out / (name + ".stl")).write_bytes(data)
    case = dict(
        triangles=len(faces),
        groups=ranges,
        mesh_volume_mm3=mesh_volume,
        sha256=hashlib.sha256(data).hexdigest(),
    )
    if warped:
        return case

    radii = (d.radius, other_radius)
    volume = (
        d.length * d.width * d.height
        - d.pocket_width * (d.height - d.floor) * d.pocket_length
        + d.pocket_length * sum(r * r for r in radii) * (1 - math.pi / 4)
    )
    # Bound the excess material between the analytic cylinders and chordal
    # patches, independently for the two arc resolutions.
    shell_bound = (
        d.pocket_length
        * math.pi
        / 4
        * sum((r * math.sin(math.pi / (4 * n))) ** 2 for r, n in zip(radii, intervals))
    )
    assert 0 < mesh_volume - volume < shell_bound + 0.001
    case.update(
        volume_mm3=volume,
        radii_mm=list(radii),
        planar_floor_width_mm=d.pocket_width - sum(radii),
        inside_void_mm=place((0, 0, (d.floor + d.height) / 2)),
    )
    corners = [
        place((x, y, z))
        for x in (-d.length / 2, d.length / 2)
        for y in (-d.width / 2, d.width / 2)
        for z in (0, d.height)
    ]
    case["bounds_mm"] = [
        [f(p[i] for p in corners) for i in range(3)] for f in (min, max)
    ]
    case["fillets"] = []
    for sign, r in zip((-1, 1), radii):
        cy, cz = sign * (d.pocket_width / 2 - r), d.floor + r
        case["fillets"].append(
            dict(
                radius_mm=r,
                area_mm2=math.pi * r * d.pocket_length / 2,
                ends_mm=[
                    place((x, cy, cz))
                    for x in (-d.pocket_length / 2, d.pocket_length / 2)
                ],
                inside_material_mm=place((0, cy + sign * 0.85 * r, cz - 0.85 * r)),
            )
        )
    return case


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?", default=OUT)
    out = parser.parse_args().output
    out.mkdir(parents=True, exist_ok=True)
    d = BASE["Dimensions"]()
    cases = {
        "nominal": write(out, "nominal", d, 3, 2, (16, 20), "nominal"),
        "staggered": write(out, "staggered", d, 3, 2, (16, 20), "staggered"),
        "rotated": write(
            out,
            "rotated",
            replace(d, pocket_length=18, pocket_width=12, radius=1.5),
            3.5,
            3,
            (20, 23),
            "retessellated",
            rotated=True,
        ),
        "warped_stop": write(
            out, "warped_stop", d, 3, 1, (16, 20), "staggered", warped=True
        ),
    }
    manifest = dict(
        unit="mm",
        positives=["nominal", "staggered", "rotated"],
        negatives=["warped_stop"],
        planes=11,
        cylinders=2,
        cases=cases,
    )
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, allow_nan=False) + "\n"
    )


if __name__ == "__main__":
    main()
