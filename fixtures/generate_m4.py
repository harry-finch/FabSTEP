"""Independent synthetic cones and chamfers, with analytical references."""
import json
import math
from pathlib import Path

import generate_m3
from generate_m2b import Mesh, boundary, transformed

ROOT = Path(__file__).resolve().parent
BASE_VOLUME = 30.0 * 20.0 * 10.0


def frustum_volume(lower, upper, height):
    return math.pi * height * (lower**2 + lower * upper + upper**2) / 3.0


def cross_sections(mesh, profile, angles, *, shifted_apex=False, warped=False):
    sections = []
    for j, (z, radius) in enumerate(profile):
        if radius == 0.0:
            sections.append([mesh.point((0.15 if shifted_apex else 0.0, 0.0, z))])
            continue
        distortion = 0.15 if warped and 0 < j < len(profile) - 1 else 0.0
        radii = [radius + distortion * math.cos(3 * angle) for angle in angles]
        points = [(r * math.cos(a), r * math.sin(a)) for a, r in zip(angles, radii)]
        sections.append(mesh.ring(points, z))
    return sections


def connect(mesh, sections, *, inner=False):
    for lower, upper in zip(sections, sections[1:]):
        if len(lower) == 1:
            for i, vertex in enumerate(upper):
                mesh.triangle(lower[0], upper[(i + 1) % len(upper)], vertex, inner)
        elif len(upper) == 1:
            for i, vertex in enumerate(lower):
                mesh.triangle(vertex, lower[(i + 1) % len(lower)], upper[0], inner)
        else:
            mesh.wall(lower, upper, inner)


def subdivide(profile, levels):
    result = []
    for (z0, r0), (z1, r1) in zip(profile, profile[1:]):
        for index in range(levels):
            t = index / levels
            result.append((z0 + (z1 - z0) * t, r0 + (r1 - r0) * t))
    return result + [profile[-1]]


def standalone(name, profile, *, levels=1, samples=48, rotate=False, shifted_apex=False, warped=False):
    mesh = Mesh()
    angles = [math.tau * i / samples for i in range(samples)]
    sections = cross_sections(
        mesh, subdivide(profile, levels), angles, shifted_apex=shifted_apex, warped=warped
    )
    connect(mesh, sections)
    for (z, radius), ring, reverse in [(profile[0], sections[0], True), (profile[-1], sections[-1], False)]:
        if radius > 0.0:
            mesh.fan(ring, (0.0, 0.0, z), reverse)
    mesh.write(name, rotate)
    return mesh, angles


def box_feature(name, profile, *, inner=False, through=False, levels=1, rotate=False):
    mesh = Mesh()
    perimeter = boundary(-15.0, 15.0, -10.0, 10.0, 16)
    angles = [math.atan2(y, x) for x, y in perimeter]
    lower = mesh.ring(perimeter, 0.0)
    upper = mesh.ring(perimeter, 10.0)
    mesh.wall(lower, upper)
    sections = cross_sections(mesh, subdivide(profile, levels), angles)
    if inner:
        mesh.annulus(upper, sections[-1])
        if through:
            mesh.annulus(lower, sections[0], True)
        else:
            mesh.fan(lower, (0.0, 0.0, 0.0), True)
    else:
        mesh.fan(lower, (0.0, 0.0, 0.0), True)
        mesh.annulus(upper, sections[0])
        mesh.fan(sections[-1], (0.0, 0.0, profile[-1][0]))
    connect(mesh, sections, inner=inner)
    mesh.write(name, rotate)
    return mesh, angles


def planar_chamfer(name):
    mesh = Mesh()
    polygon = [(0.0, 0.0), (30.0, 0.0), (30.0, 17.0), (27.0, 20.0), (0.0, 20.0)]
    lower = mesh.ring(polygon, 0.0)
    upper = mesh.ring(polygon, 10.0)
    mesh.fan(lower, (15.0, 10.0, 0.0), True)
    mesh.fan(upper, (15.0, 10.0, 10.0))
    mesh.wall(lower, upper)
    mesh.write(name)
    return mesh


def analytic_bounds(profile, rotate=False, box=False):
    axis = tuple(a - b for a, b in zip(transformed((0, 0, 1)), transformed((0, 0, 0)))) if rotate else (0, 0, 1)
    trans = transformed if rotate else lambda p: p
    points = []
    if box:
        points.extend(trans((x, y, z)) for x in (-15, 15) for y in (-10, 10) for z in (0, 10))
    for z, radius in profile:
        center = trans((0.0, 0.0, z))
        widths = [radius * math.sqrt(max(0.0, 1.0 - a * a)) for a in axis]
        points.extend(tuple(x + sign * w for x, w in zip(center, widths)) for sign in (-1, 1))
    return [list(map(fn, zip(*points))) for fn in (min, max)], list(axis)


