#pragma once

#include <BRepAdaptor_Surface.hxx>
#include <BRep_Tool.hxx>
#include <Precision.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <array>
#include <cmath>
#include <optional>

namespace cut_octant {

struct Frame {
  gp_Pnt center;
  std::array<gp_Pnt, 3> corners;
  double radius;
};

// Surface parameters and vertex order may change on STEP import. The sphere
// center and the three world-space arc endpoints identify the occupied octant.
inline std::optional<Frame> read_frame(const TopoDS_Shape& shape) {
  std::optional<Frame> result;
  for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
    const auto face = TopoDS::Face(it.Current());
    const BRepAdaptor_Surface surface(face, false);
    if (surface.GetType() != GeomAbs_Sphere) continue;
    TopTools_IndexedMapOfShape vertices;
    TopExp::MapShapes(face, TopAbs_VERTEX, vertices);
    if (result || vertices.Extent() != 3) return std::nullopt;
    const auto sphere = surface.Sphere();
    result = Frame{sphere.Location(), {}, sphere.Radius()};
    for (int k = 0; k < 3; ++k) {
      result->corners[k] = BRep_Tool::Pnt(TopoDS::Vertex(vertices(k + 1)));
    }
  }
  return result;
}

inline bool same_placement(const TopoDS_Shape& before, const TopoDS_Shape& after) {
  const auto a = read_frame(before), b = read_frame(after);
  const double tolerance = Precision::Confusion();
  if (!a || !b || a->center.Distance(b->center) > tolerance ||
      std::abs(a->radius - b->radius) > tolerance) return false;
  std::array<bool, 3> used{};
  for (const auto& corner : a->corners) {
    int match = -1;
    for (int i = 0; i < 3; ++i) {
      if (corner.Distance(b->corners[i]) > tolerance) continue;
      if (match != -1 || used[i]) return false;
      match = i;
    }
    if (match < 0) return false;
    used[match] = true;
  }
  return true;
}

} // namespace cut_octant
