#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepAlgoAPI_Check.hxx>
#include <BRepAlgoAPI_Section.hxx>
#include <BRepClass_FaceClassifier.hxx>
#include <BRepLProp_SLProps.hxx>
#include <BRepBndLib.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepTools_ReShape.hxx>
#include <BRepTools.hxx>
#include <BRepTools_WireExplorer.hxx>
#include <BRepExtrema_DistShapeShape.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepLib.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeSphere.hxx>
#include <BRepPrimAPI_MakeTorus.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <DESTEP_Parameters.hxx>
#include <GProp_GProps.hxx>
#include <Geom2d_Curve.hxx>
#include <Geom2d_Line.hxx>
#include <IMeshData_Status.hxx>
#include <Message.hxx>
#include <Message_Messenger.hxx>
#include <Poly_Triangulation.hxx>
#include <Precision.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <STEPConstruct.hxx>
#include <StepShape_AdvancedFace.hxx>
#include <XSControl_WorkSession.hxx>
#include <XSControl_TransferReader.hxx>
#include <XSControl_TransferWriter.hxx>
#include <Interface_InterfaceModel.hxx>
#include <Standard_Failure.hxx>
#include <Standard_Version.hxx>
#include <StepBasic_Product.hxx>
#include <StepBasic_ProductDefinition.hxx>
#include <StepBasic_ProductDefinitionFormation.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Iterator.hxx>
#include <TopoDS_Solid.hxx>
#include <TopoDS_Vertex.hxx>
#include <gp_Circ.hxx>
#include <gp_Elips.hxx>
#include <gp_Cone.hxx>
#include <gp_Cylinder.hxx>
#include <gp_Pln.hxx>
#include <gp_Sphere.hxx>
#include <gp_Torus.hxx>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <exception>
#include <limits>
#include <map>
#include <memory>
#include <mutex>
#include <numeric>
#include <optional>
#include <set>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

extern "C" {
struct CadMetrics {
  size_t solids;
  size_t faces;
  size_t planar_faces;
  size_t cylindrical_faces;
  size_t conical_faces;
  size_t spherical_faces;
  size_t toroidal_faces;
  double volume;
  double bounds[6];
};

struct StepOutput {
  unsigned char* data;
  size_t size;
  CadMetrics metrics;
};

struct StepPartInput {
  const unsigned char* bytes;
  size_t size;
  const char* name;
};

struct PlanarPartInput {
  const double* points;
  size_t point_count;
  const double* normals;
  size_t face_count;
  const size_t* face_loops;
  const size_t* loop_vertices;
  const size_t* vertex_ids;
};

struct ProfileLoopData {
  size_t point_start;
  size_t point_count;
  double center[2];
  double radius;
};

struct MeshOutput {
  double* points;
  size_t point_count;
  size_t* triangles;
  size_t triangle_count;
};

struct ResultFaceData {
  size_t solid_id;
  size_t family;
  size_t triangle_start;
  size_t triangle_count;
  size_t neighbor_start;
  size_t neighbor_count;
  size_t source_face_token; // Untrusted STEP label; zero means unavailable.
};

struct ResultFacesOutput {
  MeshOutput mesh;
  ResultFaceData* faces;
  size_t face_count;
  size_t* neighbors;
  size_t neighbor_count;
};

struct PlanarProfileSegmentData {
  size_t kind;
  double start[2];
  double end[2];
  double center[2];
  double radius;
  double start_angle;
  double sweep_angle;
  double major_axis[2];
  double ratio;
};

struct PlanarProfileLoopData {
  size_t start;
  size_t count;
  size_t is_outer;
};

struct PlanarProfileOutput {
  size_t solid_id;
  double origin[3];
  double x_axis[3];
  double y_axis[3];
  double normal[3];
  PlanarProfileLoopData* loops;
  size_t loop_count;
  PlanarProfileSegmentData* segments;
  size_t segment_count;
};

struct SectionLoopData {
  size_t solid_id;
  size_t component_id;
  size_t start;
  size_t count;
  size_t is_outer;
};

struct SectionOutput {
  SectionLoopData* loops;
  size_t loop_count;
  PlanarProfileSegmentData* segments;
  size_t segment_count;
};

struct MixedRingData {
  double center[3];
  double axis[3];
  double radius;
};

struct MixedCylinderData {
  size_t rings[2];
  int inner;
};

struct SphericalPatchData {
  size_t rings[2];
  double center[3];
  double radius;
  int inner;
};

struct ToroidalPatchData {
  size_t rings[2];
  double center[3];
  double major_radius;
  double minor_radius;
  double meridian[2];
  int inner;
};

struct MixedLoopData {
  size_t start;
  size_t count; // Zero means start identifies a full circular ring.
};

struct MixedArcData {
  double center[3];
  double axis[3];
  double radius;
  double sweep;
  size_t vertices[2];
};

struct MixedSegmentData {
  size_t vertices[2];
  size_t arc; // SIZE_MAX identifies a line.
};
}

namespace {
enum Status {
  success, invalid_model, invalid_solid, write_failed, read_failed, mismatch, failure,
  mesh_failed, mesh_too_complex, bodies_not_separated, document_too_large, separation_budget,
  profile_face_not_found, profile_not_planar,
  profile_unsupported_curve, profile_invalid, profile_too_complex,
  section_coplanar, section_tangent, section_open, section_invalid_plane
};

struct CadFailure {
  Status status;
  std::string message;
};

struct SurfaceCounts {
  size_t planar;
  size_t cylindrical;
  size_t partial = 0;
  size_t conical = 0;
  size_t spherical = 0;
  size_t toroidal = 0;
  size_t spherical_octants = 0;
  size_t partial_conical = 0;
};

constexpr SurfaceCounts rounded_box_surfaces{6, 12, 12, 0, 8, 0, 8};

// STEP uses process-wide OCCT configuration and a default stdout messenger.
// Serialize calls and restore the messenger so CLI JSON stays machine-readable.
std::mutex occt_mutex;

class QuietMessenger {
public:
  QuietMessenger() : messenger(Message::DefaultMessenger()),
                     saved(std::move(messenger->ChangePrinters())) {}
  ~QuietMessenger() { messenger->ChangePrinters() = std::move(saved); }

private:
  Handle(Message_Messenger) messenger;
  Message_SequenceOfPrinters saved;
};

gp_Pnt point_at(const double* coordinates, size_t index) {
  return gp_Pnt(coordinates[index * 3], coordinates[index * 3 + 1], coordinates[index * 3 + 2]);
}

class IndexedWires {
public:
  IndexedWires(const double* points, size_t point_count) : points(points), vertices(point_count) {}

  void bind_vertices(size_t from, size_t to, const TopoDS_Edge& edge) {
    TopoDS_Vertex start, end;
    TopExp::Vertices(edge, start, end, true);
    bind_vertex(from, start);
    bind_vertex(to, end);
  }

  TopoDS_Vertex canonical_vertex(size_t index, const TopoDS_Vertex& vertex) {
    if (vertex.IsNull() ||
        BRep_Tool::Pnt(vertex).Distance(point_at(points, index)) > Precision::Confusion()) {
      throw CadFailure{invalid_model, "Un sommet cylindrique dépasse son sommet source."};
    }
    if (vertices[index].IsNull()) {
      vertices[index] = vertex;
    } else if (BRep_Tool::Pnt(vertices[index]).Distance(BRep_Tool::Pnt(vertex)) > Precision::Confusion()) {
      throw CadFailure{invalid_model, "Deux sommets cylindriques partagés dépassent la tolérance native."};
    }
    return vertices[index];
  }

  // The edge argument runs from source `from` to `to`. Face traversal includes
  // the inner-face reversal, so two cylinders must traverse their rail oppositely.
  TopoDS_Edge bind_line(size_t from, size_t to, const TopoDS_Edge& edge, bool traversal) {
    bind_vertices(from, to, edge);
    const auto key = std::minmax(from, to);
    const bool canonical_traversal = traversal == (from < to);
    auto found = edges.find(key);
    if (found == edges.end()) {
      edges.emplace(key, from < to ? edge : TopoDS::Edge(edge.Reversed()));
      rail_traversals.emplace(key, canonical_traversal);
      return edge;
    }
    auto previous = rail_traversals.find(key);
    if (previous == rail_traversals.end() || previous->second == canonical_traversal) {
      throw CadFailure{invalid_model, "Les cylindres ne parcourent pas leur génératrice en sens opposés."};
    }
    rail_traversals.erase(previous); // No third cylindrical incidence.
    return from < to ? found->second : TopoDS::Edge(found->second.Reversed());
  }

  TopoDS_Edge line(const gp_Pln& plane, size_t from, size_t to) {
    for (const size_t i : {from, to}) {
      const gp_Pnt point = point_at(points, i);
      if (plane.Distance(point) > Precision::Confusion()) {
        throw CadFailure{invalid_model, "Le contour d'une face dépasse la tolérance de planéité du noyau."};
      }
      if (vertices[i].IsNull()) {
        builder.MakeVertex(vertices[i], point, Precision::Confusion());
      } else if (plane.Distance(BRep_Tool::Pnt(vertices[i])) > Precision::Confusion()) {
        throw CadFailure{invalid_model, "Une génératrice ou une extrémité d'arc sort du plan adjacent."};
      }
    }
    const auto key = std::minmax(from, to);
    auto edge = edges.find(key);
    if (edge == edges.end()) {
      BRepBuilderAPI_MakeEdge make_edge(vertices[key.first], vertices[key.second]);
      if (!make_edge.IsDone()) {
        throw CadFailure{invalid_model, "Une arête est trop courte ou invalide."};
      }
      edge = edges.emplace(key, make_edge.Edge()).first;
    }
    return from < to ? edge->second : TopoDS::Edge(edge->second.Reversed());
  }

  TopoDS_Wire polygon(const gp_Pln& plane, const size_t* ids, size_t count) {
    BRepBuilderAPI_MakeWire make_wire;
    for (size_t i = 0; i < count; ++i) {
      make_wire.Add(line(plane, ids[i], ids[(i + 1) % count]));
    }
    if (!make_wire.IsDone() || !BRep_Tool::IsClosed(make_wire.Wire())) {
      throw CadFailure{invalid_model, "Impossible de construire un contour fermé."};
    }
    return make_wire.Wire();
  }

private:
  void bind_vertex(size_t index, const TopoDS_Vertex& vertex) {
    if (vertex.IsNull() ||
        BRep_Tool::Pnt(vertex).Distance(point_at(points, index)) > Precision::Confusion() ||
        (!vertices[index].IsNull() && !vertices[index].IsSame(vertex))) {
      throw CadFailure{invalid_model, "Un sommet de cylindre partiel ne correspond pas au sommet partagé."};
    }
    vertices[index] = vertex;
  }

