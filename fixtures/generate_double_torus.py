"""Independent C2a profile: two exact quarter circles and one cylindrical land.

References are fixed by elementary integration, without recognition or CAD output.
Only shared mesh/3MF serialization helpers are reused from existing generators.
"""
import hashlib
import json
import math
import struct
from pathlib import Path

import generate_m2b
import generate_m3
from generate_m2b import Mesh, transformed

ROOT = Path(__file__).resolve().parent / "double_torus"
RADIUS = 8.0
LOWER = 1.5
UPPER = 2.0
LAND = 5.0


def band(mesh, lower, upper):
    """Triangulate rings with independently chosen angular sampling."""
    i = j = 0
    n, m = len(lower), len(upper)
    while i < n or j < m:
        if i < n and (j == m or (i + 1) * m <= (j + 1) * n):
            mesh.triangle(lower[i % n], lower[(i + 1) % n], upper[j % m])
            i += 1
        else:
            mesh.triangle(lower[i % n], upper[(j + 1) % m], upper[j % m])
            j += 1


def build(*, lower=LOWER, upper=UPPER, land=LAND, count=64, intervals=12,
          irregular=False, witnesses=2, warped=False, tapered=False, seam_noise=False, cavity=False):
    profile = []
    for i in range(intervals + 1):
        v = -math.pi / 2 + math.pi * i / (2 * intervals)
        profile.append((lower + lower * math.sin(v), RADIUS - lower + lower * math.cos(v)))
    for i in range(1, witnesses + 2):
        t = i / (witnesses + 1)
        profile.append((lower + land * t, RADIUS + (0.03 * t if tapered else 0)))
    for i in range(1, intervals + 1):
        v = math.pi * i / (2 * intervals)
        r = RADIUS - upper + upper * math.cos(v)
        if warped:
            r += 0.05 * math.sin(2 * v)
        profile.append((lower + land + upper * math.sin(v), r))
    if seam_noise:
        # The two independently noisy end circles would prescribe different
        # cylinder radii if each fillet canonicalized its shared land alone.
        profile[0] = (profile[0][0], profile[0][1] + 0.000003)
        profile[-1] = (profile[-1][0], profile[-1][1] - 0.000003)
    if land == 0:
        profile = [p for i, p in enumerate(profile) if i == 0 or p != profile[i - 1]]
    mesh = Mesh()
    rings = []
    for level, (z, r) in enumerate(profile):
        n = count + (8 if irregular and level % 3 == 1 else 0)
        rings.append(mesh.ring([(r * math.cos(math.tau * i / n), r * math.sin(math.tau * i / n)) for i in range(n)], z))
    if not cavity:
        mesh.fan(rings[0], (0., 0., profile[0][0]), True)
    for a, b in zip(rings, rings[1:]):
        first = len(mesh.faces)
        band(mesh, a, b)
        if cavity:
            mesh.faces[first:] = [(c, b, a) for a, b, c in mesh.faces[first:]]
    if cavity:
        outline = [(12. * math.cos(math.tau * i / count), 12. * math.sin(math.tau * i / count)) for i in range(count)]
        bottom = mesh.ring(outline, profile[0][0])
        top = mesh.ring(outline, profile[-1][0])
        mesh.annulus(bottom, rings[0], True)
        mesh.annulus(top, rings[-1])
        mesh.wall(bottom, top)
    else:
        mesh.fan(rings[-1], (0., 0., profile[-1][0]))
    return mesh


def volume(lower=LOWER, upper=UPPER, land=LAND):
    def end(q):
        major = RADIUS - q
        return math.pi * (major * major * q + math.pi * major * q * q / 2 + 2 * q**3 / 3)
    return math.pi * RADIUS**2 * land + end(lower) + end(upper)


def reference(lower=LOWER, upper=UPPER, land=LAND, rotate=False):
    center = transformed((0., 0., 0.)) if rotate else (0., 0., 0.)
    axis = tuple(b - a for a, b in zip(center, transformed((0., 0., 1.)))) if rotate else (0., 0., 1.)
    arcs = [(RADIUS - lower, lower, lower, -math.pi / 2, 0.),
            (RADIUS - upper, upper, lower + land, 0., math.pi / 2)]
    # Extrema of each coordinate on exact circular arcs, including interior
    # stationary points. Tessellated bounds are deliberately not the reference.
    bounds = []
    for sign, choose in [(-1., min), (1., max)]:
        bound = []
        for c, axial in zip(center, axis):
            radial = sign * math.sqrt(max(0., 1. - axial * axial))
            values = [c + axial * z + radial * RADIUS for z in (lower, lower + land)]
            for major, minor, cz, first, last in arcs:
                angles = [first, last]
                stationary = math.atan2(axial, radial)
                angles.extend(stationary + k * math.pi for k in range(-2, 3) if first <= stationary + k * math.pi <= last)
                values.extend(c + axial * (cz + minor * math.sin(v)) + radial * (major + minor * math.cos(v)) for v in angles)
            bound.append(choose(values))
        bounds.append(bound)
    return dict(faces=5, planes=2, cylinders=1, cones=0, spheres=0, tori=2,
                toroidal_fillets=2, radius_mm=RADIUS, land_height_mm=land,
                volume_mm3=volume(lower, upper, land), bounds_mm=bounds, axis=axis,
                tori_parameters=[dict(major_radius_mm=m, minor_radius_mm=q,
                                      center_mm=[c + a * z for c, a in zip(center, axis)],
                                      meridian_rad=[v0, v1]) for m, q, z, v0, v1 in arcs])


