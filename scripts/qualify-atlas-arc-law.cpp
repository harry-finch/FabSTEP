// STEP transfer of a sewn ruled collar with two exact, adjacent circular rims.
#include "../crates/fabstep-cad/src/torus_atlas_arc_law.hxx"
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepTools_ReShape.hxx>
#include <TopExp.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepLib_CheckCurveOnSurface.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <Geom_Circle.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <ShapeProcess.hxx>
#include <Standard_Version.hxx>
#include <iostream>
#include <sstream>

namespace {
void require(bool condition, const char *message) {
  if (!condition)
    throw std::runtime_error(message);
}
TopoDS_Face transfer(const TopoDS_Face &face) {
  STEPControl_Writer writer;
  DESTEP_Parameters options;
  options.WriteSurfaceCurMode = true;
  writer.SetShapeProcessFlags(ShapeProcess::OperationsFlags{});
  std::stringstream stream;
  require(writer.Transfer(face, STEPControl_AsIs, options) == IFSelect_RetDone &&
              writer.WriteStream(stream) == IFSelect_RetDone, "fixture STEP write failed");
  STEPControl_Reader reader;
  require(reader.ReadStream("two-arcs.step", stream) == IFSelect_RetDone && reader.TransferRoots() == 1,
          "fixture STEP read failed");
  TopExp_Explorer faces(reader.OneShape(), TopAbs_FACE);
  require(faces.More(), "missing ruled face");
  return TopoDS::Face(faces.Current());
}
TopoDS_Face sewn_fixture() {
  const gp_Pnt apex(14, -18, 5);
  const double radius = 11.25;
  const std::array<gp_Circ, 2> circles{
      gp_Circ(gp_Ax2(apex.Translated(gp_Vec(0, -.8, 0)), gp::DX(), gp::DY()), .8),
      gp_Circ(gp_Ax2(apex.Translated(gp_Vec(-radius, 0, 0)), gp::DZ(), gp::DX()), radius)};
  const std::array<double, 2> sweep{.02, .0131};
  TColgp_Array2OfPnt poles(1, 3, 1, 2);
  TColStd_Array2OfReal weights(1, 3, 1, 2);
  weights.Init(1.);
  for (int row = 0; row < 2; ++row) {
    const Geom_Circle circle(circles[row]);
    const auto center = circles[row].Location(), middle = circle.Value(sweep[row] * .5);
    const double weight = std::cos(sweep[row] * .5);
    poles(1, row + 1) = apex;
    poles(2, row + 1) = center.Translated(gp_Vec(center, middle) / weight);
    poles(3, row + 1) = circle.Value(sweep[row]);
    weights(2, row + 1) = weight;
  }
  const auto surface = GeomConvert::SurfaceToBSplineSurface(new Geom_BezierSurface(poles, weights));
  const auto face = BRepBuilderAPI_MakeFace(surface, 0, 1, 0, 1, Precision::Confusion()).Face();
  // Model the mixed representation produced by sewing to an analytic neighbor:
  // the shared curve is a circle and the coedge goes linearly through the net.
  BRep_Builder builder;
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    if (BRep_Tool::Degenerated(edge))
      continue;
    BRepAdaptor_Curve2d pc(edge, face);
    const auto a = pc.Value(pc.FirstParameter()), b = pc.Value(pc.LastParameter());
    if (a.Y() != b.Y())
      continue;
    const int row = a.Y() > .5 ? 1 : 0;
    const double tolerance = 1.5e-7;
    builder.UpdateEdge(edge, new Geom_Circle(circles[row]), tolerance);
    TColgp_Array1OfPnt2d controls(1, 2);
    controls(1) = {0, double(row)};
    controls(2) = {1, double(row)};
    TColStd_Array1OfReal knots(1, 2);
    knots(1) = 0.;
    knots(2) = sweep[row];
    TColStd_Array1OfInteger mult(1, 2);
    mult.Init(2);
    builder.UpdateEdge(edge, new Geom2d_BSplineCurve(controls, knots, mult, 1), face, tolerance);
    builder.Range(edge, 0., sweep[row]);
  }
  return face;
}
TopoDS_Edge large_rim(const TopoDS_Face &face) {
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    if (!BRep_Tool::Degenerated(edge)) {
      const BRepAdaptor_Curve curve(edge);
      if (curve.GetType() == GeomAbs_Circle && curve.Circle().Radius() > 5.)
        return edge;
    }
  }
  throw std::runtime_error("missing circular edge");
}
// Keep the same arc and traversal in the wire, but reverse the shared
// circle's parameterization. Its coedge must now run from u=1 to u=0.
TopoDS_Face reversed_rim(const TopoDS_Face &face) {
  const auto old = TopoDS::Edge(large_rim(face).Oriented(TopAbs_FORWARD));
  const BRepAdaptor_Curve arc(old);
  const gp_Circ c = arc.Circle();
  const auto center = c.Location();
  const gp_Ax2 axes(center, c.Axis().Direction().Reversed(), gp_Dir(gp_Vec(center, arc.Value(arc.LastParameter()))));
  const double sweep = arc.LastParameter() - arc.FirstParameter();
  const auto replacement = BRepBuilderAPI_MakeEdge(new Geom_Circle(axes, c.Radius()), TopExp::LastVertex(old),
                                                    TopExp::FirstVertex(old), 0., sweep).Edge();
  const BRepAdaptor_Curve2d pc(old, face);
  const auto direct = pc.BSpline();
  Handle(Geom2d_BSplineCurve) inverse = Handle(Geom2d_BSplineCurve)::DownCast(direct->Reversed());
  BRep_Builder builder;
  builder.UpdateEdge(replacement, inverse, face, BRep_Tool::Tolerance(old));
  builder.Range(replacement, face, 0., sweep);
  BRepTools_ReShape reshape;
  reshape.Replace(old, replacement.Reversed());
  return TopoDS::Face(reshape.Apply(face));
}