  const double* points;
  BRep_Builder builder;
  std::vector<TopoDS_Vertex> vertices;
  std::map<std::pair<size_t, size_t>, TopoDS_Edge> edges;
  std::map<std::pair<size_t, size_t>, bool> rail_traversals;
};

TopoDS_Solid solid_from_sewing(BRepBuilderAPI_Sewing& sewing) {
  sewing.Perform();
  const TopoDS_Shape& sewed = sewing.SewedShape();
  if (sewing.NbFreeEdges() != 0 || sewing.NbMultipleEdges() != 0 ||
      sewing.NbDeletedFaces() != 0 || sewed.IsNull() || sewed.ShapeType() != TopAbs_SHELL ||
      !BRep_Tool::IsClosed(sewed)) {
    throw CadFailure{invalid_solid, "Les faces ne forment pas une seule coque fermée et cousue."};
  }
  BRepBuilderAPI_MakeSolid make_solid(TopoDS::Shell(sewed));
  if (!make_solid.IsDone()) {
    throw CadFailure{invalid_solid, "Impossible de construire un solide."};
  }
  TopoDS_Solid solid = make_solid.Solid();
  if (!BRepLib::OrientClosedSolid(solid)) {
    throw CadFailure{invalid_solid, "Impossible d'orienter le volume fermé."};
  }
  return solid;
}

using FaceLabels = std::vector<std::pair<TopoDS_Shape, size_t>>;

// Preserve construction identity through the sewing history, never through
// traversal order or surface proximity. A split/merge drops the whole map.
void track_sewn_faces(const TopoDS_Solid& solid, BRepBuilderAPI_Sewing& sewing,
                      FaceLabels& labels) {
  TopTools_IndexedMapOfShape final_faces;
  TopExp::MapShapes(solid, TopAbs_FACE, final_faces);
  TopTools_IndexedMapOfShape mapped;
  if (labels.size() != static_cast<size_t>(final_faces.Extent())) {
    labels.clear();
    return;
  }
  for (auto& entry : labels) {
    if (sewing.IsModified(entry.first)) {
      entry.first = sewing.Modified(entry.first);
    } else if (sewing.IsModifiedSubShape(entry.first)) {
      entry.first = sewing.ModifiedSubShape(entry.first);
    }
    if (entry.first.IsNull() || entry.first.ShapeType() != TopAbs_FACE ||
        !final_faces.Contains(entry.first) || mapped.Contains(entry.first)) {
      labels.clear();
      return;
    }
    mapped.Add(entry.first);
  }
}

TopoDS_Solid build_solid(
    const double* points, size_t point_count, const double* normals, size_t face_count,
    const size_t* face_loops, const size_t* loop_vertices, const size_t* vertex_ids,
    const size_t* tokens = nullptr, FaceLabels* labels = nullptr) {
  IndexedWires wires(points, point_count);
  BRepBuilderAPI_Sewing sewing(Precision::Confusion());
  sewing.SetMaxTolerance(Precision::Confusion());

  for (size_t face = 0; face < face_count; ++face) {
    const size_t first_loop = face_loops[face];
    const size_t origin_id = vertex_ids[loop_vertices[first_loop]];
    const gp_Pln plane(point_at(points, origin_id),
                      gp_Dir(normals[3 * face], normals[3 * face + 1], normals[3 * face + 2]));
    BRepBuilderAPI_MakeFace make_face;

    for (size_t loop = first_loop; loop < face_loops[face + 1]; ++loop) {
      const size_t start = loop_vertices[loop];
      const size_t end = loop_vertices[loop + 1];
      const TopoDS_Wire wire = wires.polygon(plane, vertex_ids + start, end - start);
      if (loop == first_loop) {
        make_face = BRepBuilderAPI_MakeFace(plane, wire, true);
      } else {
        make_face.Add(wire);
      }
      if (!make_face.IsDone()) {
        throw CadFailure{invalid_model, "Impossible de construire une face plane et ses contours."};
      }
    }
    sewing.Add(make_face.Face());
    if (tokens && labels) {
      labels->push_back({make_face.Face(), tokens[face]});
    }
  }

  const auto solid = solid_from_sewing(sewing);
  if (labels) {
    try {
      track_sewn_faces(solid, sewing, *labels);
    } catch (const Standard_Failure&) {
      labels->clear(); // Metadata failure cannot invalidate valid geometry.
    }
  }
  return solid;
}

gp_Dir direction(const double* normal) {
  return gp_Dir(normal[0], normal[1], normal[2]);
}

double rim_difference(const gp_Circ& circle, const gp_Pnt& center,
                      const gp_Dir& axis, double radius) {
  const double angular_error = gp_Vec(circle.Axis().Direction())
      .Crossed(gp_Vec(axis)).Magnitude() * radius;
  return circle.Location().Distance(center) + std::abs(circle.Radius() - radius) + angular_error;
}

void register_rims(const TopoDS_Face& face, const size_t* ids, const MixedRingData* rings,
                   std::vector<TopoDS_Edge>& rims) {
  const double tolerance = Precision::Confusion();
  TopoDS_Edge boundaries[2];
  for (TopExp_Explorer explorer(face, TopAbs_EDGE); explorer.More(); explorer.Next()) {
    const TopoDS_Edge edge = TopoDS::Edge(explorer.Current());
    if (BRep_Tool::Degenerated(edge)) {
      continue;
    }
    const BRepAdaptor_Curve curve(edge);
    if (curve.GetType() != GeomAbs_Circle) {
      continue;
    }
    const gp_Circ circle = curve.Circle();
    for (size_t i = 0; i < 2; ++i) {
      const auto& ring = rings[ids[i]];
      if (ring.radius > 0.0 && circle.Location().Distance(point_at(ring.center, 0)) <= tolerance &&
          std::abs(circle.Radius() - ring.radius) <= tolerance &&
          gp_Vec(circle.Axis().Direction()).Crossed(gp_Vec(direction(ring.axis))).Magnitude() *
              ring.radius <= tolerance) {
        if (!boundaries[i].IsNull()) {
          throw CadFailure{invalid_model, "Un anneau de la paroi correspond à plusieurs arêtes."};
        }
        boundaries[i] = edge;
      }
    }
  }
  for (size_t i = 0; i < 2; ++i) {
    const auto index = ids[i];
    const auto& edge = boundaries[i];
    const double radius = rings[index].radius;
    if ((radius > 0.0) != !edge.IsNull()) {
      throw CadFailure{invalid_model, "Impossible de retrouver une arête circulaire de la paroi."};
    }
    // Sewing joins the p-curves of coincident rims on one edge at native tolerance.
    if (rims[index].IsNull()) {
      rims[index] = edge;
    } else {
      const auto previous = BRepAdaptor_Curve(rims[index]).Circle();
      const auto current = BRepAdaptor_Curve(edge).Circle();
      if (rim_difference(previous, current.Location(), current.Axis().Direction(), current.Radius()) >
          tolerance) {
        throw CadFailure{invalid_model, "Les arêtes circulaires de la jonction ne coïncident pas."};
      }
    }
  }
}

TopoDS_Face revolved_face(const MixedCylinderData& wall, const MixedRingData* rings,
                         std::vector<TopoDS_Edge>& rims, bool conical) {
  const auto& first = rings[wall.rings[0]];
  const auto& second = rings[wall.rings[1]];
  const gp_Pnt lower = point_at(first.center, 0);
  const gp_Pnt upper = point_at(second.center, 0);
  const gp_Dir axis = direction(first.axis);
  const gp_Vec between(lower, upper);
  const double height = between.Dot(gp_Vec(axis));
  const double off_axis = between.Crossed(gp_Vec(axis)).Magnitude();
  const double tolerance = Precision::Confusion();
  const double radial_change = second.radius - first.radius;
  if (!std::isfinite(height) || height <= tolerance ||
      !std::isfinite(off_axis) || off_axis > tolerance ||
      !axis.IsEqual(direction(second.axis), 1.0e-8) ||
      (conical ? std::abs(radial_change) <= tolerance ||
                 (first.radius > 0.0 && first.radius <= tolerance) ||
                 (second.radius > 0.0 && second.radius <= tolerance) ||
                 gp_Vec(axis).Crossed(gp_Vec(direction(second.axis))).Magnitude() * second.radius > tolerance
               : std::abs(radial_change) > tolerance || first.radius <= tolerance)) {
    throw CadFailure{invalid_model, "Les sections ne définissent pas une paroi droite de révolution."};
  }

  const gp_Ax3 frame(lower, axis);
  const double full_turn = 2.0 * std::acos(-1.0);
  const auto make_face = conical
      ? BRepBuilderAPI_MakeFace(gp_Cone(frame, std::atan2(radial_change, height), first.radius),
                               0.0, full_turn, 0.0, std::hypot(radial_change, height))
      : BRepBuilderAPI_MakeFace(gp_Cylinder(frame, first.radius), 0.0, full_turn, 0.0, height);
  if (!make_face.IsDone()) {
    throw CadFailure{invalid_model, "Impossible de borner la face de révolution."};
  }
  const auto& face = make_face.Face();
  register_rims(face, wall.rings, rings, rims);
  return wall.inner ? TopoDS::Face(face.Reversed()) : face;
}

TopoDS_Face spherical_face(const SphericalPatchData& patch, const MixedRingData* rings,
                          std::vector<TopoDS_Edge>& rims) {
  const auto center = point_at(patch.center, 0);
  const auto axis = direction(rings[patch.rings[0]].axis);
  const double tolerance = Precision::Confusion();
  const double full_turn = 2.0 * std::acos(-1.0);
  double latitude[2];
  for (size_t i = 0; i < 2; ++i) {
    const auto& ring = rings[patch.rings[i]];
    const gp_Vec delta(center, point_at(ring.center, 0));
    const double height = delta.Dot(gp_Vec(axis));
    if (!std::isfinite(height) || delta.Crossed(gp_Vec(axis)).Magnitude() > tolerance ||
        !axis.IsEqual(direction(ring.axis), 1.0e-8) ||
        std::abs(std::hypot(height, ring.radius) - patch.radius) > tolerance ||
        (ring.radius > 0.0 && ring.radius <= tolerance)) {
      throw CadFailure{invalid_model, "Une limite de sphère ne correspond pas à son anneau ou à son pôle."};
    }
    latitude[i] = std::atan2(height, ring.radius);
  }
  if ((latitude[1] - latitude[0]) * patch.radius <= tolerance) {
    throw CadFailure{invalid_model, "La latitude de la face sphérique est invalide."};
  }
  const BRepBuilderAPI_MakeFace make_face(gp_Sphere(gp_Ax3(center, axis), patch.radius),
                                         0.0, full_turn, latitude[0], latitude[1]);
  if (!make_face.IsDone()) {
    throw CadFailure{invalid_model, "Impossible de borner la face sphérique."};
  }
  const auto& face = make_face.Face();
  register_rims(face, patch.rings, rings, rims);
  return patch.inner ? TopoDS::Face(face.Reversed()) : face;
}

TopoDS_Face toroidal_face(const ToroidalPatchData& patch, const MixedRingData* rings,
                         std::vector<TopoDS_Edge>& rims) {
  const auto center = point_at(patch.center, 0);
  const auto axis = direction(rings[patch.rings[0]].axis);
  const double tolerance = Precision::Confusion();
  const double full_turn = 2.0 * std::acos(-1.0);
  const double span = patch.meridian[1] - patch.meridian[0];
  if (span * patch.minor_radius <= tolerance || span >= full_turn ||
      patch.minor_radius <= tolerance || patch.major_radius - patch.minor_radius <= tolerance) {
    throw CadFailure{invalid_model, "La face torique requiert un tore annulaire et un méridien borné."};
  }
  for (size_t i = 0; i < 2; ++i) {
    const auto& ring = rings[patch.rings[i]];
    const auto angle = patch.meridian[i];
    const double height = patch.minor_radius * std::sin(angle);
    const double radius = patch.major_radius + patch.minor_radius * std::cos(angle);
    const auto expected_center = center.Translated(gp_Vec(axis) * height);
    if (point_at(ring.center, 0).Distance(expected_center) + std::abs(ring.radius - radius) > tolerance ||
        !axis.IsEqual(direction(ring.axis), 1.0e-8) || ring.radius <= tolerance) {
      throw CadFailure{invalid_model, "Une limite de tore ne correspond pas à son anneau."};
    }
  }
  const BRepBuilderAPI_MakeFace make_face(
      gp_Torus(gp_Ax3(center, axis), patch.major_radius, patch.minor_radius),
      0.0, full_turn, patch.meridian[0], patch.meridian[1]);
  if (!make_face.IsDone()) {
    throw CadFailure{invalid_model, "Impossible de borner la face torique."};
  }
  const auto& face = make_face.Face();
  register_rims(face, patch.rings, rings, rims);
  return patch.inner ? TopoDS::Face(face.Reversed()) : face;
}

void require_circle_in_plane(const gp_Pln& plane, const gp_Circ& circle) {
  const gp_Dir normal = plane.Axis().Direction();
  const gp_Dir axis = circle.Axis().Direction();
  const double sine = gp_Vec(normal).Crossed(gp_Vec(axis)).Magnitude();
  if (plane.Distance(circle.Location()) + sine * circle.Radius() > Precision::Confusion()) {
    throw CadFailure{invalid_model, "Un contour circulaire ne repose pas dans le plan de sa face."};
  }
}

TopoDS_Wire rim_wire(const gp_Pln& plane, const TopoDS_Edge& edge, bool hole) {
  const gp_Circ circle = BRepAdaptor_Curve(edge).Circle();
  require_circle_in_plane(plane, circle);
  BRepBuilderAPI_MakeWire make_wire(TopoDS::Edge(edge.Oriented(TopAbs_FORWARD)));
  if (!make_wire.IsDone() || !BRep_Tool::IsClosed(make_wire.Wire())) {
    throw CadFailure{invalid_model, "Impossible de fermer un contour circulaire partagé."};
  }
  const bool reverse = (circle.Axis().Direction().Dot(plane.Axis().Direction()) < 0.0) != hole;
  return reverse ? TopoDS::Wire(make_wire.Wire().Reversed()) : make_wire.Wire();
}

TopoDS_Edge oriented_between(const TopoDS_Edge& edge, const gp_Pnt& from, const gp_Pnt& to) {
  const auto forward = TopoDS::Edge(edge.Oriented(TopAbs_FORWARD));
  TopoDS_Vertex a, b;
  TopExp::Vertices(forward, a, b, true);
  if (a.IsNull() || b.IsNull()) {
    return {};
  }
  const auto p = BRep_Tool::Pnt(a);
  const auto q = BRep_Tool::Pnt(b);
  const double tol = Precision::Confusion();
  const bool same = p.Distance(from) <= tol && q.Distance(to) <= tol;
  const bool reverse = q.Distance(from) <= tol && p.Distance(to) <= tol;
  if (same == reverse) {
    return {};
  }
  return same ? forward : TopoDS::Edge(forward.Reversed());
}

TopoDS_Face partial_revolved_face(
    const MixedCylinderData& cylinder, const MixedArcData* arcs, const double* points,
    std::vector<TopoDS_Edge>& shared_arcs, IndexedWires& wires, bool conical = false) {
  const auto& lower = arcs[cylinder.rings[0]];
  const auto& upper = arcs[cylinder.rings[1]];
  const auto origin = point_at(lower.center, 0);
  const auto axis = direction(lower.axis);
  const auto upper_axis = direction(upper.axis);
  const gp_Vec delta(origin, point_at(upper.center, 0));
  const double height = delta.Dot(gp_Vec(axis));
  const double lateral = delta.Crossed(gp_Vec(axis)).Magnitude();
  const double tol = Precision::Confusion();
  const double full_turn = 2.0 * std::acos(-1.0);
  if (!std::isfinite(height) || height <= tol || !std::isfinite(lateral) || lateral > tol ||
      !axis.IsEqual(upper_axis, 1.0e-8) || lower.radius <= tol ||
      gp_Vec(axis).Crossed(gp_Vec(upper_axis)).Magnitude() * std::max(lower.radius, upper.radius) > tol ||
      (!conical && std::abs(lower.radius - upper.radius) > tol) ||
      (conical && (upper.radius <= tol || std::abs(lower.radius - upper.radius) <= tol)) ||
      std::abs(lower.sweep - upper.sweep) * std::max(lower.radius, upper.radius) > tol ||
      std::min(lower.sweep, full_turn - lower.sweep) * lower.radius <= 2.0 * tol) {
    throw CadFailure{invalid_model, "Les arcs ne définissent pas une paroi de révolution partielle commune."};
  }

  const gp_Dir start(gp_Vec(origin, point_at(points, lower.vertices[0])));
  const double angle = conical ? std::atan2(upper.radius - lower.radius, height) : 0.0;
  const double cosine = std::cos(angle);
  const gp_Ax3 frame(origin, axis, start);
  BRepBuilderAPI_MakeFace make_face;
  if (conical) make_face = BRepBuilderAPI_MakeFace(gp_Cone(frame, angle, lower.radius),
                                                 0.0, lower.sweep, 0.0, height / cosine);
  else make_face = BRepBuilderAPI_MakeFace(gp_Cylinder(frame, lower.radius),
                                          0.0, lower.sweep, 0.0, height);
  if (!make_face.IsDone()) {
    throw CadFailure{invalid_model, "Impossible de borner la paroi de révolution partielle."};
  }
  // Canonicalize source vertices before recording either arcs or rails. Adjacent
  // cylinders may meet at a rail, and their planar cap arcs must share its ends.
  auto face = make_face.Face();
  BRepTools_ReShape vertex_replacements;
  for (TopExp_Explorer it(face, TopAbs_VERTEX); it.More(); it.Next()) {
    const auto vertex = TopoDS::Vertex(it.Current());
    bool matched = false;
    for (const auto index : cylinder.rings) {
      for (const auto id : arcs[index].vertices) {
        if (BRep_Tool::Pnt(vertex).Distance(point_at(points, id)) <= tol) {
          const auto canonical = wires.canonical_vertex(id, vertex);
          if (!vertex.IsSame(canonical)) {
            vertex_replacements.Replace(vertex.Oriented(TopAbs_FORWARD), canonical.Oriented(TopAbs_FORWARD));
          }
          matched = true;
          break;
        }
      }
      if (matched) break;
    }
    if (!matched) {
      throw CadFailure{invalid_model, "Un sommet de la paroi n'a pas d'extrémité source."};
    }
  }
  face = TopoDS::Face(vertex_replacements.Apply(face));
  BRepTools_ReShape rail_replacements;
  bool rails[2]{};
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    const auto type = BRepAdaptor_Curve(edge).GetType();
    if (type == GeomAbs_Circle) {
      for (const auto index : cylinder.rings) {
        const auto& arc = arcs[index];
        const auto oriented = oriented_between(edge, point_at(points, arc.vertices[0]),
                                               point_at(points, arc.vertices[1]));
        if (!oriented.IsNull()) {
          if (shared_arcs[index].IsNull()) {
            shared_arcs[index] = oriented;
            wires.bind_vertices(arc.vertices[0], arc.vertices[1], oriented);
          } else {
            const auto shared = shared_arcs[index];
            const BRepAdaptor_Curve curve(shared);
            const double first = curve.FirstParameter(), last = curve.LastParameter();
            const auto circle = curve.Circle();
            const double sense = circle.Axis().Direction().Dot(axis) > 0.0 ? 1.0 : -1.0;
            const gp_Vec radial(point_at(arc.center, 0), curve.Value(first));
            double u0 = std::atan2(radial.Dot(gp_Vec(frame.YDirection())), radial.Dot(gp_Vec(start)));
            if (std::abs(u0) * arc.radius <= tol) u0 = 0.0;
            if (u0 < 0.0) u0 += full_turn;
            const double u1 = u0 + sense * (last - first);
            if (std::min(u0, u1) < -tol / arc.radius ||
                std::max(u0, u1) > lower.sweep + tol / arc.radius ||
                circle.Location().Distance(point_at(arc.center, 0)) > tol ||
                std::abs(circle.Radius() - arc.radius) > tol) {
              throw CadFailure{invalid_model, "Un arc partagé ne correspond pas au domaine de révolution."};
            }
            const double v = gp_Vec(origin, point_at(arc.center, 0)).Dot(gp_Vec(axis)) / cosine;
            Handle(Geom2d_Curve) pcurve = new Geom2d_Line(
                gp_Pnt2d(u0 - sense * first, v), gp_Dir2d(sense, 0.0));
            BRep_Builder builder;
            builder.UpdateEdge(shared, pcurve, face, tol);
            builder.Range(shared, face, first, last);
            const auto replacement = oriented.Orientation() == TopAbs_FORWARD
                ? shared : TopoDS::Edge(shared.Reversed());
            rail_replacements.Replace(edge.Oriented(TopAbs_FORWARD), replacement);
          }
          break;
        }
      }
    } else if (type == GeomAbs_Line) {
      for (size_t i = 0; i < 2; ++i) {
        const auto from = lower.vertices[i];
        const auto to = upper.vertices[i];
        const auto oriented = oriented_between(edge, point_at(points, from), point_at(points, to));
        if (!oriented.IsNull() && !rails[i]) {
          const bool traversal = (edge.Orientation() == oriented.Orientation()) != bool(cylinder.inner);
          const auto shared = wires.bind_line(from, to, oriented, traversal);
          if (!shared.IsSame(oriented)) {
            // Parameterize the common 3D line on this cylinder. Its canonical
            // parameter direction need not agree with this cylinder's axis.
            const BRepAdaptor_Curve curve(shared);
            const double first = curve.FirstParameter();
            const double last = curve.LastParameter();
            const auto begin = curve.Value(first);
            const auto finish = curve.Value(last);
            const double z0 = gp_Vec(origin, begin).Dot(gp_Vec(axis)) / cosine;
            const double z1 = gp_Vec(origin, finish).Dot(gp_Vec(axis)) / cosine;
            const double slope = (z1 - z0) / (last - first);
            if (!std::isfinite(slope) || std::abs(std::abs(slope) - 1.0) > 1.0e-10) {
              throw CadFailure{invalid_model, "La génératrice commune ne suit pas la surface de révolution."};
            }
            const double u = i == 0 ? 0.0 : lower.sweep;
            Handle(Geom2d_Curve) pcurve = new Geom2d_Line(
                gp_Pnt2d(u, z0 - slope * first), gp_Dir2d(0.0, slope));
            BRep_Builder builder;
            builder.UpdateEdge(shared, pcurve, face, tol);
            builder.Range(shared, face, first, last);
            // Replace forward geometry; ReShape restores each wire occurrence's
            // original orientation independently of the source endpoint order.
            const auto replacement = oriented.Orientation() == TopAbs_FORWARD
                ? shared : TopoDS::Edge(shared.Reversed());
            rail_replacements.Replace(edge.Oriented(TopAbs_FORWARD), replacement);
          }
          rails[i] = true;
          break;
        }
      }
    }
  }
  if (!rails[0] || !rails[1] || shared_arcs[cylinder.rings[0]].IsNull() ||
      shared_arcs[cylinder.rings[1]].IsNull()) {
    throw CadFailure{invalid_model, "Les arcs et génératrices ne rejoignent pas leurs sommets source."};
  }
  face = TopoDS::Face(rail_replacements.Apply(face));
  if (!BRepCheck_Analyzer(face, true).IsValid()) {
    throw CadFailure{invalid_model, "La paroi de révolution partagée n'est pas valide."};
  }
  return cylinder.inner ? TopoDS::Face(face.Reversed()) : face;
}

TopoDS_Wire segmented_wire(const gp_Pln& plane, const MixedSegmentData* segments, size_t count,
                           const MixedArcData* arcs, const std::vector<TopoDS_Edge>& shared_arcs,
                           IndexedWires& wires) {
  BRepBuilderAPI_MakeWire wire;
  for (size_t i = 0; i < count; ++i) {
    const auto& segment = segments[i];
    const auto from = segment.vertices[0];
    const auto to = segment.vertices[1];
    if (segment.arc == std::numeric_limits<size_t>::max()) {
      wire.Add(wires.line(plane, from, to));
    } else {
      const auto& edge = shared_arcs[segment.arc];
      require_circle_in_plane(plane, BRepAdaptor_Curve(edge).Circle());
      wire.Add(from == arcs[segment.arc].vertices[0] ? edge : TopoDS::Edge(edge.Reversed()));
    }
  }
  if (!wire.IsDone() || !BRep_Tool::IsClosed(wire.Wire())) {
    throw CadFailure{invalid_model, "Impossible de fermer un contour plan à arcs et segments."};
  }
  return wire.Wire();
}

TopoDS_Solid build_mixed_solid(
    const double* points, size_t point_count, const MixedRingData* rings, size_t ring_count,
    const double* normals, size_t plane_count, const size_t* plane_loops, const MixedLoopData* loops,
    const MixedSegmentData* segments, const MixedCylinderData* cylinders, size_t cylinder_count,
    const MixedArcData* arcs, size_t arc_count, const MixedCylinderData* partial, size_t partial_count,
    const MixedCylinderData* cones, size_t cone_count,
    const SphericalPatchData* spheres, size_t sphere_count,
    const ToroidalPatchData* tori, size_t torus_count,
    const size_t* plane_tokens = nullptr, const size_t* cylinder_tokens = nullptr,
    FaceLabels* labels = nullptr, const size_t* cone_tokens = nullptr,
    const size_t* sphere_tokens = nullptr, const size_t* torus_tokens = nullptr,
    const size_t* partial_tokens = nullptr,
    const MixedCylinderData* partial_cones = nullptr, size_t partial_cone_count = 0,
    const size_t* partial_cone_tokens = nullptr) {
  IndexedWires wires(points, point_count);
  BRepBuilderAPI_Sewing sewing(Precision::Confusion());
  sewing.SetMaxTolerance(Precision::Confusion());
  sewing.SetLocalTolerancesMode(false);
  std::vector<TopoDS_Edge> rims(ring_count);
  for (size_t i = 0; i < cylinder_count; ++i) {
    const auto face = revolved_face(cylinders[i], rings, rims, false);
    sewing.Add(face);
    if (cylinder_tokens && labels) {
      labels->push_back({face, cylinder_tokens[i]});
    }
  }
  for (size_t i = 0; i < cone_count; ++i) {
    const auto face = revolved_face(cones[i], rings, rims, true);
    sewing.Add(face);
    if (cone_tokens && labels) {
      labels->push_back({face, cone_tokens[i]});
    }
  }
  for (size_t i = 0; i < sphere_count; ++i) {
    const auto face = spherical_face(spheres[i], rings, rims);
    sewing.Add(face);
    if (sphere_tokens && labels) labels->push_back({face, sphere_tokens[i]});
  }
  for (size_t i = 0; i < torus_count; ++i) {
    const auto face = toroidal_face(tori[i], rings, rims);
    sewing.Add(face);
    if (torus_tokens && labels) labels->push_back({face, torus_tokens[i]});
  }
  std::vector<TopoDS_Edge> shared_arcs(arc_count);
  for (size_t i = 0; i < partial_count; ++i) {
    const auto face = partial_revolved_face(partial[i], arcs, points, shared_arcs, wires);
    sewing.Add(face);
    if (partial_tokens && labels) labels->push_back({face, partial_tokens[i]});
  }

  for (size_t i = 0; i < partial_cone_count; ++i) {
    const auto face = partial_revolved_face(partial_cones[i], arcs, points, shared_arcs, wires, true);
    sewing.Add(face);
    if (partial_cone_tokens && labels) labels->push_back({face, partial_cone_tokens[i]});
  }

  for (size_t i = 0; i < plane_count; ++i) {
    const size_t first_loop = plane_loops[i];
    const auto& outer = loops[first_loop];
    const gp_Pnt origin = outer.count == 0
        ? BRepAdaptor_Curve(rims[outer.start]).Circle().Location()
        : point_at(points, segments[outer.start].vertices[0]);
    const gp_Pln plane(origin, direction(normals + 3 * i));
    BRepBuilderAPI_MakeFace make_face;
    for (size_t j = first_loop; j < plane_loops[i + 1]; ++j) {
      const auto& loop = loops[j];
      const bool hole = j != first_loop;
      const TopoDS_Wire wire = loop.count == 0
          ? rim_wire(plane, rims[loop.start], hole)
          : segmented_wire(plane, segments + loop.start, loop.count, arcs, shared_arcs, wires);
      if (hole) {
        make_face.Add(wire);
      } else {
        make_face = BRepBuilderAPI_MakeFace(plane, wire, true);
      }
      if (!make_face.IsDone()) {
        throw CadFailure{invalid_model, "Impossible de construire une face plane et ses contours mixtes."};
      }
    }
    if (!BRepCheck_Analyzer(make_face.Face(), true, false, true).IsValid()) {
      throw CadFailure{invalid_model, "Les contours mixtes ne délimitent pas une face plane valide."};
    }
    sewing.Add(make_face.Face());
    if (plane_tokens && labels) {
      labels->push_back({make_face.Face(), plane_tokens[i]});
    }
  }
  const auto solid = solid_from_sewing(sewing);
  if (labels) {
    try {
      track_sewn_faces(solid, sewing, *labels);
    } catch (const Standard_Failure&) {
      labels->clear(); // Metadata failure cannot invalidate valid geometry.
    }
  }
  return solid;
}

