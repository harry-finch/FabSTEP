// Repair a producer-built one-arc loft only when the kernel reports its
// collapsed edge as a self-intersection. A rational quadratic traces the
// measured circle exactly and keeps the original wire, including its p-curves
// on adjacent faces. All ordinary validation still runs on the complete solid.
#pragma once
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepAlgoAPI_Check.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepTools.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_BSplineSurface.hxx>
#include <gp_Circ.hxx>
#include <Geom_BezierSurface.hxx>
#include <GeomConvert.hxx>
#include <IntTools_FaceFace.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <array>
#include <algorithm>
#include <cmath>
#include <optional>
#include <stdexcept>

namespace atlas_fan {
struct Controls {
  gp_Pnt apex, first, middle, last;
  TopoDS_Edge edge;
  double angle0, angle1, weight;
  bool flip_u, flip_v;
};

// A consistent sign of all quintic Bernstein coefficients proves strict
// monotonicity of polar angle around the projected apex. All base controls in
// one open half-plane exclude a full turn and hence coincident interior rays.
inline bool injective(const Handle(Geom_BSplineSurface) & s, int base, const gp_Pnt &apex, const gp_Dir &n,
                      const gp_Pnt &middle, double tol) {
  gp_Vec direction(apex, middle), normal(n);
  direction -= normal * direction.Dot(normal);
  if (direction.Magnitude() <= tol)
    return false;
  direction.Normalize();
  std::array<gp_Vec, 4> h;
  double size = 0.;
  for (int i = 0; i < 4; ++i) {
    h[i] = gp_Vec(apex, s->Pole(i + 1, base));
    if (h[i].Dot(direction) <= tol)
      return false;
    size = std::max(size, h[i].Magnitude());
  }
  constexpr int c3[] = {1, 3, 3, 1}, c2[] = {1, 2, 1}, c5[] = {1, 5, 10, 10, 5, 1};
  double low = INFINITY, high = -INFINITY;
  for (int k = 0; k <= 5; ++k) {
    double coefficient = 0.;
    for (int i = 0; i < 4; ++i) {
      int j = k - i;
      if (j < 0 || j > 2)
        continue;
      coefficient += double(3 * c3[i] * c2[j]) / c5[k] * h[i].Crossed(h[j + 1] - h[j]).Dot(normal);
    }
    low = std::min(low, coefficient);
    high = std::max(high, coefficient);
  }
  return low > tol * size || high < -tol * size;
}

// For the rational replacement, let H_i=w_i(P_i-apex). The polar
// derivative has the sign of det(H,H'), since the denominator is positive.
// Its three quadratic Bernstein coefficients have the signs of the pairwise
// determinants below. The half-plane condition also excludes a full turn.
inline bool circle_injective(const gp_Pnt &apex, const gp_Pnt &a, const gp_Pnt &b, const gp_Pnt &c,
                             const gp_Dir &normal, double tol) {
  const gp_Vec x(apex, a), y(apex, b), z(apex, c), n(normal);
  gp_Vec direction = x + z;
  direction -= n * direction.Dot(n);
  if (direction.Magnitude() <= tol)
    return false;
  direction.Normalize();
  if (std::min({x.Dot(direction), y.Dot(direction), z.Dot(direction)}) <= tol)
    return false;
  const double margin = tol * std::max({x.Magnitude(), y.Magnitude(), z.Magnitude()});
  const double xy = x.Crossed(y).Dot(n), xz = x.Crossed(z).Dot(n), yz = y.Crossed(z).Dot(n);
  return std::min({xy, xz, yz}) > margin || std::max({xy, xz, yz}) < -margin;
}

inline std::optional<Controls> controls(const TopoDS_Face &face) {
  BRepAdaptor_Surface surface(face);
  if (surface.GetType() != GeomAbs_BSplineSurface)
    return {};
  auto net = surface.BSpline();
  if (net->UDegree() != 3 || net->VDegree() != 1 || net->NbUPoles() != 4 || net->NbVPoles() != 2 ||
      net->IsURational() || net->IsVRational() || net->IsUPeriodic() || net->IsVPeriodic() ||
      net->NbUKnots() != 2 || net->NbVKnots() != 2 || net->UKnot(1) != 0. || net->UKnot(2) != 1. ||
      net->VKnot(1) != 0. || net->VKnot(2) != 1.)
    return {};
  int apex_row = 0;
  for (int j : {1, 2}) {
    bool collapsed = true;
    for (int i = 2; i <= 4; ++i)
      collapsed = collapsed && net->Pole(i, j).IsEqual(net->Pole(1, j), 0.);
    if (collapsed) {
      if (apex_row)
        return {};
      apex_row = j;
    }
  }
  if (!apex_row)
    return {};
  TopoDS_Edge rim;
  int count = 0, degenerate = 0;
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto e = TopoDS::Edge(it.Current());
    ++count;
    if (BRep_Tool::Degenerated(e)) {
      ++degenerate;
      continue;
    }
    auto c = BRepAdaptor_Curve(e);
    if (c.GetType() == GeomAbs_Circle) {
      if (!rim.IsNull())
        return {};
      rim = e;
    } else if (c.GetType() != GeomAbs_Line &&
               (c.GetType() != GeomAbs_BSplineCurve || c.Degree() != 1 || c.NbPoles() != 2 || c.IsRational()))
      return {};
  }
  if (count != 4 || degenerate != 1 || rim.IsNull())
    return {};
  const auto c = BRepAdaptor_Curve(rim);
  const double a = c.FirstParameter(), b = c.LastParameter(), h = (b - a) / 2.;
  if (!(h > 0. && h < .5))
    return {};
  const auto circle = c.Circle();
  const auto pole = net->Pole(1, apex_row);
  const double height = gp_Vec(circle.Location(), pole).Dot(gp_Vec(circle.Axis().Direction()));
  if (std::abs(height) > Precision::Confusion())
    return {};
  const auto first = c.Value(a), last = c.Value(b), mid = c.Value(a + h);
  const auto radius = circle.Radius();
  const double tol = BRep_Tool::Tolerance(rim);
  // Inverse normalized angular map u(t)=1/2+cot(h/2)*tan(h*(t-1/2))/2.
  // Its cubic Hermite interpolant has error bounded by max|u''''|/384.
  // Multiply by the largest angular speed of the rational span and radius.
  const double tangent = std::tan(h / 2.);
  const double pcurve_error =
      radius * 4 * tangent * std::pow(h, 4) * (1 + tangent * tangent) * (2 + 3 * tangent * tangent) / 96.;
  if (!(pcurve_error < .25 * tol) ||
      !injective(net, 3 - apex_row, pole, circle.Axis().Direction(), mid, 4 * tol))
    return {};
  const double w = std::cos(h);
  const gp_Pnt control((mid.XYZ() * (1 + w) - (first.XYZ() + last.XYZ()) * .5) / w);
  if (!circle_injective(pole, first, control, last, circle.Axis().Direction(), 4 * tol))
    return {};
  const bool flip_u = net->Pole(1, 3 - apex_row).Distance(last) < net->Pole(1, 3 - apex_row).Distance(first);
  return Controls{pole, first, control, last, rim, a, b, w, flip_u, apex_row == 2};
}

