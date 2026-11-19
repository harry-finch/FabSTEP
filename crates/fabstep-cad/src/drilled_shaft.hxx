#pragma once
#include <TopoDS_Shape.hxx>
#include <TopoDS_Edge.hxx>
#include <utility>
#include <vector>

struct DrilledShaftData {
  double origin[3], shaft[3], bore[3], height, radius, bore_radius, offset, level;
};
struct DrilledShaftProof {
  std::vector<std::pair<TopoDS_Edge, double>> mesh_edges;
  DrilledShaftData domain;
  double volume_low, volume_high;
};
DrilledShaftProof verify_drilled_shaft(const TopoDS_Shape &, double deflection);
bool same_shaft_geometry(const DrilledShaftData &, const DrilledShaftData &);