TopoDS_Shape single_solid(TopoDS_Shape shape, Status status) {
  while (!shape.IsNull() && shape.ShapeType() == TopAbs_COMPOUND) {
    TopoDS_Iterator children(shape);
    if (!children.More()) {
      break;
    }
    const TopoDS_Shape child = children.Value();
    children.Next();
    if (children.More()) {
      throw CadFailure{status, "Le résultat contient plusieurs objets."};
    }
    shape = child;
  }
  if (shape.IsNull() || shape.ShapeType() != TopAbs_SOLID) {
    throw CadFailure{status, "Le résultat ne contient pas un solide fermé."};
  }
  return shape;
}

gp_Pnt profile_point(const gp_Ax3& frame, const double* coordinates) {
  return frame.Location().Translated(
      gp_Vec(frame.XDirection()) * coordinates[0] + gp_Vec(frame.YDirection()) * coordinates[1]);
}

TopoDS_Wire profile_wire(const gp_Ax3& frame, const double* points,
                         const ProfileLoopData& loop, bool is_hole) {
  if (loop.radius > 0.0) {
    const gp_Ax2 axes(profile_point(frame, loop.center), frame.Direction(), frame.XDirection());
    const gp_Circ circle(axes, loop.radius);
    BRepBuilderAPI_MakeEdge make_edge(circle);
    if (!make_edge.IsDone()) {
      throw CadFailure{invalid_model, "Impossible de construire une arête circulaire."};
    }
    BRepBuilderAPI_MakeWire make_wire(make_edge.Edge());
    if (!make_wire.IsDone() || !BRep_Tool::IsClosed(make_wire.Wire())) {
      throw CadFailure{invalid_model, "Impossible de fermer un contour circulaire."};
    }
    const TopoDS_Wire wire = make_wire.Wire();
    return is_hole ? TopoDS::Wire(wire.Reversed()) : wire;
  }

  BRepBuilderAPI_MakePolygon polygon;
  for (size_t i = 0; i < loop.point_count; ++i) {
    polygon.Add(profile_point(frame, points + (loop.point_start + i) * 2));
  }
  polygon.Close();
  if (!polygon.IsDone() || !BRep_Tool::IsClosed(polygon.Wire())) {
    throw CadFailure{invalid_model, "Impossible de fermer un contour polygonal."};
  }
  return polygon.Wire();
}

TopoDS_Solid build_extrusion(const double* origin, const double* axis, const double* x_axis,
                             double height, const double* points, const ProfileLoopData* loops,
                             size_t loop_count) {
  const gp_Ax3 frame(point_at(origin, 0), gp_Dir(axis[0], axis[1], axis[2]),
                     gp_Dir(x_axis[0], x_axis[1], x_axis[2]));
  BRepBuilderAPI_MakeFace cap(gp_Pln(frame), profile_wire(frame, points, loops[0], false), true);
  for (size_t i = 1; i < loop_count; ++i) {
    cap.Add(profile_wire(frame, points, loops[i], true));
  }
  if (!cap.IsDone() || !BRepCheck_Analyzer(cap.Face(), true, false, true).IsValid()) {
    throw CadFailure{invalid_model, "Les contours ne délimitent pas une face plane valide."};
  }

  BRepPrimAPI_MakePrism prism(cap.Face(), gp_Vec(frame.Direction()) * height, true, true);
  if (!prism.IsDone()) {
    throw CadFailure{invalid_solid, "Impossible de construire le solide extrudé."};
  }
  TopoDS_Solid solid = TopoDS::Solid(single_solid(prism.Shape(), invalid_solid));
  if (!BRepLib::OrientClosedSolid(solid)) {
    throw CadFailure{invalid_solid, "Impossible d'orienter le volume extrudé."};
  }
  return solid;
}

std::optional<gp_Pnt> surface_pole(const TopoDS_Face& face, const TopoDS_Edge& edge) {
  const BRepAdaptor_Surface surface(face);
  TopoDS_Vertex a, b;
  TopExp::Vertices(edge, a, b, true);
  if (a.IsNull() || b.IsNull() || !a.IsSame(b)) {
    return std::nullopt;
  }
  const auto point = BRep_Tool::Pnt(a);
  if (surface.GetType() == GeomAbs_Cone &&
      point.Distance(surface.Cone().Apex()) <= Precision::Confusion()) {
    return point;
  }
  if (surface.GetType() == GeomAbs_Sphere) {
    const auto sphere = surface.Sphere();
    const auto polar_radius = gp_Vec(sphere.Position().Direction()) * sphere.Radius();
    if (point.Distance(sphere.Location().Translated(polar_radius)) <= Precision::Confusion() ||
        point.Distance(sphere.Location().Translated(-polar_radius)) <= Precision::Confusion()) {
      return point;
    }
  }
  return std::nullopt;
}

void check_edge_incidence(const TopoDS_Shape& shape, const TopTools_IndexedMapOfShape& faces,
                          Status status) {
  struct EdgeUse {
    int count = 0;
    int face = 0;
    TopAbs_Orientation orientation = TopAbs_FORWARD;
    bool at_pole = false;
  };
  TopTools_IndexedMapOfShape edges;
  TopExp::MapShapes(shape, TopAbs_EDGE, edges);
  std::vector<EdgeUse> incidence(static_cast<size_t>(edges.Extent()) + 1);
  const CadFailure invalid{status, "Le solide comporte une arête libre ou non-manifold."};
  for (int i = 1; i <= faces.Extent(); ++i) {
    const TopoDS_Face face = TopoDS::Face(faces(i));
    for (TopExp_Explorer explorer(face, TopAbs_EDGE); explorer.More(); explorer.Next()) {
      const TopoDS_Edge edge = TopoDS::Edge(explorer.Current());
      const auto orientation = edge.Orientation();
      const int edge_index = edges.FindIndex(edge);
      if (edge_index == 0 || (orientation != TopAbs_FORWARD && orientation != TopAbs_REVERSED)) {
        throw invalid;
      }
      EdgeUse& use = incidence[edge_index];
      if (BRep_Tool::Degenerated(edge)) {
        if (use.count != 0 || !surface_pole(face, edge)) {
          throw invalid;
        }
        use = {1, i, orientation, true};
        continue;
      }
      if (use.count == 0) {
        use = {1, i, orientation};
        continue;
      }
      // A seam is valid only as two opposed uses on one face and only if
      // OCCT confirms both parametric representations on that face.
      if (use.count != 1 || use.orientation == orientation ||
          (use.face == i && !BRep_Tool::IsClosed(edge, face))) {
        throw invalid;
      }
      use.count = 2;
    }
  }
  if (std::any_of(incidence.begin() + 1, incidence.end(),
                  [](const EdgeUse& use) { return use.count != (use.at_pole ? 1 : 2); })) {
    throw invalid;
  }
}

struct RoundedBoxFrame {
  gp_Pnt center;
  std::array<gp_Dir, 3> axes;
  std::array<double, 3> half;
  double radius;
};

struct SolidMeasurement {
  CadMetrics metrics;
  SurfaceCounts surfaces;
  std::optional<RoundedBoxFrame> rounded_box;
};

bool quarter_turn(double span) {
  return std::abs(span - std::acos(-1.0) / 2.0) <= 1.0e-8;
}

constexpr double box_angle_tolerance = 1.0e-8;

RoundedBoxFrame infer_rounded_box_frame(const TopTools_IndexedMapOfShape& faces, Status status) {
  const CadFailure invalid{status, "Les six plans du boîtier ne forment pas un repère orthogonal."};
  RoundedBoxFrame frame{};
  std::optional<gp_Pnt> origin;
  std::array<std::array<double, 2>, 3> supports{};
  std::array<size_t, 3> counts{};
  size_t axis_count = 0;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const BRepAdaptor_Surface surface(TopoDS::Face(faces(i)));
    if (surface.GetType() == GeomAbs_Sphere) {
      frame.radius = surface.Sphere().Radius();
    } else if (surface.GetType() == GeomAbs_Plane) {
      const auto plane = surface.Plane();
      const auto normal = plane.Axis().Direction();
      if (!origin) {
        origin = plane.Location();
      }
      size_t axis = 0;
      for (; axis < axis_count; ++axis) {
        if (normal.IsParallel(frame.axes[axis], box_angle_tolerance)) {
          break;
        }
        if (!normal.IsNormal(frame.axes[axis], box_angle_tolerance)) {
          throw invalid;
        }
      }
      if (axis == axis_count) {
        if (axis_count == frame.axes.size()) {
          throw invalid;
        }
        frame.axes[axis_count++] = normal;
      }
      if (counts[axis] == 2) {
        throw invalid;
      }
      supports[axis][counts[axis]++] = gp_Vec(*origin, plane.Location()).Dot(gp_Vec(frame.axes[axis]));
    }
  }
  if (!origin || counts != std::array<size_t, 3>{2, 2, 2}) {
    throw invalid;
  }
  frame.center = *origin;
  for (size_t i = 0; i < 3; ++i) {
    const auto [a, b] = supports[i];
    frame.half[i] = std::abs(b - a) / 2.0 - frame.radius;
    frame.center.Translate(gp_Vec(frame.axes[i]) * ((a + b) / 2.0));
  }
  return frame;
}

void check_rounded_box_connections(const TopoDS_Shape& shape, Status status) {
  const CadFailure invalid{status, "Les 48 raccords tangents du boîtier arrondi ne concordent pas."};
  TopTools_IndexedDataMapOfShapeListOfShape connections;
  TopExp::MapShapesAndUniqueAncestors(shape, TopAbs_EDGE, TopAbs_FACE, connections);
  size_t plane_cylinder = 0;
  size_t sphere_cylinder = 0;
  for (int i = 1; i <= connections.Extent(); ++i) {
    const auto edge = TopoDS::Edge(connections.FindKey(i));
    if (BRep_Tool::Degenerated(edge)) {
      continue;
    }
    const auto& adjacent = connections.FindFromIndex(i);
    if (adjacent.Extent() != 2) {
      throw invalid;
    }
    const auto first = TopoDS::Face(adjacent.First());
    const auto second = TopoDS::Face(adjacent.Last());
    const BRepAdaptor_Surface left(first);
    const BRepAdaptor_Surface right(second);
    const auto a = left.GetType();
    const auto b = right.GetType();
    const BRepAdaptor_Curve curve(edge);
    if (((a == GeomAbs_Plane && b == GeomAbs_Cylinder) ||
         (a == GeomAbs_Cylinder && b == GeomAbs_Plane)) && curve.GetType() == GeomAbs_Line) {
      ++plane_cylinder;
    } else if (((a == GeomAbs_Sphere && b == GeomAbs_Cylinder) ||
                (a == GeomAbs_Cylinder && b == GeomAbs_Sphere)) && curve.GetType() == GeomAbs_Circle) {
      ++sphere_cylinder;
    } else {
      throw invalid;
    }
    double f1, l1, f2, l2;
    const auto p1 = BRep_Tool::CurveOnSurface(edge, first, f1, l1);
    const auto p2 = BRep_Tool::CurveOnSurface(edge, second, f2, l2);
    if (p1.IsNull() || p2.IsNull() || !BRep_Tool::SameParameter(edge)) {
      throw invalid;
    }
    const auto normal = [](const BRepAdaptor_Surface& surface, const TopoDS_Face& face, const gp_Pnt2d& uv) {
      gp_Pnt point;
      gp_Vec du, dv;
      surface.D1(uv.X(), uv.Y(), point, du, dv);
      gp_Dir n(du.Crossed(dv));
      return face.Orientation() == TopAbs_REVERSED ? n.Reversed() : n;
    };
    for (double fraction : {0.25, 0.5, 0.75}) {
      gp_Pnt2d uv1, uv2;
      p1->D0(f1 + fraction * (l1 - f1), uv1);
      p2->D0(f2 + fraction * (l2 - f2), uv2);
      const auto n1 = normal(left, first, uv1);
      const auto n2 = normal(right, second, uv2);
      if (n1.Dot(n2) < 1.0 - 1.0e-10) {
        throw invalid;
      }
    }
  }
  if (plane_cylinder != 24 || sphere_cylinder != 24) {
    throw invalid;
  }
}

void check_rounded_box_geometry(const TopoDS_Shape& shape, const TopTools_IndexedMapOfShape& faces,
                                const CadMetrics& metrics,
                                const RoundedBoxFrame& frame, Status status) {
  const CadFailure invalid{status, "Les supports, rayons ou domaines du boîtier arrondi ne concordent pas."};
  const auto [a, b, c] = frame.half;
  const double r = frame.radius;
  const double pi = std::acos(-1.0);
  const double tolerance = std::max(4.0 * Precision::Confusion(), (std::max({a, b, c}) + r) * 1.0e-9);
  const auto near = [tolerance](double actual, double expected) {
    return std::isfinite(actual) && std::abs(actual - expected) <= tolerance;
  };
  if (std::min({a, b, c, r}) <= Precision::Confusion()) {
    throw invalid;
  }
  const double volume = 8.0 * (a * b * c + r * (a * b + a * c + b * c))
      + 2.0 * pi * r * r * (a + b + c) + 4.0 * pi * r * r * r / 3.0;
  if (!std::isfinite(volume) || std::abs(metrics.volume - volume) > std::max(volume * 1.0e-8, 1.0e-8)) {
    throw invalid;
  }
  for (int i = 1; i <= 3; ++i) {
    double extent = r;
    for (size_t j = 0; j < 3; ++j) {
      extent += std::abs(frame.axes[j].Coord(i)) * frame.half[j];
    }
    if (!near(metrics.bounds[i - 1], frame.center.Coord(i) - extent) ||
        !near(metrics.bounds[i + 2], frame.center.Coord(i) + extent)) {
      throw invalid;
    }
  }
  const auto local = [&](const gp_Pnt& p) {
    const gp_Vec delta(frame.center, p);
    return std::array<double, 3>{delta.Dot(gp_Vec(frame.axes[0])),
        delta.Dot(gp_Vec(frame.axes[1])), delta.Dot(gp_Vec(frame.axes[2]))};
  };
  const auto axis_of = [&](const gp_Dir& direction) -> size_t {
    for (size_t i = 0; i < 3; ++i) {
      if (direction.IsParallel(frame.axes[i], box_angle_tolerance)) {
        return i;
      }
    }
    throw invalid;
  };
  const auto side = [&](double coordinate, double distance) -> size_t {
    if (!near(std::abs(coordinate), distance)) {
      throw invalid;
    }
    return coordinate > 0.0;
  };
  std::array<bool, 26> seen{};
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto face = TopoDS::Face(faces(i));
    const BRepAdaptor_Surface surface(face);
    size_t index;
    double area;
    switch (surface.GetType()) {
      case GeomAbs_Plane: {
        const auto plane = surface.Plane();
        const size_t axis = axis_of(plane.Axis().Direction());
        const size_t j = (axis + 1) % 3;
        const size_t k = (axis + 2) % 3;
        index = 2 * axis + side(local(plane.Location())[axis], frame.half[axis] + r);
        area = 4.0 * frame.half[j] * frame.half[k];
        break;
      }
      case GeomAbs_Cylinder: {
        const auto cylinder = surface.Cylinder();
        const size_t axis = axis_of(cylinder.Axis().Direction());
        const size_t j = (axis + 1) % 3;
        const size_t k = (axis + 2) % 3;
        const auto line = local(cylinder.Location());
        const size_t sj = side(line[j], frame.half[j]);
        const size_t sk = side(line[k], frame.half[k]);
        const auto middle = local(surface.Value(
            (surface.FirstUParameter() + surface.LastUParameter()) / 2.0,
            (surface.FirstVParameter() + surface.LastVParameter()) / 2.0));
        if (!near(cylinder.Radius(), r) ||
            !near(surface.LastVParameter() - surface.FirstVParameter(), 2.0 * frame.half[axis]) ||
            !near(middle[axis], 0.0) ||
            (sj ? middle[j] <= line[j] : middle[j] >= line[j]) ||
            (sk ? middle[k] <= line[k] : middle[k] >= line[k])) {
          throw invalid;
        }
        index = 6 + 4 * axis + 2 * sj + sk;
        area = pi * r * frame.half[axis];
        break;
      }
      case GeomAbs_Sphere: {
        const auto sphere = surface.Sphere();
        axis_of(sphere.Position().Direction());
        const auto center = local(sphere.Location());
        const auto middle = local(surface.Value(
            (surface.FirstUParameter() + surface.LastUParameter()) / 2.0,
            (surface.FirstVParameter() + surface.LastVParameter()) / 2.0));
        index = 18;
        for (size_t axis = 0; axis < 3; ++axis) {
          const size_t s = side(center[axis], frame.half[axis]);
          if (s ? middle[axis] <= center[axis] : middle[axis] >= center[axis]) {
            throw invalid;
          }
          index += s << axis;
        }
        if (!near(sphere.Radius(), r)) {
          throw invalid;
        }
        area = pi * r * r / 2.0;
        break;
      }
      default: throw invalid;
    }
    GProp_GProps properties;
    BRepGProp::SurfaceProperties(face, properties, true);
    if (seen[index] || !std::isfinite(properties.Mass()) ||
        std::abs(properties.Mass() - area) > std::max(area * 1.0e-8, 1.0e-8)) {
      throw invalid;
    }
    seen[index] = true;
  }
  check_rounded_box_connections(shape, status);
}

