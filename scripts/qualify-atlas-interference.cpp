// Compare whole-shape OCCT and the bounded dispatcher on both sides of a leaf boundary.
#include "../crates/fabstep-cad/src/torus_atlas_interference.hxx"
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <TopExp.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <IntTools_FaceFace.hxx>
#include <STEPControl_Reader.hxx>
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
// Mirrors the individual surface test in OCCT's BOPAlgo_CheckerSI 7.9 for
// residual spline faces; the complete solid still requires its inter-face check.
void check_ruled_faces(const char *path) {
  auto shape = [&] {
    STEPControl_Reader reader;
    if (reader.ReadFile(path) != IFSelect_RetDone || reader.TransferRoots() != 1)
      throw std::runtime_error("Cannot read one STEP root.");
    return reader.OneShape();
  }();
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  int checked = 0, invalid = 0, incomplete = 0;
  for (int index = 1; index <= faces.Extent(); ++index) {
    const auto face = TopoDS::Face(faces(index));
    const auto type = BRepAdaptor_Surface(face).GetType();
    if (type != GeomAbs_BSplineSurface && type != GeomAbs_BezierSurface)
      continue;
    IntTools_FaceFace check;
    check.Perform(face, face, false);
    ++checked;
    if (!check.IsDone())
      ++incomplete;
    else if (check.Lines().Length() || check.Points().Length())
      ++invalid;
  }
  std::cout << "spline_faces=" << checked << " self_intersections=" << invalid << " incomplete=" << incomplete
            << "\n";
  if (!checked || invalid || incomplete)
    throw std::runtime_error("A ruled face could not be verified.");
}
} // namespace
int main(int argc, char **argv) try {
  if (argc == 3 && std::string(argv[1]) == "--ruled-faces") {
    check_ruled_faces(argv[2]);
    return 0;
  }
  if (argc != 1)
    throw std::runtime_error("Expected --ruled-faces path.step or no arguments.");
  for (bool split : {false, true})
    for (bool intersect : {false, true})
      check_case(split, intersect);
} catch (const std::exception &failure) {
  std::cerr << failure.what() << "\n";
  return 1;
}
