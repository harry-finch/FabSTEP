"""Generate independent R-076 profiles and closed source meshes in millimeters."""

import hashlib
import json
import math
from pathlib import Path
import struct


ROOT = Path(__file__).resolve().parents[1] / "fixtures" / "revolution_chains"
MAJOR = 10.0
ARC_START = -math.pi / 2
AZIMUTHS = 64
ARC_INTERVALS = 20


def arc(angle, minor, branch):
    return minor * (1.0 + math.sin(angle)), MAJOR + branch * minor * math.cos(angle)


def nominal_profile(kind, branch, defect):
    minor = 2.0 if kind == "stepped" else 0.3
    end_angle = 0.0 if kind == "stepped" else math.radians(10.0)
    end_z, end_radius = arc(end_angle, minor, branch)
    if kind == "stepped":
        first_cone_radius = end_radius + 2.0
        steps = [(end_z + 3.0, end_radius), (end_z + 5.0, first_cone_radius),
                 (end_z + 8.0, first_cone_radius), (end_z + 12.0, first_cone_radius - 3.0)]
        families = ["torus", "cylinder", "cone", "cylinder", "cone"]
    else:
        slope = -branch * math.tan(end_angle)
        if defect == "non_tangent":
            slope += 0.03
        height = 0.00003 if defect == "unresolved_end" else 4.0
        steps = [(end_z + height, end_radius + slope * height)]
        families = ["torus", "cone"]
    cuts = [arc(ARC_START, minor, branch), (end_z, end_radius), *steps]
    return minor, end_angle, cuts, families


def patch_volume(minor, branch, end_angle, cuts):
    def primitive(angle):
        s, c = math.sin(angle), math.cos(angle)
        return (MAJOR**2 * s + branch * MAJOR * minor * (angle + s * c)
                + minor**2 * (s - s**3 / 3.0))

    torus = math.pi * minor * (primitive(end_angle) - primitive(ARC_START))
    straights = [math.pi * (b - a) * (r*r + r*s + s*s) / 3.0
                 for (a, r), (b, s) in zip(cuts[1:], cuts[2:])]
    return math.fsum([torus, *straights])


def placement(variant, height):
    if variant == "mirrored":
        return lambda x, y, z: (x, y, height - z), True
    if variant == "rotated":
        # Proper rational rotation about X plus a translation. Keep doubles in
        # ASCII STL so that the rotated planar faces remain coplanar.
        return lambda x, y, z: (x + 7.0, (12*y - 5*z)/13 - 4.0,
                                (5*y + 12*z)/13 + 2.0), False
    return lambda x, y, z: (x, y, z), False


def bounds(place, minor, branch, end_angle, cuts):
    origin = place(0.0, 0.0, 0.0)
    axis = tuple(b - a for a, b in zip(origin, place(0.0, 0.0, 1.0)))
    result = []
    for sign, extreme in ((-1.0, min), (1.0, max)):
        corner = []
        for center, axial in zip(origin, axis):
            radial = sign * math.sqrt(max(0.0, 1.0 - axial**2))
            critical = math.atan2(axial, radial * branch)
            angles = [critical + k * math.pi for k in range(-2, 3)]
            extrema = cuts + [arc(a, minor, branch) for a in angles if ARC_START < a < end_angle]
            corner.append(extreme(center + axial*z + radial*r for z, r in extrema))
        result.append(corner)
    return origin, axis, result


def stitch(lower, upper, triangles):
    # Merge the equally spaced azimuth lists. Each lower and upper edge is
    # consumed once, including the wraparound where indices return to zero.
    m, n = len(lower), len(upper)
    i = j = 0
    while i < m or j < n:
        a, b = lower[i % m], upper[j % n]
        if j == n or (i < m and (i + 1)*n <= (j + 1)*m):
            i += 1
            triangles.append((a, lower[i % m], b))
        else:
            j += 1
            triangles.append((a, upper[j % n], b))