SolidMeasurement measure_solid(const TopoDS_Shape& shape, Status status,
                              const SurfaceCounts* expected = nullptr,
                              const RoundedBoxFrame* expected_box = nullptr) {
  TopTools_IndexedMapOfShape shells;
  TopExp::MapShapes(shape, TopAbs_SHELL, shells);
  if (shells.Extent() != 1 || !BRep_Tool::IsClosed(shells(1))) {
    throw CadFailure{status, "Le solide ne contient pas une unique coque fermée."};
  }
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  if (expected && static_cast<size_t>(faces.Extent()) != expected->planar + expected->cylindrical +
                      expected->conical + expected->spherical + expected->toroidal) {
    throw CadFailure{status, "Le nombre de faces ne correspond pas à la reconstruction."};
  }
  SurfaceCounts found{};
  size_t quarter_cylinders = 0;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const BRepAdaptor_Surface surface(TopoDS::Face(faces(i)));
    switch (surface.GetType()) {
      case GeomAbs_Plane: ++found.planar; break;
      case GeomAbs_Cylinder:
        ++found.cylindrical;
        if (surface.LastUParameter() - surface.FirstUParameter() < 2.0 * std::acos(-1.0) - 1.0e-9) {
          ++found.partial;
          quarter_cylinders += quarter_turn(surface.LastUParameter() - surface.FirstUParameter());
        }
        break;
      case GeomAbs_Cone:
        ++found.conical;
        if (surface.LastUParameter() - surface.FirstUParameter() < 2.0 * std::acos(-1.0) - 1.0e-9) {
          ++found.partial_conical;
        }
        break;
      case GeomAbs_Sphere:
        ++found.spherical;
        if (surface.LastUParameter() - surface.FirstUParameter() < 2.0 * std::acos(-1.0) - 1.0e-9) {
          if ((expected && expected->spherical_octants == 0) ||
              !quarter_turn(surface.LastUParameter() - surface.FirstUParameter()) ||
              !quarter_turn(surface.LastVParameter() - surface.FirstVParameter()) ||
              std::min(std::abs(surface.FirstVParameter()), std::abs(surface.LastVParameter())) > 1.0e-8) {
            throw CadFailure{status, "Une face sphérique partielle hors octant est hors du périmètre validé."};
          }
          ++found.spherical_octants;
        }
        break;
      case GeomAbs_Torus:
        ++found.toroidal;
        if (surface.LastUParameter() - surface.FirstUParameter() < 2.0 * std::acos(-1.0) - 1.0e-9) {
          throw CadFailure{status, "Une face torique doit couvrir un tour complet."};
        }
        break;
      default: throw CadFailure{status, "Une face du solide présente un type de surface hors périmètre."};
    }
  }
  if (found.spherical_octants != 0 &&
      (found.spherical_octants != rounded_box_surfaces.spherical_octants ||
       found.spherical != rounded_box_surfaces.spherical ||
       found.planar != rounded_box_surfaces.planar ||
       found.cylindrical != rounded_box_surfaces.cylindrical ||
       quarter_cylinders != rounded_box_surfaces.partial || found.conical != 0 || found.toroidal != 0)) {
    throw CadFailure{status, "Les octants sphériques exigent le boîtier arrondi complet à 26 faces."};
  }
  if (expected && (found.planar != expected->planar || found.cylindrical != expected->cylindrical ||
                   found.partial != expected->partial || found.conical != expected->conical ||
                   found.spherical != expected->spherical || found.toroidal != expected->toroidal ||
                   found.spherical_octants != expected->spherical_octants ||
                   found.partial_conical != expected->partial_conical)) {
    throw CadFailure{status, "Les types de surfaces diffèrent de la reconstruction."};
  }
  check_edge_incidence(shape, faces, status);
  if (!BRepCheck_Analyzer(shape, true, false, true).IsValid()) {
    throw CadFailure{status, "Le contrôle géométrique du solide a échoué."};
  }
  BRepAlgoAPI_Check interference(shape, false, true);
  if (interference.HasErrors() || !interference.IsValid()) {
    throw CadFailure{status, "Le contrôle des auto-intersections du solide a échoué."};
  }

  GProp_GProps properties;
  BRepGProp::VolumeProperties(shape, properties, true);
  CadMetrics result{1, static_cast<size_t>(faces.Extent()), found.planar, found.cylindrical,
                    found.conical, found.spherical, found.toroidal, properties.Mass(), {}};
  if (!std::isfinite(result.volume) || result.volume <= 0.0) {
    throw CadFailure{status, "Le volume du solide n'est pas strictement positif."};
  }

  Bnd_Box box;
  BRepBndLib::AddOptimal(shape, box, false, false);
  box.Get(result.bounds[0], result.bounds[1], result.bounds[2],
          result.bounds[3], result.bounds[4], result.bounds[5]);
  for (size_t i = 0; i < 3; ++i) {
    if (!std::isfinite(result.bounds[i]) || !std::isfinite(result.bounds[i + 3]) ||
        result.bounds[i] >= result.bounds[i + 3]) {
      throw CadFailure{status, "Les dimensions du solide sont invalides."};
    }
  }
  std::optional<RoundedBoxFrame> rounded_box;
  if (found.spherical_octants != 0) {
    rounded_box = expected_box ? *expected_box : infer_rounded_box_frame(faces, status);
    check_rounded_box_geometry(shape, faces, result, *rounded_box, status);
  }
  return {result, found, rounded_box};
}

CadMetrics measure(const TopoDS_Shape& shape, SurfaceCounts expected, Status status,
                   const RoundedBoxFrame* expected_box = nullptr) {
  return measure_solid(shape, status, &expected, expected_box).metrics;
}

bool same_measurement(const CadMetrics& before, const CadMetrics& after) {
  const double volume_tolerance = std::max(1.0e-9, before.volume * 1.0e-8);
  if (std::abs(before.volume - after.volume) > volume_tolerance) {
    return false;
  }
  for (size_t i = 0; i < 6; ++i) {
    const double rounding = std::abs(before.bounds[i]) * std::numeric_limits<double>::epsilon() * 8;
    if (std::abs(before.bounds[i] - after.bounds[i]) > 1.0e-6 + rounding) {
      return false;
    }
  }
  return true;
}

void set_error(char* error, size_t capacity, const char* text) noexcept {
  std::snprintf(error, capacity, "%s", text != nullptr && *text != '\0' ? text : "Erreur interne Open CASCADE.");
}

template <typename Work>
int with_occt(Status stage, char* error, size_t capacity, Work work) noexcept {
  try {
    const std::lock_guard<std::mutex> lock(occt_mutex);
    const QuietMessenger quiet;
    work(stage);
    return success;
  } catch (const CadFailure& problem) {
    set_error(error, capacity, problem.message.c_str());
    return problem.status;
  } catch (const Standard_Failure& problem) {
    set_error(error, capacity, problem.GetMessageString());
    return stage;
  } catch (const std::exception& problem) {
    set_error(error, capacity, problem.what());
  } catch (...) {
    set_error(error, capacity, "Erreur interne Open CASCADE.");
  }
  return failure;
}

void load_step(STEPControl_Reader& reader, std::istream& stream) {
  if (reader.ReadStream("FabSTEP.step", DESTEP_Parameters{}, stream) != IFSelect_RetDone) {
    throw CadFailure{read_failed, "Le fichier STEP ne peut pas être relu."};
  }
  reader.SetSystemLengthUnit(1.0);
}

TopoDS_Shape read_step(std::istream& stream) {
  STEPControl_Reader reader;
  load_step(reader, stream);
  if (reader.NbRootsForTransfer() != 1 || reader.TransferRoots() != 1) {
    throw CadFailure{read_failed, "Le STEP relu ne contient pas une racine transférable unique."};
  }
  return single_solid(reader.OneShape(), read_failed);
}

constexpr const char* face_token_prefix = "FABSTEP_SOURCE_FACE_V1_";
constexpr size_t face_token_limit = 100'000;

size_t parse_face_token(const Handle(TCollection_HAsciiString)& name) {
  if (name.IsNull()) return 0;
  const size_t prefix_size = std::strlen(face_token_prefix);
  if (name->Length() <= static_cast<int>(prefix_size) ||
      name->Length() > static_cast<int>(prefix_size + 6)) return 0;
  const std::string value = name->ToCString();
  if (value.size() <= prefix_size || value.size() > prefix_size + 6 ||
      value.compare(0, prefix_size, face_token_prefix) != 0 || value[prefix_size] == '0') return 0;
  size_t token = 0;
  for (size_t i = prefix_size; i < value.size(); ++i) {
    if (value[i] < '0' || value[i] > '9') return 0;
    token = token * 10 + static_cast<size_t>(value[i] - '0');
  }
  return token <= face_token_limit ? token : 0;
}

// The labels remain untrusted until joined to the caller's same-result sidecar.
// Missing, duplicate, orphaned or ambiguous entities invalidate the entire map.
FaceLabels read_face_labels(STEPControl_Reader& reader, const TopoDS_Shape& shape) {
  try {
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(shape, TopAbs_FACE, faces);
    if (faces.IsEmpty() || static_cast<size_t>(faces.Extent()) > face_token_limit) return {};
    const auto& model = reader.Model();
    // The provenance walk itself has a bounded amount of transfer-table work.
    if (model.IsNull() || model->NbEntities() > 5'000'000) return {};
    const auto& process = reader.WS()->TransferReader()->TransientProcess();
    TopTools_IndexedMapOfShape mapped;
    std::set<size_t> tokens;
    FaceLabels result;
    result.reserve(faces.Extent());
    for (int i = 1; i <= model->NbEntities(); ++i) {
      const auto entity = Handle(StepShape_AdvancedFace)::DownCast(model->Value(i));
      if (entity.IsNull()) continue;
      const size_t token = parse_face_token(entity->Name());
      const auto face = STEPConstruct::FindShape(process, entity);
      if (token == 0 || face.IsNull() || face.ShapeType() != TopAbs_FACE ||
          !faces.Contains(face) || mapped.Contains(face) || !tokens.insert(token).second) return {};
      mapped.Add(face);
      result.push_back({face, token});
    }
    if (result.size() != static_cast<size_t>(faces.Extent())) return {};
    return result;
  } catch (const Standard_Failure&) {
    return {};
  }
}

// Resolve every exact shape first; never leave a partially labelled document.
bool write_face_labels(STEPControl_Writer& writer, const FaceLabels& labels) {
  if (labels.empty() || labels.size() > face_token_limit) return false;
  try {
    const auto& process = writer.WS()->TransferWriter()->FinderProcess();
    std::vector<Handle(StepShape_AdvancedFace)> entities;
    std::vector<Handle(TCollection_HAsciiString)> names;
    std::set<const Standard_Transient*> unique_entities;
    std::set<size_t> tokens;
    for (const auto& entry : labels) {
      const auto entity = Handle(StepShape_AdvancedFace)::DownCast(
          STEPConstruct::FindEntity(process, entry.first));
      if (entity.IsNull() || !unique_entities.insert(entity.get()).second ||
          entry.second == 0 || entry.second > face_token_limit ||
          !tokens.insert(entry.second).second) return false;
      entities.push_back(entity);
      names.push_back(new TCollection_HAsciiString(
          (std::string(face_token_prefix) + std::to_string(entry.second)).c_str()));
    }
    for (size_t i = 0; i < entities.size(); ++i) {
      entities[i]->SetName(names[i]);
    }
    return true;
  } catch (const Standard_Failure&) {
    // A STEP label is optional metadata. Geometry remains independently checked.
    return false;
  }
}

constexpr size_t step_document_limit = 128 * 1024 * 1024;

// Only the bytes are borrowed. The parser has no put area and cannot write
// to the Rust slice, despite std::streambuf requiring char* for its get area.
class StepInputBuffer : public std::streambuf {
public:
  StepInputBuffer(const unsigned char* bytes, size_t size) {
    auto* begin = const_cast<char*>(reinterpret_cast<const char*>(bytes));
    setg(begin, begin, begin + size);
  }

protected:
  pos_type seekoff(off_type offset, std::ios::seekdir direction,
                   std::ios::openmode mode) override {
    if ((mode & std::ios::in) == 0) {
      return pos_type(-1);
    }
    const auto size = egptr() - eback();
    const auto base = direction == std::ios::end ? size :
                      direction == std::ios::cur ? gptr() - eback() : 0;
    if (offset < -base || offset > size - base) {
      return pos_type(-1);
    }
    setg(eback(), eback() + base + offset, egptr());
    return pos_type(base + offset);
  }

  pos_type seekpos(pos_type position, std::ios::openmode mode) override {
    return seekoff(off_type(position), std::ios::beg, mode);
  }
};

class StepOutputBuffer : public std::streambuf {
public:
  const unsigned char* data() const { return bytes.get(); }
  size_t size() const { return written; }
  bool limit_reached() const { return exceeded; }
  unsigned char* release() { return bytes.release(); }

protected:
  std::streamsize xsputn(const char* source, std::streamsize count) override {
    if (count < 0 || static_cast<size_t>(count) > step_document_limit - written) {
      exceeded = true;
      return 0;
    }
    if (count == 0) {
      return 0;
    }
    const auto required = written + static_cast<size_t>(count);
    if (required > capacity) {
      capacity = std::min(step_document_limit, std::max({required, capacity * 2, size_t{4096}}));
      std::unique_ptr<unsigned char[]> next(new unsigned char[capacity]);
      if (written != 0) {
        std::memcpy(next.get(), bytes.get(), written);
      }
      bytes = std::move(next);
    }
    std::memcpy(bytes.get() + written, source, static_cast<size_t>(count));
    written = required;
    return count;
  }

  int_type overflow(int_type value) override {
    if (traits_type::eq_int_type(value, traits_type::eof())) {
      return traits_type::not_eof(value);
    }
    const char byte = traits_type::to_char_type(value);
    return xsputn(&byte, 1) == 1 ? value : traits_type::eof();
  }

private:
  std::unique_ptr<unsigned char[]> bytes;
  size_t written = 0;
  size_t capacity = 0;
  bool exceeded = false;
};

struct Body {
  TopoDS_Shape shape;
  SolidMeasurement report;
};

std::string body_pair(const StepPartInput* parts, size_t first, size_t second) {
  if (!parts) {
    return "Corps " + std::to_string(first + 1) + " et corps " + std::to_string(second + 1);
  }
  return "Corps " + std::to_string(first + 1) + " « " + parts[first].name + " » et corps " +
         std::to_string(second + 1) + " « " + parts[second].name + " »";
}

void check_bodies_separated(const std::vector<Body>& bodies, const StepPartInput* parts) {
  constexpr size_t max_face_pairs = 10'000'000;
  size_t remaining = max_face_pairs;
  const auto tol = Precision::Confusion();
  for (size_t i = 0; i < bodies.size(); ++i) {
    const auto& first = bodies[i];
    for (size_t j = i + 1; j < bodies.size(); ++j) {
      const auto& second = bodies[j];
      const auto& a = first.report.metrics;
      const auto& b = second.report.metrics;
      bool boxes_separated = false;
      for (size_t axis = 0; axis < 3; ++axis) {
        if (a.bounds[axis] - b.bounds[axis + 3] > tol ||
            b.bounds[axis] - a.bounds[axis + 3] > tol) {
          boxes_separated = true;
          break;
        }
      }
      if (boxes_separated) {
        continue;
      }
      // Bound expensive exact tests after the cheap box filter. This is an
      // admission limit based on face counts, not a native runtime guarantee.
      if (a.faces > remaining / b.faces) {
        throw CadFailure{separation_budget, body_pair(parts, i, j) +
                         " : budget de vérification de séparation dépassé."};
      }
      remaining -= a.faces * b.faces;
      BRepExtrema_DistShapeShape distance(first.shape, second.shape);
      if (!distance.IsDone() || !std::isfinite(distance.Value()) || distance.InnerSolution() ||
          distance.Value() <= tol) {
        throw CadFailure{bodies_not_separated, body_pair(parts, i, j) +
                         " : la séparation des solides ne peut pas être établie (contact, recouvrement ou imbrication possibles)."};
      }
    }
  }
}

void require_product_name(STEPControl_Reader& reader, int root_index, const char* name) {
  const auto root = Handle(StepBasic_ProductDefinition)::DownCast(reader.RootForTransfer(root_index));
  if (!root.IsNull() && !root->Formation().IsNull() && !root->Formation()->OfProduct().IsNull()) {
    const auto product_name = root->Formation()->OfProduct()->Name();
    if (!product_name.IsNull()) {
      const std::string actual = product_name->ToCString();
      const auto prefix = std::string(name) + " ";
      // OCCT's optional numeric suffix is process-wide, not the root index.
      if (actual == name ||
          (actual.size() > prefix.size() && actual.compare(0, prefix.size(), prefix) == 0 &&
           std::all_of(actual.begin() + prefix.size(), actual.end(),
                       [](char c) { return c >= '0' && c <= '9'; }))) {
        return;
      }
    }
  }
  throw CadFailure{mismatch, "Le nom d'un corps a changé après réimport du STEP."};
}