double residual(const TopoDS_Edge &edge, const TopoDS_Face &face) {
  BRepLib_CheckCurveOnSurface proof(edge, face);
  proof.Perform();
  require(proof.IsDone(), "failed curve-on-surface measurement");
  return proof.MaxDistance();
}
} // namespace
int main() try {
  const auto face = sewn_fixture();
  const auto edge = large_rim(face);
  require(residual(edge, face) > Precision::Confusion() && BRepCheck_Analyzer(face, true, false, true).IsValid(),
          "nominal residual or native fixture validity not reproduced");
  const auto legacy = transfer(face);
  const auto legacy_edge = large_rim(legacy);
  const double legacy_error = residual(legacy_edge, legacy);
  require(legacy_error > Precision::Confusion(), "legacy STEP coedge lost the measured angular mismatch");
#if OCC_VERSION_HEX == 0x070903
  require(legacy_error > BRep_Tool::Tolerance(legacy_edge) && !BRepCheck_Analyzer(legacy, true, false, true).IsValid(),
          "OCCT 7.9.3 did not reproduce the exact STEP rejection");
#endif
  const auto neighbor = TopoDS::Face(BRepPrimAPI_MakePrism(edge, gp_Vec(0, 0, 1)).Shape());
  double lo, hi;
  const auto surface = BRep_Tool::Surface(face);
  const auto curve = BRep_Tool::Curve(edge, lo, hi);
  const auto other = BRep_Tool::CurveOnSurface(edge, neighbor, lo, hi);
  const double tolerance = BRep_Tool::Tolerance(edge);
  require(!other.IsNull() && BRepCheck_Analyzer(neighbor, true, false, true).IsValid(), "invalid neighbor fixture");
  require(atlas_arc_law::align(face) == 1 && atlas_arc_law::align(face) == 0, "failed single coedge correction");
  require(BRep_Tool::CurveOnSurface(edge, neighbor, lo, hi) == other && BRep_Tool::Curve(edge, lo, hi) == curve &&
              BRep_Tool::Surface(face) == surface && BRep_Tool::Tolerance(edge) == tolerance,
          "shared geometry, tolerance or neighbor pcurve changed");
  require(residual(edge, face) < 1e-9 && BRepCheck_Analyzer(face, true, false, true).IsValid() &&
              BRepCheck_Analyzer(neighbor, true, false, true).IsValid(), "invalid corrected geometry");
  const auto read = transfer(face);
  require(BRepCheck_Analyzer(read, true, false, true).IsValid() && residual(large_rim(read), read) < 1e-9,
          "corrected arc failed exact check after STEP");
  const auto reversed = reversed_rim(sewn_fixture());
  require(BRepCheck_Analyzer(reversed, true, false, true).IsValid() && atlas_arc_law::align(reversed) == 1,
          "reversed coedge was not aligned");
  const auto reversed_read = transfer(reversed);
  require(residual(large_rim(reversed_read), reversed_read) < 1e-9 &&
              BRepCheck_Analyzer(reversed_read, true, false, true).IsValid(), "reversed coedge lost its law after STEP");
  const auto damaged = sewn_fixture();
  const auto net = Handle(Geom_BSplineSurface)::DownCast(BRep_Tool::Surface(damaged));
  net->SetPole(2, 2, net->Pole(2, 2).Translated(gp_Vec(0, 0, .00001)));
  require(atlas_arc_law::align(damaged) == 0, "incorrect circular control was accepted");
  std::cout << "Two-arc collar: exact STEP reimport, shared edge, neighbor, tolerance and negative control passed.\n";
} catch (const std::exception &e) {
  std::cerr << e.what() << '\n';
  return 1;
}
