// Certify the two-way deviation over a whole circular rolling blend. Periodic
// source triangles are clipped at the actual native Bernstein span boundaries.
#pragma once
#include "unequal_corner_certificate.hxx"
#include <BRepAdaptor_Surface.hxx>
#include <BRepTools.hxx>
#include <GeomAPI_ProjectPointOnSurf.hxx>
#include <Geom_BSplineCurve.hxx>
#include <set>

namespace rolling_bore {
using unequal_corner::Point;
using unequal_corner::Tri;
using unequal_corner::UV;
struct Vertex {
  UV uv;
  Point xyz;
};
using Polygon = std::vector<Vertex>;

inline Polygon clip(const Polygon &input, double knot, bool above) {
  Polygon out;
  if (input.empty())
    return out;
  auto previous = input.back();
  for (const auto &p : input) {
    const double a = previous.uv[1] - knot, b = p.uv[1] - knot;
    if ((a < 0 && b > 0) || (a > 0 && b < 0)) {
      double t = a / (a - b);
      Vertex intersection{
          {previous.uv[0] + t * (p.uv[0] - previous.uv[0]), knot}, {}};
      for (size_t d = 0; d < 3; ++d)
        intersection.xyz[d] =
            previous.xyz[d] + t * (p.xyz[d] - previous.xyz[d]);
      out.push_back(intersection);
    }
    if (above ? b >= 0 : b <= 0)
      out.push_back(p);
    previous = p;
  }
  return out;
}

inline double turn(double x) { return x - std::round(x); }

// Extremely thin clipped polygons can make the affine inverse numerically
// singular. Bound their diameter directly, with the derivative of the
// homogeneous Bernstein net on this entire knot span and positive weights.
inline bool sliver(const unequal_corner::Patch &patch,
                   const std::array<UV, 3> &uv, const std::array<Point, 3> &xyz,
                   double limit, unequal_corner::Certificate &proof,
                   const UV &speed, int depth = 0) {
  if (++proof.boxes > 100000)
    return false;
  const auto mid = UV{(uv[0][0] + uv[1][0] + uv[2][0]) / 3.,
                      (uv[0][1] + uv[1][1] + uv[2][1]) / 3.};
  const auto p =
      patch.surface->Value(patch.u0 + mid[0] * (patch.u1 - patch.u0),
                           patch.v0 + mid[1] * (patch.v1 - patch.v0));
  double flat = 0., curved = 0.;
  for (size_t i = 0; i < 3; ++i) {
    flat = std::max(flat, p.Distance(unequal_corner::point(xyz[i])));
    curved = std::max(curved, speed[0] * std::abs(uv[i][0] - mid[0]) +
                                  speed[1] * std::abs(uv[i][1] - mid[1]));
  }
  const double bound = flat + curved + 1e-9;
  if (bound <= limit) {
    proof.maximum = std::max(proof.maximum, bound);
    return true;
  }
  if (depth == 18)
    return false;
  size_t edge = 0;
  double longest = -1.;
  for (size_t i = 0; i < 3; ++i) {
    auto j = (i + 1) % 3;
    double d =
        speed[0] * std::abs(uv[i][0] - uv[j][0]) +
        speed[1] * std::abs(uv[i][1] - uv[j][1]) +
        unequal_corner::point(xyz[i]).Distance(unequal_corner::point(xyz[j]));
    if (d > longest) {
      longest = d;
      edge = i;
    }
  }
  auto j = (edge + 1) % 3, k = (edge + 2) % 3;
  UV v{};
  Point x{};
  for (size_t d = 0; d < 2; ++d)
    v[d] = (uv[edge][d] + uv[j][d]) / 2.;
  for (size_t d = 0; d < 3; ++d)
    x[d] = (xyz[edge][d] + xyz[j][d]) / 2.;
  return sliver(patch, {uv[edge], v, uv[k]}, {xyz[edge], x, xyz[k]}, limit,
                proof, speed, depth + 1) &&
         sliver(patch, {v, uv[j], uv[k]}, {x, xyz[j], xyz[k]}, limit, proof,
                speed, depth + 1);
}

inline UV derivative_bounds(const unequal_corner::Patch &patch) {
  auto s = Handle(Geom_BSplineSurface)::DownCast(patch.surface->Copy());
  s->Segment(patch.u0, patch.u1, patch.v0, patch.v1, 1e-13, 1e-13);
  if (s->NbUKnots() != 2 || s->NbVKnots() != 2)
    return {INFINITY, INFINITY};
  double minw = INFINITY, maxw = 0., radius = 0.;
  for (int i = 1; i <= s->NbUPoles(); ++i)
    for (int j = 1; j <= s->NbVPoles(); ++j) {
      double w = s->Weight(i, j);
      if (!(w > 0.))
        return {INFINITY, INFINITY};
      minw = std::min(minw, w);
      maxw = std::max(maxw, w);
      radius = std::max(radius, s->Pole(i, j).Distance(s->Pole(1, 1)));
    }
  UV speed{};
  // H'-S*w' is a convex combination of adjacent control differences.
  for (int i = 1; i <= s->NbUPoles(); ++i)
    for (int j = 1; j <= s->NbVPoles(); ++j) {
      for (int axis = 0; axis < 2; ++axis) {
        int k = i + int(axis == 0), l = j + int(axis == 1);
        if (k > s->NbUPoles() || l > s->NbVPoles())
          continue;
        double dw = std::abs(s->Weight(k, l) - s->Weight(i, j));
        double bound =
            (maxw * s->Pole(i, j).Distance(s->Pole(k, l)) + 2 * radius * dw) /
            minw;
        speed[axis] = std::max(
            speed[axis], bound * (axis == 0 ? s->UDegree() : s->VDegree()));
      }
    }
  return speed;
}

// Bound the actual material iso-curve against the matching source chord
// over one Bernstein interval. For H=w*C and affine L, elevate H to degree
// n+1 and subtract the coefficients of w*L before dividing by min(w).
inline double chord_bound(const Handle(Geom_BSplineCurve) & curve, double lo,
                          double hi, gp_Pnt start, gp_Pnt end, double limit,
                          int depth = 0) {
  auto part = Handle(Geom_BSplineCurve)::DownCast(curve->Copy());
  part->Segment(lo, hi, 1e-13);
  const int n = part->Degree();
  if (part->NbPoles() != n + 1 || part->NbKnots() != 2)
    return INFINITY;
  double maxerr = 0., minw = INFINITY;
  for (int k = 0; k <= n + 1; ++k) {
    gp_XYZ net(0., 0., 0.);
    for (int side : {0, 1}) {
      int j = k - side;
      if (j < 0 || j > n)
        continue;
      double w = part->Weight(j + 1);
      if (!(w > 0. && std::isfinite(w)))
        return INFINITY;
      minw = std::min(minw, w);
      const double a = double(k) / (n + 1);
      net += (part->Pole(j + 1).XYZ() - (side ? end : start).XYZ()) *
             ((side ? a : 1. - a) * w);
    }
    maxerr = std::max(maxerr, net.Modulus());
  }
  double bound = maxerr / minw + 1.e-9;
  if (bound <= limit || depth == 5)
    return bound;
  gp_Pnt mid((start.XYZ() + end.XYZ()) * .5);
  double knot = (lo + hi) / 2;
  return std::max(chord_bound(curve, lo, knot, start, mid, limit, depth + 1),
                  chord_bound(curve, knot, hi, mid, end, limit, depth + 1));
}

inline double rim_bound(const Handle(Geom_BSplineCurve) & curve,
                        const std::vector<double> &knots, double v0,
                        double period, double start, double stop, gp_Pnt a,
                        gp_Pnt b, double limit) {
  if (start > stop) {
    std::swap(start, stop);
    std::swap(a, b);
  }
  double result = 0.;
  for (int shift = int(std::floor(start)); shift <= int(std::floor(stop));
       ++shift) {
    for (size_t k = 1; k < knots.size(); ++k) {
      double lo = std::max(start, knots[k - 1] + shift),
             hi = std::min(stop, knots[k] + shift);
      if (hi <= lo)
        continue;
      const auto at = [&](double t) {
        return a.Translated(gp_Vec(a, b) * ((t - start) / (stop - start)));
      };
      const double x = v0 + (lo - shift) * period,
                   y = v0 + (hi - shift) * period;
      if (y - x < 1e-11) {
        // Snap the outer endpoints onto a small nondegenerate containing
        // interval on this span, then use its convex hull for the entire chord.
        double l = std::max(v0 + knots[k - 1] * period, x - 1e-8 * period),
               h = std::min(v0 + knots[k] * period, y + 1e-8 * period);
        auto local = Handle(Geom_BSplineCurve)::DownCast(curve->Copy());
        local->Segment(l, h, 1e-14);
        for (int j = 1; j <= local->NbPoles(); ++j) {
          if (!(local->Weight(j) > 0.))
            return INFINITY;
          result = std::max({result, local->Pole(j).Distance(at(lo)) + 1e-9,
                             local->Pole(j).Distance(at(hi)) + 1e-9});
        }
      } else
        result =
            std::max(result, chord_bound(curve, x, y, at(lo), at(hi), limit));
    }
  }
  return result;
}
inline bool certify(const TopoDS_Face &face, const std::vector<Point> &points,
                    const std::vector<Tri> &triangles,
                    unequal_corner::Certificate &certificate, double fit,
                    double limit) {
  BRepAdaptor_Surface ad(face, false);
  if (ad.GetType() != GeomAbs_BSplineSurface)
    return certificate.reject("rolling surface type");
  auto bs = ad.BSpline();
  // The native surface may be clamped with two physically identical end
  // rows. Its rational weights can differ at machine precision. Keep the
  // original net and certify each clipped material triangle on its own span.
  const bool periodic = bs->IsVPeriodic();
  if (!periodic) {
    if (bs->NbVPoles() > 256 || bs->NbUPoles() != 3)
      return certificate.reject("rolling seam size");
    double minw = INFINITY, dw = 0., gap = 0., diam = 0.;
    const int last = bs->NbVPoles();
    for (int u = 1; u <= bs->NbUPoles(); ++u) {
      double a = bs->Weight(u, 1), b = bs->Weight(u, last);
      minw = std::min({minw, a, b});
      dw = std::max(dw, std::abs(a - b));
      gap = std::max(gap, bs->Pole(u, 1).Distance(bs->Pole(u, last)));
      diam = std::max(diam, 2. * bs->Pole(u, 1).Distance(bs->Pole(1, 1)));
    }
    if (!(minw > 0. && gap + diam * dw / minw < 1e-10))
      return certificate.reject("open rolling seam");
  }
  if (triangles.size() < 12 || triangles.size() > 8192 ||
      points.size() > 24576 || !(fit > 0. && limit >= fit) ||
      bs->IsUPeriodic() || bs->NbUKnots() != 2 || bs->NbVKnots() > 64 ||
      bs->UDegree() > 12 || bs->VDegree() > 12)
    return certificate.reject("spline span or degree");
  double u0, u1, v0, v1;
  BRepTools::UVBounds(face, u0, u1, v0, v1);
  const double period = periodic ? bs->VPeriod()
                                 : bs->VKnot(bs->LastVKnotIndex()) -
                                       bs->VKnot(bs->FirstVKnotIndex());
  if (!(period > 0. && u1 > u0) || std::abs(v1 - v0 - period) > 1e-8 * period ||
      std::abs(v0 - bs->VKnot(bs->FirstVKnotIndex())) > 1e-8 * period)
    return certificate.reject("incomplete periodic interval");
  std::vector<double> knots{0.};
  for (int j = bs->FirstVKnotIndex() + 1; j < bs->LastVKnotIndex(); ++j)
    knots.push_back((bs->VKnot(j) - v0) / period);
  knots.push_back(1.);
  std::map<std::pair<size_t, size_t>, unsigned> edges;
  std::map<size_t, UV> coordinates;
  for (const auto &t : triangles)
    for (int i = 0; i < 3; ++i) {
      const auto a = t[i], b = t[(i + 1) % 3];
      if (a >= points.size() || b >= points.size() || a == b)
        return certificate.reject("source vertex index");
      auto &uses = edges[std::minmax(a, b)];
      unsigned bit = a < b ? 1 : 2;
      if (uses & bit)
        return certificate.reject("repeated material half-edge");
      uses |= bit;
    }
  std::set<size_t> boundary;
  for (const auto &[edge, uses] : edges)
    if (uses != 3) {
      boundary.insert(edge.first);
      boundary.insert(edge.second);
    }
  int sign = 0;
  for (const auto &t : triangles) {
    Polygon tri;
    for (auto id : t) {
      auto [it, inserted] = coordinates.try_emplace(id);
      if (inserted) {
        auto p = unequal_corner::point(points[id]);
        GeomAPI_ProjectPointOnSurf project(p, bs, u0, u1, v0, v1, 1.e-10);
        if (!project.NbPoints())
          return certificate.reject("projection unavailable");
        double u, v;
        project.LowerDistanceParameters(u, v);
        if (boundary.count(id))
          u = u - u0 < u1 - u ? u0 : u1;
        if (bs->Value(u, v).Distance(p) > fit)
          return certificate.reject("vertex outside surface fitting tolerance");
        for (auto k : knots) {
          const double target = v0 + k * period;
          if (std::abs(v - target) < 1e-7 * period &&
              bs->Value(u, target).Distance(p) <= fit) {
            v = target;
            break;
          }
        }
        double vv = (v - v0) / period;
        it->second = {(u - u0) / (u1 - u0), std::clamp(vv, 0., 1.) == 1.
                                                ? 0.
                                                : std::clamp(vv, 0., 1.)};
      }
      tri.push_back({it->second, points[id]});
    }
    for (size_t i = 1; i < 3; ++i)
      tri[i].uv[1] = tri[0].uv[1] + turn(tri[i].uv[1] - tri[0].uv[1]);
    if (std::abs(tri[1].uv[1] - tri[2].uv[1]) >= .5 ||
        std::abs(tri[0].uv[1] - tri[1].uv[1]) >= .5 ||
        std::abs(tri[0].uv[1] - tri[2].uv[1]) >= .5)
      return certificate.reject("ambiguous periodic triangle");
    const auto a = tri[0].uv, b = tri[1].uv, c = tri[2].uv;
    const double det =
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if (std::abs(det) < 1e-16 || (sign && sign != (det > 0 ? 1 : -1)))
      return certificate.reject("fold in the periodic chart");
    sign = det > 0 ? 1 : -1;
    const auto span = std::minmax({a[1], b[1], c[1]});
    for (int shift = int(std::floor(span.first));
         shift <= int(std::floor(span.second)); ++shift) {
      for (size_t k = 1; k < knots.size(); ++k) {
        double lo = knots[k - 1] + shift, hi = knots[k] + shift;
        if (lo >= span.second || hi <= span.first)
          continue;
        auto poly = clip(clip(tri, lo, true), hi, false);
        for (auto &p : poly)
          p.uv[1] = (p.uv[1] - lo) / (hi - lo);
        unequal_corner::Patch patch{bs, u0, u1, v0 + knots[k - 1] * period,
                                    v0 + knots[k] * period};
        for (size_t i = 1; i + 1 < poly.size(); ++i) {
          std::array<UV, 3> uv{poly[0].uv, poly[i].uv, poly[i + 1].uv};
          std::array<Point, 3> xyz{poly[0].xyz, poly[i].xyz, poly[i + 1].xyz};
          const double area =
              std::abs((uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1]) -
                       (uv[2][0] - uv[0][0]) * (uv[1][1] - uv[0][1]));
          if (area < 1.e-11) {
            if (!sliver(patch, uv, xyz, limit, certificate,
                        derivative_bounds(patch)))
              return certificate.reject("clipped sliver");
          } else if (!patch.bound(uv, xyz, limit, 0, certificate))
            return false;
        }
      }
    }
    ++certificate.triangles;
  }
  // Oppositely wound degree-one boundaries plus one orientation for every
  // projected triangle imply complete coverage on the annular parameter chart.
  std::array<double, 2> winding{};
  const std::array<Handle(Geom_BSplineCurve), 2> rims{
      Handle(Geom_BSplineCurve)::DownCast(bs->UIso(u0)),
      Handle(Geom_BSplineCurve)::DownCast(bs->UIso(u1))};
  if (rims[0].IsNull() || rims[1].IsNull())
    return certificate.reject("unsupported material iso-curve");
  const double rim_limit = std::min(0.013, 0.26 * limit);