void combine(const StepPartInput* parts, size_t count, CadMetrics* body_reports, StepOutput* output,
             Status& stage, bool preserve_labels = false) {
  std::vector<Body> before;
  FaceLabels labels;
  bool complete_labels = preserve_labels;
  before.reserve(count);
  for (size_t i = 0; i < count; ++i) {
    stage = read_failed;
    StepInputBuffer buffer(parts[i].bytes, parts[i].size);
    std::istream stream(&buffer);
    STEPControl_Reader input_reader;
    load_step(input_reader, stream);
    if (input_reader.NbRootsForTransfer() != 1 || input_reader.TransferRoots() != 1) {
      throw CadFailure{read_failed, "Le STEP relu ne contient pas une racine transférable unique."};
    }
    auto shape = single_solid(input_reader.OneShape(), read_failed);
    if (complete_labels) {
      auto part_labels = read_face_labels(input_reader, shape);
      if (part_labels.empty() || labels.size() + part_labels.size() > face_token_limit) {
        complete_labels = false;
        labels.clear();
      } else {
        labels.insert(labels.end(), part_labels.begin(), part_labels.end());
      }
    }
    stage = invalid_solid;
    auto report = measure_solid(shape, stage);
    before.push_back({std::move(shape), report});
  }
  check_bodies_separated(before, parts);

  stage = write_failed;
  STEPControl_Writer writer;
  DESTEP_Parameters parameters;
  parameters.WriteAssembly = DESTEP_Parameters::WriteMode_Assembly_Off;
  parameters.WriteUnit = UnitsMethods_LengthUnit_Millimeter;
  parameters.WriteTessellated = DESTEP_Parameters::RWMode_Tessellated_Off;
  for (size_t i = 0; i < count; ++i) {
    parameters.WriteProductName = parts[i].name;
    if (writer.Transfer(before[i].shape, STEPControl_ManifoldSolidBrep, parameters) != IFSelect_RetDone) {
      throw CadFailure{stage, "Impossible d'écrire un corps dans le document STEP."};
    }
  }
  if (complete_labels) write_face_labels(writer, labels);
  StepOutputBuffer output_buffer;
  std::ostream output_stream(&output_buffer);
  const auto write_status = writer.WriteStream(output_stream);
  if (output_buffer.limit_reached()) {
    throw CadFailure{document_too_large, "Le document STEP exporté dépasse 128 Mio."};
  }
  if (write_status != IFSelect_RetDone || !output_stream.good() || output_buffer.size() == 0) {
    throw CadFailure{stage, "Impossible de sérialiser le document STEP."};
  }

  stage = read_failed;
  StepInputBuffer read_buffer(output_buffer.data(), output_buffer.size());
  std::istream input_stream(&read_buffer);
  STEPControl_Reader reader;
  load_step(reader, input_stream);
  if (reader.NbRootsForTransfer() != static_cast<int>(count)) {
    throw CadFailure{stage, "Le nombre de racines a changé après réimport du STEP."};
  }
  std::vector<Body> after;
  after.reserve(count);
  for (size_t i = 0; i < count; ++i) {
    const auto root = static_cast<int>(i + 1);
    require_product_name(reader, root, parts[i].name);
    if (!reader.TransferOneRoot(root) || reader.NbShapes() != root) {
      throw CadFailure{stage, "Une racine STEP n'a pas produit exactement un corps."};
    }
    auto shape = single_solid(reader.Shape(root), stage);
    const auto& expected_box = before[i].report.rounded_box;
    auto report = measure_solid(shape, stage, &before[i].report.surfaces,
                                expected_box ? &*expected_box : nullptr);
    if (!same_measurement(before[i].report.metrics, report.metrics)) {
      throw CadFailure{mismatch, "Les dimensions, la position ou le volume d'un corps changent au réimport."};
    }
    after.push_back({std::move(shape), report});
  }
  check_bodies_separated(after, parts);

  auto total = after.front().report.metrics;
  for (size_t i = 0; i < count; ++i) {
    const auto& m = after[i].report.metrics;
    body_reports[i] = m;
    if (i == 0) {
      continue;
    }
    total.solids += m.solids;
    total.faces += m.faces;
    total.planar_faces += m.planar_faces;
    total.cylindrical_faces += m.cylindrical_faces;
    total.conical_faces += m.conical_faces;
    total.spherical_faces += m.spherical_faces;
    total.toroidal_faces += m.toroidal_faces;
    total.volume += m.volume;
    for (size_t axis = 0; axis < 3; ++axis) {
      total.bounds[axis] = std::min(total.bounds[axis], m.bounds[axis]);
      total.bounds[axis + 3] = std::max(total.bounds[axis + 3], m.bounds[axis + 3]);
    }
  }
  if (!std::isfinite(total.volume)) {
    throw CadFailure{mismatch, "Le volume cumulé des corps dépasse la plage numérique."};
  }
  *output = {output_buffer.release(), output_buffer.size(), total};
}

constexpr size_t mesh_triangle_limit = 100'000;
constexpr double mesh_angle = 0.35;

double mesh_angular_step(double radius, double deflection) {
  const double step = 4.0 * std::asin(std::sqrt(std::min(0.5 * deflection / radius, 1.0)));
  return std::min(step, mesh_angle);
}

void check_meshing_resolution(const TopoDS_Shape& shape, double deflection) {
  const CadFailure too_complex{mesh_too_complex, "La résolution demandée dépasse le budget de tessellation."};
  TopTools_IndexedMapOfShape edges;
  TopExp::MapShapes(shape, TopAbs_EDGE, edges);
  size_t estimated_segments = 0;
  for (int i = 1; i <= edges.Extent(); ++i) {
    const auto edge = TopoDS::Edge(edges(i));
    if (BRep_Tool::Degenerated(edge)) {
      continue;
    }
    const BRepAdaptor_Curve curve(edge);
    double segments = 1.0;
    if (curve.GetType() == GeomAbs_Circle) {
      segments = std::ceil(std::abs(curve.LastParameter() - curve.FirstParameter()) /
                           mesh_angular_step(curve.Circle().Radius(), deflection));
    } else if (curve.GetType() != GeomAbs_Line) {
      throw CadFailure{mesh_failed, "La tessellation attend uniquement des arêtes linéaires ou circulaires."};
    }
    // Preliminary admission guard; OCCT may refine further. Extracted
    // triangles are counted separately and subject to the same hard limit.
    if (!std::isfinite(segments) || segments > mesh_triangle_limit - estimated_segments) {
      throw too_complex;
    }
    estimated_segments += static_cast<size_t>(segments);
  }
  // Curvature in two directions can exhaust the mesher even when the
  // boundary curves alone are inexpensive. Bound these grids before OCCT.
  size_t estimated_curved_triangles = 0;
  for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
    const BRepAdaptor_Surface surface(TopoDS::Face(it.Current()));
    double major, minor;
    if (surface.GetType() == GeomAbs_Sphere) {
      major = minor = surface.Sphere().Radius();
    } else if (surface.GetType() == GeomAbs_Torus) {
      const auto torus = surface.Torus();
      minor = torus.MinorRadius();
      major = torus.MajorRadius() + minor;
    } else {
      continue;
    }
    const double u = std::ceil((surface.LastUParameter() - surface.FirstUParameter()) /
                               mesh_angular_step(major, deflection));
    const double v = std::ceil((surface.LastVParameter() - surface.FirstVParameter()) /
                               mesh_angular_step(minor, deflection));
    const double triangles = 2.0 * u * v;
    if (!std::isfinite(triangles) || u <= 0.0 || v <= 0.0 ||
        triangles > mesh_triangle_limit - estimated_curved_triangles) {
      throw too_complex;
    }
    estimated_curved_triangles += static_cast<size_t>(triangles);
  }
}

std::vector<int> complete_torus_seam_nodes(const TopoDS_Face& face,
    const Handle(Poly_Triangulation)& triangulation, const TopLoc_Location& location) {
  const BRepAdaptor_Surface surface(face);
  const auto full_period = [](double first, double last) {
    return std::abs(last - first - 2.0 * std::acos(-1.0)) <= Precision::PConfusion();
  };
  if (surface.GetType() != GeomAbs_Torus ||
      !full_period(surface.FirstUParameter(), surface.LastUParameter()) ||
      !full_period(surface.FirstVParameter(), surface.LastVParameter())) {
    return {};
  }
  const CadFailure bad_seam{mesh_failed, "Les deux coutures de la tessellation torique sont incohérentes."};
  TopTools_IndexedMapOfShape edges;
  TopExp::MapShapes(face, TopAbs_EDGE, edges);
  if (edges.Extent() != 2) {
    throw bad_seam;
  }
  const int nodes = triangulation->NbNodes();
  std::vector<int> parents(static_cast<size_t>(nodes) + 1);
  std::iota(parents.begin(), parents.end(), 0);
  const auto root = [&](int index) {
    while (index != parents[index]) {
      parents[index] = parents[parents[index]];
      index = parents[index];
    }
    return index;
  };
  for (int e = 1; e <= edges.Extent(); ++e) {
    const auto edge = TopoDS::Edge(edges(e));
    if (!BRep_Tool::IsClosed(edge, face) || !BRep_Tool::IsClosed(edge, triangulation, location)) {
      throw bad_seam;
    }
    const auto& first = BRep_Tool::PolygonOnTriangulation(
        TopoDS::Edge(edge.Oriented(TopAbs_FORWARD)), triangulation, location);
    const auto& second = BRep_Tool::PolygonOnTriangulation(
        TopoDS::Edge(edge.Oriented(TopAbs_REVERSED)), triangulation, location);
    if (first.IsNull() || second.IsNull() || first->NbNodes() != second->NbNodes() ||
        first->NbNodes() < 3 || first->NbNodes() > nodes) {
      throw bad_seam;
    }
    for (int i = 1; i <= first->NbNodes(); ++i) {
      const int a = first->Node(i);
      const int b = second->Node(i);
      if (std::min(a, b) < 1 || std::max(a, b) > nodes ||
          triangulation->Node(a).Distance(triangulation->Node(b)) > Precision::Confusion()) {
        throw bad_seam;
      }
      const int ra = root(a);
      const int rb = root(b);
      parents[std::max(ra, rb)] = std::min(ra, rb);
    }
  }
  for (int i = 1; i <= nodes; ++i) {
    parents[i] = parents[parents[i]];
  }
  return parents;
}

void tessellate(const TopoDS_Shape& shape, double deflection, MeshOutput* output,
                std::vector<ResultFaceData>* faces = nullptr) {
  check_meshing_resolution(shape, deflection);
  const BRepMesh_IncrementalMesh mesh(shape, deflection, false, mesh_angle, false);
  constexpr int failure_flags = IMeshData_OpenWire | IMeshData_SelfIntersectingWire |
                                IMeshData_Failure | IMeshData_UnorientedWire |
                                IMeshData_TooFewPoints | IMeshData_UserBreak;
  if (!mesh.IsDone() || (mesh.GetStatusFlags() & failure_flags) != 0) {
    throw CadFailure{mesh_failed, "La tessellation du solide STEP a échoué."};
  }

  std::vector<double> points;
  std::vector<size_t> triangles;
  const auto unused = std::numeric_limits<size_t>::max();
  struct Pole {
    gp_Pnt point;
    size_t index = std::numeric_limits<size_t>::max();
    bool covered = false;
  };
  size_t face_index = 0;
  for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next(), ++face_index) {
    const size_t triangle_start = triangles.size() / 3;
    const TopoDS_Face face = TopoDS::Face(it.Current());
    std::array<Pole, 2> poles;
    size_t pole_count = 0;
    for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
      const auto edge = TopoDS::Edge(e.Current());
      if (!BRep_Tool::Degenerated(edge)) {
        continue;
      }
      const auto pole = surface_pole(face, edge);
      if (!pole || pole_count == poles.size() ||
          (pole_count > 0 && poles[0].point.Distance(*pole) <= Precision::Confusion())) {
        throw CadFailure{mesh_failed, "La tessellation comporte une singularité de surface non admise."};
      }
      poles[pole_count++].point = *pole;
    }
    TopLoc_Location location;
    const Handle(Poly_Triangulation)& triangulation = BRep_Tool::Triangulation(face, location);
    if (triangulation.IsNull() || triangulation->NbTriangles() <= 0 ||
        triangulation->NbNodes() <= 0) {
      throw CadFailure{mesh_failed, "Une face du STEP n'a pas de triangulation."};
    }
    const size_t offset = points.size() / 3;
    if (static_cast<size_t>(triangulation->NbTriangles()) > mesh_triangle_limit - triangles.size() / 3 ||
        static_cast<size_t>(triangulation->NbNodes()) > 3 * mesh_triangle_limit - offset) {
      throw CadFailure{mesh_too_complex, "La tessellation dépasse 100 000 triangles."};
    }
    const gp_Trsf transformation = location.Transformation();
    const auto seam_nodes = complete_torus_seam_nodes(face, triangulation, location);
    const bool remap = pole_count != 0 || !seam_nodes.empty();
    std::vector<size_t> node_indices;
    if (remap) {
      node_indices.reserve(static_cast<size_t>(triangulation->NbNodes()));
    }
    for (int i = 1; i <= triangulation->NbNodes(); ++i) {
      gp_Pnt point = triangulation->Node(i).Transformed(transformation);
      if (!std::isfinite(point.X()) || !std::isfinite(point.Y()) || !std::isfinite(point.Z())) {
        throw CadFailure{mesh_failed, "La tessellation contient un sommet non fini."};
      }
      if (!seam_nodes.empty() && seam_nodes[i] != i) {
        node_indices.push_back(node_indices[static_cast<size_t>(seam_nodes[i] - 1)]);
        continue;
      }
      if (remap) {
        const auto point_index = points.size() / 3;
        size_t index = point_index;
        for (size_t j = 0; j < pole_count; ++j) {
          auto& pole = poles[j];
          if (point.Distance(pole.point) <= Precision::Confusion()) {
            // Periodic UV samples may share the same BRep pole. Keep one
            // audit vertex, with no change to the STEP or source.
            if (pole.index == unused) {
              pole.index = point_index;
            }
            index = pole.index;
            point = pole.point;
            break;
          }
        }
        node_indices.push_back(index);
        if (index != point_index) {
          continue;
        }
      }
      points.insert(points.end(), {point.X(), point.Y(), point.Z()});
    }
    const auto node_index = [&](int node) {
      const auto index = static_cast<size_t>(node - 1);
      return remap ? node_indices[index] : offset + index;
    };
    const bool reversed = (face.Orientation() == TopAbs_REVERSED) != transformation.IsNegative();
    for (int i = 1; i <= triangulation->NbTriangles(); ++i) {
      int a, b, c;
      triangulation->Triangle(i).Get(a, b, c);
      if (reversed) {
        std::swap(b, c);
      }
      const int last = triangulation->NbNodes();
      if (std::min({a, b, c}) < 1 || std::max({a, b, c}) > last) {
        throw CadFailure{mesh_failed, "La tessellation référence un sommet absent."};
      }
      const auto ia = node_index(a);
      const auto ib = node_index(b);
      const auto ic = node_index(c);
      std::array<int, 2> pole_uses{};
      bool collapsed_pole = false;
      for (size_t j = 0; j < pole_count; ++j) {
        const auto& pole = poles[j];
        const int uses = (ia == pole.index) + (ib == pole.index) + (ic == pole.index);
        pole_uses[j] = uses;
        collapsed_pole = collapsed_pole || uses > 1;
      }
      if (collapsed_pole) {
        continue;
      }
      const gp_Pnt p = point_at(points.data(), ia);
      const gp_Pnt q = point_at(points.data(), ib);
      const gp_Pnt r = point_at(points.data(), ic);
      const double area2 = gp_Vec(p, q).Crossed(gp_Vec(p, r)).Magnitude();
      if (!std::isfinite(area2) || area2 <= 0.0) {
        throw CadFailure{mesh_failed, "La tessellation contient un triangle dégénéré."};
      }
      triangles.insert(triangles.end(), {ia, ib, ic});
      for (size_t j = 0; j < pole_count; ++j) {
        poles[j].covered = poles[j].covered || pole_uses[j] == 1;
      }
    }
    for (size_t j = 0; j < pole_count; ++j) {
      if (!poles[j].covered) {
        throw CadFailure{mesh_failed, "La tessellation ne couvre pas un pôle de surface."};
      }
    }
    if (faces) {
      auto& metadata = faces->at(face_index);
      metadata.triangle_start = triangle_start;
      metadata.triangle_count = triangles.size() / 3 - triangle_start;
      if (metadata.triangle_count == 0) {
        throw CadFailure{mesh_failed, "Une face STEP ne contient aucun triangle non dégénéré."};
      }
    }
  }
  if (triangles.empty()) {
    throw CadFailure{mesh_failed, "La tessellation du STEP est vide."};
  }
  auto point_data = std::make_unique<double[]>(points.size());
  auto triangle_data = std::make_unique<size_t[]>(triangles.size());
  std::copy(points.begin(), points.end(), point_data.get());
  std::copy(triangles.begin(), triangles.end(), triangle_data.get());
  *output = {point_data.release(), points.size() / 3,
             triangle_data.release(), triangles.size() / 3};
}

// Shared traversal is the identity contract for display and exact profiles.
void read_result_topology(STEPControl_Reader& reader, TopoDS_Compound& document,
                          TopTools_IndexedMapOfShape& face_map,
                          std::vector<ResultFaceData>& faces,
                          std::vector<TopoDS_Face>& native_faces) {
  const int root_count = reader.NbRootsForTransfer();
  if (root_count < 1 || root_count > 32) {
    throw CadFailure{read_failed, "L'inspection STEP attend de 1 à 32 racines solides."};
  }
  BRep_Builder builder;
  builder.MakeCompound(document);
  for (int root = 1; root <= root_count; ++root) {
    if (!reader.TransferOneRoot(root) || reader.NbShapes() != root) {
      throw CadFailure{read_failed, "Une racine STEP ne produit pas un solide unique."};
    }
    const auto solid = single_solid(reader.Shape(root), read_failed);
    builder.Add(document, solid);
    for (TopExp_Explorer it(solid, TopAbs_FACE); it.More(); it.Next()) {
      if (faces.size() >= mesh_triangle_limit) {
        throw CadFailure{mesh_too_complex, "L'inspection dépasse 100 000 faces STEP."};
      }
      const auto face = TopoDS::Face(it.Current());
      if (face_map.Contains(face)) {
        throw CadFailure{mesh_failed, "Une face STEP est partagée par plusieurs solides."};
      }
      face_map.Add(face);
      native_faces.push_back(face);
      size_t family = 5;
      switch (BRepAdaptor_Surface(face).GetType()) {
        case GeomAbs_Plane: family = 0; break;
        case GeomAbs_Cylinder: family = 1; break;
        case GeomAbs_Cone: family = 2; break;
        case GeomAbs_Sphere: family = 3; break;
        case GeomAbs_Torus: family = 4; break;
        default: break;
      }
      faces.push_back({static_cast<size_t>(root - 1), family, 0, 0, 0, 0, 0});
    }
  }
}

// IDs identify only this STEP read. Optional labels are recovered through the
// exact transfer table and need the caller's trusted source sidecar.
void inspect_result_faces(const unsigned char* bytes, size_t size, double deflection,
                          ResultFacesOutput* output) {
  StepInputBuffer buffer(bytes, size);
  std::istream stream(&buffer);
  STEPControl_Reader reader;
  load_step(reader, stream);
  TopoDS_Compound document;
  TopTools_IndexedMapOfShape face_map;
  std::vector<ResultFaceData> faces;
  std::vector<TopoDS_Face> native_faces;
  read_result_topology(reader, document, face_map, faces, native_faces);
  for (const auto& entry : read_face_labels(reader, document)) {
    const auto index = face_map.FindIndex(entry.first);
    if (index > 0) faces[static_cast<size_t>(index - 1)].source_face_token = entry.second;
  }
  constexpr size_t incidence_limit = 600'000;
  const auto absent = std::numeric_limits<size_t>::max();
  TopTools_IndexedMapOfShape edge_map;
  std::vector<std::array<size_t, 2>> edge_faces;
  size_t incidences = 0;
  for (size_t id = 0; id < faces.size(); ++id) {
    for (TopExp_Explorer it(native_faces[id], TopAbs_EDGE); it.More(); it.Next()) {
      if (++incidences > incidence_limit) {
        throw CadFailure{mesh_too_complex, "L'inspection dépasse le budget d'incidences STEP."};
      }
      const auto edge = TopoDS::Edge(it.Current());
      if (BRep_Tool::Degenerated(edge)) {
        continue; // A pole is not an edge between two faces.
      }
      const auto edge_id = static_cast<size_t>(edge_map.Add(edge) - 1);
      if (edge_id == edge_faces.size()) {
        edge_faces.push_back({id, absent});
      } else {
        auto& pair = edge_faces[edge_id];
        if (pair[0] == id || pair[1] == id) {
          continue; // Periodic seam: never self-adjacency.
        }
        if (pair[1] != absent || faces[pair[0]].solid_id != faces[id].solid_id) {
          throw CadFailure{mesh_failed, "Une arête STEP a des incidences incompatibles."};
        }
        pair[1] = id;
      }
    }
  }
  std::vector<std::vector<size_t>> adjacent(faces.size());
  for (const auto& pair : edge_faces) {
    if (pair[1] != absent) {
      adjacent[pair[0]].push_back(pair[1]);
      adjacent[pair[1]].push_back(pair[0]);
    }
  }
  std::vector<size_t> neighbors;
  for (size_t id = 0; id < faces.size(); ++id) {
    auto& row = adjacent[id];
    std::sort(row.begin(), row.end());
    row.erase(std::unique(row.begin(), row.end()), row.end());
    faces[id].neighbor_start = neighbors.size();
    faces[id].neighbor_count = row.size();
    neighbors.insert(neighbors.end(), row.begin(), row.end());
  }
  auto face_data = std::make_unique<ResultFaceData[]>(faces.size());
  auto neighbor_data = std::make_unique<size_t[]>(neighbors.size());
  std::copy(neighbors.begin(), neighbors.end(), neighbor_data.get());
  // All metadata allocations precede the only ownership transfer to output.
  tessellate(document, deflection, &output->mesh, &faces);
  std::copy(faces.begin(), faces.end(), face_data.get());
  output->faces = face_data.release();
  output->face_count = faces.size();
  output->neighbors = neighbor_data.release();
  output->neighbor_count = neighbors.size();
}

