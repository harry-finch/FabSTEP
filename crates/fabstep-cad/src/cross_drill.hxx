#pragma once
#include <TopoDS_Edge.hxx>
#include <TopoDS_Shape.hxx>
#include <cstdint>
#include <utility>
#include <vector>
struct CrossDrillData {
  double small_offset[2], center[3], axes[3][3], half[3], radii[2], large_axis_cosine;
  double fillet_radius;
  int32_t fillet_y_sign, fillet_z_sign;
};
struct CrossDrillResult {
  CrossDrillData data;
  double volume_low, volume_high;
  std::vector<std::pair<TopoDS_Edge, double>> mesh_edges;
};
// Throws on indeterminate incidence, geometry, certificate or budget.
CrossDrillResult verify_cross_drill(const TopoDS_Shape&, const CrossDrillData*, double deflection);
