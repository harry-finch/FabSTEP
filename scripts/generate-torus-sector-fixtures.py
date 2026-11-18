#!/usr/bin/env python3
"""Build public STL references from a radial section swept through 80 degrees."""
import argparse
from collections import Counter
import json
import math
from pathlib import Path
import struct


R, FILLET, INNER, HEIGHT = 8.0, 0.5, 3.0, 6.0
SWEEP = math.radians(80)
FACET = struct.Struct("<12fH")


def exact_volume():
    # Integral of annular area through the straight wall and quarter-round.
    straight = (HEIGHT - FILLET) * (R + FILLET)**2
    rounded = R**2 * FILLET + math.pi * R * FILLET**2 / 2 + 2 * FILLET**3 / 3
    return SWEEP / 2 * (straight + rounded - INNER**2 * HEIGHT)


def make_faces(name, azimuths, meridians):
    arc = [(R + FILLET * math.cos(j * math.pi / (2 * meridians)),
            HEIGHT - FILLET + FILLET * math.sin(j * math.pi / (2 * meridians)))
           for j in range(meridians + 1)]
    section = [(INNER, 0), (R + FILLET, 0), *arc, (INNER, HEIGHT)]
    labels = ["bottom", "outer_cylinder", *(["torus"] * meridians), "top", "inner_cylinder"]

    def place(i, j, rho, z):
        step = float(i)
        if name == "folded" and (i, j) == (azimuths // 2, 2 + meridians // 2):
            step += 1.75
        if name == "dented" and (i, j) == (azimuths // 2, 2 + meridians // 2):
            rho += 0.12
        phi = SWEEP * step / azimuths
        x, y = rho * math.cos(phi), rho * math.sin(phi)
        if name == "oblique":
            # Rotation about X followed by a translation.
            return (x + 2, (3*y - 4*z)/5 - 1, (4*y + 3*z)/5 + 3)
        if name == "mirrored":
            return (x, -y, z)
        return (x, y, z)

    slices = [[place(i, j, *point) for j, point in enumerate(section)]
              for i in range(azimuths + 1)]
    faces = []
    for j, label in enumerate(labels):
        k = (j + 1) % len(section)
        for i in range(azimuths):
            a, b = slices[i][j], slices[i + 1][j]
            c, d = slices[i + 1][k], slices[i][k]
            pair = [(a, b, d), (b, c, d)] if (i + j) % 2 else [(a, b, c), (a, c, d)]
            faces.extend((label, tri) for tri in pair)
    for i, label in [(0, "start"), (azimuths, "end")]:
        if name == "open" and i == azimuths:
            continue
        center = place(i, -1, (INNER + R) / 2, HEIGHT / 2)
        for j, a in enumerate(slices[i]):
            b = slices[i][(j + 1) % len(section)]
            faces.append((label, (center, a, b) if i == 0 else (center, b, a)))
    return [(label, tri[::-1] if name == "mirrored" else tri) for label, tri in faces]


def write_case(directory, name, azimuths, meridians):
    faces = make_faces(name, azimuths, meridians)
    data = bytearray(80) + struct.pack("<I", len(faces))
    for _, tri in faces:
        data.extend(FACET.pack(0, 0, 0, *(x for p in tri for x in p), 0))
    path = directory / (name + ".stl")
    path.write_bytes(data)
    edges = Counter()
    for entry in FACET.iter_unpack(path.read_bytes()[84:]):
        a, b, c = entry[3:6], entry[6:9], entry[9:12]
        assert len({a, b, c}) == 3
        edges.update(((a, b), (b, c), (c, a)))
    assert max(edges.values()) == 1
    boundary = sum((b, a) not in edges for a, b in edges)
    assert boundary == (meridians + 4 if name == "open" else 0)
    return dict(triangles=len(faces), triangles_per_family=dict(Counter(k for k, _ in faces)),
                open_edges=boundary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?",
                        default=Path(__file__).resolve().parents[1] / "fixtures/torus_sectors")
    directory = parser.parse_args().output
    directory.mkdir(exist_ok=True, parents=True)
    cases = {name: write_case(directory, name, *resolution) for name, resolution in [
        ("nominal", (48, 16)), ("oblique", (48, 16)), ("mirrored", (48, 16)),
        ("retessellated", (65, 21)), ("folded", (48, 16)), ("open", (48, 16)),
        ("dented", (48, 16))]}
    manifest = dict(reference_volume_mm3=exact_volume(), positives=list(cases)[:4], cases=cases)
    (directory / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