inline bool reports_apex_only(const TopoDS_Face &face, const gp_Pnt &apex) {
  IntTools_FaceFace self;
  self.Perform(face, face, false);
  if (!self.IsDone() || self.Points().Length() || self.Lines().Length() != 1)
    return false;
  const auto &line = self.Lines().Value(1);
  const auto spline = Handle(Geom_BSplineCurve)::DownCast(line.Curve());
  if (spline.IsNull() || spline->NbPoles() > 64)
    return false;
  // This is a restriction on when rebuilding may be tried. The candidate
  // must pass the ordinary BOP and exact curve-on-surface checks afterwards.
  const double radius = BRep_Tool::Tolerance(face) + 2 * line.TangentialTolerance() + line.Tolerance();
  if (!std::isfinite(radius) || radius > 8 * Precision::Confusion())
    return false;
  for (int i = 1; i <= spline->NbPoles(); ++i)
    if (!(spline->Weight(i) > 0.) || spline->Pole(i).Distance(apex) > radius)
      return false;
  return true;
}

inline Handle(Geom2d_Curve)
    boundary(const TopoDS_Edge &e, const TopoDS_Face &face, const Controls &q, double &first, double &last) {
  auto original = BRep_Tool::CurveOnSurface(e, face, first, last);
  if (original.IsNull() || !(last > first))
    return {};
  const bool arc = q.edge.IsSame(e);
  const int degree = arc ? 3 : 1;
  TColgp_Array1OfPnt2d u(1, degree + 1);
  if (arc) {
    if (std::abs(first - q.angle0) + std::abs(last - q.angle1) > 1e-12)
      return {};
    const double delta = q.angle1 - q.angle0, slope = delta / (2 * std::sin(delta / 2.));
    u.SetValue(1, {0, 1});
    u.SetValue(2, {slope / 3., 1});
    u.SetValue(3, {1 - slope / 3., 1});
    u.SetValue(4, {1, 1});
  } else {
    for (int j : {1, 2}) {
      auto p = original->Value(j == 1 ? first : last);
      // Every material side and the collapsed side must be a complete iso.
      if (std::min(std::abs(p.X()), std::abs(1 - p.X())) > 1e-9 ||
          std::min(std::abs(p.Y()), std::abs(1 - p.Y())) > 1e-9)
        return {};
      u.SetValue(j, {q.flip_u ? 1 - p.X() : p.X(), q.flip_v ? 1 - p.Y() : p.Y()});
    }
    const double dx = std::abs(u.Value(1).X() - u.Value(2).X()),
                 dy = std::abs(u.Value(1).Y() - u.Value(2).Y());
    if (BRep_Tool::Degenerated(e) ? dx < 1 - 1e-9 || dy > 1e-9 : dx > 1e-9 || dy < 1 - 1e-9)
      return {};
  }
  TColStd_Array1OfReal knots(1, 2);
  knots.SetValue(1, first);
  knots.SetValue(2, last);
  TColStd_Array1OfInteger mult(1, 2);
  mult.SetValue(1, degree + 1);
  mult.SetValue(2, degree + 1);
  return new Geom2d_BSplineCurve(u, knots, mult, degree);
}