// Shared strict boundary conversion for native faces and section wires.
void append_profile_wire(const TopoDS_Wire& wire, const gp_Pln& plane,
                         const gp_Dir& x_axis, const gp_Dir& y_axis, const gp_Dir& normal,
                         std::vector<PlanarProfileSegmentData>& segments, size_t& total_edges,
                         const TopoDS_Face* face_context = nullptr) {
  constexpr size_t edge_limit = 100'000;
  constexpr double tolerance = 1e-7;
  constexpr double tau = 6.283185307179586476925286766559;
  const gp_Pnt origin = plane.Location();
  auto project = [&](const gp_Pnt& p, double* result) {
    if (!std::isfinite(p.X()) || !std::isfinite(p.Y()) || !std::isfinite(p.Z()) ||
        plane.Distance(p) > tolerance) {
      throw CadFailure{profile_invalid, "Une arête native sort du plan de la face."};
    }
    const gp_Vec delta(origin, p);
    result[0] = delta.Dot(gp_Vec(x_axis));
    result[1] = delta.Dot(gp_Vec(y_axis));
  };
  TopTools_IndexedMapOfShape expected;
  size_t edge_count = 0;
  for (TopoDS_Iterator edges(wire); edges.More(); edges.Next()) {
    if (++total_edges > edge_limit) {
      throw CadFailure{profile_too_complex, "Le profil dépasse 100 000 arêtes."};
    }
    if (edges.Value().ShapeType() != TopAbs_EDGE || expected.Contains(edges.Value())) {
      throw CadFailure{profile_invalid, "Un contour contient une arête répétée ou invalide."};
    }
    expected.Add(edges.Value());
    ++edge_count;
  }
  if (edge_count == 0 || !BRep_Tool::IsClosed(wire)) {
    throw CadFailure{profile_invalid, "Un contour natif est vide ou ouvert."};
  }
  TopTools_IndexedMapOfShape visited;
  TopoDS_Vertex first_vertex, previous_vertex;
  gp_Pnt first_point, previous_point;
  BRepTools_WireExplorer ordered;
  if (face_context) ordered.Init(wire, *face_context); else ordered.Init(wire);
  for (auto& edges = ordered; edges.More(); edges.Next()) {
    const auto edge = edges.Current();
    if (visited.Extent() >= static_cast<int>(edge_count) || !expected.Contains(edge) ||
        visited.Contains(edge) || BRep_Tool::Degenerated(edge) ||
        (edge.Orientation() != TopAbs_FORWARD && edge.Orientation() != TopAbs_REVERSED)) {
      throw CadFailure{profile_invalid, "Le parcours d'un contour natif est incohérent."};
    }
    visited.Add(edge);
    TopoDS_Vertex start_vertex, end_vertex;
    TopExp::Vertices(edge, start_vertex, end_vertex, true);
    if (start_vertex.IsNull() || end_vertex.IsNull() ||
        (!previous_vertex.IsNull() && !previous_vertex.IsSame(start_vertex))) {
      throw CadFailure{profile_invalid, "Les arêtes du contour ne partagent pas leurs sommets."};
    }
    const BRepAdaptor_Curve curve(edge);
    double first = curve.FirstParameter(), last = curve.LastParameter();
    if (!std::isfinite(first) || !std::isfinite(last) || last <= first) {
      throw CadFailure{profile_invalid, "Le domaine d'une arête native est invalide."};
    }
    if (edge.Orientation() == TopAbs_REVERSED) std::swap(first, last);
    const gp_Pnt start = curve.Value(first), end = curve.Value(last);
    if (start.Distance(BRep_Tool::Pnt(start_vertex)) > tolerance ||
        end.Distance(BRep_Tool::Pnt(end_vertex)) > tolerance ||
        (!previous_vertex.IsNull() && previous_point.Distance(start) > tolerance)) {
      throw CadFailure{profile_invalid, "Un raccord du contour dépasse la tolérance native."};
    }
    PlanarProfileSegmentData segment{};
    project(start, segment.start);
    project(end, segment.end);
    if (curve.GetType() == GeomAbs_Line) {
      if (start.Distance(end) <= tolerance) {
        throw CadFailure{profile_invalid, "Une ligne native est dégénérée."};
      }
      segment.kind = 0;
    } else if (curve.GetType() == GeomAbs_Circle) {
      const gp_Circ circle = curve.Circle();
      project(circle.Location(), segment.center);
      const double alignment = circle.Axis().Direction().Dot(normal);
      if (std::abs(std::abs(alignment) - 1.) > 1e-12 ||
          !std::isfinite(circle.Radius()) || circle.Radius() <= tolerance) {
        throw CadFailure{profile_invalid, "Un cercle natif n'est pas dans le plan de la face."};
      }
      segment.radius = circle.Radius();
      segment.start_angle = std::atan2(segment.start[1] - segment.center[1],
                                      segment.start[0] - segment.center[0]);
      segment.sweep_angle = (last - first) * (alignment > 0. ? 1. : -1.);
      const double sweep = std::abs(segment.sweep_angle);
      if (start_vertex.IsSame(end_vertex) && std::abs(sweep - tau) <= 1e-12) {
        if (edge_count != 1 || start.Distance(end) > tolerance) {
          throw CadFailure{profile_invalid, "Un cercle fermé n'est pas une boucle indépendante."};
        }
        segment.kind = 2;
      } else {
        if (sweep <= 1e-12 || sweep >= tau) {
          throw CadFailure{profile_invalid, "Le balayage d'un arc natif est invalide."};
        }
        segment.kind = 1;
      }
    } else if (curve.GetType() == GeomAbs_Ellipse) {
      const gp_Elips ellipse = curve.Ellipse();
      project(ellipse.Location(), segment.center);
      const double alignment = ellipse.Axis().Direction().Dot(normal);
      const double major = ellipse.MajorRadius(), minor = ellipse.MinorRadius();
      if (std::abs(std::abs(alignment) - 1.) > 1e-12 || !std::isfinite(major) ||
          !std::isfinite(minor) || minor <= tolerance || major < minor) {
        throw CadFailure{profile_invalid, "Une ellipse native est dégénérée ou sort du plan du profil."};
      }
      const gp_Vec major_vector = gp_Vec(ellipse.Position().XDirection()) * major;
      segment.major_axis[0] = major_vector.Dot(gp_Vec(x_axis));
      segment.major_axis[1] = major_vector.Dot(gp_Vec(y_axis));
      segment.ratio = minor / major;
      const double sense = alignment > 0. ? 1. : -1.;
      segment.start_angle = first * sense;
      segment.sweep_angle = (last - first) * sense;
      const double sweep = std::abs(segment.sweep_angle);
      if (start_vertex.IsSame(end_vertex) && std::abs(sweep - tau) <= 1e-12) {
        if (edge_count != 1 || start.Distance(end) > tolerance) {
          throw CadFailure{profile_invalid, "Une ellipse fermée n'est pas une boucle indépendante."};
        }
        segment.sweep_angle = std::copysign(tau, segment.sweep_angle);
      } else if (sweep <= 1e-12 || sweep >= tau) {
        throw CadFailure{profile_invalid, "Le domaine d'un arc elliptique natif est invalide."};
      }
      segment.kind = 3;
    } else {
      throw CadFailure{profile_unsupported_curve, "Le contour contient une courbe autre qu'une ligne, un cercle ou une ellipse."};
    }
    if (first_vertex.IsNull()) { first_vertex = start_vertex; first_point = start; }
    previous_vertex = end_vertex;
    previous_point = end;
    segments.push_back(segment);
  }
  if (static_cast<size_t>(visited.Extent()) != edge_count || first_vertex.IsNull() ||
      !previous_vertex.IsSame(first_vertex) || previous_point.Distance(first_point) > tolerance) {
    throw CadFailure{profile_invalid, "Le parcours du contour est incomplet ou ouvert."};
  }
}

// Extract exact native line/circle geometry; tessellation never participates.
void extract_planar_profile(const unsigned char* bytes, size_t size, size_t face_id,
                            PlanarProfileOutput* output) {
  constexpr size_t loop_limit = 4096;
  if (size == 0 || size > 128 * 1024 * 1024) {
    throw CadFailure{read_failed, "Le STEP doit être non vide et ne pas dépasser 128 Mio."};
  }
  StepInputBuffer buffer(bytes, size);
  std::istream stream(&buffer);
  STEPControl_Reader reader;
  load_step(reader, stream);
  TopoDS_Compound document;
  TopTools_IndexedMapOfShape face_map;
  std::vector<ResultFaceData> faces;
  std::vector<TopoDS_Face> native_faces;
  read_result_topology(reader, document, face_map, faces, native_faces);
  if (face_id >= faces.size()) {
    throw CadFailure{profile_face_not_found, "La face STEP sélectionnée n'existe pas."};
  }
  const auto selected = native_faces[face_id];
  const BRepAdaptor_Surface surface(selected);
  if (surface.GetType() != GeomAbs_Plane) {
    throw CadFailure{profile_not_planar, "Le profil DXF exige une face STEP plane."};
  }
  const gp_Pln plane = surface.Plane();
  const gp_Pnt origin = plane.Location();
  const gp_Dir x_axis = plane.Position().XDirection();
  gp_Dir normal = plane.Axis().Direction();
  if (selected.Orientation() == TopAbs_REVERSED) normal.Reverse();
  else if (selected.Orientation() != TopAbs_FORWARD) {
    throw CadFailure{profile_invalid, "La face possède une orientation non prise en charge."};
  }
  const gp_Dir y_axis = normal.Crossed(x_axis);
  const auto face = TopoDS::Face(selected.Oriented(TopAbs_FORWARD));
  const auto outer = BRepTools::OuterWire(face);
  if (outer.IsNull()) throw CadFailure{profile_invalid, "La face ne possède pas de contour extérieur."};
  std::vector<PlanarProfileLoopData> loops;
  std::vector<PlanarProfileSegmentData> segments;
  size_t outer_count = 0;
  size_t total_edges = 0;
  for (TopoDS_Iterator wires(face); wires.More(); wires.Next()) {
    if (wires.Value().ShapeType() != TopAbs_WIRE || loops.size() >= loop_limit) {
      throw CadFailure{loops.size() >= loop_limit ? profile_too_complex : profile_invalid,
                       "Les contours de la face sont invalides ou dépassent 4 096 boucles."};
    }
    const auto wire = TopoDS::Wire(wires.Value());
    const size_t first_segment = segments.size();
    append_profile_wire(wire, plane, x_axis, y_axis, normal, segments, total_edges, &face);
    const bool is_outer = wire.IsSame(outer);
    outer_count += is_outer;
    loops.push_back({first_segment, segments.size() - first_segment, static_cast<size_t>(is_outer)});
  }
  if (outer_count != 1 || loops.empty()) {
    throw CadFailure{profile_invalid, "Le profil doit posséder un unique contour extérieur."};
  }
  auto loop_data = std::make_unique<PlanarProfileLoopData[]>(loops.size());
  auto segment_data = std::make_unique<PlanarProfileSegmentData[]>(segments.size());
  std::copy(loops.begin(), loops.end(), loop_data.get());
  std::copy(segments.begin(), segments.end(), segment_data.get());
  output->solid_id = faces[face_id].solid_id;
  for (int axis = 1; axis <= 3; ++axis) {
    output->origin[axis - 1] = origin.Coord(axis);
    output->x_axis[axis - 1] = x_axis.Coord(axis);
    output->y_axis[axis - 1] = y_axis.Coord(axis);
    output->normal[axis - 1] = normal.Coord(axis);
  }
  output->loops = loop_data.release();
  output->loop_count = loops.size();
  output->segments = segment_data.release();
  output->segment_count = segments.size();
}

void planar_frame_from_face(const TopoDS_Face& face, double* frame) {
  const BRepAdaptor_Surface surface(face);
  if (surface.GetType() != GeomAbs_Plane) {
    throw CadFailure{profile_not_planar, "Le repère exige une face STEP plane."};
  }
  const auto plane = surface.Plane();
  auto normal = plane.Axis().Direction();
  if (face.Orientation() == TopAbs_REVERSED) normal.Reverse();
  else if (face.Orientation() != TopAbs_FORWARD) {
    throw CadFailure{profile_invalid, "L'orientation de la face est invalide."};
  }
  const auto x = plane.Position().XDirection();
  const auto y = normal.Crossed(x);
  for (int axis = 1; axis <= 3; ++axis) {
    frame[axis - 1] = plane.Location().Coord(axis);
    frame[axis + 2] = x.Coord(axis);
    frame[axis + 5] = y.Coord(axis);
    frame[axis + 8] = normal.Coord(axis);
  }
}

// Returns an explicit face frame even when its boundary curves cannot be exported.
void extract_planar_face_frame(const unsigned char* bytes, size_t size, size_t face_id, double* frame) {
  if (size == 0 || size > 128 * 1024 * 1024) {
    throw CadFailure{read_failed, "Le STEP doit être non vide et ne pas dépasser 128 Mio."};
  }
  StepInputBuffer buffer(bytes, size);
  std::istream stream(&buffer);
  STEPControl_Reader reader;
  load_step(reader, stream);
  TopoDS_Compound document;
  TopTools_IndexedMapOfShape face_map;
  std::vector<ResultFaceData> faces;
  std::vector<TopoDS_Face> native_faces;
  read_result_topology(reader, document, face_map, faces, native_faces);
  if (face_id >= native_faces.size()) {
    throw CadFailure{profile_face_not_found, "La face STEP sélectionnée n'existe pas."};
  }
  planar_frame_from_face(native_faces[face_id], frame);
}

struct SectionWire {
  TopoDS_Wire wire;
  TopoDS_Face interior; // Temporary classification domain only, never a model cap.
  gp_Pnt witness;
  size_t edge_count;
  std::vector<size_t> containers;
};

// Build closed cycles using native shared vertices only: no sewing, tolerance
// matching, endpoint replacement, or newly constructed connecting edge.
std::vector<SectionWire> section_wires(const TopoDS_Shape& section, const gp_Pln& plane,
                                      size_t& global_edges, size_t& global_loops) {
  constexpr size_t edge_limit = 100'000;
  TopTools_IndexedMapOfShape edges, vertices;
  std::vector<std::array<size_t, 2>> endpoints;
  std::vector<std::vector<size_t>> incidences;
  for (TopExp_Explorer it(section, TopAbs_EDGE); it.More(); it.Next()) {
    if (edges.Contains(it.Current())) continue;
    if (++global_edges > edge_limit) throw CadFailure{profile_too_complex, "La section dépasse 100 000 arêtes."};
    const auto edge = TopoDS::Edge(it.Current());
    if (BRep_Tool::Degenerated(edge)) throw CadFailure{section_tangent, "La coupe contient une arête dégénérée ou tangente."};
    const auto id = static_cast<size_t>(edges.Add(edge) - 1);
    TopoDS_Vertex a, b;
    TopExp::Vertices(edge, a, b, true);
    if (a.IsNull() || b.IsNull()) throw CadFailure{section_open, "La coupe contient une arête sans extrémités."};
    const size_t ia = vertices.Add(a) - 1, ib = vertices.Add(b) - 1;
    incidences.resize(vertices.Extent());
    incidences[ia].push_back(id);
    incidences[ib].push_back(id);
    if (incidences[ia].size() > 2 || incidences[ib].size() > 2) {
      throw CadFailure{section_tangent, "La coupe présente un embranchement ou une tangence."};
    }
    endpoints.push_back({ia, ib});
  }
  size_t vertex_visits = 0;
  for (TopExp_Explorer it(section, TopAbs_VERTEX); it.More(); it.Next()) {
    if (++vertex_visits > 2 * edge_limit) throw CadFailure{profile_too_complex, "Le parcours des sommets de coupe dépasse son budget."};
    if (!vertices.Contains(it.Current())) throw CadFailure{section_tangent, "La coupe touche le solide en un point isolé."};
  }
  for (const auto& row : incidences) {
    if (row.size() != 2) throw CadFailure{section_open, "La section contient un contour ouvert."};
  }
  std::vector<bool> visited(endpoints.size());
  std::vector<SectionWire> result;
  for (size_t first = 0; first < endpoints.size(); ++first) {
    if (visited[first]) continue;
    if (++global_loops > 4096) throw CadFailure{profile_too_complex, "La section dépasse 4 096 contours."};
    TopoDS_Wire wire;
    BRep_Builder builder;
    builder.MakeWire(wire);
    size_t edge_id = first, vertex = endpoints[first][0];
    const size_t first_vertex = vertex;
    size_t count = 0;
    do {
      if (visited[edge_id]) throw CadFailure{section_open, "Le parcours de la section ne forme pas un cycle simple."};
      visited[edge_id] = true;
      auto edge = TopoDS::Edge(edges(static_cast<int>(edge_id + 1)));
      const auto& pair = endpoints[edge_id];
      if (pair[0] == vertex) vertex = pair[1];
      else if (pair[1] == vertex) { vertex = pair[0]; edge.Reverse(); }
      else throw CadFailure{section_open, "Les sommets de coupe ne sont pas partagés."};
      builder.Add(wire, edge);
      ++count;
      const auto& next = incidences[vertex];
      edge_id = next[0] == edge_id ? next[1] : next[0];
    } while (vertex != first_vertex);
    if (!BRep_Tool::IsClosed(wire)) throw CadFailure{section_open, "Un contour de coupe n'est pas fermé."};
    BRepBuilderAPI_MakeFace classification(plane, wire, true);
    if (!classification.IsDone()) throw CadFailure{profile_invalid, "Le contour ne définit pas un domaine de classification plan."};
    result.push_back({wire, classification.Face(), BRep_Tool::Pnt(TopoDS::Vertex(vertices(static_cast<int>(first_vertex + 1)))), count, {}});
  }
  return result;
}