def build(kind, branch, variant="plain", defect=None):
    minor, end_angle, cuts, families = nominal_profile(kind, branch, defect)
    arc_levels = [arc(ARC_START + (end_angle - ARC_START) * i / ARC_INTERVALS, minor, branch)
                  for i in range(ARC_INTERVALS)]
    if defect != "missing_ring":
        arc_levels.append(cuts[1])
    if defect == "bad_fit":
        i = ARC_INTERVALS // 2
        z, r = arc_levels[i]
        arc_levels[i] = (z, r + 0.004)
    levels = arc_levels + cuts[2:]
    place, flipped = placement(variant, levels[-1][0])
    vertices, rings, triangles = [], [], []
    for level, (z, r) in enumerate(levels):
        if variant == "mixed_azimuths":
            count = 362 if level == len(levels) - 1 else 360
            phase = math.pi - math.tau / 741
        else:
            count = AZIMUTHS
            phase = 0.0
        rings.append(list(range(len(vertices), len(vertices) + count)))
        for i in range(count):
            theta = phase + math.tau * i / count
            vertices.append(place(r * math.cos(theta), r * math.sin(theta), z))

    def cap(ring, z, lower):
        center = len(vertices)
        vertices.append(place(0.0, 0.0, z))
        for i, a in enumerate(ring):
            b = ring[(i + 1) % len(ring)]
            triangles.append((center, b, a) if lower else (center, a, b))

    cap(rings[0], levels[0][0], True)
    patch_triangles = [len(rings[0]), 0]
    for i, (lower, upper) in enumerate(zip(rings, rings[1:])):
        start = len(triangles)
        stitch(lower, upper, triangles)
        count = len(triangles) - start
        if i < len(arc_levels) - 1:
            patch_triangles[1] += count
        else:
            patch_triangles.append(count)
    cap(rings[-1], levels[-1][0], False)
    patch_triangles.append(len(rings[-1]))

    if defect == "crossing_triangle":
        # Exchange the diagonal between triangles on opposite sides of the
        # proposed boundary, retaining closed oriented manifold connectivity.
        seam = len(arc_levels) - 1
        before, mid, after = rings[seam - 1:seam + 2]
        old = [(before[1], mid[1], mid[0]), (mid[0], mid[1], after[0])]
        triangles[triangles.index(old[0])] = (before[1], mid[1], after[0])
        triangles[triangles.index(old[1])] = (before[1], after[0], mid[0])
    if flipped:
        triangles = [(c, b, a) for a, b, c in triangles]

    origin, axis, bbox = bounds(place, minor, branch, end_angle, cuts)
    spans = [[0.0, 0.0], *[[a, b] for (a, _), (b, _) in zip(cuts, cuts[1:])],
             [cuts[-1][0], cuts[-1][0]]]
    reference = dict(
        triangles=len(triangles), volume_mm3=patch_volume(minor, branch, end_angle, cuts),
        bounds_mm=bbox, origin_mm=origin, axis=axis, branch=branch, minor_mm=minor,
        sweep_rad=end_angle - ARC_START, rings_mm=cuts,
        patches=[dict(family=family, span_mm=span, triangles=count) for family, span, count
                 in zip(["plane", *families, "plane"], spans, patch_triangles)],
    )
    return vertices, triangles, reference


def write(name, vertices, triangles, *, ascii_stl=False):
    path = ROOT / f"{name}.stl"
    if ascii_stl:
        with path.open("w") as out:
            out.write(f"solid {name}\n")
            for triangle in triangles:
                out.write("facet normal 0 0 0\nouter loop\n")
                for index in triangle:
                    out.write("vertex " + " ".join(f"{v:.17g}" for v in vertices[index]) + "\n")
                out.write("endloop\nendfacet\n")
            out.write(f"endsolid {name}\n")
    else:
        with path.open("wb") as out:
            out.write(bytes(80))
            out.write(struct.pack("<I", len(triangles)))
            for triangle in triangles:
                coordinates = [x for index in triangle for x in vertices[index]]
                out.write(struct.pack("<12fH", 0.0, 0.0, 0.0, *coordinates, 0))
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    ROOT.mkdir(exist_ok=True)
    cases = {}
    for name, kind, branch, variant in [
        ("stepped", "stepped", 1, "plain"),
        ("stepped_mirror", "stepped", 1, "mirrored"),
        ("arc100", "arc100", 1, "plain"),
        ("arc100_mirror", "arc100", 1, "mirrored"),
        ("arc100_inward", "arc100", -1, "plain"),
        ("arc100_inward_mirror", "arc100", -1, "mirrored"),
        ("arc100_rotated", "arc100", 1, "rotated"),
        ("arc100_mixed_azimuths", "arc100", 1, "mixed_azimuths"),
    ]:
        vertices, triangles, reference = build(kind, branch, variant)
        cases[name] = dict(reference, sha256=write(name, vertices, triangles, ascii_stl=variant == "rotated"))
    negatives = {}
    for name, kind, reason in [
        ("missing_ring", "stepped", "True tangent ring omitted"),
        ("crossing_triangle", "arc100", "Flipped diagonal crosses the terminal torus ring"),
        ("bad_fit", "arc100", "One torus level shifted by 0.004 mm radially"),
        ("non_tangent", "arc100", "End cone slope changed without changing the torus"),
        ("unresolved_end", "arc100", "End cone is only 0.00003 mm high"),
    ]:
        vertices, triangles, _ = build(kind, 1, defect=name)
        negatives[name] = dict(reason=reason, triangles=len(triangles), sha256=write(name, vertices, triangles))
    (ROOT / "manifest.json").write_text(json.dumps(dict(
        unit="mm", major_mm=MAJOR, cases=cases, negative_cases=negatives,
    ), indent=2) + "\n")


if __name__ == "__main__":
    main()
