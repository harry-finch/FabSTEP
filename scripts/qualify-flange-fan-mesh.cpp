// Compare certified triangular ruled patches against OCCT on the same edge knots.
#include "../crates/fabstep-cad/src/trim_native_fan_mesh.hxx"
#include <BRepAdaptor_Curve.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <Poly_PolygonOnTriangulation.hxx>
#include <STEPControl_Reader.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <Geom_BezierSurface.hxx>
#include <GeomConvert.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <iostream>

int controls() {
  TColgp_Array2OfPnt grid(1, 3, 1, 2);
  grid(1, 1) = gp_Pnt(0, 0, 0);
  grid(2, 1) = gp_Pnt(.5, 0, .0002);
  grid(3, 1) = gp_Pnt(1, 0, 0);
  for (int i = 1; i <= 3; ++i)
    grid(i, 2) = gp_Pnt(0, 1, 0);
  auto native = [](const TColgp_Array2OfPnt &g) {
    return GeomConvert::SurfaceToBSplineSurface(new Geom_BezierSurface(g));
  };
  auto support = native(grid);
  auto face = BRepBuilderAPI_MakeFace(support, 0, 1, 0, 1, 1e-7).Face();
  TopTools_IndexedMapOfShape edges;
  TopExp::MapShapes(face, TopAbs_EDGE, edges);
  std::vector<TrimEdgeSamples> paths(edges.Extent() + 1);
  int pole = 0, first = 0;
  for (int i = 1; i <= edges.Extent(); ++i) {
    auto e = TopoDS::Edge(edges(i));
    if (BRep_Tool::Degenerated(e)) {
      pole += trimfan::pole(face, e).has_value();
      continue;
    }
    if (!first)
      first = i;
    BRepAdaptor_Curve c(e);
    auto &out = paths[i];
    for (int j = 0; j <= 2; ++j) {
      double p = c.FirstParameter() + (c.LastParameter() - c.FirstParameter()) * j / 2.;
      out.push_back({p, c.Value(p)});
    }
  }
  auto get = [&](const TopoDS_Edge &e) -> const TrimEdgeSamples & { return paths.at(edges.FindIndex(e)); };
  auto valid = trimfan::mesh(face, .005, .35, get);
  if (pole != 1 || valid.IsNull() || valid->NbNodes() != 6 || valid->NbTriangles() != 4)
    return 1;
  auto &path = paths[first];
  path[1].point.SetZ(path[1].point.Z() + .001);
  if (!trimfan::mesh(face, .005, .35, get).IsNull())
    return 1;
  path[1].point.SetZ(path[1].point.Z() - .001);
  std::swap(path[0], path[1]);
  if (!trimfan::mesh(face, .005, .35, get).IsNull())
    return 1;
  if (trimfan::profile(face, 1e-8, .35))
    return 1;
  auto sub = BRepBuilderAPI_MakeFace(support, .2, .8, .2, .8, 1e-7).Face();
  auto empty = [](const TopoDS_Edge &) -> const TrimEdgeSamples & {
    static const TrimEdgeSamples none;
    return none;
  };
  if (!trimfan::mesh(sub, .005, .35, empty).IsNull())
    return 1;
  grid(2, 2) = gp_Pnt(.003, 1, 0);
  if (trimfan::profile(BRepBuilderAPI_MakeFace(native(grid), 0, 1, 0, 1, 1e-7).Face(), .05, .35))
    return 1;
  std::cout << "triangular fan: 4 triangles, shifted/disordered edges, tighter deflection, subtrim and false "
               "pole refused\n";
  return 0;
}
int main(int argc, char **argv) {
  if (argc == 2 && std::string(argv[1]) == "--self-test")
    return controls();
  if (argc != 3)
    return 2;
  STEPControl_Reader reader;
  if (reader.ReadFile(argv[1]) != IFSelect_RetDone || reader.TransferRoots() != 1)
    return 1;
  auto shape = reader.OneShape();
  double deflection = std::stod(argv[2]);
  BRepMesh_IncrementalMesh mesher(shape, deflection, false, .35, false);
  if (!mesher.IsDone() || mesher.GetStatusFlags() != 0)
    return 1;
  TopTools_IndexedMapOfShape edgeIds;
  TopExp::MapShapes(shape, TopAbs_EDGE, edgeIds);
  std::vector<TrimEdgeSamples> cache(edgeIds.Extent() + 1);
  int bsplines = 0, accepted = 0, cert = 0, ordinary = 0, compact = 0, all = 0, degenerate = 0,
      accepted_poles = 0;
  for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
    auto face = TopoDS::Face(it.Current().Oriented(TopAbs_FORWARD));
    TopLoc_Location location;
    auto mesh = BRep_Tool::Triangulation(face, location);
    if (mesh.IsNull())
      return 1;
    int count = mesh->NbTriangles();
    all += count;
    if (BRepAdaptor_Surface(face).GetType() != GeomAbs_BSplineSurface)
      continue;
    ++bsplines;
    ordinary += count;
    for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
      auto edge = TopoDS::Edge(e.Current());
      if (!BRep_Tool::Degenerated(edge))
        continue;
      ++degenerate;
      if (trimfan::pole(face, edge))
        ++accepted_poles;
    }
    if (trimfan::profile(face, deflection, .35))
      ++cert;
    auto get = [&](const TopoDS_Edge &edge) -> const TrimEdgeSamples & {
      auto &path = cache.at(edgeIds.FindIndex(edge));
      if (path.empty()) {
        auto poly = BRep_Tool::PolygonOnTriangulation(edge, mesh, location);
        if (poly.IsNull() || !poly->HasParameters())
          return path;
        BRepAdaptor_Curve curve(edge);
        for (int i = 1; i <= poly->NbNodes(); ++i) {
          double parameter = poly->Parameter(i);
          path.push_back({parameter, curve.Value(parameter)});
        }
      }
      return path;
    };
    auto result = trimfan::mesh(face, deflection, .35, get);
    if (!result.IsNull()) {
      ++accepted;
      compact += result->NbTriangles();
    } else
      compact += count;
  }
  std::cout << "ruled_faces=" << bsplines << " certified=" << cert << " remeshed=" << accepted
            << " native_triangles=" << ordinary << " integrated_triangles=" << compact
            << " poles=" << accepted_poles << "/" << degenerate
            << " all_with_other_native_faces=" << all - ordinary + compact << "\n";
  return 0;
}