inline std::optional<TopoDS_Face> quadratic(const TopoDS_Face &supplied, const Controls &q) {
  TColgp_Array2OfPnt poles(1, 3, 1, 2);
  TColStd_Array2OfReal weights(1, 3, 1, 2);
  for (int i = 1; i <= 3; ++i) {
    poles.SetValue(i, 1, q.apex);
    poles.SetValue(i, 2, i == 1 ? q.first : i == 2 ? q.middle : q.last);
    weights.SetValue(i, 1, i == 2 ? q.weight : 1.);
    weights.SetValue(i, 2, i == 2 ? q.weight : 1.);
  }
  Handle(Geom_BezierSurface) bezier = new Geom_BezierSurface(poles, weights);
  auto surface = GeomConvert::SurfaceToBSplineSurface(bezier);
  BRep_Builder builder;
  TopoDS_Face candidate;
  builder.MakeFace(candidate, surface, BRep_Tool::Tolerance(supplied));
  const auto face = TopoDS::Face(supplied.Oriented(TopAbs_FORWARD));
  struct Side {
    TopoDS_Edge edge;
    Handle(Geom2d_Curve) parameter;
    double first, last;
  };
  std::array<Side, 4> sides;
  size_t count = 0;
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    if (count == sides.size())
      return {};
    auto &side = sides[count++];
    side.edge = TopoDS::Edge(it.Current());
    side.parameter = boundary(side.edge, face, q, side.first, side.last);
    if (side.parameter.IsNull())
      return {};
  }
  if (count != sides.size())
    return {};
  size_t touched = 0;
  try {
    for (auto &side : sides) {
      ++touched;
      builder.UpdateEdge(side.edge, side.parameter, candidate, BRep_Tool::Tolerance(side.edge));
      builder.Range(side.edge, candidate, side.first, side.last);
    }
    auto wire = BRepTools::OuterWire(face);
    if (q.flip_u != q.flip_v)
      wire.Reverse();
    builder.Add(candidate, wire);
    candidate.Orientation(supplied.Orientation());
    if (q.flip_u != q.flip_v)
      candidate.Reverse();
    if (BRepCheck_Analyzer(candidate, true, false, true).IsValid()) {
      BRepAlgoAPI_Check bop(candidate, false, true);
      if (!bop.HasErrors() && bop.IsValid())
        return candidate;
    }
  } catch (const Standard_Failure &) {
    // Restore the shared edges before leaving a rejected trial.
  }
  try {
    for (size_t i = 0; i < touched; ++i)
      builder.UpdateEdge(sides[i].edge, Handle(Geom2d_Curve)(), candidate,
                         BRep_Tool::Tolerance(sides[i].edge));
  } catch (const Standard_Failure &) {
    throw std::runtime_error("Impossible d’annuler un raccord d’essai.");
  }
  return {};
}

inline std::optional<TopoDS_Face> repair(const TopoDS_Face &face) {
  try {
    const auto q = controls(face);
    if (q && reports_apex_only(face, q->apex))
      return quadratic(face, *q);
  } catch (const Standard_Failure &) {
    // The complete solid (including the original face) retains its full checks.
  }
  return {};
}
} // namespace atlas_fan