def reference(mesh, profile, angles, *, planes, rotate=False, box=False, inner=False):
    bounds, axis = analytic_bounds(profile, rotate, box)
    cylinders, cones = [], []
    for (z0, r0), (z1, r1) in zip(profile, profile[1:]):
        height = z1 - z0
        if r0 == r1:
            cylinders.append({"height_mm": height, "radius_mm": r0})
        else:
            cones.append({
                "height_mm": height, "end_radii_mm": sorted([r0, r1]),
                "semi_angle_deg": math.degrees(math.atan2(abs(r1 - r0), height)),
            })
    volume = math.fsum(frustum_volume(r0, r1, z1 - z0) for (z0, r0), (z1, r1) in zip(profile, profile[1:]))
    gap = max((b - a) % math.tau for a, b in zip(angles, angles[1:] + angles[:1]))
    return {
        "triangles": len(mesh.faces), "planes": planes, "faces": planes + len(cylinders) + len(cones),
        "cylinders": cylinders, "cones": cones, "axis": axis, "bounds_mm": bounds,
        "volume_mm3": (BASE_VOLUME if box else 0.0) + (-volume if inner else volume),
        "max_chord_error_mm": max(radius for _, radius in profile) * (1.0 - math.cos(gap / 2)),
    }


def main():
    cases = {}
    for name, profile, levels, samples, rotated in [
        ("m4_frustum", [(0.0, 7.0), (8.0, 3.0)], 1, 48, False),
        ("m4_frustum_rotated", [(0.0, 7.0), (8.0, 3.0)], 3, 48, True),
        ("m4_cone_tip", [(0.0, 6.0), (9.0, 0.0)], 2, 48, False),
        ("m4_chamfered_cylinder", [(0.0, 5.0), (8.0, 5.0), (10.0, 3.0)], 1, 32, False),
    ]:
        mesh, angles = standalone(name, profile, levels=levels, samples=samples, rotate=rotated)
        planes = sum(profile[i][1] > 0 for i in (0, -1))
        cases[name] = reference(mesh, profile, angles, planes=planes, rotate=rotated)
        if name == "m4_frustum":
            resource = generate_m3.mesh_object(1, mesh.vertices, mesh.faces, "Cône tronqué")
            for file, translation in [
                ("m4_frustum_instances", (30.0, 10.0, 2.0)),
                ("m4_frustum_touching", (14.0, 0.0, 0.0)),
            ]:
                items = [(1, None), (1, generate_m3.translate(*translation))]
                generate_m3.write(file, generate_m3.model([resource], items))
                bounds = cases[name]["bounds_mm"]
                cases[file] = {
                    "format": "3mf", "triangles": 2 * len(mesh.faces),
                    "bodies": [name, name],
                    "body_bounds_mm": [bounds, [[p + t for p, t in zip(corner, translation)] for corner in bounds]],
                }
            cases["m4_frustum_touching"]["expected_refusal"] = "contact entre les deux bases circulaires"
        if name == "m4_chamfered_cylinder":
            translation = (11.0, -7.0, 4.0)
            resource = generate_m3.mesh_object(1, mesh.vertices, mesh.faces, "Cylindre chanfreiné")
            generate_m3.write(
                "m4_chamfered_cylinder_placed",
                generate_m3.model([resource], [(1, generate_m3.translate(*translation))]),
            )
            cases["m4_chamfered_cylinder_placed"] = {
                **cases[name], "format": "3mf",
                "bounds_mm": [[p + t for p, t in zip(corner, translation)] for corner in cases[name]["bounds_mm"]],
            }
    for name, profile, inner, through in [
        ("m4_chamfered_boss", [(10.0, 5.0), (15.0, 5.0), (17.0, 3.0)], False, False),
        ("m4_countersink", [(0.0, 3.0), (7.0, 3.0), (10.0, 6.0)], True, True),
        ("m4_drill_tip", [(3.0, 0.0), (5.0, 3.0), (10.0, 3.0)], True, False),
    ]:
        mesh, angles = box_feature(name, profile, inner=inner, through=through, levels=2)
        cases[name] = reference(mesh, profile, angles, planes=6 if inner else 7, box=True, inner=inner)
    mesh = planar_chamfer("m4_planar_chamfer")
    cases["m4_planar_chamfer"] = {
        "triangles": len(mesh.faces), "planes": 7, "faces": 7, "cylinders": [], "cones": [],
        "bounds_mm": [[0.0, 0.0, 0.0], [30.0, 20.0, 10.0]], "volume_mm3": 5955.0,
        "mode": "planar",
    }
    for name, profile, options, reason in [
        ("m4_sparse_cone", [(0.0, 6.0), (9.0, 0.0)], {"samples": 8}, "anneau de huit sommets"),
        ("m4_shifted_tip", [(0.0, 6.0), (9.0, 0.0)], {"shifted_apex": True}, "pointe hors de l'axe"),
        ("m4_warped_frustum", [(0.0, 7.0), (8.0, 3.0)], {"warped": True, "levels": 3},
         "paroi intermédiaire à trois lobes"),
        ("m4_piecewise_taper", [(0.0, 7.0), (4.0, 6.0), (8.0, 3.0)], {},
         "deux pentes coniques contiguës, hors périmètre"),
        ("m4_curved_meridian", [(float(z), math.sqrt(49.0 - z * z)) for z in range(7)], {},
         "méridienne sphérique à sept anneaux coaxiaux"),
        ("m4_alternating_taper",
         [(0.0, 7.0), (1.0, 6.6), (1.00006, 6.59999), (2.0, 6.3), (2.00006, 6.29999), (3.0, 5.9)],
         {}, "pentes distinctes avec deux transitions coniques très courtes et presque cylindriques"),
    ]:
        mesh, _ = standalone(name, profile, **options)
        cases[name] = {"triangles": len(mesh.faces), "expected_refusal": reason}
    (ROOT / "m4_manifest.json").write_text(
        json.dumps({"unit": "mm", "cases": cases}, ensure_ascii=False, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
