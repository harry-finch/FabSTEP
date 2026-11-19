#pragma once

#include <gp_Pnt.hxx>
#include <vector>

// Samples are shared by every incident face of the same FORWARD edge. Values
// use its ascending 3D-curve parameters, include both ends, and are expressed
// in world coordinates, including the edge location.
struct TrimEdgeSample {
  double parameter;
  gp_Pnt point;
};
using TrimEdgeSamples = std::vector<TrimEdgeSample>;
