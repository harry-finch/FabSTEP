// Certify a rectangular, single-span, rational corner against its actual mesh.
// The caller validates source indices and fit tolerances. The
// chart-specific bounds below limit native work independently.
#pragma once

#include <Geom_BSplineSurface.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <map>
#include <limits>
#include <utility>
#include <vector>

namespace unequal_corner {

using UV = std::array<double, 2>;
using Point = std::array<double, 3>;
using Tri = std::array<size_t, 3>;

inline Point coordinates(const gp_Pnt &p) { return {p.X(), p.Y(), p.Z()}; }
inline gp_Pnt point(const Point &p) { return gp_Pnt(p[0], p[1], p[2]); }

struct Certificate {
  size_t triangles = 0;
  size_t boxes = 0;
  double maximum = 0;
  const char *reason = nullptr;
  bool reject(const char *text) {
    reason = text;
    return false;
  }
};

struct Patch {
  Handle(Geom_BSplineSurface) surface;
  double u0, u1, v0, v1;

  // Returning false just rejects this candidate. Multiple local minima can
  // make seed selection less effective; the final 3D distance is decisive.
  bool project(const Point &p, double fit, UV &uv) const {
    const gp_Pnt target = point(p);
    double nearest = std::numeric_limits<double>::infinity();
    for (double u : {0., .5, 1.})
      for (double v : {0., .5, 1.}) {
        const double d = surface->Value(u0 + u * (u1 - u0), v0 + v * (v1 - v0)).SquareDistance(target);
        if (d < nearest) {
          nearest = d;
          uv = {u, v};
        }
      }
    for (int i = 0; i < 24; ++i) {
      gp_Pnt a;
      gp_Vec su, sv;
      surface->D1(u0 + uv[0] * (u1 - u0), v0 + uv[1] * (v1 - v0), a, su, sv);
      su.Multiply(u1 - u0);
      sv.Multiply(v1 - v0);
      const gp_Vec residual(a, target);
      const double aa = su.SquareMagnitude(), bb = sv.SquareMagnitude(), ab = su.Dot(sv);
      const double det = aa * bb - ab * ab;
      if (!(det > 1e-15 * aa * bb))
        break;
      const double du = (bb * su.Dot(residual) - ab * sv.Dot(residual)) / det;
      const double dv = (aa * sv.Dot(residual) - ab * su.Dot(residual)) / det;
      const UV next = {std::clamp(uv[0] + du, 0., 1.), std::clamp(uv[1] + dv, 0., 1.)};
      if (next == uv)
        break;
      uv = next;
    }
    for (int i = 0; i < 2; ++i) {
      if (uv[i] > 1e-4 && uv[i] < 1. - 1e-4)
        continue;
      UV boundary = uv;
      boundary[i] = uv[i] < .5 ? 0. : 1.;
      if (surface->Value(u0 + boundary[0] * (u1 - u0), v0 + boundary[1] * (v1 - v0)).Distance(target) <=
          fit)
        uv = boundary;
    }
    return surface->Value(u0 + uv[0] * (u1 - u0), v0 + uv[1] * (v1 - v0)).Distance(target) <= fit;
  }

