#pragma once
#include <cstddef>
struct AtlasCornerData {
  size_t vertex;
  double uv[2];
  int constant_u;
};
struct AtlasLoopData {
  const AtlasCornerData *corners;
  size_t count;
};
struct AtlasChartData {
  double center[3], major, minor, bound;
  int orientation;
  const size_t *owners;
  size_t owner_count;
  const AtlasLoopData *loops;
  size_t loop_count;
};
struct AtlasPlaneData {
  double normal[3];
  const size_t *boundary;
  size_t boundary_count;
  const size_t *owners;
  size_t owner_count;
};
