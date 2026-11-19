#pragma once
#include <BRepAlgoAPI_Check.hxx>
#include <BRepBndLib.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <Precision.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <stdexcept>
#include <vector>

// Check self-interference with the same exact OCCT operation on bounded
// compounds. Tight physical face bounds cover every owned subshape, including
// its stored tolerance. Every face is checked and every pair with overlapping
// bounds occurs together in a check. Other pairs are geometrically disjoint.
namespace atlas_interference {
struct Failure : std::runtime_error {
  using std::runtime_error::runtime_error;
};
struct Entry {
  TopoDS_Face face;
  Bnd_Box bounds;
  std::array<double, 3> center;
};
struct Node {
  Bnd_Box box;
  size_t first, last;
  int left = -1, right = -1;
};
struct Checker {
  static constexpr size_t leaf_size = 48;
  std::vector<Entry> faces;
  std::vector<Node> tree;
  size_t groups = 0, pairs = 0, face_box_tests = 0;
  int build(size_t first, size_t last) {
    Node n{Bnd_Box(), first, last};
    std::array<double, 3> lo = faces[first].center, hi = lo;
    for (size_t i = first; i < last; ++i) {
      n.box.Add(faces[i].bounds);
      for (size_t k = 0; k < 3; ++k) {
        lo[k] = std::min(lo[k], faces[i].center[k]);
        hi[k] = std::max(hi[k], faces[i].center[k]);
      }
    }
    const int id = int(tree.size());
    tree.push_back(n);
    if (last - first <= leaf_size)
      return id;
    size_t k = 0;
    for (size_t j = 1; j < 3; ++j)
      if (hi[j] - lo[j] > hi[k] - lo[k])
        k = j;
    const size_t mid = first + (last - first) / 2;
    std::nth_element(faces.begin() + first, faces.begin() + mid, faces.begin() + last,
                     [k](const Entry &a, const Entry &b) { return a.center[k] < b.center[k]; });
    const int a = build(first, mid), b = build(mid, last);
    tree[id].left = a;
    tree[id].right = b;
    return id;
  }
  void inspect(int x, int y) {
    const auto &a = tree[x];
    const auto &b = tree[y];
    if (x != y && a.box.IsOut(b.box))
      return;
    if (a.left >= 0 && x == y) {
      inspect(a.left, a.left);
      inspect(a.left, a.right);
      inspect(a.right, a.right);
      return;
    }
    if (a.left >= 0 && (b.left < 0 || a.last - a.first >= b.last - b.first)) {
      inspect(a.left, y);
      inspect(a.right, y);
      return;
    }
    if (b.left >= 0) {
      inspect(x, b.left);
      inspect(x, b.right);
      return;
    }
    TopoDS_Compound group;
    BRep_Builder builder;
    builder.MakeCompound(group);
    if (x == y) {
      for (size_t i = a.first; i < a.last; ++i)
        builder.Add(group, faces[i].face);
    } else {
      face_box_tests += (a.last - a.first) * (b.last - b.first);
      if (face_box_tests > 80'000'000)
        throw Failure("Budget de comparaison des faces dépassé.");
      std::array<bool, leaf_size> add_b{};
      bool any = false;
      for (size_t i = a.first; i < a.last; ++i) {
        bool included = false;
        for (size_t j = b.first; j < b.last; ++j)
          if (!faces[i].bounds.IsOut(faces[j].bounds)) {
            add_b[j - b.first] = true;
            included = true;
            ++pairs;
          }
        if (included) {
          builder.Add(group, faces[i].face);
          any = true;
        }
      }
      if (!any)
        return;
      for (size_t j = b.first; j < b.last; ++j)
        if (add_b[j - b.first])
          builder.Add(group, faces[j].face);
    }
    if (++groups > 200000 || pairs > 10000000)
      throw Failure("Budget de contrôle des interférences dépassé.");
    BRepAlgoAPI_Check check(group, false, true);
    if (check.HasErrors() || !check.IsValid())
      throw Failure("Le contrôle des auto-intersections du solide a échoué.");
  }
  void run(const TopTools_IndexedMapOfShape &input) {
    for (int i = 1; i <= input.Extent(); ++i) {
      const auto f = TopoDS::Face(input(i));
      Bnd_Box box;
      BRepBndLib::AddOptimal(f, box, false, true);
      if (box.IsVoid() || box.IsOpen())
        throw Failure("Face sans boîte de contrôle finie.");
      // Subshape tolerances can exceed the face tolerance used by AddOptimal.
      double tolerance = BRep_Tool::Tolerance(f);
      for (TopExp_Explorer it(f, TopAbs_EDGE); it.More(); it.Next())
        tolerance = std::max(tolerance, BRep_Tool::Tolerance(TopoDS::Edge(it.Current())));
      for (TopExp_Explorer it(f, TopAbs_VERTEX); it.More(); it.Next())
        tolerance = std::max(tolerance, BRep_Tool::Tolerance(TopoDS::Vertex(it.Current())));
      box.Enlarge(tolerance + Precision::Confusion());
      auto mid = (box.CornerMin().XYZ() + box.CornerMax().XYZ()) * .5;
      if (!std::isfinite(tolerance) || !std::isfinite(mid.X()) || !std::isfinite(mid.Y()) ||
          !std::isfinite(mid.Z()))
        throw Failure("Face sans boîte de contrôle finie.");
      faces.push_back({f, box, {mid.X(), mid.Y(), mid.Z()}});
    }
    if (!faces.empty()) {
      build(0, faces.size());
      inspect(0, 0);
    }
  }
};
} // namespace atlas_interference
