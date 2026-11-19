// Research-only measurement of a STEP drawing with centered 2/5 mm bores.
// Optional second path writes the analytically reparameterized drawing.
#include "../crates/fabstep-cad/src/cross_drill.hxx"
#include "../crates/fabstep-cad/src/cross_drill_certificate.hxx"
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRep_Builder.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <Geom_BSplineCurve.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <TopoDS.hxx>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include "../crates/fabstep-cad/src/cross_drill_junction.hxx"

namespace {
int qualify(int argc, char** argv) {
  if (argc < 2 || argc > 3) return 2;
  STEPControl_Reader reader;
  if (reader.ReadFile(argv[1]) != IFSelect_RetDone || !reader.TransferRoots()) return 2;
  const auto shape = reader.OneShape();
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  std::array<gp_Dir, 2> axes;
  for (int i = 1; i <= faces.Extent(); ++i) {
    BRepAdaptor_Surface surface(TopoDS::Face(faces(i)));
    if (surface.GetType() != GeomAbs_Cylinder) continue;
    const auto cylinder = surface.Cylinder();
    auto axis = cylinder.Axis().Direction();
    const bool large = cylinder.Radius() > 3.;
    if (large ? axis.X() < 0. : axis.Z() < 0.) axis.Reverse();
    axes[large ? 0 : 1] = axis;
  }
  const gp_Vec small_axis(axes[1]), large_axis(axes[0]);
  const auto side = small_axis.Crossed(large_axis).Normalized();
  const auto across = side.Crossed(small_axis).Normalized();
  const double x[]{axes[0].X(), axes[0].Y(), axes[0].Z()};
  const double z[]{axes[1].X(), axes[1].Y(), axes[1].Z()};
  const double origin[]{0., 0., 0.};
  cross_drill_lab::certificate_detail::LocalFrame frame(origin, x, z, true);

  // A nearby but different inclination must fail the same physical bound.
  const double angle = std::acos(large_axis.Dot(small_axis)) + .04;
  const auto wrong_z = large_axis * std::cos(angle)
      + (small_axis - large_axis * large_axis.Dot(small_axis)).Normalized() * std::sin(angle);
  const double tilted[]{wrong_z.X(),wrong_z.Y(),wrong_z.Z()};
  cross_drill_lab::certificate_detail::LocalFrame wrong(origin,x,tilted,true);
  TopTools_IndexedDataMapOfShapeListOfShape incidences;
  TopExp::MapShapesAndUniqueAncestors(shape, TopAbs_EDGE, TopAbs_FACE, incidences);
  bool passed = true;
  int tested = 0;
  for (int i = 1; i <= incidences.Extent(); ++i) {
    const auto& row = incidences.FindFromIndex(i);
    if (row.Extent() != 2) continue;
    const auto a = TopoDS::Face(row.First()), b = TopoDS::Face(row.Last());
    if (BRepAdaptor_Surface(a).GetType() != GeomAbs_Cylinder ||
        BRepAdaptor_Surface(b).GetType() != GeomAbs_Cylinder) continue;
    const auto edge = TopoDS::Edge(incidences.FindKey(i));
    if (argc == 3) {
      set_cross_drill_junction(edge, a, b, gp_Pnt(0,0,0), across, side, small_axis,
          across.Dot(large_axis), large_axis.Dot(small_axis), 0., 2., 5.,
          std::runtime_error("unproved endpoint"));
    }
    cross_drill_lab::Budget budget;
    budget.tolerance_mm = 1e-7;
    const auto cert = cross_drill_lab::certifyEdge(edge, a, b, 2., 5., budget, &frame);
    if (cross_drill_lab::certifyEdge(edge, a, b, 2., 5., budget, &wrong).ok)
      throw std::runtime_error("wrong inclination accepted");
    ++tested;
    passed = passed && cert.ok;
    std::cout << "edge=" << i << " ok=" << cert.ok << " residual3d=" << cert.last_bound3d
      << " residual_uv=" << cert.last_boundPcurves << " bounded3d=" << cert.bound3d
      << " bounded_uv=" << cert.boundPcurves << " operations=" << cert.work
      << " leaves=" << cert.leaves << " reason=" << cert.reason << std::endl;
  }
  if (!passed || !tested) return 1;
  TopTools_IndexedMapOfShape solids;
  TopExp::MapShapes(shape, TopAbs_SOLID, solids);
  if (solids.Extent() != 1) return 2;
  const auto proof = verify_cross_drill(solids(1), nullptr, .001);
  std::cout << "solid certified; volume interval [" << std::setprecision(15) << proof.volume_low
    << ", " << proof.volume_high << "]" << std::endl;
  if (argc == 3) {
    STEPControl_Writer writer;
    if (writer.Transfer(shape, STEPControl_AsIs) != IFSelect_RetDone ||
        writer.Write(argv[2]) != IFSelect_RetDone) return 2;
  }
  return 0;
}
}

int main(int argc, char** argv) {
  try {
    return qualify(argc, argv);
  } catch (const std::exception& e) {
    std::cerr << e.what() << std::endl;
  } catch (const cross_drill_lab::certificate_detail::Failure& e) {
    std::cerr << e.reason << std::endl;
  }
  return 1;
}
