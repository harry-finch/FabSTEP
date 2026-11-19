#pragma once
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Curve2d.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <gp_Circ.hxx>
#include <GeomConvert.hxx>
#include <Geom_BSplineSurface.hxx>
#include <Geom_BezierSurface.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <limits>

namespace atlas_arc_law {
// Sewing can replace a rational quadratic boundary curve with the adjacent
// torus's analytic circle. The two traces describe the same arc, but the
// linear pcurve on the ruled face advances at the wrong angular speed.
inline bool align(const TopoDS_Face &face, const TopoDS_Edge &edge, const Handle(Geom_BSplineSurface) &net) {
  if (BRep_Tool::Degenerated(edge) || !BRep_Tool::SameParameter(edge))
    return false;
  const BRepAdaptor_Curve circle(edge);
  const BRepAdaptor_Curve2d pc(edge, face);
  if (circle.GetType() != GeomAbs_Circle || pc.GetType() != GeomAbs_BSplineCurve)
    return false;
  const auto old = pc.BSpline();
  const double start = circle.FirstParameter(), finish = circle.LastParameter(), h = (finish - start) / 2.;
  if (!(h > 0. && h < .5) || old->IsPeriodic() || old->IsRational() || old->Degree() != 1 || old->NbPoles() != 2 ||
      old->NbKnots() != 2 || old->Multiplicity(1) != 2 || old->Multiplicity(2) != 2 ||
      std::abs(start - old->Knot(1)) > Precision::PConfusion() ||
      std::abs(finish - old->Knot(2)) > Precision::PConfusion() || start != pc.FirstParameter() || finish != pc.LastParameter())
    return false;
  // Reuse the two canonical UV controls. Surface and circle parameters can
  // differ at a knot by a rounding unit; the new curve is defined on the
  // actual circle interval, entirely in [0,1] on the indicated support row.
  const auto a = old->Pole(1), b = old->Pole(2);
  if (a.Y() != b.Y() || (a.Y() != 0. && a.Y() != 1.) ||
      !((a.X() == 0. && b.X() == 1.) || (a.X() == 1. && b.X() == 0.)))
    return false;
  // An observed excessive residual nominates this coedge; it is not a proof
  // of the new curve. Already accurate pcurves keep their representation.
  bool needed = false;
  for (const double t : {.2113248654051871, .7886751345948129}) {
    const double parameter = start + (finish - start) * t;
    const auto u = pc.Value(parameter);
    needed = needed || net->Value(u.X(), u.Y()).Distance(circle.Value(parameter)) > Precision::Confusion();
  }
  if (!needed)
    return false;
  const int row = a.Y() == 0. ? 1 : 2;
  const auto support = circle.Circle();
  const auto center = support.Location(), mid = circle.Value(start + h);
  const double w = std::cos(h), k = std::tan(h * .5);
  const std::array<gp_Pnt, 3> ideal{circle.Value(start), center.Translated(gp_Vec(center, mid) / w), circle.Value(finish)};
  const double scale = center.XYZ().Modulus() + support.Radius();
  double drift = 0., diameter = 0.;
  std::array<double, 3> ratios;
  for (int i = 0; i < 3; ++i) {
    const int index = a.X() == 0. ? i + 1 : 3 - i;
    const double displacement = ideal[i].Distance(net->Pole(index, row));
    ratios[i] = net->Weight(index, row) / (i == 1 ? w : 1.);
    if (!std::isfinite(displacement) || !std::isfinite(ratios[i]) || ratios[i] <= 0.)
      return false;
    drift = std::max(drift, displacement);
    for (const auto &point : ideal) {
      const double distance = ideal[i].Distance(point);
      if (!std::isfinite(distance))
        return false;
      diameter = std::max(diameter, distance);
    }
  }
  const auto limits = std::minmax_element(ratios.begin(), ratios.end());
  // Positive Bernstein blending bounds pole perturbation by its maximum.
  // Reweighting the same convex mean adds diameter * spread/minimum.
  const double control_error = drift + diameter * (*limits.second - *limits.first) / *limits.first;
  // For normalized angle x, the exact net parameter is
  // t=(1+tan(h*(x-1/2))/k)/2. Hermite matches t and t' at both ends.
  // |t''''|/384 bounds its error; the arc speed per unit t is <=4Rk.
  const double hermite = support.Radius() * 4 * k * std::pow(h, 4) * (1 + k * k) * (2 + 3 * k * k) / 96.;
  if (!(control_error + hermite + 128. * std::numeric_limits<double>::epsilon() * scale < .5 * Precision::Confusion()))
    return false;
  const double slope = h / std::sin(h);
  TColgp_Array1OfPnt2d controls(1, 4);
  controls(1) = a;
  controls(2) = gp_Pnt2d(a.XY() + (b.XY() - a.XY()) * (slope / 3.));
  controls(3) = gp_Pnt2d(b.XY() - (b.XY() - a.XY()) * (slope / 3.));
  controls(4) = b;
  TColStd_Array1OfReal knots(1, 2);
  knots(1) = start;
  knots(2) = finish;
  TColStd_Array1OfInteger multiplicities(1, 2);
  multiplicities.Init(4);
  BRep_Builder builder;
  builder.UpdateEdge(edge, new Geom2d_BSplineCurve(controls, knots, multiplicities, 3), face, BRep_Tool::Tolerance(edge));
  builder.Range(edge, face, start, finish);
  return true;
}

inline size_t align(const TopoDS_Face &face) {
  const BRepAdaptor_Surface s(face, false);
  if ((s.GetType() != GeomAbs_BezierSurface && s.GetType() != GeomAbs_BSplineSurface) ||
      s.UDegree() != 2 || s.VDegree() != 1)
    return 0;
  const auto net = s.GetType() == GeomAbs_BSplineSurface ? s.BSpline() : GeomConvert::SurfaceToBSplineSurface(s.Bezier());
  if (net->IsUPeriodic() || net->IsVPeriodic() || net->NbUPoles() != 3 || net->NbVPoles() != 2 ||
      net->NbUKnots() != 2 || net->NbVKnots() != 2 || net->UMultiplicity(1) != 3 || net->UMultiplicity(2) != 3 ||
      net->VMultiplicity(1) != 2 || net->VMultiplicity(2) != 2 || net->UKnot(1) != 0. || net->UKnot(2) != 1. ||
      net->VKnot(1) != 0. || net->VKnot(2) != 1. || !net->Pole(1, 1).IsEqual(net->Pole(1, 2), 0.))
    return 0;
  size_t count = 0;
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next())
    count += align(face, TopoDS::Edge(it.Current()), net);
  return count;
}
} // namespace atlas_arc_law
