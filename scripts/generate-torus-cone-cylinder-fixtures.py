"""Generate independent R-075 meshes and exact profile references in millimeters."""

import hashlib
import json
import math
from pathlib import Path
import struct
import sys

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"
sys.path.insert(0, str(FIXTURES))
import generate_m3
from generate_m2b import Mesh, transformed

ROOT = FIXTURES / "torus_cone_cylinder"
MAJOR = 10.0
MINOR = 0.5
SLOPE = 1.0 / 15.0
SWEEP = math.atan(1.0 / SLOPE)
CONE_HEIGHT = 2.0
HEIGHT = 5.0
AZIMUTHS = 64
ARC_INTERVALS = 12


def arc(theta, branch):
    return MINOR * (1.0 - math.cos(theta)), MAJOR + branch * MINOR * math.sin(theta)


def straight_ends(branch, slope_offset=0.0):
    z, radius = arc(SWEEP, branch)
    return (z + CONE_HEIGHT, radius + (branch * SLOPE + slope_offset) * CONE_HEIGHT), (
        HEIGHT,
        radius + (branch * SLOPE + slope_offset) * CONE_HEIGHT,
    )


def build(branch=-1, variant="nominal"):
    intervals = range(ARC_INTERVALS + (0 if variant == "missing_ring" else 1))
    profile = [arc(SWEEP * i / ARC_INTERVALS, branch) for i in intervals]
    profile.extend(straight_ends(branch, 0.025 if variant == "non_tangent" else 0.0))
    mesh = Mesh()
    rings = []
    for level, (z, radius) in enumerate(profile):
        if variant == "warped" and level == ARC_INTERVALS // 2:
            radius += 0.002
        points = []
        for i in range(AZIMUTHS):
            angle = math.tau * i / AZIMUTHS
            r = radius
            if variant == "seam_noise" and level == 0:
                r += 0.000002 * (-1) ** i
            points.append((r * math.cos(angle), r * math.sin(angle)))
        rings.append(mesh.ring(points, z))
    bore = variant == "bore"
    if bore:
        outline = [
            (12.0 * math.cos(math.tau * i / AZIMUTHS), 12.0 * math.sin(math.tau * i / AZIMUTHS))
            for i in range(AZIMUTHS)
        ]
        bottom = mesh.ring(outline, 0.0)
        top = mesh.ring(outline, HEIGHT)
        mesh.annulus(bottom, rings[0], True)
        mesh.annulus(top, rings[-1])
        mesh.wall(bottom, top)
    else:
        mesh.fan(rings[0], (0.0, 0.0, 0.0), True)
    for lower, upper in zip(rings, rings[1:]):
        mesh.wall(lower, upper, reverse=bore)
    if not bore:
        mesh.fan(rings[-1], (0.0, 0.0, HEIGHT))
    if variant == "crossing_triangle":
        before, seam, after = rings[ARC_INTERVALS - 1 : ARC_INTERVALS + 2]
        # Flip a single seam edge. The two changed triangles span three levels
        # collectively, and the new diagonal crosses the tangent boundary.
        previous = AZIMUTHS + (ARC_INTERVALS - 1) * 2 * AZIMUTHS
        following = previous + 2 * AZIMUTHS
        mesh.faces[previous + 1] = (before[0], seam[1], after[1])
        mesh.faces[following] = (before[0], after[1], seam[0])
    if variant == "mirrored":
        mesh.vertices = [(x, y, HEIGHT - z) for x, y, z in mesh.vertices]
        mesh.faces = [(c, b, a) for a, b, c in mesh.faces]
    if variant == "rotated":
        # Preserve the terminal planes exactly through binary STL quantization.
        mesh.vertices = [(y, z + 8.0, x) for x, y, z in mesh.vertices]
    if variant == "rotated_f32_nonplanar":
        mesh.vertices = [transformed(p) for p in mesh.vertices]
    return mesh


def exact_volume(branch):
    # Integral of πρ² dz, with dz = r sin(θ) dθ on the arc.
    sin_theta, cos_theta = math.sin(SWEEP), math.cos(SWEEP)
    first = 1.0 - cos_theta
    second = (SWEEP - sin_theta * cos_theta) / 2.0
    third = 2.0 / 3.0 - cos_theta + cos_theta**3 / 3.0
    arc_volume = math.pi * MINOR * (
        MAJOR**2 * first + 2.0 * branch * MAJOR * MINOR * second + MINOR**2 * third
    )
    tangent_z, tangent_radius = arc(SWEEP, branch)
    (cone_z, radius), _ = straight_ends(branch)
    assert cone_z == tangent_z + CONE_HEIGHT
    cone_volume = math.pi * CONE_HEIGHT * (tangent_radius**2 + tangent_radius * radius + radius**2) / 3.0
    return arc_volume + cone_volume + math.pi * radius**2 * (HEIGHT - cone_z)


