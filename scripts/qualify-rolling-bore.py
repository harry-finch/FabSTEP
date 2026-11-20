"""R-129. Independent source fit for a round shaft with a rounded transverse bore mouth.

FreeCAD Python. Measurements use only binary STL vertices. Reference CAD labels
are used afterwards to check ownership, and nominal radii only to score the fit.
For this bench the shaft and bore axes are parallel to Z and X respectively.
To verify four FabSTEP CLI exports independently, first generate the references
in OUT, then run OUT --check-step EXPORTS [--bop]. Each STEP in EXPORTS must
have the same basename as its source STL. --bop adds FreeCAD's extended boolean
diagnostics for comparison with each reference; the ordinary solid validity,
geometry, volume and bounding box remain separate checks.
"""

import argparse
from collections import Counter
import json
import math
from pathlib import Path

import FreeCAD as App
import MeshPart
import Part
from r113_mesh import read_stl
from r113_shaft_fit import circle, require

V = App.Vector
FIT = 1.0e-5


def radius_at_contact(p, shaft, bore, y0, z0):
    # On the shaft, the rolling centre lies r inside the cylindrical stock.
    # Its simultaneous contact with the bore fixes r from this measured point.
    _, y, z = p
    dy = y - y0
    a = (y / shaft) ** 2 - 1
    b = -2 * (dy * y / shaft + bore)
    c = dy**2 + (z - z0) ** 2 - bore**2
    disc = b * b - 4 * a * c
    require(disc >= 0 and a < -0.05, "degenerate offset intersection")
    roots = [
        2 * c / (-b + math.copysign(math.sqrt(disc), -b)),
        (-b + math.copysign(math.sqrt(disc), -b)) / (2 * a),
    ]
    valid = [r for r in roots if FIT < r < bore / 2]
    require(len(valid) == 1, "ambiguous rolling radius")
    return valid[0]


