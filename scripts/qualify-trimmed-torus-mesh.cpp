// Compare the bounded torus mesh against independently discretized OCCT edges.
// Run on a STEP or use --self-test for a translated, trimmed ring torus.
#include "../crates/fabstep-cad/src/trimmed_torus_mesh.hxx"
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepPrimAPI_MakeTorus.hxx>
#include <BRepTools.hxx>
#include <ElSLib.hxx>
#include <STEPControl_Reader.hxx>
#include <TopExp.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <iomanip>
#include <iostream>
#include <string>

namespace {
void check_metric(double deflection) {
  using UV = trimmed_torus_mesh::UV;
  for (double major : {2.01, 3.9, 15.}) {
    constexpr double minor = 2.;
    const gp_Torus surface(gp_Ax3(), major, minor);
    for (double start : {-0.2, 1.1, 2.8, 5.9}) {
      const std::array<UV, 4> corners{
          {{.3, start}, {1.5, start}, {1.5, start + 1.2}, {.3, start + 1.2}}};
      std::vector<UV> loop;
      for (int k = 0; k < 4; ++k)
        for (int j = 0; j < 96; ++j) {
          const auto a = corners[k], b = corners[(k + 1) % 4];
          double t = j / 96.;
          loop.push_back({a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t});
        }
      const auto mesh = trimmed_torus_mesh::build(loop, major, minor, deflection, 100000);
      auto value = [&](UV uv) { return ElSLib::Value(uv[0], uv[1], surface).XYZ(); };
      for (auto tri : mesh.triangles) {
        auto a = mesh.points[tri[0]], b = mesh.points[tri[1]], c = mesh.points[tri[2]];
        for (int j = 0; j <= 4; ++j)
          for (int k = 0; k <= 4 - j; ++k) {
            double u = j * .25, v = k * .25, w = 1. - u - v;
            UV uv{a[0] * u + b[0] * v + c[0] * w, a[1] * u + b[1] * v + c[1] * w};
            const auto difference = value(uv) - (value(a) * u + value(b) * v + value(c) * w);
            if (difference.Modulus() > mesh.bound + 1e-10)
              throw std::runtime_error("torus interpolation bound");
          }
      }
    }
  }
}
} // namespace
int main(int argc, char **argv) {
  if (argc != 3)
    return 2;
  try {
    TopoDS_Shape shape;
    const bool self = std::string(argv[1]) == "--self-test";
    if (self) {
      check_metric(std::stod(argv[2]));
      gp_Ax2 axis(gp_Pnt(11, 2, -6), gp_Dir(.2, .5, 1));
      shape = BRepPrimAPI_MakeTorus(axis, 10, 2, -.8, .6, 3.7).Shape();
    } else {
      STEPControl_Reader reader;
      if (reader.ReadFile(argv[1]) != IFSelect_RetDone || !reader.TransferRoots())
        return 2;
      shape = reader.OneShape();
    }
    const double deflection = std::stod(argv[2]);
    BRepMesh_IncrementalMesh raw(shape, deflection, false, .35, false);
    int tori = 0, original = 0, triangles = 0, rejected = 0;
    double bound = 0;
    for (TopExp_Explorer faces(shape, TopAbs_FACE); faces.More(); faces.Next()) {
      const auto face = TopoDS::Face(faces.Current().Oriented(TopAbs_FORWARD));
      BRepAdaptor_Surface surface(face);
      if (surface.GetType() != GeomAbs_Torus)
        continue;
      ++tori;
      TopLoc_Location loc;
      auto tri = BRep_Tool::Triangulation(face, loc);
      if (tri.IsNull())
        return 3;
      original += tri->NbTriangles();
      try {
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(face, TopAbs_EDGE, edges);
        std::vector<TrimEdgeSamples> cache(edges.Extent());
        for (int j = 1; j <= edges.Extent(); ++j) {
          auto edge = TopoDS::Edge(edges(j).Oriented(TopAbs_FORWARD));
          auto chain = BRep_Tool::PolygonOnTriangulation(edge, tri, loc);
          if (chain.IsNull() || !chain->HasParameters())
            return 4;
          for (int k = 1; k <= chain->NbNodes(); ++k) {
            cache[j - 1].push_back(
                {chain->Parameter(k), tri->Node(chain->Node(k)).Transformed(loc.Transformation())});
          }
        }
        auto samples = [&](const TopoDS_Edge &edge) -> const TrimEdgeSamples & {
          return cache.at(edges.FindIndex(edge) - 1);
        };
        auto mesh = trimmed_torus_mesh::mesh(face, deflection, .35, 100000, samples);
        triangles += mesh->NbTriangles();
        bound = std::max(bound, mesh->Deflection());
        if (mesh->Deflection() > deflection)
          return 5;
        for (int i = 1; i <= mesh->NbTriangles(); ++i) {
          int a, b, c;
          mesh->Triangle(i).Get(a, b, c);
          const auto uv = (mesh->UVNode(a).XY() + mesh->UVNode(b).XY() + mesh->UVNode(c).XY()) / 3.;
          auto p = surface.Value(uv.X(), uv.Y());
          auto chord = (mesh->Node(a).XYZ() + mesh->Node(b).XYZ() + mesh->Node(c).XYZ()) / 3.;
          if (p.Distance(gp_Pnt(chord)) > mesh->Deflection() + 1e-9)
            return 6;
        }
        if (self) {
          try {
            trimmed_torus_mesh::mesh(face, deflection, .35, 2, samples);
            return 7;
          } catch (const std::runtime_error &) {
          }
          cache.front().front().point.Translate(gp_Vec(.05, 0, 0));
          try {
            trimmed_torus_mesh::mesh(face, deflection, .35, 100000, samples);
            return 8;
          } catch (const std::runtime_error &) {
          }
        }
      } catch (std::exception &e) {
        ++rejected;
        std::cerr << "torus " << tori << " refused: " << e.what() << "\n";
      }
    }
    std::cout << std::setprecision(9) << "tori=" << tori << " native=" << original
              << " bounded=" << triangles << " deflection=" << bound << " refused=" << rejected
              << "\n";
    return tori > 0 && !rejected ? 0 : 1;
  } catch (Standard_Failure &e) {
    std::cerr << e.GetMessageString() << "\n";
    return 1;
  }
}
