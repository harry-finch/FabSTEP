"""Validation indépendante d'un STEP avec le Python et le module Part de FreeCAD.

macOS : PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python scripts/validate-freecad.py fichier.step
"""
import argparse
import json
import math

import FreeCAD
import Part

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("step")
parser.add_argument("--solids", type=int, default=1)
parser.add_argument("--faces", type=int)
parser.add_argument("--cylinders", type=int, help="Nombre attendu de faces cylindriques analytiques")
parser.add_argument("--cones", type=int, help="Nombre attendu de faces coniques analytiques")
parser.add_argument("--spheres", type=int, help="Nombre attendu de faces sphériques analytiques")
parser.add_argument("--tori", type=int, help="Nombre attendu de faces toriques analytiques")
parser.add_argument("--toroidal-fillets", type=int, help="Nombre attendu de raccords toriques tangents")
parser.add_argument("--planes", type=int, help="Nombre attendu de faces planes analytiques")
parser.add_argument("--volume", type=float)
parser.add_argument("--radii", type=float, nargs="+", help="Rayons des cylindres, sans ordre imposé")
parser.add_argument("--heights", type=float, nargs="+", help="Hauteurs/profondeurs des surfaces cylindriques")
parser.add_argument("--angles-deg", type=float, nargs="+", help="Balayages angulaires des surfaces cylindriques en degrés")
parser.add_argument("--cone-end-radii", type=float, nargs="+", help="Rayons aux deux extrémités par cône")
parser.add_argument("--cone-heights", type=float, nargs="+", help="Hauteurs axiales des surfaces coniques")
parser.add_argument("--cone-angles-deg", type=float, nargs="+", help="Demi-angles des cônes en degrés")
parser.add_argument("--sphere-radii", type=float, nargs="+", help="Rayons des sphères")
parser.add_argument("--torus-radii", type=float, nargs="+", help="Rayons majeur et mineur par tore")
parser.add_argument("--axis", type=float, nargs=3, help="Axe commun des surfaces de révolution, au signe près")
parser.add_argument("--dimensions", type=float, nargs=3)
parser.add_argument("--bounds", type=float, nargs=6, help="Boîte mondiale xmin ymin zmin xmax ymax zmax")
args = parser.parse_args()
if args.cone_end_radii is not None and len(args.cone_end_radii) % 2:
    parser.error("--cone-end-radii attend deux rayons par cône")
if args.torus_radii is not None and len(args.torus_radii) % 2:
    parser.error("--torus-radii attend deux rayons par tore")
shape = Part.Shape()
shape.read(args.step)
faces = shape.Faces
surfaces = [face.Surface for face in faces]
cylinder_faces = [face for face in faces if isinstance(face.Surface, Part.Cylinder)]
cylinders = [face.Surface for face in cylinder_faces]
cone_faces = [face for face in faces if isinstance(face.Surface, Part.Cone)]
sphere_faces = [face for face in faces if isinstance(face.Surface, Part.Sphere)]
torus_faces = [face for face in faces if isinstance(face.Surface, Part.Toroid)]
planes = [surface for surface in surfaces if isinstance(surface, Part.Plane)]


def vector(v):
    return [v.x, v.y, v.z]


def shape_bounds(s):
    # The regular box can include material outside a trimmed toroidal patch.
    use_optimal = args.bounds is not None or bool(sphere_faces or torus_faces)
    b = s.optimalBoundingBox(False, False) if use_optimal else s.BoundBox
    return [[b.XMin, b.YMin, b.ZMin], [b.XMax, b.YMax, b.ZMax]]


bounds = shape_bounds(shape)
dimensions = [b - a for a, b in zip(*bounds)]


def cone_measurements(face):
    surface = face.Surface
    u0, u1, v0, v1 = face.ParameterRange
    sine = math.sin(surface.SemiAngle)
    return {
        "axis": vector(surface.Axis),
        "center_mm": vector(surface.Center),
        "reference_radius_mm": surface.Radius,
        "end_radii_mm": sorted([abs(surface.Radius + v * sine) for v in (v0, v1)]),
        "height_mm": abs((v1 - v0) * math.cos(surface.SemiAngle)),
        "semi_angle_deg": math.degrees(abs(surface.SemiAngle)),
        "angular_span_rad": abs(u1 - u0),
    }


