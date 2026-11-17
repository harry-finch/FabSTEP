"""Independent R-078 circular bands and volume references in millimeters."""

import hashlib
import json
import math
from pathlib import Path
import struct

from generate_m2b import Mesh


ROOT = Path(__file__).resolve().parent / "adjacent_chains"
AZIMUTHS = 64
THIRDS = (1 / 3, 2 / 3)
THREE = [(0., 10.), (3., 13.), (6., 14.5), (10., 12.)]
MIXED = [(0., 10.), (3., 10.), (6., 13.), (9., 14.5), (12., 14.5),
         (15., 13.), (19., 13.)]
NINE = [(3. * i, r) for i, r in enumerate((10., 13., 14.5, 13., 15., 16., 13., 14., 12., 15.))]


def subdivide(profile, defect):
    rings = [profile[0]]
    ends = [0]
    for i, ((z, r), (end_z, end_r)) in enumerate(zip(profile, profile[1:])):
        witnesses = THIRDS
        if defect == "missing_witness":
            witnesses = (0.5,) if i == 1 else (0.28, 0.4, 0.6, 0.72)
        if defect == "ambiguous" and i == 1:
            witnesses = (0.00008 / (end_z - z), *THIRDS)
        rings.extend((z + t * (end_z - z), r + t * (end_r - r)) for t in witnesses)
        rings.append((end_z, end_r))
        ends.append(len(rings) - 1)
    return rings, ends


def write(name, profile, *, placement="plain", defect=None, smooth=False):
    levels, ends = (profile, list(range(len(profile)))) if smooth else subdivide(profile, defect)
    mesh = Mesh()
    angles = [math.tau * i / AZIMUTHS for i in range(AZIMUTHS)]
    rings = [mesh.ring([(r * math.cos(a), r * math.sin(a)) for a in angles], z)
             for z, r in levels]
    mesh.fan(rings[0], (0., 0., levels[0][0]), True)
    for lower, upper in zip(rings, rings[1:]):
        mesh.wall(lower, upper)
    mesh.fan(rings[-1], (0., 0., levels[-1][0]))
    if defect == "crossed":
        before, seam, after = rings[ends[1] - 1:ends[1] + 2]
        # Replace the seam edge with a diagonal spanning both adjacent runs.
        i = mesh.faces.index((before[0], seam[1], seam[0]))
        j = mesh.faces.index((seam[0], seam[1], after[1]))
        mesh.faces[i] = (before[0], seam[1], after[1])
        mesh.faces[j] = (before[0], after[1], seam[0])

    if placement == "rotated":
        mesh.vertices = [(z + 1., x - 3., y + 5.) for x, y, z in mesh.vertices]
    elif placement == "mirrored":
        mesh.vertices = [(x, y, profile[-1][0] - z) for x, y, z in mesh.vertices]
        mesh.faces = [(c, b, a) for a, b, c in mesh.faces]
    path = ROOT / f"{name}.stl"
    with path.open("wb") as out:
        out.write(bytes(80))
        out.write(struct.pack("<I", len(mesh.faces)))
        for face in mesh.faces:
            coordinates = [x for v in face for x in mesh.vertices[v]]
            out.write(struct.pack("<12fH", 0., 0., 0., *coordinates, 0))

    volume = math.fsum(math.pi * (b - a) * (r*r + r*s + s*s) / 3.
                       for (a, r), (b, s) in zip(profile, profile[1:]))
    max_radius = max(r for _, r in profile)
    z0, z1 = profile[0][0], profile[-1][0]
    bounds = [[-max_radius, -max_radius, z0], [max_radius, max_radius, z1]]
    if placement == "rotated":
        bounds = [[z + 1., x - 3., y + 5.] for x, y, z in bounds]
    kinds = ["cylinder" if r == s else "cone" for (_, r), (_, s) in zip(profile, profile[1:])]
    counts = [AZIMUTHS, *[2 * AZIMUTHS * (b - a) for a, b in zip(ends, ends[1:])], AZIMUTHS]
    return dict(sha256=hashlib.sha256(path.read_bytes()).hexdigest(), triangles=len(mesh.faces),
                volume_mm3=volume, bounds_mm=bounds, rings_mm=profile, placement=placement,
                cones=kinds.count("cone"), cylinders=kinds.count("cylinder"),
                patches=list(zip(["plane", *kinds, "plane"], counts)))


def main():
    ROOT.mkdir(exist_ok=True)
    cases = {name: write(name, p, placement=t) for name, p, t in (
        ("three", THREE, "plain"), ("three_mirror", THREE, "mirrored"),
        ("eight", NINE[:-1], "plain"), ("mixed", MIXED, "plain"),
        ("mixed_rotated", MIXED, "rotated"),
    )}
    negatives = {}
    for name, p, defect, smooth in (
        ("missing_witness", THREE, "missing_witness", False),
        ("crossed", THREE, "crossed", False),
        ("unresolved_slope", [(0., 10.), (3., 13.), (6., 16.00015), (9., 14.)], None, False),
        ("ambiguous", [(0., 10.), (3., 13.), (6., 15.94), (9., 14.)], "ambiguous", False),
        ("smooth", [(z / 2, 10. + .01 * z*z) for z in range(19)], None, True),
        ("nine", NINE, None, False),
    ):
        item = write(name, p, defect=defect, smooth=smooth)
        negatives[name] = {key: item[key] for key in ("triangles", "sha256")}
    (ROOT / "manifest.json").write_text(json.dumps(dict(
        unit="mm", cases=cases, negative_cases=negatives,
    ), indent=2) + "\n")


if __name__ == "__main__":
    main()
