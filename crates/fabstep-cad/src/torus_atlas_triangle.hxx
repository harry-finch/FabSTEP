// A regular triangular domain avoids the singular apex of a shallow collar
// and of a polynomial quadratic loft over a sufficiently short circular arc.
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
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Wire.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <limits>
#include <gp_Circ.hxx>
#include <optional>
namespace atlas_triangle {
struct Patch {
  gp_Pnt apex, first, last, opposite;
  double weight, deviation, slope;
  bool shallow;
};
inline bool quadratic_pole(const TopoDS_Face &face) {
  const BRepAdaptor_Surface surface(face, false);
  if (surface.GetType() != GeomAbs_BSplineSurface)
    return false;
  const auto net = surface.BSpline();
  if (net->UDegree() != 2 || net->VDegree() != 1 || net->NbUPoles() != 3 || net->NbVPoles() != 2 ||
      net->IsURational() || net->IsVRational() || net->IsUPeriodic() || net->IsVPeriodic() ||
      net->NbUKnots() != 2 || net->NbVKnots() != 2)
    return false;
  auto collapsed = [&](int v) {
    return net->Pole(1, v).IsEqual(net->Pole(2, v), 0.) && net->Pole(1, v).IsEqual(net->Pole(3, v), 0.);
  };
  return collapsed(1) != collapsed(2);
}
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
  const bool shallow = height <= circle.Radius() * (1 - w);
  // For quadratic interpolation at 0, 1/2 and 1, the circle's normalized
  // third derivative has norm R*(2h)^3 and max|t(t-1/2)(t-1)| < 1/8.
  // A non-shallow face enters this domain only for such a small arc, and
  // the producer additionally checks the actual loft's collapsed control net.
  const double quadratic_error = circle.Radius() * std::pow(2 * h, 3) / 48.;
  if (!(height > Precision::Confusion() && (shallow || quadratic_error < .25 * Precision::Confusion())))
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
               2 * w - 1, bound, h / std::sin(h), shallow};
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
// A STEP reader can insert a fourth, degenerate coedge when closing the
// pcurves at an end of the circular rim. Prove the whole coedge lies within
// native confusion of its sole topological vertex on this regular surface.
inline std::optional<gp_Pnt> corner(const TopoDS_Edge &edge, const TopoDS_Face &face) {
  if (!BRep_Tool::Degenerated(edge) || !BRep_Tool::SameParameter(edge))
    return {};
  TopoDS_Vertex first, last;
  TopExp::Vertices(edge, first, last);
  double lo, hi;
  if (first.IsNull() || last.IsNull() || !first.IsSame(last) || !BRep_Tool::Curve(edge, lo, hi).IsNull() ||
      BRep_Tool::Tolerance(edge) > Precision::Confusion() || BRep_Tool::Tolerance(first) > Precision::Confusion())
    return {};
  const BRepAdaptor_Surface surface(face, false);
  if (surface.GetType() != GeomAbs_BSplineSurface || surface.UDegree() != 1 || surface.VDegree() != 1)
    return {};
  const auto net = surface.BSpline();
  if (net->NbUPoles() != 2 || net->NbVPoles() != 2 || net->IsUPeriodic() || net->IsVPeriodic() ||
      net->UKnot(1) != 0. || net->UKnot(2) != 1. || net->VKnot(1) != 0. || net->VKnot(2) != 1.)
    return {};
  const BRepAdaptor_Curve2d pc(edge, face);
  if (pc.GetType() != GeomAbs_Line || !(pc.LastParameter() > pc.FirstParameter()))
    return {};
  const auto a = pc.Value(pc.FirstParameter()), b = pc.Value(pc.LastParameter());
  const gp_Pnt2d end = a.X() < .5 ? gp_Pnt2d(0, 1) : gp_Pnt2d(1, 0);
  if (!(a.Distance(end) <= 1e-8 && b.Distance(end) <= 1e-8))
    return {};
  const auto vertex = BRep_Tool::Pnt(first);
  // A bilinear homogeneous net on a straight UV segment is quadratic.
  // Its middle Bernstein control is the average of the two mixed evaluations.
  // Positive weights then enclose the entire image in the control-point ball.
  struct Homogeneous {
    gp_XYZ delta;
    double weight;
  };
  auto evaluate = [&](double u, double v) {
    Homogeneous result{{0., 0., 0.}, 0.};
    for (int i = 0; i < 2; ++i)
      for (int j = 0; j < 2; ++j) {
        const double w = net->Weight(i + 1, j + 1) * (i ? u : 1. - u) * (j ? v : 1. - v);
        result.delta += (net->Pole(i + 1, j + 1).XYZ() - vertex.XYZ()) * w;
        result.weight += w;
      }
    return result;
  };
  const auto m = evaluate(a.X(), b.Y()), n = evaluate(b.X(), a.Y());
  const Homogeneous middle{(m.delta + n.delta) * .5, (m.weight + n.weight) * .5};
  double scale = 0.;
  for (int i = 1; i <= 2; ++i)
    for (int j = 1; j <= 2; ++j)
      scale = std::max(scale, net->Weight(i, j) * net->Pole(i, j).Distance(vertex));
  const double rounding = 64. * std::numeric_limits<double>::epsilon() * scale;
  for (const auto &h : {evaluate(a.X(), a.Y()), middle, evaluate(b.X(), b.Y())})
    if (!(h.weight > 0. && h.delta.Modulus() + rounding < h.weight * (.5 * Precision::Confusion())))
      return {};
  return vertex;
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
  TopExp_Explorer wires(face, TopAbs_WIRE);
  if (!wires.More())
    return false;
  wires.Next();
  if (wires.More())
    return false;
  std::array<bool, 3> seen{};
  TopoDS_Vertex collapsed;
  std::array<TopoDS_Edge, 3> material;
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    if (BRep_Tool::Degenerated(edge)) {
      if (!collapsed.IsNull() || !corner(edge, face))
        return false;
      collapsed = TopExp::FirstVertex(edge);
      continue;
    }
    if (!BRep_Tool::SameParameter(edge))
      return false;
    int side = 0;
    while (side < 3 && !material_side(edge, face, side))
      ++side;
    if (side == 3 || seen[side])
      return false;
    seen[side] = true;
    material[side] = edge;
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
  if (!(seen[0] && seen[1] && seen[2]))
    return false;
  if (!collapsed.IsNull()) {
    const auto touches = [&](const TopoDS_Edge &edge) {
      return collapsed.IsSame(TopExp::FirstVertex(edge)) || collapsed.IsSame(TopExp::LastVertex(edge));
    };
    return touches(material[2]) && (touches(material[0]) != touches(material[1]));
  }
  return true;
}
} // namespace atlas_triangle
