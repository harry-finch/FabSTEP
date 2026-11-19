// Public independent exercise of the bounded iso-circle parameter correction.
#include "torus_atlas_pcurve.hxx"
#include <GeomToStep_MakeToroidalSurface.hxx>
#include <Geom2d_Curve.hxx>
#include <Geom_Curve.hxx>
#include <gp_Ax3.hxx>
#include <iostream>
namespace {
void require(bool ok) {
  if (!ok)
    throw std::runtime_error("Toric p-curve regression");
}
Handle(StepGeom_CartesianPoint) point(const gp_Pnt &pos) {
  Handle(StepGeom_CartesianPoint) out = new StepGeom_CartesianPoint;
  out->Init3D(new TCollection_HAsciiString(""), pos.X(), pos.Y(), pos.Z());
  return out;
}
void sample(bool varying_u, double delta, double middle_motion, bool expected) {
  Handle(TCollection_HAsciiString) name = new TCollection_HAsciiString("");
  Handle(Geom_ToroidalSurface) geom =
      new Geom_ToroidalSurface(gp_Ax3(gp_Pnt(7, -2, 11), gp_Dir(1, 2, 3), gp_Dir(2, -1, 0)), 26., 3.);
  const auto torus = GeomToStep_MakeToroidalSurface(geom).Value();
  const gp_Pnt2d first(.64, -.31),
      last(first.X() + (varying_u ? delta : 0.), first.Y() + (varying_u ? 0. : delta));
  const auto a = geom->Value(first.X(), first.Y()), b = geom->Value(last.X(), last.Y()),
             mid = geom->Value((first.X() + last.X()) * .5, (first.Y() + last.Y()) * .5);
  const double w = std::cos(delta * .5);
  const gp_Pnt m((mid.XYZ() * (2. * (1. + w)) - a.XYZ() - b.XYZ()) / (2. * w) + gp_XYZ(middle_motion, 0, 0));
  Handle(StepGeom_HArray1OfCartesianPoint) poles = new StepGeom_HArray1OfCartesianPoint(1, 3);
  poles->SetValue(1, point(a));
  poles->SetValue(2, point(m));
  poles->SetValue(3, point(b));
  Handle(TColStd_HArray1OfReal) knots = new TColStd_HArray1OfReal(1, 2);
  knots->SetValue(1, 0);
  knots->SetValue(2, 1);
  Handle(TColStd_HArray1OfReal) weights = new TColStd_HArray1OfReal(1, 3, 1.);
  weights->SetValue(2, w);
  Handle(atlas_pcurve::Quadratic) arc = new atlas_pcurve::Quadratic;
  arc->Init(name, 2, poles, StepGeom_bscfCircularArc, StepData_LFalse, StepData_LFalse,
            new TColStd_HArray1OfInteger(1, 2, 3), knots, StepGeom_ktPiecewiseBezierKnots, weights);
  Handle(StepGeom_HArray1OfCartesianPoint) uvs = new StepGeom_HArray1OfCartesianPoint(1, 2);
  for (int i = 1; i <= 2; ++i) {
    Handle(StepGeom_CartesianPoint) q = new StepGeom_CartesianPoint;
    auto p = i == 1 ? first : last;
    q->Init2D(name, p.X(), p.Y());
    uvs->SetValue(i, q);
  }
  Handle(StepGeom_BSplineCurveWithKnots) uv = new StepGeom_BSplineCurveWithKnots;
  uv->Init(name, 1, uvs, StepGeom_bscfUnspecified, StepData_LFalse, StepData_LFalse,
           new TColStd_HArray1OfInteger(1, 2, 2), knots, StepGeom_ktPiecewiseBezierKnots);
  Handle(StepRepr_HArray1OfRepresentationItem) items = new StepRepr_HArray1OfRepresentationItem(1, 1);
  items->SetValue(1, uv);
  Handle(StepRepr_DefinitionalRepresentation) rep = new StepRepr_DefinitionalRepresentation;
  rep->Init(name, items, {});
  Handle(StepGeom_Pcurve) pc = new StepGeom_Pcurve;
  pc->Init(name, torus, rep);
  Handle(StepGeom_HArray1OfPcurveOrSurface) uses = new StepGeom_HArray1OfPcurveOrSurface(1, 1);
  StepGeom_PcurveOrSurface item;
  item.SetValue(pc);
  uses->SetValue(1, item);
  Handle(StepGeom_SurfaceCurve) sc = new StepGeom_SurfaceCurve;
  sc->Init(name, arc, uses, StepGeom_pscrPcurveS1);
  Handle(StepData_StepModel) model = new StepData_StepModel;
  model->AddEntity(sc);
  require(atlas_pcurve::align(model) == size_t(expected));
  require(atlas_pcurve::align(model) == 0);
  if (expected) {
    auto curve = StepToGeom::MakeCurve(arc);
    auto chosen = sc->AssociatedGeometryValue(1).Pcurve()->ReferenceToCurve()->ItemsValue(1);
    auto spline = Handle(StepGeom_BSplineCurveWithKnots)::DownCast(chosen);
    require(spline->Degree() == 3 && uv->Degree() == 1 && sc->Curve3d() == arc);
    auto corrected = StepToGeom::MakeCurve2d(spline);
    auto initial = StepToGeom::MakeCurve2d(uv);
    double worst = 0.;
    for (int i = 0; i <= 200; ++i) {
      double t = double(i) / 200.;
      auto p = corrected->Value(t), old = initial->Value(t);
      auto v = curve->Value(t);
      require(v.Distance(geom->Value(p.X(), p.Y())) < Precision::Confusion() / 2);
      worst = std::max(worst, v.Distance(geom->Value(old.X(), old.Y())));
    }
    require(worst > Precision::Confusion());
  }
}
} // namespace
int main() try {
  sample(true, .04, 0, true);
  sample(true, -.04, 0, true);
  sample(false, .065, 0, true);
  sample(false, -.065, 0, true);
  sample(true, .04, .001, false);
  sample(true, 1e-4, 0, false);
  sample(true, .7, 0, false);
  std::cout << "Toric p-curves: two isos, two orientations, immutable 3D arcs, idempotence and three "
               "refusals passed.\n";
  return 0;
} catch (const std::exception &e) {
  std::cerr << e.what() << "\n";
  return 1;
}