def same_values(actual, expected, absolute):
    actual, expected = sorted(actual), sorted(expected)
    return len(actual) == len(expected) and all(
        math.isclose(a, b, rel_tol=1e-8, abs_tol=absolute) for a, b in zip(actual, expected)
    )


def spherical_measurements(face):
    surface = face.Surface
    u0, u1, v0, v1 = face.ParameterRange
    u, v = (u0 + u1) / 2, (v0 + v1) / 2
    radial = face.valueAt(u, v) - surface.Center
    return {
        "axis": vector(surface.Axis),
        "center_mm": vector(surface.Center),
        "radius_mm": surface.Radius,
        "latitudes_deg": [math.degrees(v0), math.degrees(v1)],
        "height_mm": surface.Radius * (math.sin(v1) - math.sin(v0)),
        "inner": face.normalAt(u, v).dot(radial) < 0,
        "angular_span_rad": abs(u1 - u0),
    }


def toroidal_measurements(face):
    surface = face.Surface
    u0, u1, v0, v1 = face.ParameterRange
    connections = []
    for adjacent in faces:
        if adjacent.isSame(face):
            continue
        other_edges = adjacent.Edges
        shared = [edge for edge in face.Edges if any(edge.isSame(other) for other in other_edges)]
        if not shared:
            continue
        # Sample opposite sides of each shared edge as an independent
        # check of the normals on both underlying analytic surfaces.
        normal_dots = []
        for edge in shared:
            for fraction in (0.25, 0.75):
                p = edge.valueAt(edge.FirstParameter + fraction * (edge.LastParameter - edge.FirstParameter))
                a = face.normalAt(*surface.parameter(p))
                b = adjacent.normalAt(*adjacent.Surface.parameter(p))
                normal_dots.append(a.dot(b))
        connections.append({
            "surface": type(adjacent.Surface).__name__,
            "edge_count": len(shared),
            "minimum_normal_dot": min(normal_dots),
            "minimum_abs_normal_dot": min(map(abs, normal_dots)),
        })
    return {
        "axis": vector(surface.Axis),
        "center_mm": vector(surface.Center),
        "major_radius_mm": surface.MajorRadius,
        "minor_radius_mm": surface.MinorRadius,
        "meridian_deg": [math.degrees(v0), math.degrees(v1)],
        "angular_span_rad": abs(u1 - u0),
        "connections": connections,
        "tangent_fillet": len(connections) == 2
        and {c["surface"] for c in connections} == {"Cylinder", "Plane"}
        and all(c["minimum_normal_dot"] >= 1 - 1e-8 for c in connections),
    }


report = {
    "file": args.step,
    "freecad": ".".join(FreeCAD.Version()[:3]),
    "valid": shape.isValid(),
    "closed": shape.isClosed(),
    "solids": len(shape.Solids),
    "faces": len(shape.Faces),
    "volume_mm3": shape.Volume,
    "dimensions_mm": dimensions,
    "bounds_mm": bounds,
    "surface_types": sorted({type(face.Surface).__name__ for face in shape.Faces}),
    "planes": len(planes),
    "horizontal_plane_z_mm": sorted({
        round(plane.Position.z, 9) for plane in planes if abs(abs(plane.Axis.z) - 1.0) < 1e-10
    }),
    "cylinders": [{"radius_mm": surface.Radius, "axis": vector(surface.Axis),
                   "center_mm": vector(face.Surface.Center),
                   "height_mm": abs(face.ParameterRange[3]-face.ParameterRange[2]),
                   "angular_span_rad": abs(face.ParameterRange[1]-face.ParameterRange[0])}
                  for face in cylinder_faces for surface in [face.Surface]],
    "cones": [cone_measurements(face) for face in cone_faces],
    "spheres": [spherical_measurements(face) for face in sphere_faces],
    "tori": [toroidal_measurements(face) for face in torus_faces],
}
report["bodies"] = [
    {
        "valid": s.isValid(), "closed": s.isClosed(), "volume_mm3": s.Volume,
        "faces": len(s.Faces), "bounds_mm": shape_bounds(s),
        "planes": sum(isinstance(face.Surface, Part.Plane) for face in s.Faces),
        "cylinders": sum(isinstance(face.Surface, Part.Cylinder) for face in s.Faces),
        "cones": sum(isinstance(face.Surface, Part.Cone) for face in s.Faces),
        "spheres": sum(isinstance(face.Surface, Part.Sphere) for face in s.Faces),
        "tori": sum(isinstance(face.Surface, Part.Toroid) for face in s.Faces),
    }
    for s in shape.Solids
]
ok = report["valid"] and report["closed"] and report["solids"] == args.solids and shape.Volume > 0
if args.cylinders is not None:
    ok = ok and len(cylinders) == args.cylinders