def reference(branch=-1, variant="nominal"):
    origin, axis = (0.0, 0.0, 0.0), (0.0, 0.0, 1.0)
    if variant == "mirrored":
        origin, axis = (0.0, 0.0, HEIGHT), (0.0, 0.0, -1.0)
    if variant == "rotated":
        origin, axis = (0.0, 8.0, 0.0), (0.0, 1.0, 0.0)
    if variant == "rotated_oblique":
        origin = transformed(origin)
        axis = tuple(b - a for a, b in zip(origin, transformed(axis)))
    bounds = []
    for sign, extremum in [(-1.0, min), (1.0, max)]:
        limit = []
        for c, axial in zip(origin, axis):
            radial = sign * math.sqrt(max(0.0, 1.0 - axial**2))
            angles = [0.0, SWEEP]
            stationary = math.atan2(-branch * radial, axial)
            angles.extend(
                theta for k in range(-2, 3)
                if 0.0 < (theta := stationary + k * math.pi) < SWEEP
            )
            extrema = [arc(theta, branch) for theta in angles] + list(straight_ends(branch))
            limit.append(extremum(c + axial * z + radial * radius for z, radius in extrema))
        bounds.append(limit)
    return dict(
        volume_mm3=exact_volume(branch),
        bounds_mm=bounds,
        origin_mm=origin,
        axis=axis,
        branch=branch,
        planes=2,
        tori=1,
        cones=1,
        cylinders=1,
        triangles_per_face=dict(plane=AZIMUTHS, torus=2 * AZIMUTHS * ARC_INTERVALS,
                                cone=2 * AZIMUTHS, cylinder=2 * AZIMUTHS),
        rings_mm=[(0.0, MAJOR), arc(SWEEP, branch), *straight_ends(branch)],
    )


def write_stl(name, mesh):
    with (ROOT / f"{name}.stl").open("wb") as out:
        out.write(bytes(80))
        out.write(struct.pack("<I", len(mesh.faces)))
        for face in mesh.faces:
            coordinates = [x for index in face for x in mesh.vertices[index]]
            out.write(struct.pack("<12fH", 0.0, 0.0, 0.0, *coordinates, 0))


def main():
    ROOT.mkdir(exist_ok=True)
    generate_m3.ROOT = ROOT
    cases = {}
    for name, branch in [
        ("nominal", -1), ("mirrored", -1), ("rotated", -1),
        ("opposite_branch", 1), ("seam_noise", -1),
    ]:
        mesh = build(branch, name)
        write_stl(name, mesh)
        cases[name] = dict(reference(branch, name), triangles=len(mesh.faces))
    mesh = build(variant="bore")
    write_stl("bore", mesh)
    cases["bore"] = dict(
        reference(),
        volume_mm3=math.pi * 12.0**2 * HEIGHT - exact_volume(-1),
        bounds_mm=[[-12.0, -12.0, 0.0], [12.0, 12.0, HEIGHT]],
        cylinders=2,
        triangles_per_face=dict(plane=2 * AZIMUTHS, torus=2 * AZIMUTHS * ARC_INTERVALS,
                                cone=2 * AZIMUTHS, cylinder=2 * AZIMUTHS),
        triangles=len(mesh.faces),
    )
    negatives = {}
    for name, reason in [
        ("non_tangent", "Cone slope changed without moving the source torus"),
        ("missing_ring", "The true tangent level is omitted from the source"),
        ("crossing_triangle", "A flipped edge spans the tangent boundary"),
        ("warped", "One arc level has a 0.002 mm radial offset"),
    ]:
        mesh = build(variant=name)
        write_stl(name, mesh)
        negatives[name] = dict(reason=reason, triangles=len(mesh.faces))
    nominal = build()
    obj = generate_m3.mesh_object(1, nominal.vertices, nominal.faces, "Torus, cone and cylinder")
    origin = transformed((0.0, 0.0, 0.0))
    vectors = [
        tuple(x - t for x, t in zip(transformed(basis), origin))
        for basis in [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)]
    ]
    # 3MF applies the rigid transformation after reading full-precision coordinates.
    placement = " ".join(str(value) for row in [*vectors, origin] for value in row)
    generate_m3.write("rotated_oblique", generate_m3.model([obj], [(1, placement)]))
    cases["rotated_oblique"] = dict(
        reference(variant="rotated_oblique"), format="3mf", triangles=len(nominal.faces)
    )
    generate_m3.write(
        "instances", generate_m3.model([obj], [(1, None), (1, "1 0 0 0 1 0 0 0 -1 28 3 12")])
    )
    cases["instances"] = dict(
        format="3mf",
        bodies=2,
        triangles=2 * len(nominal.faces),
        volume_mm3=2 * exact_volume(-1),
        body_bounds_mm=[reference()["bounds_mm"], [[18.0, -7.0, 7.0], [38.0, 13.0, 12.0]]],
    )
    mesh = build(variant="rotated_f32_nonplanar")
    write_stl("rotated_f32_nonplanar", mesh)
    limitations = {
        "rotated_f32_nonplanar": dict(
            triangles=len(mesh.faces),
            reason="Oblique rotation rounded to binary STL f32 exceeds native circular boundary coplanarity tolerance",
            surfaces_error="cad.invalid_model",
        )
    }
    for name, entry in (cases | negatives | limitations).items():
        suffix = entry.get("format", "stl")
        entry["sha256"] = hashlib.sha256((ROOT / f"{name}.{suffix}").read_bytes()).hexdigest()
    (ROOT / "manifest.json").write_text(
        json.dumps(
            dict(unit="mm", major_mm=MAJOR, minor_mm=MINOR, sweep_rad=SWEEP, cases=cases,
                 negative_cases=negatives, limitations=limitations, insufficient_deviation_mm=0.001),
            indent=2,
        ) + "\n"
    )


if __name__ == "__main__":
    main()
