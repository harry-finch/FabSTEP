#!/usr/bin/env python3
"""Construct the shared edge of two unequal orthogonal pocket fillets in FreeCAD.

For large radius R, small radius r and q = 1-r/R, use the corner as origin
and z=0 as the pocket floor. On the inner branch of both cylinder equations,

    D = (1-q*t²)² + 4*t² = (1+q*t²)² + 4*(r/R)*t²
    (x-R)*D = -4*sqrt(R*r)*t*(1+q*t²)
    (y-r)*D = -4*r*t*(1-q*t²)
    z*D = 8*r*t².

The two expressions for D prove the cylinder identities for every t: e.g.
D²*((x-R)²+z²-2*R*z) = 16*R*r*t²*((1+q*t²)²+4*(r/R)*t²-D).
The y identity uses the first expression for D. The small wall angle runs
from 0 to pi/2 with tan(angle/2)=2*t/(1-q*t²), monotonically up to T below.

For a plane through (R,r,0), multiply its linear equation by D. Zero
coefficients at t and t³ force both horizontal normal components to vanish
when q is nonzero. The t² coefficient then forces the last component to
vanish. In particular this branch cannot be represented by a planar conic.

This script measures the transfer of the 3D edge alone. Each supporting
surface needs a separate pcurve and a shared topological edge in a solid.
Run with FreeCAD's Python and an output directory for the STEP examples.
"""
import argparse
import json
import math
from pathlib import Path

import FreeCAD as App
import Part


def seam(large, small):
    if not 0 < small < large:
        raise ValueError("Expected distinct positive radii")
    q = 1 - small / large
    end = 1 / (1 + math.sqrt(1 + q))
    denominator = [1, 0, 4 - 2 * q, 0, q**2]
    root = math.sqrt(small * large)
    # Homogeneous power coefficients (X,Y,Z,W) on t=end*u, 0 <= u <= 1.
    coefficients = [
        [large * d - x for d, x in zip(denominator, [0, 4 * root, 0, 4 * root * q, 0])],
        [
            small * d - y
            for d, y in zip(denominator, [0, 4 * small, 0, -4 * small * q, 0])
        ],
        [0, 0, 8 * small, 0, 0],
        denominator,
    ]
    controls = []
    for i in range(5):
        controls.append(
            [
                sum(
                    row[k] * end**k * math.comb(i, k) / math.comb(4, k)
                    for k in range(i + 1)
                )
                for row in coefficients
            ]
        )
    # D has constant 1 and nonnegative power coefficients; all weights >= 1.
    if min(p[3] for p in controls) < 1:
        raise ValueError("Unexpected denominator weights")
    curve = Part.BezierCurve()
    curve.setPoles([App.Vector(x / w, y / w, z / w) for x, y, z, w in controls])
    for index, (_, _, _, w) in enumerate(controls, 1):
        curve.setWeight(index, w)
    return curve, end


def measure(large, small, path):
    curve, end = seam(large, small)
    curve.toShape().exportStep(str(path))
    edges = Part.read(str(path)).Edges
    if len(edges) != 1:
        raise ValueError("Expected one transferred edge")
    transfer = edges[0].Curve
    if not (
        isinstance(transfer, Part.BSplineCurve)
        and transfer.isRational()
        and transfer.Degree == 4
        and transfer.NbPoles == 5
        and transfer.getKnots() == [0.0, 1.0]
        and min(transfer.getWeights()) > 0
    ):
        raise ValueError(
            "Unexpected STEP representation of the rational quarter-branch"
        )

    q = 1 - small / large
    max_distance = 0.0
    radial_errors = [0.0, 0.0]
    for i in range(1001):
        u = i / 1000
        t = u * end
        z = 8 * small * t**2 / ((1 - q * t**2) ** 2 + 4 * t**2)
        point = App.Vector(
            large - math.sqrt(z * (2 * large - z)),
            small - math.sqrt(z * (2 * small - z)),
            z,
        )
        actual = transfer.value(u)
        max_distance = max(max_distance, (actual - point).Length)
        for j, (radius, coord) in enumerate(((large, actual.x), (small, actual.y))):
            error = abs(math.hypot(coord - radius, actual.z - radius) - radius)
            radial_errors[j] = max(error, radial_errors[j])

    endpoints = [
        App.Vector(large, small, 0),
        App.Vector(large - math.sqrt(small * (2 * large - small)), 0, small),
    ]
    endpoint_errors = [(transfer.value(i) - p).Length for i, p in enumerate(endpoints)]
    if max(max_distance, *radial_errors, *endpoint_errors) > 1e-8:
        raise ValueError(
            f"Transferred edge failed the independent branch check for {path}"
        )

    return {
        "radii_mm": [large, small],
        "degree": transfer.Degree,
        "weights": transfer.getWeights(),
        "max_sampled_distance_mm": max_distance,
        "sampled_radial_errors_mm": radial_errors,
        "endpoint_errors_mm": endpoint_errors,
        "additional_circular_arc_sweep_rad": math.asin(1 - small / large),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("output", type=Path)
    output = parser.parse_args().output
    output.mkdir(parents=True, exist_ok=True)
    cases = [(3, 2), (7, 1), (4, 3.99)]
    results = [measure(R, r, output / f"junction-{R}-{r}.step") for R, r in cases]
    print(json.dumps({"samples_per_edge": 1001, "edges": results}, indent=2))


if __name__ == "__main__":
    main()
