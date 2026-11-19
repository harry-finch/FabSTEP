// An angular iso on a torus and a rational quadratic circle share their locus,
// but a linear p-curve does not share the 3D Bezier's parameter. Operates only
// on the producer's millimetre/radian STEP model, before bounded serialization.
#pragma once
#include <StepData_StepModel.hxx>
#include <StepGeom_BSplineCurveWithKnotsAndRationalBSplineCurve.hxx>
#include <StepGeom_BSplineCurveWithKnots.hxx>
#include <StepGeom_CartesianPoint.hxx>
#include <StepGeom_HArray1OfPcurveOrSurface.hxx>
#include <StepGeom_PcurveOrSurface.hxx>
#include <StepGeom_Pcurve.hxx>
#include <StepGeom_SurfaceCurve.hxx>
#include <StepGeom_ToroidalSurface.hxx>
#include <StepRepr_DefinitionalRepresentation.hxx>
#include <StepToGeom.hxx>
#include <Geom_ToroidalSurface.hxx>
#include <Precision.hxx>
#include <gp_Pnt2d.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <stdexcept>

namespace atlas_pcurve {
struct Parameters {
  gp_Pnt2d first, last;
  double signed_slope;
};
using Quadratic = StepGeom_BSplineCurveWithKnotsAndRationalBSplineCurve;
inline bool parameters(const Handle(Quadratic) & arc, const Handle(StepGeom_ToroidalSurface) & torus,
                       const Handle(StepGeom_BSplineCurveWithKnots) & uv, Parameters &result) {
  if (arc->Degree() != 2 || arc->NbControlPointsList() != 3 || arc->NbWeightsData() != 3 ||
      arc->NbKnots() != 2 || arc->KnotMultiplicitiesValue(1) != 3 || arc->KnotMultiplicitiesValue(2) != 3 ||
      uv->Degree() != 1 || uv->NbControlPointsList() != 2 || uv->NbKnots() != 2 ||
      uv->KnotMultiplicitiesValue(1) != 2 || uv->KnotMultiplicitiesValue(2) != 2 ||
      uv->KnotsValue(1) != arc->KnotsValue(1) || uv->KnotsValue(2) != arc->KnotsValue(2) ||
      !(uv->KnotsValue(2) > uv->KnotsValue(1)))
    return false;
  const auto a = uv->ControlPointsListValue(1), b = uv->ControlPointsListValue(2);
  if (a->NbCoordinates() != 2 || b->NbCoordinates() != 2)
    return false;
  result.first = {a->CoordinatesValue(1), a->CoordinatesValue(2)};
  result.last = {b->CoordinatesValue(1), b->CoordinatesValue(2)};
  const auto d = result.last.XY() - result.first.XY();
  if ((d.X() == 0.) == (d.Y() == 0.))
    return false;
  const bool varying_u = d.Y() == 0.;
  const double delta = varying_u ? d.X() : d.Y(), h = delta * .5;
  const double radius = varying_u ? torus->MajorRadius() + torus->MinorRadius() * std::cos(result.first.Y())
                                  : torus->MinorRadius();
  if (!(radius > 0. && std::abs(delta) < 1.))
    return false;
  // Exact rational quadratic angle as a function of its normalized parameter.
  // Skip already sub-confusion discrepancies. Complete native validation is
  // still required after the serialized model is read again.
  constexpr double t = .21132486540518712;
  const double angle = 2. * std::atan2(t * std::sin(h), 1. - t + t * std::cos(h));
  if (!(radius * std::abs(angle - t * delta) > .9 * Precision::Confusion()))
    return false;
  const double w = std::cos(h), first_weight = arc->WeightsDataValue(1);
  if (!(first_weight > 0.) || !std::isfinite(first_weight) || !std::isfinite(arc->WeightsDataValue(2)) ||
      !std::isfinite(arc->WeightsDataValue(3)) ||
      std::abs(arc->WeightsDataValue(2) / first_weight - w) > 1.e-12 ||
      std::abs(arc->WeightsDataValue(3) / first_weight - 1.) > 1.e-12)
    return false;
  const auto support = StepToGeom::MakeToroidalSurface(torus);
  if (support.IsNull())
    return false;
  const auto start = support->Value(result.first.X(), result.first.Y()),
             end = support->Value(result.last.X(), result.last.Y());
  const auto mid =
      support->Value((result.first.X() + result.last.X()) * .5, (result.first.Y() + result.last.Y()) * .5);
  const std::array<gp_Pnt, 3> poles{
      start, gp_Pnt((mid.XYZ() * (2. * (1. + w)) - start.XYZ() - end.XYZ()) / (2. * w)), end};
  double mismatch = 0.;
  for (int i = 1; i <= 3; ++i) {
    const auto measured = arc->ControlPointsListValue(i);
    if (measured->NbCoordinates() != 3)
      return false;
    const gp_Pnt p(measured->CoordinatesValue(1), measured->CoordinatesValue(2),
                   measured->CoordinatesValue(3));
    const double distance = p.Distance(poles[i - 1]);
    if (!std::isfinite(distance))
      return false;
    mismatch = std::max(mismatch, distance);
  }
  // Hermite error <= max|theta''''| / 384. For z=1+t(exp(i*h)-1),
  // theta=2 arg(z), |z|>=cos(h/2), and |sin(4 arg(z'/z))|<=2|h|.
  const double hermite =
      radius * std::pow(2. * std::tan(std::abs(h) / 2.), 4) * std::min(1., std::abs(delta)) / 32.;
  if (!(mismatch + hermite + 4.e-12 * radius < .5 * Precision::Confusion()))
    return false;
  result.signed_slope = 2. * std::sin(h);
  return true;
}

inline size_t align(const Handle(StepData_StepModel) & model) {
  // The spatial certificate is in millimetres, just like the producer.
  if (model->WriteLengthUnit() != 1. || model->LocalLengthUnit() != 1.)
    return 0;
  const int initial = model->NbEntities();
  size_t count = 0;
  for (int i = 1; i <= initial; ++i) {
    const auto sc = Handle(StepGeom_SurfaceCurve)::DownCast(model->Value(i));
    if (sc.IsNull() || sc->NbAssociatedGeometry() > 2)
      continue;
    const auto arc = Handle(Quadratic)::DownCast(sc->Curve3d());
    if (arc.IsNull())
      continue;
    for (int side = 1; side <= sc->NbAssociatedGeometry(); ++side) {
      const auto pc = sc->AssociatedGeometryValue(side).Pcurve();
      if (pc.IsNull())
        continue;
      const auto torus = Handle(StepGeom_ToroidalSurface)::DownCast(pc->BasisSurface());
      const auto rep = pc->ReferenceToCurve();
      if (torus.IsNull() || rep.IsNull() || rep->NbItems() != 1)
        continue;
      const auto uv = Handle(StepGeom_BSplineCurveWithKnots)::DownCast(rep->ItemsValue(1));
      Parameters p;
      if (uv.IsNull() || !parameters(arc, torus, uv, p))
        continue;
      if (++count > 100000)
        throw std::length_error("Trop de raccords paramétriques STEP.");
      const auto direction =
          (p.first.X() == p.last.X()) ? gp_XY(0., p.signed_slope / 3.) : gp_XY(p.signed_slope / 3., 0.);
      Handle(StepGeom_HArray1OfCartesianPoint) poles = new StepGeom_HArray1OfCartesianPoint(1, 4);
      poles->SetValue(1, uv->ControlPointsListValue(1));
      poles->SetValue(4, uv->ControlPointsListValue(2));
      for (int j = 2; j <= 3; ++j) {
        const auto pos = j == 2 ? p.first.XY() + direction : p.last.XY() - direction;
        Handle(StepGeom_CartesianPoint) pt = new StepGeom_CartesianPoint;
        pt->Init2D(uv->Name(), pos.X(), pos.Y());
        poles->SetValue(j, pt);
        model->AddEntity(pt);
      }
      Handle(TColStd_HArray1OfInteger) mult = new TColStd_HArray1OfInteger(1, 2, 4);
      Handle(StepGeom_BSplineCurveWithKnots) spline = new StepGeom_BSplineCurveWithKnots;
      spline->Init(uv->Name(), 3, poles, uv->CurveForm(), uv->ClosedCurve(), uv->SelfIntersect(), mult,
                   uv->Knots(), uv->KnotSpec());
      model->AddEntity(spline);
      Handle(StepRepr_HArray1OfRepresentationItem) item = new StepRepr_HArray1OfRepresentationItem(1, 1);
      item->SetValue(1, spline);
      Handle(StepRepr_DefinitionalRepresentation) local = new StepRepr_DefinitionalRepresentation;
      local->Init(rep->Name(), item, rep->ContextOfItems());
      model->AddEntity(local);
      Handle(StepGeom_Pcurve) replacement = new StepGeom_Pcurve;
      replacement->Init(pc->Name(), torus, local);
      model->AddEntity(replacement);
      // Don't mutate a representation that may also describe a different edge.
      Handle(StepGeom_HArray1OfPcurveOrSurface) uses =
          new StepGeom_HArray1OfPcurveOrSurface(1, sc->NbAssociatedGeometry());
      for (int j = 1; j <= sc->NbAssociatedGeometry(); ++j)
        uses->SetValue(j, sc->AssociatedGeometryValue(j));
      StepGeom_PcurveOrSurface selected;
      selected.SetValue(replacement);
      uses->SetValue(side, selected);
      sc->SetAssociatedGeometry(uses);
    }
  }
  return count;
}
} // namespace atlas_pcurve
