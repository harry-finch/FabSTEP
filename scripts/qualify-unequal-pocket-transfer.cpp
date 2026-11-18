// R-090 laboratory. Insert the exact quartic into the known two-fillet solid.
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepLib_CheckCurveOnSurface.hxx>
#include <BRepTools.hxx>
#include <BRepTools_ReShape.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <ElSLib.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <Geom_BSplineCurve.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <array>
#include <cmath>
#include <fstream>
#include <iostream>
#include <stdexcept>

namespace {
constexpr double small = 1.75, large = 3.25, tolerance = 1.e-7;
constexpr double pi = 3.14159265358979323846;
void check(bool condition, const char* detail) {
  if (!condition) throw std::runtime_error(detail);
}
TopoDS_Shape read(const char* path) {
  STEPControl_Reader step;
  check(step.ReadFile(path) == IFSelect_RetDone && step.TransferRoots() == 1, "STEP read");
  return step.OneShape();
}
struct Join {
  TopoDS_Edge edge;
  std::array<TopoDS_Face, 2> walls; // smaller radius first
};
Join join(const TopoDS_Shape& solid) {
  TopTools_IndexedDataMapOfShapeListOfShape owners;
  TopExp::MapShapesAndUniqueAncestors(solid, TopAbs_EDGE, TopAbs_FACE, owners);
  Join result;
  for (int i = 1; i <= owners.Extent(); ++i) {
    const auto& faces = owners.FindFromIndex(i);
    check(faces.Extent() == 2, "open or nonmanifold edge");
    auto a = TopoDS::Face(faces.First());
    auto b = TopoDS::Face(faces.Last());
    const BRepAdaptor_Surface sa(a), sb(b);
    if (sa.GetType() != GeomAbs_Cylinder || sb.GetType() != GeomAbs_Cylinder) continue;
    check(result.edge.IsNull(), "two common edges");
    result.edge = TopoDS::Edge(owners.FindKey(i).Oriented(TopAbs_FORWARD));
    if (sa.Cylinder().Radius() > sb.Cylinder().Radius()) std::swap(a, b);
    result.walls = {a, b};
  }
  check(!result.edge.IsNull(), "missing common edge");
  return result;
}

// The laboratory charts use angle measured from the floor and an axial
// distance from the corner. OCCT may choose either sign and any chart origin.
void attach(const TopoDS_Edge& edge, const TopoDS_Face& face, const gp_Pnt& floorEnd,
            bool smallWall, bool reverse, int n, std::istream& polesFile) {
  const gp_Cylinder cyl = BRepAdaptor_Surface(face).Cylinder();
  check(std::abs(cyl.Radius() - (smallWall ? small : large)) <= tolerance, "radius");
  double u0, v0, low, high, bottom, top;
  ElSLib::Parameters(cyl, floorEnd, u0, v0);
  BRepTools::UVBounds(face, low, high, bottom, top);
  u0 += 2*pi * std::round(((low+high)/2 - u0) / (2*pi));
  const gp_Dir axis = smallWall ? gp_Dir(1,0,0) : gp_Dir(0,1,0);
  const gp_Dir tangent = smallWall ? gp_Dir(0,-1,0) : gp_Dir(-1,0,0);
  const double uSign = cyl.Axis().Direction().Crossed(gp_Dir(0,0,-1)).Dot(tangent);
  const double vSign = cyl.Axis().Direction().Dot(axis);
  check(std::abs(std::abs(uSign)-1) < 1.e-10 && std::abs(std::abs(vSign)-1) < 1.e-10,
        "unexpected cylinder frame");
  const double axial0 = smallWall ? large : small;
  TColgp_Array1OfPnt2d poles(1, 3*n+1);
  for (int i = 1; i <= poles.Length(); ++i) {
    double angle, axial;
    check(bool(polesFile >> angle >> axial) && std::isfinite(angle) && std::isfinite(axial),
          "invalid Hermite pole");
    poles.SetValue(i, gp_Pnt2d(u0+uSign*angle, v0+vSign*(axial-axial0)));
  }
  TColStd_Array1OfReal knots(1, n+1);
  TColStd_Array1OfInteger mults(1, n+1);
  for (int i = 1; i <= n+1; ++i) {
    knots.SetValue(i, double(i-1)/n);
    mults.SetValue(i, (i == 1 || i == n+1) ? 4 : 3);
  }
  Handle(Geom2d_BSplineCurve) trace = new Geom2d_BSplineCurve(poles, knots, mults, 3);
  if (reverse) trace->Reverse();
  BRep_Builder builder;
  builder.UpdateEdge(edge, trace, face, tolerance);
  builder.Range(edge, face, 0., 1.);
}

void inspect(const TopoDS_Shape& solid, const char* tag) {
  check(BRepCheck_Analyzer(solid, true).IsValid(), "invalid BRep");
  const auto shared = join(solid);
  double first, last;
  const auto curve = Handle(Geom_BSplineCurve)::DownCast(BRep_Tool::Curve(shared.edge, first, last));
  check(!curve.IsNull() && curve->Degree() == 4 && curve->IsRational() && curve->NbPoles() == 5,
        "lost exact rational quartic");
  check(BRep_Tool::Tolerance(shared.edge) <= tolerance, "edge tolerance was enlarged");
  for (const auto& f : shared.walls) {
    check(!BRep_Tool::CurveOnSurface(shared.edge, f, first, last).IsNull(), "missing pcurve");
    BRepLib_CheckCurveOnSurface verify(shared.edge, f);
    verify.Perform();
    check(verify.IsDone() && verify.MaxDistance() < 1.e-8, "incorrect curve on cylinder");
    std::cout << tag << " radius_mm=" << BRepAdaptor_Surface(f).Cylinder().Radius()
              << " pcurve_max_mm=" << verify.MaxDistance() << "\n";
  }
}
} // namespace

