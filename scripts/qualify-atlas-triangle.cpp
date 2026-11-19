// Standalone OCCT qualification of a shallow circular collar and its STEP trim.
#include "../crates/fabstep-cad/src/torus_atlas_triangle.hxx"
#include <BRepAlgoAPI_Check.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <TopExp.hxx>
#include <ShapeProcess.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_Circle.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopoDS_Compound.hxx>
#include <GCE2d_MakeSegment.hxx>
#include <Geom2d_BezierCurve.hxx>
#include <Geom2d_Line.hxx>
#include <iostream>
#include <sstream>
namespace {
void require(bool condition, const char *message) {
  if (!condition)
    throw std::runtime_error(message);
}
bool valid(const TopoDS_Shape &s) {
  BRepAlgoAPI_Check check(s, false, true);
  return !check.HasErrors() && check.IsValid();
}
TopoDS_Face step_face(const TopoDS_Face &face) {
  STEPControl_Writer writer;
  DESTEP_Parameters params;
  params.WriteSurfaceCurMode = true;
  writer.SetShapeProcessFlags(ShapeProcess::OperationsFlags{});
  std::stringstream stream;
  require(writer.Transfer(face, STEPControl_AsIs, params) == IFSelect_RetDone &&
              writer.WriteStream(stream) == IFSelect_RetDone, "STEP write failed");
  STEPControl_Reader reader;
  require(reader.ReadStream("collar.step", stream) == IFSelect_RetDone && reader.TransferRoots() == 1,
          "STEP read failed");
  TopExp_Explorer it(reader.OneShape(), TopAbs_FACE);
  require(it.More(), "missing STEP face");
  return TopoDS::Face(it.Current());
}
void sewn_generator(const TopoDS_Face &face, const TopoDS_Edge &edge) {
  BRepAdaptor_Curve line(edge);
  const double a = line.FirstParameter(), b = line.LastParameter();
  TColStd_Array1OfReal knots(1, 2);
  knots(1) = a;
  knots(2) = b;
  BRep_Builder builder;
  for (int degree : {1, 2}) {
    TColgp_Array1OfPnt poles(1, degree + 1);
    poles(1) = line.Value(a);
    poles(degree + 1) = line.Value(b);
    if (degree == 2)
      poles(2) = line.Value((a + b) / 2).Translated(gp_Vec(0, 0, .001));
    TColStd_Array1OfInteger mult(1, 2);
    mult.Init(degree + 1);
    Handle(Geom_BSplineCurve) curve = new Geom_BSplineCurve(poles, knots, mult, degree);
    builder.UpdateEdge(edge, curve, Precision::Confusion());
    require(atlas_triangle::matches(face) == (degree == 1), "sewn material generator misclassified");
  }
}
void sample(bool reverse) {
  const double radius = 30., sweep = .027;
  Handle(Geom_Circle) base = new Geom_Circle(gp_Ax2(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), radius);
  const auto a = base->Value(.6), b = base->Value(.6 + sweep);
  const gp_Pnt apex((radius - .012) * std::cos(.6), (radius - .012) * std::sin(.6), .00008);
  const auto va = BRepBuilderAPI_MakeVertex(a).Vertex(), vb = BRepBuilderAPI_MakeVertex(b).Vertex(),
             vz = BRepBuilderAPI_MakeVertex(apex).Vertex();
  const auto arc = BRepBuilderAPI_MakeEdge(base, va, vb, .6, .6 + sweep).Edge();
  std::array<TopoDS_Edge, 3> rim{arc, BRepBuilderAPI_MakeEdge(vb, vz).Edge(),
                                 BRepBuilderAPI_MakeEdge(vz, va).Edge()};
  gp_Vec normal = gp_Vec(apex, a).Crossed(gp_Vec(apex, b)).Normalized();
  if (reverse) {
    std::swap(rim[1], rim[2]);
    for (auto &e : rim)
      e.Reverse();
    normal.Reverse();
  }
  const auto p = atlas_triangle::geometry(rim[0], apex);
  require(bool(p), "should recognize a shallow rim");
  require(!atlas_triangle::geometry(rim[0], apex.Translated(gp_Vec(0, 0, 1))), "tall collar accepted");
  require(!atlas_triangle::make(rim, 0, *p, -normal), "reversed source normal accepted");
  const auto face = atlas_triangle::make(rim, 0, *p, normal);
  require(face && atlas_triangle::matches(*face), "could not certify material trim");
  const BRepAdaptor_Surface surface(*face, false);
  for (int i = 0; i <= 24; ++i)
    for (int j = 0; j <= 24 - i; ++j) {
      const double u = i / 24., v = j / 24.;
      const gp_Pnt source(apex.XYZ() * (1 - u - v) + a.XYZ() * u + b.XYZ() * v);
      require(surface.Value(u, v).Distance(source) <= p->deviation + 1e-12, "invalid fidelity bound");
    }
  gp_Pnt middle;
  gp_Vec du, dv;
  surface.D1(.25, .25, middle, du, dv);
  const auto n = du.Crossed(dv).Normalized();
  const auto crossing = BRepBuilderAPI_MakeEdge(middle.Translated(n), middle.Translated(-n)).Edge();
  BRep_Builder make;
  TopoDS_Compound compound;
  make.MakeCompound(compound);
  make.Add(compound, *face);
  make.Add(compound, crossing);
  require(valid(*face) && !valid(compound), "real penetration must be rejected");
  const auto read = step_face(*face);
  require(BRepCheck_Analyzer(read, true, false, true).IsValid() && valid(read) &&
              atlas_triangle::matches(read),
          "STEP triangle lost its certified trim");
  sewn_generator(*face, rim[1]);
}
// Independent long collar over a very short circular rim. The polynomial
// loft is valid in memory. OCCT 7.9.3 reported an intersection with itself
// next to its collapsed pole after writing and reading this fixture.
void short_quadratic() {
  const gp_Pnt center(-25.3, 42.7, 19.6);
  const gp_Ax2 frame(center, gp_Dir(.27, .82, .52));
  Handle(Geom_Circle) c = new Geom_Circle(frame, 2.4856144855167726);
  const double start = 2.5537052150029842, sweep = .00097725404753004922;
  const auto mid = c->Value(start + sweep * .5);
  const auto radial = gp_Vec(center, mid).Normalized();
  const auto tangent = gp_Vec(frame.Direction()).Crossed(radial);
  const auto apex = mid.Translated(radial * -2.8963918955163317 + tangent * -.36531098589351052 +
                                   gp_Vec(frame.Direction()) * .068325057670249698);
  const auto arc = BRepBuilderAPI_MakeEdge(c, start, start + sweep).Edge();
  const auto vz = BRepBuilderAPI_MakeVertex(apex).Vertex();
  BRepOffsetAPI_ThruSections loft(false, true, Precision::Confusion());
  loft.CheckCompatibility(false);
  loft.SetMutableInput(false);
  loft.AddWire(BRepBuilderAPI_MakeWire(arc).Wire());
  loft.AddVertex(vz);
  loft.Build();
  require(loft.IsDone(), "short loft construction failed");
  TopExp_Explorer it(loft.Shape(), TopAbs_FACE);
  require(it.More(), "short loft missing");
  const auto original = TopoDS::Face(it.Current());
  require(atlas_triangle::quadratic_pole(original) && valid(original), "expected native quadratic pole");
  const bool legacy_valid = valid(step_face(original));
  std::cout << "Short quadratic legacy verdict after STEP = " << legacy_valid << "\n";
  const auto patch = atlas_triangle::geometry(arc, apex);
  require(patch && !patch->shallow && patch->deviation < 1e-6, "short arc outside certified bound");
  const auto va = TopExp::FirstVertex(arc), vb = TopExp::LastVertex(arc);
  std::array<TopoDS_Edge, 3> rim{arc, BRepBuilderAPI_MakeEdge(vb, vz).Edge(),
                                 BRepBuilderAPI_MakeEdge(vz, va).Edge()};
  const auto normal =
      gp_Vec(apex, c->Value(start)).Crossed(gp_Vec(apex, c->Value(start + sweep))).Normalized();
  const auto face = atlas_triangle::make(rim, 0, *patch, normal);
  require(face && !atlas_triangle::quadratic_pole(*face) && valid(*face), "invalid short circular triangle");
  const auto read = step_face(*face);
  require(atlas_triangle::matches(read) && BRepCheck_Analyzer(read, true, false, true).IsValid() &&
              valid(read), "reimported short circular triangle is invalid");
}
// The STEP reader can close slightly mismatched 2D endpoints by adding an
// edge at the endpoint of a long, independently generated circular collar.
void reimported_corner() {
  const gp_Ax2 axes(gp_Pnt(14, 35, 9), gp_Dir(.000008, .000011, 1));
  Handle(Geom_Circle) circle = new Geom_Circle(axes, .75);
  const double start = .246, sweep = .00204;
  gp_Pnt last;
  gp_Vec tangent;
  circle->D1(start + sweep, last, tangent);
  const auto first = circle->Value(start);
  const auto apex = first.Translated(gp_Vec(axes.Direction()) * 55. + gp_Vec(.00000013, .00000008, 0));
  const auto va = BRepBuilderAPI_MakeVertex(first).Vertex(), vz = BRepBuilderAPI_MakeVertex(apex).Vertex();
  const auto vb = BRepBuilderAPI_MakeVertex(last.Translated(tangent.Normalized() * 2.7e-12)).Vertex();
  const auto arc = BRepBuilderAPI_MakeEdge(circle, va, vb, start, start + sweep).Edge();
  std::array<TopoDS_Edge, 3> edges{arc, BRepBuilderAPI_MakeEdge(vb, vz).Edge(),
                                   BRepBuilderAPI_MakeEdge(vz, va).Edge()};
  const auto patch = atlas_triangle::geometry(arc, apex).value();
  const auto normal = gp_Vec(apex, first).Crossed(gp_Vec(apex, last)).Normalized();
  const auto built = atlas_triangle::make(edges, 0, patch, normal);
  require(built && valid(*built) && atlas_triangle::matches(*built), "native collar with shared corner failed");
  const auto read = step_face(*built);
  require(valid(read) && BRepCheck_Analyzer(read, true, false, true).IsValid() && atlas_triangle::matches(read),
          "STEP corner lost the regular triangle");
  TopoDS_Edge corner;
  for (TopExp_Explorer it(read, TopAbs_EDGE); it.More(); it.Next())
    if (const auto e = TopoDS::Edge(it.Current()); BRep_Tool::Degenerated(e))
      corner = e;
  BRep_Builder builder;
  if (corner.IsNull()) {
    // Another OCCT patch may close the wire directly. Exercise the same
    // degenerate topology explicitly on that build for the adversarial cases.
    TopoDS_Vertex vertex;
    for (TopExp_Explorer it(read, TopAbs_VERTEX); it.More(); it.Next()) {
      const auto v = TopoDS::Vertex(it.Current());
      if (BRep_Tool::Pnt(v).Distance(last) < Precision::Confusion())
        vertex = v;
    }
    require(!vertex.IsNull(), "missing circular endpoint");
    builder.MakeEdge(corner);
    builder.Degenerated(corner, true);
    builder.Add(corner, vertex.Oriented(TopAbs_FORWARD));
    builder.Add(corner, vertex.Oriented(TopAbs_REVERSED));
    builder.UpdateEdge(corner, new Geom2d_Line(gp_Pnt2d(0, 1), gp_Dir2d(1, -1)), read, Precision::Confusion());
    builder.Range(corner, read, 0., 1e-9);
    TopExp_Explorer wires(read, TopAbs_WIRE);
    auto wire = TopoDS::Wire(wires.Current());
    wire.Free(true);
    builder.Add(wire, corner);
  }
  require(atlas_triangle::matches(read) && bool(atlas_triangle::corner(corner, read)), "missing pointlike corner");
  // Stay within the 1e-8 UV neighborhood, but move in a direction in which
  // the surface covers more than native confusion. Endpoint incidence alone is insufficient.
  Handle(Geom2d_Curve) long_edge = new Geom2d_Line(gp_Pnt2d(0, 1), gp_Dir2d(0, -1));
  builder.UpdateEdge(corner, long_edge, read, Precision::Confusion());
  builder.Range(corner, read, 0., 8e-9);
  require(!atlas_triangle::matches(read), "coedge with material 3D extent was accepted");
  TColgp_Array1OfPnt2d poles(1, 3);
  poles(1) = gp_Pnt2d(0, 1);
  poles(2) = gp_Pnt2d(5e-9, 1 - 5e-9);
  poles(3) = poles(1);
  builder.UpdateEdge(corner, new Geom2d_BezierCurve(poles), read, Precision::Confusion());
  builder.Range(corner, read, 0., 1.);
  require(!atlas_triangle::matches(read), "nonmonotone corner curve was accepted");
  // Add a separate closed inner trim to the otherwise regular native face.
  const auto support = BRep_Tool::Surface(*built);
  const gp_Pnt2d inner[]{{.2, .2}, {.2, .3}, {.3, .2}};
  BRepBuilderAPI_MakeWire hole;
  for (int i = 0; i < 3; ++i) {
    Handle(Geom2d_Curve) segment = GCE2d_MakeSegment(inner[i], inner[(i + 1) % 3]).Value();
    hole.Add(BRepBuilderAPI_MakeEdge(segment, support).Edge());
  }
  require(hole.IsDone() && hole.Wire().Closed(), "could not prepare inner wire");
  TopoDS_Face perforated = *built;
  builder.Add(perforated, hole.Wire());
  require(!atlas_triangle::matches(perforated), "regular triangle with an inner loop was accepted");
}
// Displace the virtual fourth pole by less than the support-comparison
// tolerance while reversing its projected corner. Its three material
// vertices and its pcurves still match the intended signature.
void nearly_folded() {
  const double h = .016, r = 32., z = 4e-7;
  Handle(Geom_Circle) circle = new Geom_Circle(gp_Ax2(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), r);
  const auto a = circle->Value(0.), b = circle->Value(2 * h);
  const gp_Pnt mid((a.XYZ() + b.XYZ()) * .5);
  const gp_Vec radial(std::cos(h), std::sin(h), 0);
  const auto va = BRepBuilderAPI_MakeVertex(a).Vertex(), vb = BRepBuilderAPI_MakeVertex(b).Vertex();
  const auto arc = BRepBuilderAPI_MakeEdge(circle, va, vb, 0., 2 * h).Edge();
  double lo = .0001, hi = .05;
  for (int i = 0; i < 50; ++i) {
    const double distance = (lo + hi) * .5;
    const auto apex = mid.Translated(radial * distance + gp_Vec(0, 0, z));
    const auto p = atlas_triangle::geometry(arc, apex).value();
    const auto normal = gp_Vec(apex, a).Crossed(gp_Vec(apex, b)).Normalized();
    if (atlas_triangle::positive({apex, a, p.opposite, b}, normal))
      hi = distance;
    else
      lo = distance;
  }
  const auto apex = mid.Translated(radial * (hi + 2e-8) + gp_Vec(0, 0, z));
  const auto p = atlas_triangle::geometry(arc, apex).value();
  const auto vz = BRepBuilderAPI_MakeVertex(apex).Vertex();
  std::array<TopoDS_Edge, 3> edges{arc, BRepBuilderAPI_MakeEdge(vb, vz).Edge(),
                                   BRepBuilderAPI_MakeEdge(vz, va).Edge()};
  const auto normal = gp_Vec(apex, a).Crossed(gp_Vec(apex, b)).Normalized();
  const auto face = atlas_triangle::make(edges, 0, p, normal);
  require(face && atlas_triangle::matches(*face), "near-limit seed failed");
  auto net = Handle(Geom_BSplineSurface)::DownCast(BRep_Tool::Surface(*face));
  net->SetPole(2, 2, net->Pole(2, 2).Translated(radial * 6e-8));
  require(!atlas_triangle::matches(*face), "near-limit inverted STEP net accepted");
}
} // namespace
int main() try {
  sample(false);
  sample(true);
  nearly_folded();
  short_quadratic();
  reimported_corner();
  std::cout << "Two orientations, fidelity bound, STEP roundtrip, pointlike collar corners and penetrating controls passed.\n";
} catch (const std::exception &e) {
  std::cerr << e.what() << '\n';
  return 1;
}
