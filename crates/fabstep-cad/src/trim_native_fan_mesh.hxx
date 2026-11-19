#pragma once
#include "trim_edge_samples.hxx"

#include <BRepAdaptor_Surface.hxx>
#include <BRepAdaptor_Curve2d.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <TopExp.hxx>
#include <algorithm>
#include <BRepTools_WireExplorer.hxx>
#include <BRep_Tool.hxx>
#include <Geom_BSplineSurface.hxx>
#include <Poly_Triangulation.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Wire.hxx>
#include <array>
#include <cmath>
#include <optional>
#include <vector>

namespace trimfan {
using Row = std::vector<gp_Vec>;
using Weights = std::vector<double>;
struct Profile {
  std::array<Row, 2> poles; // relative to the shared apex
  std::array<Weights, 2> weights;
  std::array<gp_Pnt, 3> corners;
  gp_Pnt apex;
  gp_Vec normal;
  double linear_error{};
};
inline int choose(int n, int k) {
  int c = 1;
  for (int i = 1; i <= k; ++i)
    c = c * (n + 1 - i) / i;
  return c;
}
template <class A, class B, class Op> auto product(const A &a, const B &b, Op op) {
  using Value = decltype(op(a.front(), b.front()));
  int n = int(a.size()) - 1, m = int(b.size()) - 1;
  std::vector<Value> out(n + m + 1);
  for (int i = 0; i <= n; ++i)
    for (int j = 0; j <= m; ++j)
      out[i + j] += op(a[i], b[j]) * (double(choose(n, i) * choose(m, j)) / choose(n + m, i + j));
  return out;
}
inline bool normal_cone(const Profile &p, double cosine) {
  auto weighted = [](gp_Vec v, double w) { return v * w; };
  auto n = p.poles;
  for (int k = 0; k < 2; ++k)
    for (size_t i = 0; i < n[k].size(); ++i)
      n[k][i] *= p.weights[k][i];
  auto left = product(n[0], p.weights[1], weighted), right = product(n[1], p.weights[0], weighted);
  Row gap = right;
  for (size_t i = 0; i < gap.size(); ++i)
    gap[i] -= left[i];
  bool positive = false;
  for (int k = 0; k < 2; ++k) {
    int degree = int(n[k].size()) - 1;
    Row dn(degree);
    Weights dw(degree);
    for (int i = 0; i < degree; ++i) {
      dn[i] = (n[k][i + 1] - n[k][i]) * degree;
      dw[i] = (p.weights[k][i + 1] - p.weights[k][i]) * degree;
    }
    auto d = product(dn, p.weights[k], weighted), s = product(n[k], dw, weighted);
    for (size_t i = 0; i < d.size(); ++i)
      d[i] -= s[i];
    auto normals = product(d, gap, [](gp_Vec a, gp_Vec b) { return a.Crossed(b); });
    for (auto v : normals) {
      double length = v.Magnitude();
      if (length == 0)
        continue;
      if (v.Dot(p.normal) < length * cosine)
        return false;
      positive = true;
    }
  }
  return positive;
}
inline std::optional<Profile> profile(const TopoDS_Face &face, double deflection, double angle) {
  if (!(deflection > 0 && angle > 0 && angle <= 3.141592653589793))
    return {};
  const BRepAdaptor_Surface native(face);
  if (native.GetType() != GeomAbs_BSplineSurface)
    return {};
  auto s = native.BSpline();
  if (s->NbUKnots() != 2 || s->NbVKnots() != 2 || s->UDegree() > 3 || s->VDegree() > 3 || s->IsUPeriodic() ||
      s->IsVPeriodic())
    return {};
  const bool exchanged = s->VDegree() != 1;
  if (exchanged) {
    if (s->UDegree() != 1)
      return {};
    s = Handle(Geom_BSplineSurface)::DownCast(s->Copy());
    s->ExchangeUV();
  }
  const int degree = s->UDegree();
  if (s->NbUPoles() != degree + 1 || s->NbVPoles() != 2 || degree < 1 || s->UMultiplicity(1) != degree + 1 ||
      s->UMultiplicity(2) != degree + 1 || s->VMultiplicity(1) != 2 || s->VMultiplicity(2) != 2)
    return {};
  // The full Bezier chart has four sides, one collapsed. Verifying the three
  // material boundaries against its corners below rejects arbitrary subtrims.
  std::array<gp_Pnt, 4> uvCorners{s->Pole(1, 1), s->Pole(degree + 1, 1), s->Pole(degree + 1, 2),
                                  s->Pole(1, 2)};
  std::vector<gp_Pnt> distinct;
  for (auto &pt : uvCorners) {
    if (std::none_of(distinct.begin(), distinct.end(), [&](auto q) { return pt.Distance(q) <= 1e-9; }))
      distinct.push_back(pt);
  }
  if (distinct.size() != 3)
    return {};
  Profile p;
  std::copy(distinct.begin(), distinct.end(), p.corners.begin());
  p.normal = gp_Vec(distinct[0], distinct[1]).Crossed(gp_Vec(distinct[0], distinct[2]));
  if (p.normal.Magnitude() < 1e-12)
    return {};
  p.normal.Normalize();
  bool collapsed = false;
  for (int k = 0; k < 4; ++k)
    if (uvCorners[k].Distance(uvCorners[(k + 1) % 4]) <= 1e-9) {
      p.apex = uvCorners[k];
      collapsed = true;
    }
  if (!collapsed)
    return {};
  for (int row = 1; row <= 2; ++row) {
    if (s->Pole(1, row).Distance(s->Pole(degree + 1, row)) <= 1e-9)
      for (int i = 1; i <= degree + 1; ++i)
        if (s->Pole(i, row).Distance(p.apex) > 1e-9)
          return {};
  }
  for (int k = 0; k < 2; ++k) {
    for (int i = 0; i <= degree; ++i) {
      p.poles[k].emplace_back(p.apex, s->Pole(i + 1, k + 1));
      double w = s->Weight(i + 1, k + 1);
      if (!(w > 0 && std::isfinite(w)))
        return {};
      p.weights[k].push_back(w);
    }
    const auto &pts = p.poles[k];
    const auto &w = p.weights[k];
    // N-W*L, degree d+1. Convexity bounds the discrepancy between the
    // complete rational row and its linearly parameterized endpoint chord.
    double bound = 0;
    for (int j = 0; j <= degree + 1; ++j) {
      gp_Vec delta;
      if (j <= degree)
        delta += (pts[j] - pts.front()) * (w[j] * (degree + 1 - j) / (degree + 1));
      if (j > 0)
        delta += (pts[j - 1] - pts.back()) * (w[j - 1] * j / (degree + 1));
      bound = std::max(bound, delta.Magnitude());
    }
    p.linear_error = std::max(p.linear_error, bound / *std::min_element(w.begin(), w.end()));
  }
  if (2 * p.linear_error + 1e-7 > deflection || !normal_cone(p, std::cos(angle / 2)))
    return {};
  if (exchanged)
    p.normal.Reverse();
  return p;
}

// Recognize the actual collapsed boundary separately from the export or
// preview resolution. A closed loop merely returning to the same vertex cannot
// establish a pole. The full three-sided patch and common projected normal
// still have to be proved by the control net.
inline std::optional<gp_Pnt> pole(const TopoDS_Face &face, const TopoDS_Edge &edge) {
  if (!BRep_Tool::Degenerated(edge))
    return {};
  auto v = TopExp::FirstVertex(edge), last = TopExp::LastVertex(edge);
  if (v.IsNull() || !v.IsSame(last))
    return {};
  auto p = profile(face, .05, 3.141592653589793 - 1e-5);
  if (!p || BRep_Tool::Pnt(v).Distance(p->apex) > 1e-7)
    return {};
  return p->apex;
}

template <class EdgeSamples>
Handle(Poly_Triangulation)
    mesh(const TopoDS_Face &forward, double deflection, double angle, EdgeSamples samples) {
  auto certified = profile(forward, deflection, angle);
  if (!certified)
    return {};
  TopExp_Explorer wires(forward, TopAbs_WIRE);
  if (!wires.More())
    return {};
  auto wire = TopoDS::Wire(wires.Current());
  wires.Next();
  if (wires.More())
    return {};
  std::array<std::vector<int>, 3> sides;
  std::vector<gp_Pnt> boundary;
  std::array<gp_Pnt, 3> ends;
  int side = 0, collapsed = 0;
  for (BRepTools_WireExplorer it(wire, forward); it.More(); it.Next()) {
    TopoDS_Edge edge = it.Current();
    if (BRep_Tool::Degenerated(edge)) {
      if (++collapsed != 1 || BRep_Tool::Pnt(TopExp::FirstVertex(edge)).Distance(certified->apex) > 1e-7)
        return {};
      continue;
    }
    if (side >= 3 || !BRep_Tool::SameParameter(edge))
      return {};
    double lo, hi;
    auto pc = BRep_Tool::CurveOnSurface(edge, forward, lo, hi);
    if (pc.IsNull())
      return {};
    const auto &path = samples(TopoDS::Edge(edge.Oriented(TopAbs_FORWARD)));
    if (path.size() < 2 || path.size() > 100000 || std::abs(path.front().parameter - lo) > 1e-8 ||
        std::abs(path.back().parameter - hi) > 1e-8)
      return {};
    BRepAdaptor_Surface surface(forward);
    // Only full isoparametric sides qualify. A line pcurve or a spline whose
    // poles all lie on one side bounds the complete trim on the native chart.
    BRepAdaptor_Curve2d trim(edge, forward);
    auto uv0 = pc->Value(lo), uv1 = pc->Value(hi);
    bool constU = std::abs(uv0.X() - uv1.X()) < 1e-9, constV = std::abs(uv0.Y() - uv1.Y()) < 1e-9;
    if (constU == constV)
      return {};
    double constant = constU ? uv0.X() : uv0.Y();
    double fixedLo = constU ? surface.FirstUParameter() : surface.FirstVParameter();
    double fixedHi = constU ? surface.LastUParameter() : surface.LastVParameter();
    double runLo = constU ? surface.FirstVParameter() : surface.FirstUParameter();
    double runHi = constU ? surface.LastVParameter() : surface.LastUParameter();
    auto along = [&](gp_Pnt2d p) { return constU ? p.Y() : p.X(); };
    const double begin = along(uv0), end = along(uv1);
    if (std::min(std::abs(constant - fixedLo), std::abs(constant - fixedHi)) > 1e-8 ||
        std::abs(std::min(begin, end) - runLo) > 1e-8 || std::abs(std::max(begin, end) - runHi) > 1e-8)
      return {};
    auto onSide = [&](gp_Pnt2d uv) {
      return (constU ? std::abs(uv.X() - uv0.X()) : std::abs(uv.Y() - uv0.Y())) < 1e-9;
    };
    if (trim.GetType() != GeomAbs_Line) {
      if (trim.GetType() != GeomAbs_BSplineCurve)
        return {};
      auto bs = trim.BSpline();
      if (bs->IsPeriodic())
        return {};
      int poleDir = 0;
      for (int j = 1; j <= bs->NbPoles(); ++j) {
        if (!onSide(bs->Pole(j)) || bs->Weight(j) <= 0)
          return {};
        if (j == 1)
          continue;
        double delta = along(bs->Pole(j)) - along(bs->Pole(j - 1));
        int step = (delta > 0) - (delta < 0);
        if (step && poleDir && step != poleDir)
          return {};
        if (step)
          poleDir = step;
      }
    }
    for (size_t j = 0; j < path.size(); ++j) {
      auto &sample = path[j];
      if (j && !(sample.parameter > path[j - 1].parameter))
        return {};
      auto uv = pc->Value(sample.parameter);
      if (surface.Value(uv.X(), uv.Y()).Distance(sample.point) > 1e-7)
        return {};
    }
    bool reverse = edge.Orientation() == TopAbs_REVERSED;
    for (size_t j = 0; j < path.size(); ++j) {
      auto &pt = path[reverse ? path.size() - 1 - j : j].point;
      if (j + 1 == path.size()) {
        ends[side] = pt;
        break;
      }
      sides[side].push_back(int(boundary.size()));
      boundary.push_back(pt);
    }
    ++side;
  }
  if (side != 3 || collapsed != 1 || boundary.size() > 100000)
    return {};
  std::array<int, 3> corners;
  std::array<bool, 3> found{};
  for (int k = 0; k < 3; ++k) {
    corners[k] = sides[k].front();
    if (ends[(k + 2) % 3].Distance(boundary[corners[k]]) > 1e-7)
      return {};
    int match = -1;
    for (int j = 0; j < 3; ++j)
      if (boundary[corners[k]].Distance(certified->corners[j]) < 1e-7)
        match = j;
    if (match < 0 || found[match])
      return {};
    found[match] = true;
  }
  std::vector<std::array<int, 3>> tris{corners};
  // Refine the three boundary sides of a triangle. Every boundary knot from
  // neighboring faces is preserved, with exactly B-2 nondegenerate triangles.
  for (int k = 0; k < 3; ++k) {
    auto &knots = sides[k];
    if (knots.size() == 1)
      continue;
    auto start = corners[k], end = corners[(k + 1) % 3];
    int apex = -1;
    for (size_t t = 0; t < tris.size() && apex < 0; ++t)
      for (int j = 0; j < 3; ++j)
        if (tris[t][j] == start && tris[t][(j + 1) % 3] == end) {
          apex = tris[t][(j + 2) % 3];
          tris.erase(tris.begin() + t);
          break;
        }
    if (apex < 0)
      return {};
    for (size_t i = 0; i < knots.size(); ++i)
      tris.push_back({knots[i], i + 1 < knots.size() ? knots[i + 1] : end, apex});
  }
  for (auto t : tris) {
    auto n = gp_Vec(boundary[t[0]], boundary[t[1]]).Crossed(gp_Vec(boundary[t[0]], boundary[t[2]]));
    if (n.Magnitude() < 1e-14 || n.Dot(certified->normal) < n.Magnitude() * std::cos(angle / 2))
      return {};
  }
  Handle(Poly_Triangulation) out =
      new Poly_Triangulation(int(boundary.size()), int(tris.size()), false, false);
  for (size_t i = 0; i < boundary.size(); ++i)
    out->SetNode(i + 1, boundary[i]);
  for (size_t i = 0; i < tris.size(); ++i) {
    auto q = tris[i];
    out->SetTriangle(i + 1, Poly_Triangle(q[0] + 1, q[1] + 1, q[2] + 1));
  }
  return out;
}
} // namespace trimfan
