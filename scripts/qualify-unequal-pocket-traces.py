"""Lab: build both cylinder pcurves on the shared quartic's own parameter.

For t in [0,T], T <= 1/2, a cylindrical chart has coordinates (angle,
axial position). The derivatives of both angles are rational, as are both
axial positions. All have the same positive denominator D >= 1. The cubic
Hermite error per coordinate is at most max|f''''| * (T/n)^4 / 384.

For angular speed the numerator coefficients are bounded in magnitude by
4*(1+t²). For the nonconstant axial displacement divided by the large radius
they are bounded by 4*(t+t³). The bounds below follow by differentiating
D*f=P. They concern interpolation in exact arithmetic. The comparisons to
FreeCAD poles/values measure floating-point construction independently.
"""

import json
import math
from fractions import Fraction
from pathlib import Path
import runpy

import FreeCAD as App
import Part


def derivative_bound(coefficients, order):
    return sum(
        abs(c) * math.perm(i, order) * Fraction(1, 2) ** (i - order)
        for i, c in enumerate(coefficients)
        if i >= order
    )


def quotient_bound(numerator, order):
    bounds = []
    for j in range(order + 1):
        bound = derivative_bound(numerator, j)
        for i in range(1, j + 1):
            bound += (
                math.comb(j, i) * derivative_bound([1, 0, 4, 0, 1], i) * bounds[j - i]
            )
        bounds.append(bound)  # division by D <= multiplication by 1
    return bounds[-1]


def chart(t, large, small, small_wall):
    """Physical chart and its derivative with respect to t, before t=T*u."""
    q = 1 - small / large
    d = 1 + (4 - 2 * q) * t**2 + q**2 * t**4
    dd = 2 * (4 - 2 * q) * t + 4 * q**2 * t**3
    if small_wall:
        a, b, offset, factor = 2.0, -q, large, 4 * math.sqrt(large * small)
    else:
        a, b, offset, factor = 2 * math.sqrt(small / large), q, small, 4 * small
    angle = 2 * math.atan2(a * t, 1 + b * t**2)
    speed = 2 * a * (1 - b * t**2) / d
    p = factor * t * (1 - b * t**2)
    dp = factor * (1 - 3 * b * t**2)
    v = App.Base.Vector2d
    return v(angle, offset - p / d), v(speed, -(dp * d - p * dd) / d**2)


def trace(large, small, end, small_wall, intervals):
    poles = []
    dt = end / intervals
    for i in range(intervals + 1):
        point, tangent = chart(end * i / intervals, large, small, small_wall)
        handle = tangent * (dt / 3)
        if i:
            poles.append(point - handle)
        poles.append(point)
        if i < intervals:
            poles.append(point + handle)
    curve = Part.Geom2d.BSplineCurve2d()
    curve.buildFromPolesMultsKnots(
        poles,
        [4] + [3] * (intervals - 1) + [4],
        [i / intervals for i in range(intervals + 1)],
        False,
        3,
    )
    return curve


def qualify(large, small, seam):
    edge, end = seam(large, small)
    intervals = 1
    # |delta_xyz| <= R*|delta_angle| + |delta_axial| on either cylinder.
    scale = Fraction(large) * (
        quotient_bound([4, 0, 4], 3) + quotient_bound([0, 4, 0, 4], 4)
    )
    while scale / (384 * (2 * intervals) ** 4) > Fraction(1, 100_000_000):
        if intervals == 1024:
            raise ValueError("Hermite span budget exceeded")
        intervals *= 2
    limit = float(scale / (384 * (2 * intervals) ** 4))
    errors = []
    for small_wall in (True, False):
        curve = trace(large, small, end, small_wall, intervals)
        r = small if small_wall else large
        max_error = 0.0
        for i in range(4 * intervals + 1):
            u = i / (4 * intervals)
            uv = curve.value(u)
            angle, axial = uv.x, uv.y
            transverse, height = r * (1 - math.sin(angle)), r * (1 - math.cos(angle))
            x, y = (axial, transverse) if small_wall else (transverse, axial)
            max_error = max(
                max_error, (edge.value(u) - App.Vector(x, y, height)).Length
            )
        if max_error > limit:
            raise ValueError("Sampled chart exceeded its interpolation budget")
        errors.append(max_error)
    return dict(
        radii_mm=[large, small],
        spans=intervals,
        interpolation_bound_mm=limit,
        sampled_distance_mm=errors,
        samples_per_wall=4 * intervals + 1,
    )


if __name__ == "__main__":
    seam = runpy.run_path(
        str(Path(__file__).with_name("qualify-unequal-pocket-curve.py"))
    )["seam"]
    print(
        json.dumps(
            [qualify(R, r, seam) for R, r in ((3, 2), (7, 1), (4, 3.99))], indent=2
        )
    )
