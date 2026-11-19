// Explicit reimport test of a producer-certified checkpoint. This laboratory
// does not register a generic STEP input or bypass the app's source proof.
#include "../crates/fabstep-cad/src/occt.cpp"
#include "trim_native_fan_mesh.hxx"
#include "trimmed_torus_mesh.hxx"
#include <iostream>

namespace {
struct Policy {
  double deflection;
  bool candidate(const TopoDS_Face &face) const {
    return trimmed_torus_mesh::supports(face) || bool(trimfan::profile(face, deflection, mesh_angle));
  }
  template <class Samples>
  Handle(Poly_Triangulation) mesh(const TopoDS_Face &face, size_t remaining, const Samples &samples) const {
    if (BRepAdaptor_Surface(face).GetType() == GeomAbs_Torus)
      return trimmed_torus_mesh::mesh(face, deflection, mesh_angle, remaining, samples);
    return trimfan::mesh(face, deflection, mesh_angle, samples);
  }
  std::optional<gp_Pnt> pole(const TopoDS_Face &face, const TopoDS_Edge &edge) const {
    return trimfan::pole(face, edge);
  }
};

// Weld for topology inspection only; every candidate must be within native
// confusion in 3D. Ambiguous welds, a collapsed segment, or a seam without
// exactly two opposed uses fail the test, including at periodic charts.
struct MeshAudit {
  size_t vertices, edges;
  double volume;
};
MeshAudit audit(const MeshOutput &mesh) {
  using Cell = std::array<long long, 3>;
  std::map<Cell, std::vector<int>> cells;
  std::vector<gp_Pnt> vertices;
  std::vector<int> ids;
  const double tolerance = Precision::Confusion();
  for (size_t i = 0; i < mesh.point_count; ++i) {
    const auto point = point_at(mesh.points, i);
    Cell cell{};
    for (int k = 0; k < 3; ++k) {
      const double key = std::floor(point.Coord(k + 1) / tolerance);
      trimcache::require(std::isfinite(key) && std::abs(key) < 9.e18, "Audit coordinate out of range.");
      cell[k] = static_cast<long long>(key);
    }
    int match = -1;
    for (int x = -1; x <= 1; ++x)
      for (int y = -1; y <= 1; ++y)
        for (int z = -1; z <= 1; ++z) {
          const auto bucket = cells.find({cell[0] + x, cell[1] + y, cell[2] + z});
          if (bucket == cells.end())
            continue;
          for (int id : bucket->second)
            if (vertices[id].Distance(point) <= tolerance) {
              trimcache::require(match < 0, "Ambiguous tolerance weld.");
              match = id;
            }
        }
    if (match < 0) {
      match = static_cast<int>(vertices.size());
      vertices.push_back(point);
      cells[cell].push_back(match);
    }
    ids.push_back(match);
  }
  std::map<std::pair<int, int>, std::array<int, 2>> uses;
  double volume = 0.;
  const auto origin = vertices.front();
  for (size_t i = 0; i < mesh.triangle_count; ++i) {
    const auto t = mesh.triangles + 3 * i;
    std::array<int, 3> v{ids.at(t[0]), ids.at(t[1]), ids.at(t[2])};
    for (int j = 0; j < 3; ++j) {
      int a = v[j], b = v[(j + 1) % 3];
      trimcache::require(a != b, "Welded triangle collapsed.");
      ++uses[std::minmax(a, b)][a > b];
    }
    const gp_Vec a(origin, vertices[v[0]]), b(origin, vertices[v[1]]), c(origin, vertices[v[2]]);
    volume += a.Dot(b.Crossed(c)) / 6.;
  }
  size_t free = 0, overused = 0, unoriented = 0;
  for (const auto &entry : uses) {
    const auto &pair = entry.second;
    if (pair[0] + pair[1] == 1)
      ++free;
    else if (pair[0] + pair[1] != 2)
      ++overused;
    else if (pair[0] != 1)
      ++unoriented;
  }
  if (free || overused || unoriented)
    throw std::runtime_error("Unclosed mesh: " + std::to_string(free) + " free, " + std::to_string(overused) +
                             " nonmanifold, " + std::to_string(unoriented) + " unoriented edges.");
  trimcache::require(volume > 0. && std::isfinite(volume), "Invalid signed mesh volume.");
  return {vertices.size(), uses.size(), volume};
}
} // namespace

int main(int argc, char **argv) try {
  if (argc != 2)
    throw std::runtime_error("Expected one producer-certified solid STEP checkpoint.");
  STEPControl_Reader reader;
  if (std::string(argv[1]) == "--witness") {
    const TorusSectorSolidData data{{7., -3., 12.}, {0., 0., 1.}, {1., 0., 0.}, 5., 2., 7., 1., 1.2};
    const auto source = torus_sector::build(data, nullptr, nullptr);
    STEPControl_Writer writer;
    std::stringstream stream;
    trimcache::require(writer.Transfer(source, STEPControl_AsIs) == IFSelect_RetDone &&
                           writer.WriteStream(stream) == IFSelect_RetDone,
                       "Synthetic witness serialization failed.");
    load_step(reader, stream);
  } else {
    trimcache::require(reader.ReadFile(argv[1]) == IFSelect_RetDone, "STEP checkpoint read failed.");
  }
  trimcache::require(reader.NbRootsForTransfer() == 1 && reader.TransferRoots() == 1, "STEP reimport failed.");
  const auto solid = single_solid(reader.OneShape(), read_failed);
  trimcache::require(BRepCheck_Analyzer(solid, true, false, true).IsValid(), "Reimported solid is invalid.");
  constexpr double deflection = .005;
  std::vector<ResultFaceData> faces;
  for (TopExp_Explorer f(solid, TopAbs_FACE); f.More(); f.Next())
    faces.emplace_back();
  MeshOutput mesh{};
  tessellate_certified_trim_faces(solid, deflection, &mesh, &faces, Policy{deflection});
  std::unique_ptr<double[]> points(mesh.points);
  std::unique_ptr<size_t[]> triangles(mesh.triangles);
  size_t end = 0;
  for (const auto &face : faces) {
    trimcache::require(face.triangle_count > 0 && face.triangle_start == end, "Uncovered STEP face.");
    end += face.triangle_count;
  }
  trimcache::require(end == mesh.triangle_count && end <= mesh_triangle_limit, "Invalid triangle ranges.");
  const auto result = audit(mesh);
  GProp_GProps native, area;
  BRepGProp::VolumeProperties(solid, native);
  BRepGProp::SurfaceProperties(solid, area);
  // Independent scale check, in addition to topological closure and the
  // face-wise mesher bounds. The source producer validates the exact BRep.
  trimcache::require(std::abs(native.Mass() - result.volume) <= deflection * area.Mass(), "Volume drift.");
  std::cout.precision(15);
  std::cout << "faces=" << faces.size() << " triangles=" << end << " output_nodes=" << mesh.point_count
            << " welded_nodes=" << result.vertices << " opposed_edges=" << result.edges
            << " mesh_volume_mm3=" << result.volume << " reimported_mm3=" << native.Mass() << '\n';
  return 0;
} catch (const CadFailure &error) {
  std::cerr << error.message << '\n';
  return 1;
} catch (const std::exception &error) {
  std::cerr << error.what() << '\n';
  return 1;
}