def fit(vertices, triangles):
    bottom, top = min(v[2] for v in vertices), max(v[2] for v in vertices)
    ring = [v for v in vertices if abs(v[2] - bottom) < FIT]
    base, shaft = circle(ring, (0, 0, 1))
    local = [(x - base[0], y - base[1], z - base[2]) for x, y, z in vertices]

    def side(p):
        return abs(math.hypot(*p[:2]) - shaft) <= FIT

    def middle(p):
        return FIT < p[2] < top - bottom - FIT

    # The untouched mouth is on the positive X side in this first domain.
    mouth = [p for p in local if middle(p) and side(p) and p[0] > 0]
    center, bore = circle(mouth, (1, 0, 0))
    y0, z0 = center[1:]

    def bore_wall(p):
        return abs(math.hypot(p[1] - y0, p[2] - z0) - bore) <= FIT

    labels = []
    for tri in triangles:
        pts = [local[i] for i in tri]
        if all(abs(p[2]) <= FIT for p in pts) or all(
            abs(p[2] - (top - bottom)) <= FIT for p in pts
        ):
            labels.append("cap")
        elif all(side(p) for p in pts):
            labels.append("shaft")
        elif all(bore_wall(p) for p in pts):
            labels.append("bore")
        else:
            labels.append("blend")
    by_vertex = [set() for _ in local]
    for name, ids in zip(labels, triangles):
        for v in ids:
            by_vertex[v].add(name)
    rim = [p for p, tags in zip(local, by_vertex) if tags == {"blend", "shaft"}]
    require(len(rim) >= 12 and all(p[0] < 0 for p in rim), "unresolved outer contact")
    estimates = sorted(radius_at_contact(p, shaft, bore, y0, z0) for p in rim)
    radius = estimates[len(estimates) // 2]
    scatter = max(abs(r - radius) for r in estimates)
    # OCCT's blend is an approximating spline. Distinguish the measurement
    # of its actual errors from the stricter engine admission threshold.
    return (shaft, bore, y0, z0, radius), local, labels, scatter


def rolling_error(p, dimensions):
    shaft, bore, y0, z0, r = dimensions

    def center(t):
        y = y0 + (bore + r) * math.cos(t)
        return (-math.sqrt((shaft - r) ** 2 - y * y), y, z0 + (bore + r) * math.sin(t))

    # This is a local sampled comparison with the rolling envelope. It does
    # not certify the triangle interior or license publication of a STEP.
    angle = math.atan2(p[2] - z0, p[1] - y0)
    lo, hi = angle - 0.8, angle + 0.8
    for _ in range(45):
        l, h = (2 * lo + hi) / 3, (lo + 2 * hi) / 3
        if math.dist(p, center(l)) < math.dist(p, center(h)):
            hi = h
        else:
            lo = l
    return abs(math.dist(p, center((lo + hi) / 2)) - r)


def run_case(folder, name, shaft, bore, offset, level, height, fillet):
    src = Part.makeCylinder(shaft, height)
    src = src.cut(
        Part.makeCylinder(bore, 4 * shaft, V(-2 * shaft, offset, level), V(1, 0, 0))
    )
    mouths = [
        e
        for e in src.Edges
        if e.CenterOfMass.x < -shaft / 2 and e.Curve.TypeId == "Part::GeomBSplineCurve"
    ]
    require(len(mouths) == 1, "expected one isolated far mouth")
    ref_path = folder / (name + ".step")
    src.makeFillet(fillet, mouths).exportStep(str(ref_path))
    results = {}
    for density, angle in [("coarse", 0.34), ("fine", 0.24)]:
        ref = Part.read(str(ref_path))
        require(
            len(ref.Solids) == 1 and ref.Solids[0].isClosed() and ref.isValid(),
            "invalid reference",
        )
        mesh = MeshPart.meshFromShape(
            Shape=ref,
            LinearDeflection=0.025,
            AngularDeflection=angle,
            Relative=False,
            Segments=True,
        )
        stl = folder / (name + "-" + density + ".stl")
        mesh.write(str(stl))
        points, triangles = read_stl(stl)
        params, local, labels, scatter = fit(points, triangles)
        expected = {}
        for i, face in enumerate(ref.Faces):
            surface = face.Surface
            if isinstance(surface, Part.Plane):
                k = "cap"
            elif isinstance(surface, Part.Cylinder):
                k = "shaft" if abs(surface.Axis.z) > 0.9 else "bore"
            elif surface.TypeId == "Part::GeomBSplineSurface":
                k = "blend"
            else:
                raise ValueError(surface.TypeId)
            for t in mesh.getSegment(i):
                expected[t] = k
        require(
            all(expected[i] == label for i, label in enumerate(labels)),
            "triangle ownership mismatch",
        )
        blends = [t for t, k in zip(triangles, labels) if k == "blend"]
        ids = sorted({i for t in blends for i in t})
        error = max(rolling_error(local[i], params) for i in ids)
        mid = max(
            rolling_error(
                tuple(sum(local[i][j] for i in t) / 3 for j in range(3)), params
            )
            for t in blends
        )
        interior = next(
            i
            for i in ids
            if all(labels[k] == "blend" for k, t in enumerate(triangles) if i in t)
        )
        # Same contact curves; one material point moved well inside the blend.
        changed = list(local[interior])
        changed[0] += 0.12
        bad = rolling_error(changed, params)
        require(
            abs(params[-1] - fillet) < 0.001 and error < 0.002 and bad > 0.015,
            "rolling fit failed",
        )
        with stl.with_suffix(".patch").open("w") as data:
            print(len(points), len(blends), interior, file=data)
            for p in points:
                print(*p, file=data)
            for t in blends:
                print(*t, file=data)
        results[density] = dict(
            triangles=len(triangles),
            blend_triangles=len(blends),
            params_mm=params,
            rim_scatter_mm=scatter,
            max_vertex_error_mm=error,
            max_centroid_error_mm=mid,
            altered_interior_error_mm=bad,
        )
    return results


def extended_bop(shape):
    try:
        shape.check(True)
    except ValueError as error:
        return {"status": "diagnostics", "details": str(error).splitlines()}
    return {"status": "pass"}


def check_cli_exports(folder, exports, run_bop):
    results = {}
    for design in ("shaft", "short-shaft"):
        ref = Part.read(str(folder / (design + ".step")))
        ref.check(False)
        reference = {"volume_mm3": ref.Volume}
        if run_bop:
            reference["extended_bop"] = extended_bop(ref)
        measurements = {}
        for density in ("coarse", "fine"):
            name = f"{design}-{density}"
            path = exports / (name + ".step")
            require(path.is_file(), f"missing CLI export {path}")
            body = Part.read(str(path))
            body.check(False)
            require(len(body.Solids) == 1 and body.Solids[0].isClosed()
                    and body.isValid(), f"{name}: invalid solid")
            families = Counter(f.Surface.TypeId for f in body.Faces)
            require(families == {"Part::GeomCylinder": 2, "Part::GeomPlane": 2,
                                  "Part::GeomBSplineSurface": 1} and len(body.Edges) == 11,
                    f"{name}: incorrect analytic faces or boundary edges")
            bounds_delta = max(abs(getattr(body.BoundBox, k) - getattr(ref.BoundBox, k))
                               for k in ("XMin", "XMax", "YMin", "YMax", "ZMin", "ZMax"))
            require(bounds_delta < 0.001 and abs(body.Volume - ref.Volume) < ref.Volume * 1e-5,
                    f"{name}: dimensions or volume differ from the independent STEP")
            measurements[density] = {"brep_check": "pass", "closed_solids": 1,
                                     "families": dict(families), "edges": len(body.Edges),
                                     "volume_mm3": body.Volume, "bounds_delta_mm": bounds_delta}
            if run_bop:
                measurements[density]["extended_bop"] = extended_bop(body)
        results[design] = {"reference": reference, "exports": measurements}
    (exports / "freecad-reimport.json").write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out", type=Path)
    parser.add_argument("--check-step", type=Path, metavar="EXPORTS",
                        help="check four CLI STEP outputs against the references already in OUT")
    parser.add_argument("--bop", action="store_true", help="record extended FreeCAD BOP diagnostics")
    options = parser.parse_args()
    folder = options.out.resolve()
    if options.check_step:
        check_cli_exports(folder, options.check_step.resolve(), options.bop)
        raise SystemExit(0)
    if options.bop:
        parser.error("--bop requires --check-step")
    folder.mkdir(parents=True, exist_ok=True)
    results = {}
    for args in [
        ("shaft", 10, 2.8, 1, 8, 18, 0.65),
        ("short-shaft", 10, 2.8, 1, 5.5, 13, 0.65),
    ]:
        results[args[0]] = run_case(folder, *args)
    try:
        run_case(folder, "spindle", 8.25, 1.9, 1.2, 5.75, 13, 0.42)
    except ValueError as error:
        require(str(error) == "unresolved outer contact", "unexpected spindle refusal")
        results["spindle"] = {"refused": str(error)}
    else:
        raise ValueError("expected the ambiguous contact to be refused")
    (folder / "measurements.json").write_text(json.dumps(results, indent=2) + "\n")
