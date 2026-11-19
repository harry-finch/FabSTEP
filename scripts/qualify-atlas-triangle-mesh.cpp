// Public fixture for the one-chord certificate, before and after STEP transfer.
#include "../crates/fabstep-cad/src/torus_atlas_triangle_mesh.hxx"
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <Geom_Circle.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <iostream>
#include <sstream>

namespace {
void require(bool value, const char *message) {
  if (!value)
    throw std::runtime_error(message);
}
TopoDS_Face fixture() {
  Handle(Geom_Circle) circle = new Geom_Circle(gp_Ax2(gp_Pnt(-4., 3., 1.), gp_Dir(0, 0, 1)), 12.);
  const double first = .7, last = first + .002;
  const auto a = circle->Value(first), b = circle->Value(last);
  const auto va = BRepBuilderAPI_MakeVertex(a).Vertex(), vb = BRepBuilderAPI_MakeVertex(b).Vertex();
  const gp_Pnt p(4., 8., 2.);
  const auto vp = BRepBuilderAPI_MakeVertex(p).Vertex();
  const auto arc = BRepBuilderAPI_MakeEdge(circle, va, vb, first, last).Edge();
  std::array<TopoDS_Edge, 3> rim{arc, BRepBuilderAPI_MakeEdge(vb, vp).Edge(), BRepBuilderAPI_MakeEdge(vp, va).Edge()};
  const auto patch = atlas_triangle::geometry(arc, p);
  require(bool(patch), "invalid public collar");
  const auto face = atlas_triangle::make(rim, 0, *patch, gp_Vec(p, a).Crossed(gp_Vec(p, b)).Normalized());
  require(bool(face), "failed to construct public collar");
  return TopoDS::Face(face->Oriented(TopAbs_FORWARD));
}
enum class Path { complete, intermediate, near_corner, moved, repeated };
void check(const TopoDS_Face &face) {
  auto sample = [](Path mode) {
    return [=](const TopoDS_Edge &edge) {
      require(edge.Orientation() == TopAbs_FORWARD, "noncanonical shared edge");
      const BRepAdaptor_Curve c(edge);
      const double lo = c.FirstParameter(), hi = c.LastParameter(), mid = (lo + hi) * .5;
      TrimEdgeSamples p{{lo, c.Value(lo)}, {hi, c.Value(hi)}};
      if (mode == Path::intermediate)
        p.insert(p.begin() + 1, {mid, c.Value(mid)});
      if (c.GetType() == GeomAbs_Circle && (mode == Path::moved || mode == Path::near_corner))
        p[1].point.Translate(gp_Vec(0., 0., mode == Path::moved ? 2 * Precision::Confusion()
                                                                : .25 * Precision::Confusion()));
      if (mode == Path::repeated)
        p[1] = p[0];
      return p;
    };
  };
  const auto tri = atlas_triangle_mesh::mesh(face, 1, .005, .35, sample(Path::complete));
  require(!tri.IsNull() && tri->NbTriangles() == 1 && tri->NbNodes() == 3, "certified face did not give one triangle");
  const auto near = atlas_triangle_mesh::mesh(face, 1, .005, .35, sample(Path::near_corner));
  require(!near.IsNull(), "nearby prescription was not anchored to the topological corner");
  for (int i = 1; i <= 3; ++i)
    require(near->Node(i).Distance(tri->Node(i)) == 0., "shared topological corner moved");
  const auto surf = BRepAdaptor_Surface(face, false);
  const auto normal = gp_Vec(tri->Node(1), tri->Node(2)).Crossed(gp_Vec(tri->Node(1), tri->Node(3)));
  for (int i = 0; i <= 48; ++i)
    for (int j = 0; i + j <= 48; ++j) {
      const double u = i / 48., v = j / 48.;
      const gp_Pnt chord(tri->Node(1).XYZ() * (1 - u - v) + tri->Node(2).XYZ() * u + tri->Node(3).XYZ() * v);
      gp_Pnt point;
      gp_Vec du, dv;
      surf.D1(u, v, point, du, dv);
      require(chord.Distance(point) <= tri->Deflection(), "understated continuous deflection");
      require(normal.Angle(du.Crossed(dv)) < .175, "normal left certified cone");
    }
  for (auto mode : {Path::intermediate, Path::moved, Path::repeated})
    require(atlas_triangle_mesh::mesh(face, 1, .005, .35, sample(mode)).IsNull(), "incompatible contour accepted");
  require(atlas_triangle_mesh::mesh(face, 0, .005, .35, sample(Path::complete)).IsNull(), "empty budget accepted");
  require(atlas_triangle_mesh::mesh(face, 1, 1e-9, .35, sample(Path::complete)).IsNull(), "ineligible accuracy");
  require(atlas_triangle_mesh::mesh(face, 1, .005, 1e-8, sample(Path::complete)).IsNull(), "ineligible normals");
  require(atlas_triangle_mesh::mesh(TopoDS::Face(face.Reversed()), 1, .005, .35, sample(Path::complete)).IsNull(),
          "unchecked face orientation");
}
}
int main() {
  try {
    const auto face = fixture();
    check(face);
    STEPControl_Writer writer;
    require(writer.Transfer(face, STEPControl_AsIs) == IFSelect_RetDone, "STEP transfer failed");
    std::stringstream step;
    require(writer.WriteStream(step) == IFSelect_RetDone, "STEP write failed");
    STEPControl_Reader reader;
    require(reader.ReadStream("triangle.step", step) == IFSelect_RetDone && reader.TransferRoots(), "STEP read failed");
    TopExp_Explorer faces(reader.OneShape(), TopAbs_FACE);
    require(faces.More(), "missing STEP face");
    check(TopoDS::Face(faces.Current().Oriented(TopAbs_FORWARD)));
    std::cout << "One-chord mesh: orientation, continuous deflection, normals, fallback, STEP passed.\n";
    return 0;
  } catch (const std::exception &e) {
    std::cerr << e.what() << '\n';
    return 1;
  }
}