  for (auto [edge, uses] : edges)
    if (uses != 3) {
      const auto a = coordinates.at(edge.first),
                 b = coordinates.at(edge.second);
      if (a[0] != b[0] || (a[0] != 0. && a[0] != 1.))
        return certificate.reject("open source material contour");
      const double dv = turn(b[1] - a[1]) * (uses == 1 ? 1 : -1);
      if (dv * (a[0] == 1. ? sign : -sign) <= 0)
        return certificate.reject("reversed material iso-curve");
      const auto dis = rim_bound(
          rims[a[0] == 1.], knots, v0, period, a[1], a[1] + turn(b[1] - a[1]),
          unequal_corner::point(points[edge.first]),
          unequal_corner::point(points[edge.second]), rim_limit);
      if (!(dis <= rim_limit))
        return certificate.reject("rim chord outside budget");
      winding[a[0] == 1.] += dv;
    }
  return std::abs(winding[0] + sign) < 1e-8 &&
         std::abs(winding[1] - sign) < 1e-8;
}
struct SourceProof {
  std::vector<Point> vertices;
  std::vector<Tri> triangles;
  double fit, limit;
  bool accepts(const TopoDS_Face &face) const {
    unequal_corner::Certificate proof;
    return certify(face, vertices, triangles, proof, fit, limit);
  }
};
} // namespace rolling_bore