int main(int argc, char** argv) {
  try {
    check(argc == 5, "usage: probe reference.step curve.step poles.dat result.step");
    const auto source = read(argv[1]);
    const auto old = join(source);
    const auto standalone = read(argv[2]);
    TopExp_Explorer edge(standalone, TopAbs_EDGE);
    check(edge.More(), "missing standalone curve");
    double first, last;
    auto curve = BRep_Tool::Curve(TopoDS::Edge(edge.Current()), first, last);
    check(!curve.IsNull() && first == 0. && last == 1., "wrong quartic domain");
    const gp_Pnt floorEnd = curve->Value(first);
    const auto a = TopExp::FirstVertex(old.edge), b = TopExp::LastVertex(old.edge);
    const bool reverse = floorEnd.Distance(BRep_Tool::Pnt(a)) > tolerance;
    if (reverse) curve = curve->Reversed();
    check(curve->Value(first).Distance(BRep_Tool::Pnt(a)) <= tolerance &&
          curve->Value(last).Distance(BRep_Tool::Pnt(b)) <= tolerance, "quartic endpoints");
    const auto replacement = BRepBuilderAPI_MakeEdge(curve, a, b, first, last).Edge();

    std::ifstream input(argv[3]);
    int n;
    check(bool(input >> n) && n >= 1 && n <= 1024, "invalid span count");
    for (int i = 0; i < 2; ++i) attach(replacement, old.walls[i], floorEnd, i == 0, reverse, n, input);
    Handle(BRepTools_ReShape) reshape = new BRepTools_ReShape;
    reshape->Replace(old.edge, replacement);
    const auto result = reshape->Apply(source);
    inspect(result, "before");
    STEPControl_Writer writer;
    check(writer.Transfer(result, STEPControl_AsIs) == IFSelect_RetDone &&
          writer.Write(argv[4]) == IFSelect_RetDone, "STEP write");
    inspect(read(argv[4]), "after");
    return 0;
  } catch (const Standard_Failure& e) {
    std::cerr << e.GetMessageString() << "\n";
  } catch (const std::exception& e) {
    std::cerr << e.what() << "\n";
  }
  return 1;
}
