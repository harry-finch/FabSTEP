// The reflected solid has the same volume and the same axis-aligned box.
// Build with the local OCCT headers and TKPrim, TKTopAlgo, TKBRep, TKMath,
// TKG3d and TKernel.
#include "../crates/fabstep-cad/src/cut_octant_frame.hxx"
#include <BRepPrimAPI_MakeSphere.hxx>
#include <iostream>

int main() {
  const double r = 5., quarter = std::acos(-1.) / 2.;
  const auto solid = [=](gp_Pnt center, gp_Dir z, gp_Dir x) {
    return BRepPrimAPI_MakeSphere(gp_Ax2(center, z, x), r, 0., quarter, quarter).Solid();
  };
  const auto source = solid(gp::Origin(), gp::DZ(), gp::DX());
  const auto reordered = solid(gp::Origin(), gp::DX(), gp::DY());
  const auto reflected = solid(gp_Pnt(r, r, r), -gp::DZ(), -gp::DY());
  const auto other_corners = solid(gp::Origin(), gp::DZ(), -gp::DX());
  if (!cut_octant::same_placement(source, reordered) ||
      cut_octant::same_placement(source, reflected) ||
      cut_octant::same_placement(source, other_corners)) return 1;
  std::cout << "Octant placement: reordered axes preserved, reflected center and changed corners rejected.\n";
}