// Conservative exclusion only. Native geometry (never display triangles) and
// shape tolerances bound the entire trimmed face/body. Unusable boxes retain
// the original checks rather than guessing that an intersection is absent.
bool section_bounds_may_intersect(const TopoDS_Shape& shape, const gp_Pln& plane) {
  Bnd_Box bounds;
  BRepBndLib::AddOptimal(shape, bounds, false, true);
  if (bounds.IsVoid() || bounds.IsOpen()) return true;
  double minimum[3], maximum[3];
  bounds.Get(minimum[0], minimum[1], minimum[2], maximum[0], maximum[1], maximum[2]);
  double lower = 0., upper = 0., scale = 1.;
  for (int axis = 0; axis < 3; ++axis) {
    const double origin = plane.Location().Coord(axis + 1);
    const double normal = plane.Axis().Direction().Coord(axis + 1);
    const double a = (minimum[axis] - origin) * normal;
    const double b = (maximum[axis] - origin) * normal;
    if (!std::isfinite(a) || !std::isfinite(b)) return true;
    lower += std::min(a, b);
    upper += std::max(a, b);
    scale += std::abs(minimum[axis]) + std::abs(maximum[axis]) + std::abs(origin);
  }
  const double margin = 1e-7 + 64. * std::numeric_limits<double>::epsilon() * scale;
  if (!std::isfinite(lower) || !std::isfinite(upper) || !std::isfinite(margin)) return true;
  return lower <= margin && upper >= -margin;
}

// A periodic seam has two pcurves on one native face, but no second incident
// face. No geometric matching is used to identify a common section edge.
const TopoDS_Face* exact_section_seam_face(const TopoDS_Edge& edge,
                                          const TopTools_IndexedMapOfShape& source_edges,
                                          const std::vector<size_t>& source_ancestors,
                                          const std::vector<TopoDS_Face>& native_faces) {
  const int index = source_edges.FindIndex(edge);
  if (index <= 0) return nullptr;
  const size_t ancestor = source_ancestors[static_cast<size_t>(index - 1)];
  if (ancestor >= native_faces.size()) return nullptr;
  const auto& face = native_faces[ancestor];
  return BRep_Tool::IsClosed(edge, face) ? &face : nullptr;
}

// Analytic critical heights exclude tangencies over the entire native support,
// not just sampled points of an intersection curve. This is deliberately
// conservative for trimmed patches: an excluded support level remains refused.
void check_section_surface_support(const BRepAdaptor_Surface& surface, const gp_Pln& plane) {
  constexpr double tolerance = 1e-7;
  const gp_Dir normal = plane.Axis().Direction();
  auto signed_height = [&](const gp_Pnt& center) { return gp_Vec(center, plane.Location()).Dot(gp_Vec(normal)); };
  auto tangent = [] { throw CadFailure{section_tangent, "Le plan atteint un niveau tangent ou singulier d'une surface support ; modifier le décalage."}; };
  switch (surface.GetType()) {
    case GeomAbs_Plane: {
      const auto support = surface.Plane();
      if (std::abs(support.Axis().Direction().Dot(normal)) >= 1. - 1e-12 && plane.Distance(support.Location()) <= tolerance) {
        throw CadFailure{section_coplanar, "Le plan de coupe coïncide avec une face du solide ; modifier le décalage."};
      }
      return;
    }
    case GeomAbs_Sphere: {
      const auto sphere = surface.Sphere();
      if (std::abs(std::abs(signed_height(sphere.Location())) - sphere.Radius()) <= tolerance) tangent();
      return;
    }
    case GeomAbs_Cylinder: {
      const auto cylinder = surface.Cylinder();
      if (std::abs(cylinder.Axis().Direction().Dot(normal)) <= 1e-12 &&
          std::abs(std::abs(signed_height(cylinder.Location())) - cylinder.Radius()) <= tolerance) tangent();
      return;
    }
    case GeomAbs_Cone: {
      const auto cone = surface.Cone();
      if (plane.Distance(cone.Apex()) <= tolerance) tangent();
      return;
    }
    case GeomAbs_Torus: {
      const auto torus = surface.Torus();
      if (torus.MajorRadius() <= torus.MinorRadius() || torus.MinorRadius() <= tolerance) {
        throw CadFailure{profile_unsupported_curve, "Seuls les tores annulaires réguliers sont pris en charge en section."};
      }
      const double axial = normal.Dot(torus.Axis().Direction());
      const double radial = gp_Vec(normal).Crossed(gp_Vec(torus.Axis().Direction())).Magnitude();
      const double height = signed_height(torus.Location());
      for (const double major_sign : {-1., 1.}) {
        for (const double minor_sign : {-1., 1.}) {
          if (std::abs(height - major_sign * torus.MajorRadius() * radial - minor_sign * torus.MinorRadius()) <= tolerance) tangent();
        }
      }
      const bool horizontal = std::abs(axial) >= 1. - 1e-12;
      const bool meridian = std::abs(axial) <= 1e-12 && std::abs(height) <= tolerance;
      if (!horizontal && !meridian) {
        throw CadFailure{profile_unsupported_curve, "Le tore exige une coupe perpendiculaire à son axe ou un plan contenant son axe."};
      }
      return;
    }
    default: throw CadFailure{profile_unsupported_curve, "La transversalité de cette famille de surfaces n'est pas prise en charge."};
  }
}

// Whitelist of exact analytic intersections with globally controlled normals.
// Native pcurve samples below are additional consistency checks, not the proof.
void check_section_curve_family(const BRepAdaptor_Surface& surface, const BRepAdaptor_Curve& curve,
                                const gp_Pln& plane) {
  const bool circle = curve.GetType() == GeomAbs_Circle;
  const bool ellipse = curve.GetType() == GeomAbs_Ellipse;
  const bool line = curve.GetType() == GeomAbs_Line;
  auto parallel = [](const gp_Dir& a, const gp_Dir& b) { return std::abs(a.Dot(b)) >= 1. - 1e-12; };
  bool supported = false;
  switch (surface.GetType()) {
    case GeomAbs_Plane: supported = line; break;
    case GeomAbs_Sphere: supported = circle; break;
    case GeomAbs_Cylinder: {
      const auto axis = surface.Cylinder().Axis().Direction();
      // A non-parallel plane cuts a circular cylinder in an exact ellipse.
      // Smooth cylinder tangencies require a plane parallel to its axis and
      // were excluded by the global analytic support check.
      supported = (circle && parallel(curve.Circle().Axis().Direction(), axis)) ||
                  (line && parallel(curve.Line().Direction(), axis)) ||
                  (ellipse && std::abs(plane.Axis().Direction().Dot(axis)) > 1e-12);
      break;
    }
    case GeomAbs_Cone:
      // Every tangent plane of a cone contains its apex. The support preflight
      // excludes all such planes (conservatively also for trimmed frusta).
      supported = ellipse || (circle && parallel(curve.Circle().Axis().Direction(), surface.Cone().Axis().Direction()));
      break;
    case GeomAbs_Torus:
      check_section_surface_support(surface, plane);
      supported = circle;
      break;
    default: break;
  }
  if (!supported) throw CadFailure{profile_unsupported_curve, "Cette intersection analytique n'a pas de preuve de transversalité prise en charge."};
}

// Exact native pcurves locate the section on the ancestor surface. A section
// along a tangency (including a closed tangent torus circle) is never a profile.
void check_section_transversality(BRepAlgoAPI_Section& operation, const TopoDS_Edge& edge,
                                 const gp_Pln& plane,
                                 const TopTools_IndexedMapOfShape& source_edges,
                                 const std::vector<size_t>& source_ancestors,
                                 const std::vector<TopoDS_Face>& native_faces) {
  const gp_Dir normal = plane.Axis().Direction();
  TopoDS_Shape ancestor;
  if (!operation.HasAncestorFaceOn1(edge, ancestor) || ancestor.ShapeType() != TopAbs_FACE) {
    const auto* seam_face = exact_section_seam_face(edge, source_edges, source_ancestors, native_faces);
    if (!seam_face) {
      throw CadFailure{section_tangent, "La coupe coïncide avec une arête native ambiguë ou une tangence."};
    }
    ancestor = *seam_face;
  }
  const auto face = TopoDS::Face(ancestor);
  const BRepAdaptor_Surface surface(face);
  const BRepAdaptor_Curve native_curve(edge);
  check_section_curve_family(surface, native_curve, plane);
  if (surface.GetType() == GeomAbs_Sphere) {
    // A spherical pole is a UV singularity, not a singularity of the sphere.
    // The critical-height proof above is global; these radial normals are
    // coordinate-independent consistency checks, including at both poles.
    const auto sphere = surface.Sphere();
    const double first = native_curve.FirstParameter(), last = native_curve.LastParameter();
    for (const double fraction : {0., 0.5, 1.}) {
      const gp_Vec radial(sphere.Location(), native_curve.Value(first + (last - first) * fraction));
      if (std::abs(radial.Magnitude() - sphere.Radius()) > 1e-7 ||
          radial.Magnitude() <= 1e-7 ||
          radial.Crossed(gp_Vec(normal)).Magnitude() <= 1e-8 * radial.Magnitude()) {
        throw CadFailure{section_tangent, "La coupe est tangente à la surface sphérique native."};
      }
    }
    return;
  }
  double first, last;
  const auto curve = BRep_Tool::CurveOnSurface(edge, face, first, last);
  if (curve.IsNull() || !std::isfinite(first) || !std::isfinite(last)) {
    throw CadFailure{profile_invalid, "La coupe n'a pas de courbe paramétrique native vérifiable."};
  }
  for (const double fraction : {0., 0.5, 1.}) {
    const auto uv = curve->Value(first + (last - first) * fraction);
    BRepLProp_SLProps properties(surface, uv.X(), uv.Y(), 1, 1e-12);
    if (!properties.IsNormalDefined() || gp_Vec(properties.Normal()).Crossed(gp_Vec(normal)).Magnitude() <= 1e-8) {
      throw CadFailure{section_tangent, "La coupe est tangente à une surface native ou passe par une singularité."};
    }
  }
}

void extract_section_profile(const unsigned char* bytes, size_t size, const double* frame,
                             const size_t* solid_ids, size_t solid_count, SectionOutput* output) {
  constexpr double tolerance = 1e-7;
  if (size == 0 || size > 128 * 1024 * 1024) throw CadFailure{read_failed, "Le STEP doit être non vide et ne pas dépasser 128 Mio."};
  if (!frame || !solid_ids || solid_count == 0 || solid_count > 32) throw CadFailure{section_invalid_plane, "Le plan et le choix des corps sont invalides."};
  for (size_t i = 0; i < 12; ++i) if (!std::isfinite(frame[i])) throw CadFailure{section_invalid_plane, "Le repère de coupe doit être fini."};
  const gp_Vec x(frame[3], frame[4], frame[5]), y(frame[6], frame[7], frame[8]), n(frame[9], frame[10], frame[11]);
  if (std::abs(x.Magnitude() - 1.) > 1e-9 || std::abs(y.Magnitude() - 1.) > 1e-9 ||
      std::abs(n.Magnitude() - 1.) > 1e-9 || x.Crossed(y).Subtracted(n).Magnitude() > 1e-9) {
    throw CadFailure{section_invalid_plane, "Le repère de coupe doit être orthonormé direct."};
  }
  const gp_Dir normal(n), x_axis(x), y_axis(y);
  const gp_Pln plane(gp_Ax3(gp_Pnt(frame[0], frame[1], frame[2]), normal, x_axis));
  StepInputBuffer buffer(bytes, size);
  std::istream stream(&buffer);
  STEPControl_Reader reader;
  load_step(reader, stream);
  TopoDS_Compound document;
  TopTools_IndexedMapOfShape face_map;
  std::vector<ResultFaceData> faces;
  std::vector<TopoDS_Face> native_faces;
  read_result_topology(reader, document, face_map, faces, native_faces);
  std::vector<TopoDS_Shape> solids;
  for (TopoDS_Iterator it(document); it.More(); it.Next()) solids.push_back(it.Value());
  for (size_t i = 0; i < solid_count; ++i) {
    if (solid_ids[i] >= solids.size() || (i && solid_ids[i - 1] >= solid_ids[i])) {
      throw CadFailure{profile_invalid, "La sélection des corps est vide, répétée ou inconnue."};
    }
  }
  std::vector<SectionLoopData> loops;
  std::vector<PlanarProfileSegmentData> segments;
  size_t global_edges = 0, global_loops = 0, converted_edges = 0, classification_work = 0;
  size_t source_edge_visits = 0;
  for (size_t selected = 0; selected < solid_count; ++selected) {
    const size_t solid_id = solid_ids[selected];
    const auto& solid = solids[solid_id];
    // A disjoint conservative body box proves this selected body empty; the
    // Rust result still contains its requested identity with an empty loop list.
    if (!section_bounds_may_intersect(solid, plane)) continue;
    TopTools_IndexedMapOfShape source_edges;
    std::vector<size_t> source_ancestors;
    const auto multiple_faces = std::numeric_limits<size_t>::max();
    for (size_t i = 0; i < faces.size(); ++i) {
      if (faces[i].solid_id != solid_id) continue;
      const auto& face = native_faces[i];
      if (section_bounds_may_intersect(face, plane)) {
        check_section_surface_support(BRepAdaptor_Surface(face), plane);
      }
      for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
        if (++source_edge_visits > 600'000) throw CadFailure{profile_too_complex, "Le contrôle des arêtes source dépasse son budget."};
        const auto index = static_cast<size_t>(source_edges.Add(it.Current()) - 1);
        if (index == source_ancestors.size()) source_ancestors.push_back(i);
        else if (source_ancestors[index] != i) source_ancestors[index] = multiple_faces;
      }
    }
    for (int source_id = 1; source_id <= source_edges.Extent(); ++source_id) {
      const auto edge = TopoDS::Edge(source_edges(source_id));
      if (BRep_Tool::Degenerated(edge)) continue;
      const BRepAdaptor_Curve curve(edge);
      bool coincides = false;
      if (curve.GetType() == GeomAbs_Line) {
        coincides = plane.Distance(curve.Value(curve.FirstParameter())) <= tolerance &&
                    plane.Distance(curve.Value(curve.LastParameter())) <= tolerance;
      } else if (curve.GetType() == GeomAbs_Circle) {
        const auto circle = curve.Circle();
        coincides = plane.Distance(circle.Location()) <= tolerance &&
                    std::abs(circle.Axis().Direction().Dot(normal)) >= 1. - 1e-12;
      } else if (curve.GetType() == GeomAbs_Ellipse) {
        const auto ellipse = curve.Ellipse();
        coincides = plane.Distance(ellipse.Location()) <= tolerance &&
                    std::abs(ellipse.Axis().Direction().Dot(normal)) >= 1. - 1e-12;
      }
      if (coincides && !exact_section_seam_face(edge, source_edges, source_ancestors, native_faces)) {
        throw CadFailure{section_tangent, "Le plan de coupe contient une arête native entre faces ; modifier le décalage."};
      }
    }
    BRepAlgoAPI_Section operation(solid, plane, false);
    operation.SetRunParallel(false);
    operation.Approximation(false);
    operation.ComputePCurveOn1(true);
    operation.Build();
    if (!operation.IsDone() || operation.HasErrors()) throw CadFailure{profile_invalid, "Le noyau ne peut pas construire cette section."};
    TopTools_IndexedMapOfShape checked_edges;
    for (TopExp_Explorer it(operation.Shape(), TopAbs_EDGE); it.More(); it.Next()) {
      if (checked_edges.Contains(it.Current())) continue;
      if (checked_edges.Extent() >= 100'000) throw CadFailure{profile_too_complex, "La section dépasse 100 000 arêtes."};
      checked_edges.Add(it.Current());
      const auto edge = TopoDS::Edge(it.Current());
      const auto kind = BRepAdaptor_Curve(edge).GetType();
      if (kind != GeomAbs_Line && kind != GeomAbs_Circle && kind != GeomAbs_Ellipse) throw CadFailure{profile_unsupported_curve, "La section contient une courbe non prise en charge."};
      check_section_transversality(operation, edge, plane, source_edges, source_ancestors, native_faces);
    }
    auto wires = section_wires(operation.Shape(), plane, global_edges, global_loops);
    // Non-intersecting closed cycles can be nested using one native witness.
    // Pair distance rejects contacts/crossings before containment is considered.
    for (size_t a = 0; a < wires.size(); ++a) {
      for (size_t b = a + 1; b < wires.size(); ++b) {
        const size_t cost = wires[a].edge_count * wires[b].edge_count;
        if (cost > 1'000'000 - classification_work) throw CadFailure{profile_too_complex, "La classification des contours dépasse son budget."};
        classification_work += cost;
        BRepExtrema_DistShapeShape distance(wires[a].wire, wires[b].wire);
        if (!distance.IsDone() || distance.Value() <= tolerance) throw CadFailure{section_tangent, "Deux contours de section se touchent ou se croisent."};
        const BRepClass_FaceClassifier a_in_b(wires[b].interior, wires[a].witness, tolerance, false, tolerance);
        const BRepClass_FaceClassifier b_in_a(wires[a].interior, wires[b].witness, tolerance, false, tolerance);
        if (a_in_b.State() == TopAbs_IN) wires[a].containers.push_back(b);
        else if (a_in_b.State() != TopAbs_OUT) throw CadFailure{profile_invalid, "La position d'un contour de section est ambiguë."};
        if (b_in_a.State() == TopAbs_IN) wires[b].containers.push_back(a);
        else if (b_in_a.State() != TopAbs_OUT) throw CadFailure{profile_invalid, "La position d'un contour de section est ambiguë."};
      }
    }
    size_t next_component = 0;
    std::vector<size_t> components(wires.size(), std::numeric_limits<size_t>::max());
    for (size_t i = 0; i < wires.size(); ++i) if (wires[i].containers.size() % 2 == 0) components[i] = next_component++;
    for (size_t i = 0; i < wires.size(); ++i) {
      const bool is_outer = wires[i].containers.size() % 2 == 0;
      if (!is_outer) {
        size_t parent = std::numeric_limits<size_t>::max();
        for (const size_t candidate : wires[i].containers) {
          if (wires[candidate].containers.size() + 1 == wires[i].containers.size()) {
            if (parent != std::numeric_limits<size_t>::max()) throw CadFailure{profile_invalid, "L'imbrication des contours est ambiguë."};
            parent = candidate;
          }
        }
        if (parent == std::numeric_limits<size_t>::max() || components[parent] == std::numeric_limits<size_t>::max()) throw CadFailure{profile_invalid, "Un trou de section n'a pas de contour extérieur."};
        components[i] = components[parent];
      }
      const size_t start = segments.size();
      append_profile_wire(wires[i].wire, plane, x_axis, y_axis, normal, segments, converted_edges);
      loops.push_back({solid_id, components[i], start, segments.size() - start, static_cast<size_t>(is_outer)});
    }
  }
  auto loop_data = std::make_unique<SectionLoopData[]>(loops.size());
  auto segment_data = std::make_unique<PlanarProfileSegmentData[]>(segments.size());
  std::copy(loops.begin(), loops.end(), loop_data.get());
  std::copy(segments.begin(), segments.end(), segment_data.get());
  output->loops = loop_data.release();
  output->loop_count = loops.size();
  output->segments = segment_data.release();
  output->segment_count = segments.size();
}

template <typename BuildSolid>
int validated_step(BuildSolid build, SurfaceCounts expected, StepOutput* output,
                   char* error, size_t error_capacity,
                   const RoundedBoxFrame* expected_box = nullptr,
                   FaceLabels* labels = nullptr) noexcept {
  *output = {};
  return with_occt(invalid_model, error, error_capacity, [&](Status& stage) {
    const TopoDS_Solid solid = build();
    stage = invalid_solid;
    const CadMetrics before = measure(solid, expected, stage, expected_box);

    stage = write_failed;
    DESTEP_Parameters parameters;
    parameters.WriteProductName = "FabSTEP";
    parameters.WriteUnit = UnitsMethods_LengthUnit_Millimeter;
    parameters.WriteTessellated = DESTEP_Parameters::RWMode_Tessellated_Off;
    STEPControl_Writer writer;
    if (writer.Transfer(solid, STEPControl_ManifoldSolidBrep, parameters) != IFSelect_RetDone) {
      throw CadFailure{stage, "Impossible de traduire le solide en STEP."};
    }
    if (labels) write_face_labels(writer, *labels);
    std::stringstream stream;
    if (writer.WriteStream(stream) != IFSelect_RetDone || !stream.good()) {
      throw CadFailure{stage, "Impossible de sérialiser le fichier STEP."};
    }

    stage = read_failed;
    stream.seekg(0);
    const CadMetrics after = measure(read_step(stream), expected, stage, expected_box);
    if (!same_measurement(before, after)) {
      throw CadFailure{mismatch, "Les dimensions ou le volume changent après réimport du STEP."};
    }

    const std::string step = stream.str();
    auto* data = new unsigned char[step.size()];
    std::memcpy(data, step.data(), step.size());
    *output = {data, step.size(), after};
  });
}
}

