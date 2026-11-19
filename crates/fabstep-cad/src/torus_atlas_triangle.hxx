// A shallow circular collar can be nearly tangent to the next trim at its
// pinched loft apex. Use a regular triangular parameter domain when the
// base sagitta exceeds the collar's height above the circle plane.
#pragma once
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Curve2d.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <GeomConvert.hxx>
#include <Geom_BSplineSurface.hxx>
#include <Geom_BezierSurface.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Wire.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <gp_Circ.hxx>
#include <optional>
namespace atlas_triangle {
struct Patch {
  gp_Pnt apex, first, last, opposite;
  double weight, deviation, slope;
};
inline bool positive(const std::array<gp_Pnt, 4> &corners, const gp_Vec &normal) {
  // In homogeneous projected coordinates H, det(H,H_u,H_v) is bilinear.
  // Its four corner signs are those of the projected corner turns because
  // the weights are positive. The convex, ordered quadrilateral boundary
  // and positive Jacobian certify injectivity on the rectangle and its trim.
  for (size_t i = 0; i < 4; ++i)
    if (!(gp_Vec(corners[(i + 3) % 4], corners[i])
              .Crossed(gp_Vec(corners[i], corners[(i + 1) % 4]))
              .Dot(normal) > 1e-14))
      return false;
  return true;
}
inline std::optional<Patch> geometry(const TopoDS_Edge &edge, const gp_Pnt &apex) {
  const BRepAdaptor_Curve curve(edge);
  if (curve.GetType() != GeomAbs_Circle)
    return {};
  const double a = curve.FirstParameter(), b = curve.LastParameter(), h = (b - a) / 2;
  if (!(h > 0 && h < .5))
    return {};
  const auto circle = curve.Circle();
  const double w = std::cos(h),
               height = std::abs(gp_Vec(circle.Location(), apex).Dot(circle.Axis().Direction()));
  if (!(height > Precision::Confusion() && height <= circle.Radius() * (1 - w)))
    return {};
  // Bound the cubic Hermite inverse-circle pcurve (angle -> rational span).
  const double t = std::tan(h / 2);
  if (!(circle.Radius() * 4 * t * std::pow(h, 4) * (1 + t * t) * (2 + 3 * t * t) / 96 <
        .25 * Precision::Confusion()))
    return {};
  const auto first = curve.Value(a), last = curve.Value(b), middle = curve.Value(a + h);
  const gp_Pnt chord((first.XYZ() + last.XYZ()) * .5);
  const gp_Pnt control((middle.XYZ() * (1 + w) - chord.XYZ()) / w);
  // In barycentric material coordinates (u,v), the affine source triangle
  // L and new surface S satisfy (S-L)*D = 2uv[w(C-chord)+(1-w)(L-chord)].
  // Since u+v<=1, 2uv/D <= 1/(1+w). This bounds the whole face both ways.
  const double bound =
      (w * control.Distance(chord) + (1 - w) * std::max(apex.Distance(chord), first.Distance(chord))) /
      (1 + w);
  return Patch{apex,      first, last,           gp_Pnt((control.XYZ() * (2 * w) - apex.XYZ()) / (2 * w - 1)),
               2 * w - 1, bound, h / std::sin(h)};
}
inline std::optional<TopoDS_Face> make(const std::array<TopoDS_Edge, 3> &edges, int arc, const Patch &p,
                                       const gp_Vec &source_normal) {
  const bool reversed = edges[arc].Orientation() == TopAbs_REVERSED;
  if (!positive({p.apex, p.first, p.opposite, p.last}, reversed ? -source_normal : source_normal))
    return {};
  TColgp_Array2OfPnt poles(1, 2, 1, 2);
  poles(1, 1) = p.apex;
  poles(2, 1) = p.first;
  poles(1, 2) = p.last;
  poles(2, 2) = p.opposite;
  TColStd_Array2OfReal weights(1, 2, 1, 2);
  weights.Init(1.);
  weights(2, 2) = p.weight;
  Handle(Geom_BezierSurface) bezier = new Geom_BezierSurface(poles, weights);
  auto surface = GeomConvert::SurfaceToBSplineSurface(bezier);
  BRep_Builder builder;
  TopoDS_Face face;
  builder.MakeFace(face, surface, Precision::Confusion());
  TopoDS_Wire wire;
  builder.MakeWire(wire);
  const gp_Pnt points[]{p.apex, p.first, p.last};
  const gp_Pnt2d uv[]{{0, 0}, {1, 0}, {0, 1}};
  std::array<Handle(Geom2d_Curve), 3> traces;
  for (int i = 0; i < 3; ++i) {
    const BRepAdaptor_Curve curve(edges[i]);
    const double a = curve.FirstParameter(), b = curve.LastParameter();
    const int degree = i == arc ? 3 : 1;
    TColgp_Array1OfPnt2d controls(1, degree + 1);
    if (i == arc) {
      const double ts[]{0, p.slope / 3, 1 - p.slope / 3, 1};
      for (int k = 0; k < 4; ++k)
        controls(k + 1) = {1 - ts[k], ts[k]};
    } else {
      if (curve.GetType() != GeomAbs_Line)
        return {};
      for (int k = 0; k < 2; ++k) {
        const auto endpoint = curve.Value(k ? b : a);
        int match = -1;
        for (int j = 0; j < 3; ++j)
          if (endpoint.Distance(points[j]) <= Precision::Confusion()) {
            if (match != -1)
              return {};
            match = j;
          }
        if (match == -1)
          return {};
        controls(k + 1) = uv[match];
      }
    }
    TColStd_Array1OfReal knots(1, 2);
    knots(1) = a;
    knots(2) = b;
    TColStd_Array1OfInteger multiplicities(1, 2);
    multiplicities.Init(degree + 1);
    traces[i] = new Geom2d_BSplineCurve(controls, knots, multiplicities, degree);
  }
  size_t touched = 0;
  try {
    for (size_t i = 0; i < edges.size(); ++i) {
      ++touched;
      BRepAdaptor_Curve curve(edges[i]);
      builder.UpdateEdge(edges[i], traces[i], face, BRep_Tool::Tolerance(edges[i]));
      builder.Range(edges[i], face, curve.FirstParameter(), curve.LastParameter());
      builder.Add(wire, edges[i]);
    }
    wire.Closed(true);
    if (reversed)
      wire.Reverse();
    builder.Add(face, wire);
    if (reversed)
      face.Reverse();
    if (BRepCheck_Analyzer(face, true, false, true).IsValid())
      return face;
  } catch (const Standard_Failure &) {
    // Fall back to the original producer after discarding this trial.
  }
  for (size_t i = 0; i < touched; ++i)
    builder.UpdateEdge(edges[i], Handle(Geom2d_Curve)(), face, BRep_Tool::Tolerance(edges[i]));
  return {};
}
// Reconstruct the support and check all three pcurves after STEP transfer.
// Only the two full axes and the monotone full diagonal bound this triangle.
inline bool material_side(const TopoDS_Edge &edge, const TopoDS_Face &face, int side) {
  const BRepAdaptor_Curve2d pc(edge, face);
  const auto a = pc.Value(pc.FirstParameter()), b = pc.Value(pc.LastParameter());
  const auto along = [&](gp_Pnt2d p) { return side == 1 ? p.Y() : p.X(); };
  const auto distance = [&](gp_Pnt2d p) {
    return std::abs(side == 0 ? p.Y() : side == 1 ? p.X() : p.X() + p.Y() - 1);
  };
  constexpr double tol = 1e-8;
  if (std::max(distance(a), distance(b)) > tol || std::abs(std::min(along(a), along(b))) > tol ||
      std::abs(std::max(along(a), along(b)) - 1) > tol)
    return false;
  if (pc.GetType() == GeomAbs_Line)
    return true;
  if (pc.GetType() != GeomAbs_BSplineCurve)
    return false;
  const auto curve = pc.BSpline();
  if (curve->IsPeriodic() || curve->NbPoles() > 256 || curve->Degree() > 3)
    return false;
  const double sign = along(a) < along(b) ? 1. : -1.;
  for (int i = 1; i <= curve->NbPoles(); ++i) {
    const auto p = curve->Pole(i);
    if (distance(p) > tol || !(curve->Weight(i) > 0.) ||
        (i > 1 && sign * (along(p) - along(curve->Pole(i - 1))) < 0))
      return false;
  }
  return true;
}
inline bool matches(const TopoDS_Face &face) {
  const BRepAdaptor_Surface s(face, false);
  if (s.GetType() != GeomAbs_BSplineSurface || s.UDegree() != 1 || s.VDegree() != 1)
    return false;
  const auto net = s.BSpline();
  if (net->NbUPoles() != 2 || net->NbVPoles() != 2 || net->IsUPeriodic() || net->IsVPeriodic() ||
      net->UKnot(1) != 0. || net->UKnot(2) != 1. || net->VKnot(1) != 0. || net->VKnot(2) != 1.)
    return false;
  const double weight = net->Weight(1, 1);
  if (!(weight > 0.) || std::abs(net->Weight(2, 1) / weight - 1.) > 1e-12 ||
      std::abs(net->Weight(1, 2) / weight - 1.) > 1e-12)
    return false;
  std::array<bool, 3> seen{};
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    if (BRep_Tool::Degenerated(edge) || !BRep_Tool::SameParameter(edge))
      return false;
    int side = 0;
    while (side < 3 && !material_side(edge, face, side))
      ++side;
    if (side == 3 || seen[side])
      return false;
    seen[side] = true;
    if (side < 2) {
      // Sewing may keep the adjacent loft's degree-one generator.
      const BRepAdaptor_Curve curve(edge);
      if (curve.GetType() != GeomAbs_Line &&
          (curve.GetType() != GeomAbs_BSplineCurve || curve.Degree() != 1 || curve.NbPoles() != 2 ||
           curve.IsRational()))
        return false;
    } else {
      const auto p = geometry(edge, net->Pole(1, 1));
      if (!p || std::abs(p->weight - net->Weight(2, 2) / weight) > 1e-10 ||
          p->opposite.Distance(net->Pole(2, 2)) > Precision::Confusion() ||
          p->first.Distance(net->Pole(2, 1)) > Precision::Confusion() ||
          p->last.Distance(net->Pole(1, 2)) > Precision::Confusion())
        return false;
      const auto normal = gp_Vec(p->apex, p->first).Crossed(gp_Vec(p->apex, p->last));
      if (normal.Magnitude() <= 1e-14 ||
          !positive({net->Pole(1, 1), net->Pole(2, 1), net->Pole(2, 2), net->Pole(1, 2)},
                    normal.Normalized()))
        return false;
    }
  }
  return seen[0] && seen[1] && seen[2];
}
} // namespace atlas_triangle