if args.cones is not None:
    ok = ok and len(cone_faces) == args.cones and all(
        math.isclose(c["angular_span_rad"], math.tau, rel_tol=1e-8) for c in report["cones"]
    )
if args.spheres is not None:
    ok = ok and len(sphere_faces) == args.spheres and all(
        math.isclose(c["angular_span_rad"], math.tau, rel_tol=1e-8) for c in report["spheres"]
    )
if args.tori is not None:
    ok = ok and len(torus_faces) == args.tori and all(
        math.isclose(c["angular_span_rad"], math.tau, rel_tol=1e-8) for c in report["tori"]
    )
if args.toroidal_fillets is not None:
    ok = ok and sum(t["tangent_fillet"] for t in report["tori"]) == args.toroidal_fillets
if args.planes is not None:
    ok = ok and len(planes) == args.planes
if args.faces is not None:
    ok = ok and len(shape.Faces) == args.faces
if args.volume is not None:
    ok = ok and math.isclose(shape.Volume, args.volume, rel_tol=1e-8, abs_tol=1e-8)
if args.dimensions is not None:
    ok = ok and all(math.isclose(a, b, rel_tol=1e-8, abs_tol=1e-7) for a, b in zip(dimensions, args.dimensions))
if args.bounds is not None:
    ok = ok and all(math.isclose(a, b, rel_tol=1e-8, abs_tol=1e-7) for a, b in zip(sum(bounds, []), args.bounds))
if args.radii is not None:
    ok = ok and same_values((c["radius_mm"] for c in report["cylinders"]), args.radii, 1e-8)
if args.heights is not None:
    ok = ok and same_values((c["height_mm"] for c in report["cylinders"]), args.heights, 1e-8)
if args.angles_deg is not None:
    ok = ok and same_values((math.degrees(c["angular_span_rad"]) for c in report["cylinders"]), args.angles_deg, 1e-7)
if args.cone_end_radii is not None:
    actual = sorted(c["end_radii_mm"] for c in report["cones"])
    expected = sorted(sorted(pair) for pair in zip(args.cone_end_radii[::2], args.cone_end_radii[1::2]))
    ok = ok and len(actual) == len(expected) and all(same_values(a, b, 1e-7) for a, b in zip(actual, expected))
if args.cone_heights is not None:
    ok = ok and same_values((c["height_mm"] for c in report["cones"]), args.cone_heights, 1e-7)
if args.cone_angles_deg is not None:
    ok = ok and same_values((c["semi_angle_deg"] for c in report["cones"]), args.cone_angles_deg, 1e-7)
if args.sphere_radii is not None:
    ok = ok and same_values((c["radius_mm"] for c in report["spheres"]), args.sphere_radii, 1e-7)
if args.torus_radii is not None:
    actual = sorted((t["major_radius_mm"], t["minor_radius_mm"]) for t in report["tori"])
    expected = sorted(zip(args.torus_radii[::2], args.torus_radii[1::2]))
    ok = ok and len(actual) == len(expected) and all(
        all(math.isclose(x, y, rel_tol=1e-8, abs_tol=1e-7) for x, y in zip(a, b))
        for a, b in zip(actual, expected)
    )
if args.axis is not None:
    def parallel(axis):
        dot = math.fsum(a * b for a, b in zip(axis, args.axis))
        return math.isclose(abs(dot), math.hypot(*args.axis), rel_tol=1e-9)
    axes = [c["axis"] for key in ("cylinders", "cones", "spheres", "tori") for c in report[key]]
    ok = ok and bool(axes) and math.hypot(*args.axis) > 0.0 and all(map(parallel, axes))
report["passed"] = ok
print(json.dumps(report, indent=2))
raise SystemExit(0 if ok else 1)