extern "C" int fabstep_build_step(
    const double* points, size_t point_count, const double* normals, size_t face_count,
    const size_t* face_loops, const size_t* loop_vertices, const size_t* vertex_ids,
    StepOutput* output, char* error, size_t error_capacity) noexcept {
  return validated_step(
      [&] { return build_solid(points, point_count, normals, face_count, face_loops, loop_vertices, vertex_ids); },
      {face_count, 0}, output, error, error_capacity);
}

extern "C" int fabstep_build_step_with_provenance(
    const double* points, size_t point_count, const double* normals, size_t face_count,
    const size_t* face_loops, const size_t* loop_vertices, const size_t* vertex_ids,
    const size_t* tokens, StepOutput* output, char* error, size_t error_capacity) noexcept {
  FaceLabels labels;
  return validated_step(
      [&] { return build_solid(points, point_count, normals, face_count, face_loops,
                              loop_vertices, vertex_ids, tokens, &labels); },
      {face_count, 0}, output, error, error_capacity, nullptr, &labels);
}

extern "C" int fabstep_validate_source(
    const double* points, size_t point_count, const double* normals, size_t face_count,
    const size_t* face_loops, const size_t* loop_vertices, const size_t* vertex_ids,
    CadMetrics* output, char* error, size_t error_capacity) noexcept {
  *output = {};
  return with_occt(invalid_model, error, error_capacity, [&](Status& stage) {
    const auto solid = build_solid(points, point_count, normals, face_count,
                                   face_loops, loop_vertices, vertex_ids);
    stage = invalid_solid;
    *output = measure(solid, {face_count, 0}, stage);
  });
}

extern "C" int fabstep_build_extrusion_step(
    const double* origin, const double* axis, const double* x_axis, double height,
    const double* points, const ProfileLoopData* loops, size_t loop_count,
    StepOutput* output, char* error, size_t error_capacity) noexcept {
  SurfaceCounts expected{2, 0};
  for (size_t i = 0; i < loop_count; ++i) {
    if (loops[i].radius > 0.0) {
      ++expected.cylindrical;
    } else {
      expected.planar += loops[i].point_count;
    }
  }
  return validated_step(
      [&] { return build_extrusion(origin, axis, x_axis, height, points, loops, loop_count); },
      expected, output, error, error_capacity);
}

extern "C" int fabstep_build_mixed_step(
    const double* points, size_t point_count, const MixedRingData* rings, size_t ring_count,
    const double* normals, size_t plane_count, const size_t* plane_loops, const MixedLoopData* loops,
    const MixedSegmentData* segments, const MixedCylinderData* cylinders, size_t cylinder_count,
    const MixedArcData* arcs, size_t arc_count, const MixedCylinderData* partial, size_t partial_count,
    const MixedCylinderData* cones, size_t cone_count,
    const SphericalPatchData* spheres, size_t sphere_count,
    const ToroidalPatchData* tori, size_t torus_count,
    const MixedCylinderData* partial_cones, size_t partial_cone_count,
    StepOutput* output, char* error, size_t error_capacity) noexcept {
  return validated_step(
      [&] {
        return build_mixed_solid(points, point_count, rings, ring_count, normals, plane_count,
                                 plane_loops, loops, segments, cylinders, cylinder_count,
                                 arcs, arc_count, partial, partial_count, cones, cone_count,
                                 spheres, sphere_count, tori, torus_count,
                                 nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr,
                                 partial_cones, partial_cone_count);
      },
      {plane_count, cylinder_count + partial_count, partial_count, cone_count + partial_cone_count,
       sphere_count, torus_count, 0, partial_cone_count},
      output, error, error_capacity);
}

extern "C" int fabstep_build_mixed_step_with_provenance(
    const double* points, size_t point_count, const MixedRingData* rings, size_t ring_count,
    const double* normals, size_t plane_count, const size_t* plane_loops, const MixedLoopData* loops,
    const MixedSegmentData* segments, const MixedCylinderData* cylinders, size_t cylinder_count,
    const size_t* plane_tokens, const size_t* cylinder_tokens,
    StepOutput* output, char* error, size_t error_capacity) noexcept {
  FaceLabels labels;
  return validated_step(
      [&] {
        return build_mixed_solid(points, point_count, rings, ring_count, normals, plane_count,
                                 plane_loops, loops, segments, cylinders, cylinder_count,
                                 nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0,
                                 plane_tokens, cylinder_tokens, &labels);
      },
      {plane_count, cylinder_count}, output, error, error_capacity, nullptr, &labels);
}

extern "C" int fabstep_build_revolved_step_with_provenance(
    const double* points, size_t point_count, const MixedRingData* rings, size_t ring_count,
    const double* normals, size_t plane_count, const size_t* plane_loops, const MixedLoopData* loops,
    const MixedSegmentData* segments, const MixedCylinderData* cylinders, size_t cylinder_count,
    const MixedCylinderData* cones, size_t cone_count,
    const size_t* plane_tokens, const size_t* cylinder_tokens, const size_t* cone_tokens,
    StepOutput* output, char* error, size_t error_capacity) noexcept {
  FaceLabels labels;
  return validated_step(
      [&] {
        return build_mixed_solid(points, point_count, rings, ring_count, normals, plane_count,
                                 plane_loops, loops, segments, cylinders, cylinder_count,
                                 nullptr, 0, nullptr, 0, cones, cone_count, nullptr, 0, nullptr, 0,
                                 plane_tokens, cylinder_tokens, &labels, cone_tokens);
      },
      {plane_count, cylinder_count, 0, cone_count}, output, error, error_capacity, nullptr, &labels);
}

extern "C" int fabstep_build_rounded_step_with_provenance(
    const double* points, size_t point_count, const MixedRingData* rings, size_t ring_count,
    const double* normals, size_t plane_count, const size_t* plane_loops, const MixedLoopData* loops,
    const MixedSegmentData* segments, const MixedCylinderData* cylinders, size_t cylinder_count,
    const MixedArcData* arcs, size_t arc_count, const MixedCylinderData* partial, size_t partial_count,
    const MixedCylinderData* cones, size_t cone_count,
    const SphericalPatchData* spheres, size_t sphere_count,
    const ToroidalPatchData* tori, size_t torus_count,
    const size_t* plane_tokens, const size_t* cylinder_tokens, const size_t* partial_tokens,
    const size_t* cone_tokens, const size_t* sphere_tokens, const size_t* torus_tokens,
    const MixedCylinderData* partial_cones, size_t partial_cone_count,
    const size_t* partial_cone_tokens,
    StepOutput* output, char* error, size_t error_capacity) noexcept {
  FaceLabels labels;
  return validated_step(
      [&] {
        return build_mixed_solid(points, point_count, rings, ring_count, normals, plane_count,
                                 plane_loops, loops, segments, cylinders, cylinder_count,
                                 arcs, arc_count, partial, partial_count, cones, cone_count,
                                 spheres, sphere_count, tori, torus_count,
                                 plane_tokens, cylinder_tokens, &labels, cone_tokens,
                                 sphere_tokens, torus_tokens, partial_tokens,
                                 partial_cones, partial_cone_count, partial_cone_tokens);
      },
      {plane_count, cylinder_count + partial_count, partial_count, cone_count + partial_cone_count,
       sphere_count, torus_count, 0, partial_cone_count},
      output, error, error_capacity, nullptr, &labels);
}

static int build_complete_sphere_step_impl(
    const double* center, double radius, StepOutput* output, char* error, size_t error_capacity, size_t token) noexcept {
  FaceLabels labels;
  return validated_step(
      [&] {
        if (!std::isfinite(radius) || radius <= Precision::Confusion()) {
          throw CadFailure{invalid_model, "Le rayon de la sphère complète est inférieur à la précision du noyau."};
        }
        BRepPrimAPI_MakeSphere sphere(point_at(center, 0), radius);
        const auto solid = sphere.Solid();
        if (!sphere.IsDone() || solid.IsNull()) {
          throw CadFailure{invalid_solid, "Impossible de construire la sphère complète."};
        }
        if (token != 0) {
          TopExp_Explorer faces(solid, TopAbs_FACE);
          if (faces.More()) {
            const auto face = TopoDS::Face(faces.Current());
            faces.Next();
            if (!faces.More()) labels.push_back({face, token});
          }
        }
        return solid;
      },
      {0, 0, 0, 0, 1}, output, error, error_capacity, nullptr, token ? &labels : nullptr);
}

extern "C" int fabstep_build_complete_sphere_step(
    const double* center, double radius, StepOutput* output, char* error, size_t error_capacity) noexcept {
  return build_complete_sphere_step_impl(center, radius, output, error, error_capacity, 0);
}

extern "C" int fabstep_build_complete_sphere_step_with_provenance(
    const double* center, double radius, size_t token, StepOutput* output, char* error, size_t error_capacity) noexcept {
  return build_complete_sphere_step_impl(center, radius, output, error, error_capacity, token);
}

static int build_complete_torus_step_impl(
    const double* center, const double* axis, double major, double minor,
    StepOutput* output, char* error, size_t error_capacity, size_t token) noexcept {
  FaceLabels labels;
  return validated_step(
      [&] {
        const double axis_norm = std::hypot(axis[0], axis[1], axis[2]);
        const double tolerance = Precision::Confusion();
        if (!std::isfinite(center[0]) || !std::isfinite(center[1]) || !std::isfinite(center[2]) ||
            !std::isfinite(axis_norm) || std::abs(axis_norm - 1.0) > 1e-8 ||
            !std::isfinite(major + minor) || minor <= tolerance || major - minor <= tolerance) {
          throw CadFailure{invalid_model, "Le tore complet présente un axe, un centre ou des rayons invalides."};
        }
        BRepPrimAPI_MakeTorus torus(gp_Ax2(point_at(center, 0), direction(axis)), major, minor);
        const auto solid = torus.Solid();
        if (!torus.IsDone() || solid.IsNull()) {
          throw CadFailure{invalid_solid, "Impossible de construire le tore complet."};
        }
        if (token != 0) {
          TopExp_Explorer faces(solid, TopAbs_FACE);
          if (faces.More()) {
            const auto face = TopoDS::Face(faces.Current());
            faces.Next();
            if (!faces.More()) labels.push_back({face, token});
          }
        }
        return solid;
      },
      {0, 0, 0, 0, 0, 1}, output, error, error_capacity, nullptr, token ? &labels : nullptr);
}

extern "C" int fabstep_build_complete_torus_step(
    const double* center, const double* axis, double major, double minor, StepOutput* output, char* error, size_t error_capacity) noexcept {
  return build_complete_torus_step_impl(center, axis, major, minor, output, error, error_capacity, 0);
}

extern "C" int fabstep_build_complete_torus_step_with_provenance(
    const double* center, const double* axis, double major, double minor, size_t token, StepOutput* output, char* error, size_t error_capacity) noexcept {
  return build_complete_torus_step_impl(center, axis, major, minor, output, error, error_capacity, token);
}

extern "C" int fabstep_build_rounded_box_step(
    const double* center, const double* axes, const double* half, double radius,
    StepOutput* output, char* error, size_t error_capacity) noexcept {
  RoundedBoxFrame frame{};
  return validated_step(
      [&] {
        const CadFailure invalid{invalid_model, "Le boîtier arrondi exige un repère direct et des dimensions résolues."};
        const double tolerance = Precision::Confusion();
        if (!std::isfinite(radius) || radius <= tolerance) {
          throw invalid;
        }
        frame.center = point_at(center, 0);
        frame.radius = radius;
        gp_Pnt lower = frame.center;
        for (size_t i = 0; i < 3; ++i) {
          const double* axis = axes + 3 * i;
          const double norm = std::hypot(axis[0], axis[1], axis[2]);
          if (!std::isfinite(center[i]) || !std::isfinite(norm) || std::abs(norm - 1.0) > box_angle_tolerance ||
              !std::isfinite(2.0 * (half[i] + radius)) || half[i] <= tolerance) {
            throw invalid;
          }
          frame.axes[i] = direction(axis);
          frame.half[i] = half[i];
          lower.Translate(-gp_Vec(frame.axes[i]) * (half[i] + radius));
        }
        const auto& [x, y, z] = frame.axes;
        if (!x.IsNormal(y, box_angle_tolerance) || !x.IsNormal(z, box_angle_tolerance) ||
            !y.IsNormal(z, box_angle_tolerance) || x.Crossed(y).Dot(z) <= 0.0) {
          throw invalid;
        }
        BRepPrimAPI_MakeBox box(gp_Ax2(lower, z, x), 2.0 * (half[0] + radius),
                               2.0 * (half[1] + radius), 2.0 * (half[2] + radius));
        const auto original = box.Solid();
        TopTools_IndexedMapOfShape edges;
        TopExp::MapShapes(original, TopAbs_EDGE, edges);
        if (!box.IsDone() || edges.Extent() != 12) {
          throw CadFailure{invalid_solid, "Le pavé du boîtier ne contient pas douze arêtes."};
        }
        BRepFilletAPI_MakeFillet fillet(original);
        for (int i = 1; i <= edges.Extent(); ++i) {
          fillet.Add(radius, TopoDS::Edge(edges(i)));
        }
        fillet.Build();
        if (!fillet.IsDone()) {
          throw CadFailure{invalid_solid, "Impossible de construire les douze congés du boîtier."};
        }
        return TopoDS::Solid(single_solid(fillet.Shape(), invalid_solid));
      },
      rounded_box_surfaces, output, error, error_capacity, &frame);
}

extern "C" void fabstep_free_step(unsigned char* bytes) noexcept {
  delete[] bytes;
}

extern "C" int fabstep_combine_steps(const StepPartInput* parts, size_t count, CadMetrics* bodies,
                                    StepOutput* output, char* error, size_t error_capacity) noexcept {
  *output = {};
  return with_occt(read_failed, error, error_capacity, [&](Status& stage) {
    combine(parts, count, bodies, output, stage);
  });
}

extern "C" int fabstep_combine_steps_with_provenance(
    const StepPartInput* parts, size_t count, CadMetrics* bodies, StepOutput* output,
    char* error, size_t error_capacity) noexcept {
  *output = {};
  return with_occt(read_failed, error, error_capacity, [&](Status& stage) {
    combine(parts, count, bodies, output, stage, true);
  });
}

extern "C" int fabstep_validate_sources_separated(const PlanarPartInput* parts, size_t count,
                                                   char* error, size_t error_capacity) noexcept {
  return with_occt(invalid_model, error, error_capacity, [&](Status& stage) {
    std::vector<Body> bodies;
    bodies.reserve(count);
    for (size_t i = 0; i < count; ++i) {
      const auto& p = parts[i];
      stage = invalid_model;
      auto shape = build_solid(p.points, p.point_count, p.normals, p.face_count,
                               p.face_loops, p.loop_vertices, p.vertex_ids);
      stage = invalid_solid;
      const SurfaceCounts expected{p.face_count, 0};
      auto report = measure_solid(shape, stage, &expected);
      bodies.push_back({std::move(shape), report});
    }
    check_bodies_separated(bodies, nullptr);
  });
}

extern "C" int fabstep_tessellate_step(const unsigned char* bytes, size_t size, double deflection,
                                      MeshOutput* output, char* error, size_t error_capacity) noexcept {
  *output = {};
  return with_occt(read_failed, error, error_capacity, [&](Status& stage) {
    std::istringstream stream(std::string(reinterpret_cast<const char*>(bytes), size));
    const auto shape = read_step(stream);
    stage = mesh_failed;
    tessellate(shape, deflection, output);
  });
}

extern "C" void fabstep_free_mesh(double* points, size_t* triangles) noexcept {
  delete[] points;
  delete[] triangles;
}

extern "C" int fabstep_result_faces(const unsigned char* bytes, size_t size, double deflection,
                                    ResultFacesOutput* output, char* error, size_t capacity) noexcept {
  *output = {};
  return with_occt(read_failed, error, capacity, [&](Status&) {
    inspect_result_faces(bytes, size, deflection, output);
  });
}

extern "C" void fabstep_free_result_faces(ResultFaceData* faces, size_t* neighbors) noexcept {
  delete[] faces;
  delete[] neighbors;
}

extern "C" int fabstep_planar_face_profile(const unsigned char* bytes, size_t size, size_t face_id,
                                           PlanarProfileOutput* output, char* error, size_t capacity) noexcept {
  *output = {};
  return with_occt(profile_invalid, error, capacity, [&](Status&) {
    extract_planar_profile(bytes, size, face_id, output);
  });
}

extern "C" void fabstep_free_planar_profile(PlanarProfileLoopData* loops,
                                            PlanarProfileSegmentData* segments) noexcept {
  delete[] loops;
  delete[] segments;
}

extern "C" int fabstep_planar_face_frame(const unsigned char* bytes, size_t size, size_t face_id,
                                         double* frame, char* error, size_t capacity) noexcept {
  return with_occt(profile_invalid, error, capacity, [&](Status&) {
    extract_planar_face_frame(bytes, size, face_id, frame);
  });
}

extern "C" int fabstep_section_profile(const unsigned char* bytes, size_t size, const double* frame,
                                       const size_t* solid_ids, size_t solid_count,
                                       SectionOutput* output, char* error, size_t capacity) noexcept {
  *output = {};
  return with_occt(profile_invalid, error, capacity, [&](Status&) {
    extract_section_profile(bytes, size, frame, solid_ids, solid_count, output);
  });
}

extern "C" void fabstep_free_section_profile(SectionLoopData* loops, PlanarProfileSegmentData* segments) noexcept {
  delete[] loops;
  delete[] segments;
}

extern "C" const char* fabstep_cad_version() noexcept {
  return "Open CASCADE " OCC_VERSION_COMPLETE;
}
