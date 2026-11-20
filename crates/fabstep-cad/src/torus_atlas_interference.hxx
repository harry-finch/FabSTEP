#pragma once
#include "torus_atlas_filter.hxx"
#include <BRepAlgoAPI_Check.hxx>
#include <BRepBndLib.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <Precision.hxx>
#ifdef FABSTEP_LARGE_STL_SOURCE
#include <OSD_Parallel.hxx>
#include <OSD_ThreadPool.hxx>
#endif
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
// its stored tolerance. Diagonal groups are always checked. Cross groups
// contain all pairs not proved disjoint by bounds or the torus filter.
namespace atlas_interference {
#ifdef FABSTEP_LARGE_STL_SOURCE
// Set only by the standalone, single-job worker before any other OCCT call.
// A previously allocated pool with another size keeps the serial checker.
inline bool isolated_pool_ready = false;
inline void prepare_isolated_pool() noexcept {
  try {
    isolated_pool_ready = OSD_Parallel::NbLogicalProcessors() >= 2 &&
                          OSD_ThreadPool::DefaultPool(2)->NbThreads() == 2;
  } catch (...) {
    isolated_pool_ready = false;
  }
}
class ScopedCheckPool {
  bool restore_backend;
public:
  ScopedCheckPool() : restore_backend(isolated_pool_ready && !OSD_Parallel::ToUseOcctThreads()) {
    if (restore_backend)
      OSD_Parallel::SetUseOcctThreads(true);
  }
  ~ScopedCheckPool() {
    if (restore_backend)
      OSD_Parallel::SetUseOcctThreads(false);
  }
  bool parallel() const { return isolated_pool_ready && OSD_Parallel::ToUseOcctThreads(); }
};
#endif

struct Failure : std::runtime_error {
  using std::runtime_error::runtime_error;
};
struct Entry {
  TopoDS_Face face;
  Bnd_Box bounds;
  std::array<double, 3> center;
  size_t id, torus;
  double tolerance;
};
struct Node {
  Bnd_Box box;
  size_t first, last;
  int left = -1, right = -1;
};
struct Checker {
  static constexpr size_t leaf_size = 48;
  TorusFilter filter;
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
  bool torus_disjoint(const Entry &a, const Entry &b) {
    return filter.excludes(a.torus, b.id, b.face, b.tolerance, b.bounds) ||
           filter.excludes(b.torus, a.id, a.face, a.tolerance, a.bounds);
  }
  template <class Visit> void visit(int x, int y, const Visit &accept) {
    visit(x, y, accept, [](const Entry &, const Entry &) {});
  }
  template <class Visit, class Pair>
  void visit(int x, int y, const Visit &accept, const Pair &record_pair) {
    const auto &a = tree[x];
    const auto &b = tree[y];
    if (x != y && a.box.IsOut(b.box))
      return;
    if (a.left >= 0 && x == y) {
      visit(a.left, a.left, accept, record_pair);
      visit(a.left, a.right, accept, record_pair);
      visit(a.right, a.right, accept, record_pair);
      return;
    }
    if (a.left >= 0 && (b.left < 0 || a.last - a.first >= b.last - b.first)) {
      visit(a.left, y, accept, record_pair);
      visit(a.right, y, accept, record_pair);
      return;
    }
    if (b.left >= 0) {
      visit(x, b.left, accept, record_pair);
      visit(x, b.right, accept, record_pair);
      return;
    }
    TopoDS_Compound group;
    BRep_Builder builder;
    builder.MakeCompound(group);
    bool needs_check = x == y;
    if (x == y) {
      for (size_t i = a.first; i < a.last; ++i)
        builder.Add(group, faces[i].face);
    } else {
      face_box_tests += (a.last - a.first) * (b.last - b.first);
      if (face_box_tests > 80'000'000)
        throw Failure("Budget de comparaison des faces dépassé.");
      std::array<bool, leaf_size> add_b{};
      const size_t pairs_before = pairs;
      for (size_t i = a.first; i < a.last; ++i) {
        bool included = false;
        for (size_t j = b.first; j < b.last; ++j)
          if (!faces[i].bounds.IsOut(faces[j].bounds)) {
            ++pairs;
            if (torus_disjoint(faces[i], faces[j]))
              continue;
            record_pair(faces[i], faces[j]);
            add_b[j - b.first] = true;
            included = true;
          }
        if (included) {
          builder.Add(group, faces[i].face);
          needs_check = true;
        }
      }
      if (pairs_before == pairs)
        return;
      for (size_t j = b.first; j < b.last; ++j)
        if (add_b[j - b.first])
          builder.Add(group, faces[j].face);
    }
    // Charge the original AABB groups and pairs, including every filtered pair.
    if (++groups > 200000 || pairs > 10000000)
      throw Failure("Budget de contrôle des interférences dépassé.");
    if (!needs_check)
      return;
    accept(group);
  }
  static void verify_group(const TopoDS_Compound &group, bool parallel) {
    BRepAlgoAPI_Check check;
    check.SetData(group, false, true);
    check.SetRunParallel(parallel);
    check.Perform();
    if (check.HasErrors() || !check.IsValid())
      throw Failure("Le contrôle des auto-intersections du solide a échoué.");
  }
  void inspect(int x, int y, bool parallel = false) {
    visit(x, y, [parallel](const auto &group) { verify_group(group, parallel); });
  }
  void prepare(const TopTools_IndexedMapOfShape &input) {
    if (!faces.empty())
      *this = Checker{};
    filter.neighbors.resize(input.Extent());
    for (int i = 1; i <= input.Extent(); ++i) {
      const auto f = TopoDS::Face(input(i));
      Bnd_Box box;
      BRepBndLib::AddOptimal(f, box, false, true);
      if (box.IsVoid() || box.IsOpen())
        throw Failure("Face sans boîte de contrôle finie.");
      // Subshape tolerances can exceed the face tolerance used by AddOptimal.
      double tolerance = BRep_Tool::Tolerance(f);
      bool finite_tolerances = std::isfinite(tolerance) && tolerance >= 0.;
      for (const auto type : {TopAbs_EDGE, TopAbs_VERTEX})
        for (TopExp_Explorer it(f, type); it.More(); it.Next()) {
          const auto &shape = it.Current();
          const double t = type == TopAbs_EDGE ? BRep_Tool::Tolerance(TopoDS::Edge(shape))
                                               : BRep_Tool::Tolerance(TopoDS::Vertex(shape));
          finite_tolerances = finite_tolerances && std::isfinite(t) && t >= 0.;
          tolerance = std::max(tolerance, t);
        }
      box.Enlarge(tolerance + Precision::Confusion());
      auto mid = (box.CornerMin().XYZ() + box.CornerMax().XYZ()) * .5;
      if (!std::isfinite(tolerance) || !std::isfinite(mid.X()) || !std::isfinite(mid.Y()) ||
          !std::isfinite(mid.Z()))
        throw Failure("Face sans boîte de contrôle finie.");
      const double padding = finite_tolerances ? tolerance : std::numeric_limits<double>::quiet_NaN();
      faces.push_back({f, box, {mid.X(), mid.Y(), mid.Z()}, faces.size(), filter.record(f, padding), padding});
    }
    if (!faces.empty())
      build(0, faces.size());
  }
  void run(const TopTools_IndexedMapOfShape &input) {
    prepare(input);
    if (!faces.empty()) {
#ifdef FABSTEP_LARGE_STL_SOURCE
      ScopedCheckPool pool;
      inspect(0, 0, pool.parallel());
#else
      inspect(0, 0);
#endif
    }
  }
};
} // namespace atlas_interference