  // On the bounding UV rectangle L is the affine map to the source triangle.
  // Degree-elevate H=w*S and multiply the Bernstein net of w by the bilinear
  // (in fact affine) net of L. Positive rational weights bound |S-L| by the
  // largest norm of a numerator coefficient divided by the minimum weight.
  bool bound(const std::array<UV, 3> &uv, const std::array<Point, 3> &xyz, double deviation, int depth,
             Certificate &certificate) const {
    if (++certificate.boxes > 160000)
      return certificate.reject("subdivision budget");
    auto x = std::minmax({uv[0][0], uv[1][0], uv[2][0]});
    auto y = std::minmax({uv[0][1], uv[1][1], uv[2][1]});
    auto clipped = Handle(Geom_BSplineSurface)::DownCast(surface->Copy());
    clipped->Segment(u0 + x.first * (u1 - u0), u0 + x.second * (u1 - u0), v0 + y.first * (v1 - v0),
                     v0 + y.second * (v1 - v0), 1e-13, 1e-13);
    if (clipped->NbUKnots() != 2 || clipped->NbVKnots() != 2)
      return certificate.reject("multiple Bernstein spans");
    const int p = clipped->UDegree(), q = clipped->VDegree();
    const double a = uv[1][0] - uv[0][0], b = uv[2][0] - uv[0][0];
    const double c = uv[1][1] - uv[0][1], d = uv[2][1] - uv[0][1], det = a * d - b * c;
    if (!std::isfinite(det) || std::abs(det) < 1e-18)
      return certificate.reject("singular source triangle");
    Point corners[2][2];
    for (int i : {0, 1})
      for (int j : {0, 1}) {
        const double u = (i ? x.second : x.first) - uv[0][0], v = (j ? y.second : y.first) - uv[0][1];
        const double s = (d * u - b * v) / det, t = (a * v - c * u) / det;
        for (int k = 0; k < 3; ++k)
          corners[i][j][k] = xyz[0][k] + s * (xyz[1][k] - xyz[0][k]) + t * (xyz[2][k] - xyz[0][k]);
      }
    double max_numerator = 0., min_weight = std::numeric_limits<double>::infinity();
    for (int i = 0; i <= p + 1; ++i)
      for (int j = 0; j <= q + 1; ++j) {
        Point numerator{};
        for (int h : {0, 1})
          for (int l : {0, 1}) {
            const int s = i - h, t = j - l;
            if (s < 0 || s > p || t < 0 || t > q)
              continue;
            const double weight = clipped->Weight(s + 1, t + 1);
            if (!std::isfinite(weight) || weight <= 0)
              return certificate.reject("nonpositive weight");
            const double bu = double(i) / (p + 1), bv = double(j) / (q + 1);
            const double coeff = (h ? bu : 1 - bu) * (l ? bv : 1 - bv) * weight;
            min_weight = std::min(min_weight, weight);
            const auto pole = coordinates(clipped->Pole(s + 1, t + 1));
            for (int k = 0; k < 3; ++k)
              numerator[k] += coeff * (pole[k] - corners[h][l][k]);
          }
        max_numerator = std::max(max_numerator, std::hypot(numerator[0], numerator[1], numerator[2]));
      }
    const double limit = max_numerator / min_weight + 1e-9;
    if (limit <= deviation) {
      certificate.maximum = std::max(certificate.maximum, limit);
      return true;
    }
    if (depth == 7)
      return certificate.reject("interior deviation bound");
    std::array<UV, 3> m;
    std::array<Point, 3> n;
    for (size_t i = 0; i < 3; ++i) {
      const auto j = (i + 1) % 3;
      for (int k = 0; k < 2; ++k)
        m[i][k] = (uv[i][k] + uv[j][k]) / 2.;
      for (int k = 0; k < 3; ++k)
        n[i][k] = (xyz[i][k] + xyz[j][k]) / 2.;
    }
    for (size_t i = 0; i < 3; ++i) {
      const auto j = (i + 2) % 3;
      if (!bound({uv[i], m[i], m[j]}, {xyz[i], n[i], n[j]}, deviation, depth + 1, certificate))
        return false;
    }
    return bound(m, n, deviation, depth + 1, certificate);
  }

  bool certify(const std::vector<Point> &vertices, const std::vector<Tri> &triangles, double fit,
               double deviation, Certificate &certificate) const {
    if (surface.IsNull() || triangles.size() < 4 || triangles.size() > 16384 || vertices.size() > 49152 ||
        !(fit > 0 && deviation > fit) || surface->NbUKnots() != 2 || surface->NbVKnots() != 2 ||
        surface->UDegree() > 9 || surface->VDegree() > 9)
      return certificate.reject("unsupported patch");
    std::map<size_t, UV> chart;
    std::map<std::pair<size_t, size_t>, int> edges;
    int sign = 0;
    for (const auto &ids : triangles) {
      std::array<UV, 3> uv;
      std::array<Point, 3> xyz;
      for (size_t i = 0; i < 3; ++i) {
        const auto id = ids[i];
        if (id >= vertices.size())
          return certificate.reject("source index");
        auto [it, fresh] = chart.try_emplace(id);
        if (fresh && !project(vertices[id], fit, it->second))
          return certificate.reject("interior vertex off surface");
        uv[i] = it->second;
        xyz[i] = vertices[id];
        const auto j = ids[(i + 1) % 3];
        auto &incidence = edges[std::minmax(id, j)];
        // Distinct directed edges on an oriented manifold have one forward
        // and one reverse occurrence. Reject repetitions immediately.
        const int bit = id < j ? 1 : 2;
        if (id == j || (incidence & bit))
          return certificate.reject("patch incidence");
        incidence |= bit;
      }
      const double determinant =
          (uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1]) - (uv[2][0] - uv[0][0]) * (uv[1][1] - uv[0][1]);
      const int orientation = determinant > 0 ? 1 : -1;
      if (std::abs(determinant) < 1e-15 || (sign && sign != orientation))
        return certificate.reject("folded UV patch");
      sign = orientation;
      if (!bound(uv, xyz, deviation, 0, certificate))
        return false;
      ++certificate.triangles;
    }
    // Same signed triangle orientation + one degree-one rectangular boundary
    // implies unit coverage everywhere by winding number. Thus the continuous
    // bound above is bidirectional over the entire native patch.
    std::array<std::vector<std::pair<double, double>>, 4> sides;
    for (const auto &[ids, incidence] : edges) {
      if (incidence == 3)
        continue;
      const auto a = chart.at(ids.first), b = chart.at(ids.second);
      bool placed = false;
      for (int k : {0, 1})
        for (int h : {0, 1})
          if (a[k] == h && b[k] == h && a[1 - k] != b[1 - k]) {
            sides[2 * k + h].push_back(std::minmax(a[1 - k], b[1 - k]));
            placed = true;
          }
      if (!placed)
        return certificate.reject("nonrectangular boundary");
    }
    for (auto &side : sides) {
      std::sort(side.begin(), side.end());
      double end = 0;
      for (const auto &interval : side) {
        if (interval.first != end)
          return certificate.reject("boundary gap or overlap");
        end = interval.second;
      }
      if (end != 1)
        return certificate.reject("uncovered side");
    }
    return true;
  }
};
} // namespace unequal_corner