def main():
    ROOT.mkdir(exist_ok=True)
    generate_m2b.ROOT = generate_m3.ROOT = ROOT
    cases = {}
    for name, kwargs in [("nominal", {}), ("equal_radii", dict(lower=2.)),
                         ("rotated", {}), ("irregular", dict(irregular=True)),
                         ("seam_noise", dict(seam_noise=True)),
                         ("dense", dict(count=96, intervals=18))]:
        mesh = build(**kwargs)
        mesh.write(name, name == "rotated")
        cases[name] = reference(lower=kwargs.get("lower", LOWER), rotate=name == "rotated")
        cases[name]["triangles"] = len(mesh.faces)
        if name == "seam_noise":
            cases[name]["terminal_radial_noise_mm"] = [0.000003, -0.000003]
    nominal = build()
    with (ROOT / "binary.stl").open("wb") as out:
        out.write(bytes(80))
        out.write(struct.pack("<I", len(nominal.faces)))
        for face in nominal.faces:
            out.write(struct.pack("<12fH", 0., 0., 0., *(x for vertex in face for x in nominal.vertices[vertex]), 0))
    cases["binary"] = dict(reference(), binary_f32=True, triangles=len(nominal.faces))
    obj = generate_m3.mesh_object(1, nominal.vertices, nominal.faces, "Two measured toroidal end fillets")
    height = LOWER + LAND + UPPER
    generate_m3.write("instances", generate_m3.model([obj], [(1, None), (1, f"1 0 0 0 1 0 0 0 -1 24 0 {height}")]))
    cases["instances"] = dict(format="3mf", bodies=2, volume_mm3=2 * volume(),
                               body_bounds_mm=[[[-8., -8., 0.], [8., 8., height]], [[16., -8., 0.], [32., 8., height]]])
    generate_m3.write("touching", generate_m3.model([obj], [(1, None), (1, generate_m3.translate(16., 0., 0.))]))
    negatives = {}
    for name, kwargs, reason in [
        ("no_witness", dict(witnesses=0), "No interior source ring on the central land"),
        ("one_witness", dict(witnesses=1), "Only one interior source ring on the central land"),
        ("short_land", dict(land=0.00006), "Central land unresolved at the default source uncertainty"),
        ("no_land", dict(land=0.), "The two circular arcs meet without a cylindrical separator"),
        ("warped_arc", dict(warped=True), "Noncircular meridian with unchanged terminal circles"),
        ("tapered_land", dict(tapered=True), "Incompatible radii on the claimed common cylindrical land"),
        ("sparse_meridian", dict(intervals=2), "Only three distinct heights per quarter circle"),
        ("sparse_azimuth", dict(count=8), "Eight samples cannot establish the planar circular contours"),
        ("concave", dict(cavity=True), "Two toroidal fillets bound a source through-cavity, outside the exterior C2a contract"),
    ]:
        mesh = build(**kwargs)
        mesh.write(name)
        negatives[name] = dict(reason=reason, surfaces_error="recognition.unsupported" if name == "sparse_azimuth" else "recognition.surface_unproven")
        negatives[name]["app_surfaces_error"] = "recognition.surface_unproven"
    for name, case in {**cases, **negatives}.items():
        case["sha256"] = hashlib.sha256((ROOT / (name + "." + case.get("format", "stl"))).read_bytes()).hexdigest()
    (ROOT / "manifest.json").write_text(json.dumps(dict(unit="mm", construction="Full-azimuth exact quarter circles; R8 cylinder with two interior rings; elementary disk integral fixed before recognition", cases=cases, negative_cases=negatives, touching_error="cad.bodies_separation_budget", touching_refusal_scope="Source separation budget is exhausted before the geometric contact classification; the whole document is refused without partial export"), indent=2) + "\n")


if __name__ == "__main__":
    main()
