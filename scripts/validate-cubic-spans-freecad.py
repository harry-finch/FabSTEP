"""Check the two exported tangent cubic meridians in a FreeCAD Python session."""

import argparse
import json
import math
from pathlib import Path

import FreeCAD as App
import Part


def radius(controls, t):
    while len(controls) > 1:
        controls = [(1 - t) * a + t * b for a, b in zip(controls, controls[1:])]
    return controls[0]


def check(case, path):
    shape = Part.read(str(path))
    assert len(shape.Solids) == 1 and shape.isValid() and shape.Solids[0].isClosed()
    walls = [f for f in shape.Faces if isinstance(f.Surface, Part.SurfaceOfRevolution)]
    assert len(walls) == 1 and len(shape.Faces) == 3
    wall = walls[0]
    seams = [e.Curve for e in wall.Edges if isinstance(e.Curve, Part.BSplineCurve)]
    assert len(seams) == 1 and seams[0].Degree == 3 and seams[0].NbPoles == 6
    assert seams[0].getMultiplicities() == [4, 2, 4]
    params = case["parameters"]
    height, left = params["height_mm"], params["radii_mm"]
    origin, axis = App.Vector(*params["origin_mm"]), App.Vector(*params["axis"])
    split = case["second_span"]["start_fraction"]
    right = [left[-1], *case["second_span"]["next_radii_mm"]]
    errors = []
    u0, u1, v0, v1 = wall.ParameterRange
    for station in range(41):
        for azimuth in range(6):
            p = wall.valueAt(
                u0 + (u1 - u0) * azimuth / 6, v0 + (v1 - v0) * station / 40
            )
            relative = p - origin
            t = relative.dot(axis) / height
            assert -1e-6 <= t <= 1 + 1e-6
            r = (
                radius(left, t / split)
                if t <= split
                else radius(right, (t - split) / (1 - split))
            )
            errors.append(abs(relative.cross(axis).Length - r))
    assert max(errors) < 2e-5
    # FreeCAD's generic Shape.Volume can integrate across the knot in one
    # quadrature. Integrate each extracted span independently. Four-point
    # Gauss-Legendre is exact for its degree-six squared radius and linear z.
    nodes = [
        -0.8611363115940526,
        -0.3399810435848563,
        0.3399810435848563,
        0.8611363115940526,
    ]
    weights = [
        0.3478548451374539,
        0.6521451548625461,
        0.6521451548625461,
        0.3478548451374539,
    ]
    volume = 0.0
    knots = seams[0].getKnots()
    # The physical wall V orientation is independent of the edge direction.
    start_z = (wall.valueAt(u0, v0) - origin).dot(axis)
    wall_knot = split if abs(start_z) < 2e-5 else 1 - split
    assert min(abs(knots[1] - wall_knot), abs(1 - knots[1] - wall_knot)) < 1e-6
    for low, high in [
        (v0, v0 + (v1 - v0) * wall_knot),
        (v0 + (v1 - v0) * wall_knot, v1),
    ]:
        dz = abs((wall.valueAt(u0, high) - wall.valueAt(u0, low)).dot(axis))
        for node, weight in zip(nodes, weights):
            v = (low + high + node * (high - low)) / 2
            actual_r = (wall.valueAt(u0, v) - origin).cross(axis).Length
            volume += math.pi * 0.5 * dz * weight * actual_r**2
    assert math.isclose(volume, params["volume_mm3"], rel_tol=1e-6)
    for p in case["material_probes"]:
        assert shape.isInside(App.Vector(*p["point_mm"]), 1e-7, False) == p["material"]
    return {
        "volume_mm3": volume,
        "generic_volume_mm3": shape.Volume,
        "positions": len(errors),
        "max_radius_error_mm": max(errors),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("step_dir", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    cases = json.loads((root / "fixtures/smooth_revolution/manifest.json").read_text())[
        "cases"
    ]
    print(
        json.dumps(
            {
                n: check(cases[n], args.step_dir / f"{n}.step")
                for n in ["two_span", "two_span_mirror"]
            }
        )
    )
