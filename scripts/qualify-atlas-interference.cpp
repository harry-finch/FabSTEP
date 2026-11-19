// Compare whole-shape OCCT and the bounded dispatcher on both sides of a leaf boundary.
#include "../crates/fabstep-cad/src/torus_atlas_interference.hxx"
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <TopExp.hxx>
#include <iostream>

namespace {
TopoDS_Face triangle(gp_Pnt a, gp_Pnt b, gp_Pnt c) {
  return BRepBuilderAPI_MakeFace(BRepBuilderAPI_MakePolygon(a, b, c, true).Wire()).Face();
}

void check_case(bool split, bool intersect) {
  const double far = split ? 900. : 12.;
  const double height = intersect ? 0. : 4.;
  const auto a = triangle({-far, -5, 0}, {2, -5, 0}, {2, 5, 0});
  const auto b = triangle({-2, 0, -1 + height}, {-2, 0, 1 + height}, {far, 0, height});
  BRep_Builder builder;
  TopoDS_Compound shape;
  builder.MakeCompound(shape);
  builder.Add(shape, a);
  builder.Add(shape, b);
  for (int i = -160; i < 161; ++i) {
    const double x = i * 6.;
    builder.Add(shape, triangle({x, 20, 0}, {x + 1, 20, 0}, {x, 21, 0}));
  }
  // A legitimate shared coedge and shared vertices must be accepted.
  builder.Add(shape, BRepPrimAPI_MakeBox(gp_Pnt(40, 30, 0), 2, 2, 2).Shape());
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  BRepAlgoAPI_Check whole(shape, false, true);
  bool valid = true;
  atlas_interference::Checker checker;
  try {
    checker.run(faces);
  } catch (const atlas_interference::Failure &) {
    valid = false;
  }
  size_t owner_a = 0, owner_b = 0;
  for (const auto &node : checker.tree)
    if (node.left < 0) {
      for (size_t j = node.first; j < node.last; ++j) {
        if (checker.faces[j].face.IsSame(a))
          owner_a = node.first + 1;
        if (checker.faces[j].face.IsSame(b))
          owner_b = node.first + 1;
      }
    }
  if (!owner_a || !owner_b || (split && owner_a == owner_b) || whole.HasErrors() ||
      whole.IsValid() == intersect || valid == intersect)
    throw std::runtime_error("Unexpected whole/bounded interference result or leaf assignment.");
  std::cout << "whole=" << whole.IsValid() << " bounded=" << valid << " leaf-a=" << owner_a
            << " leaf-b=" << owner_b << " faces=" << faces.Extent() << "\n";
}
} // namespace
int main() try {
  for (bool split : {false, true})
    for (bool intersect : {false, true})
      check_case(split, intersect);
} catch (const std::exception &failure) {
  std::cerr << failure.what() << "\n";
  return 1;
}
