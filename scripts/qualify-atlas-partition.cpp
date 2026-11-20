// Measure F3/F4 using the exact cross-pair stream and unchanged OCCT checker.
// A diagonal group, including a single face's self-interference, is never
// split.
#include "../crates/fabstep-cad/src/torus_atlas_interference.hxx"
#include "atlas-benchmark-fixtures.hxx"
#include <BRepAdaptor_Surface.hxx>
#include <Geom_BSplineSurface.hxx>
#include <STEPControl_Reader.hxx>
#include <TopExp.hxx>
#include <chrono>
#include <filesystem>
#include <iostream>
#include <map>
#include <optional>
#include <set>
using atlas_interference::Entry;

namespace {
struct Slab {
  gp_Trsf to_local;
  double lo, hi;
};

// Protect the actual surface, material 3D edges and vertices. Each native
// bounding box includes geometry tolerance; enlarge again by the largest
// stored subshape tolerance and OCCT's confusion length in millimetres.
std::optional<std::pair<double, double>> physical_z(const Entry &entry,
                                                    const gp_Trsf &tr) {
  if (!(entry.tolerance >= 0.) || !std::isfinite(entry.tolerance))
    return {};
  const auto shape = entry.face.Moved(TopLoc_Location(tr));
  Bnd_Box box;
  BRepBndLib::AddOptimal(shape, box, false, true);
  for (auto kind : {TopAbs_EDGE, TopAbs_VERTEX})
    for (TopExp_Explorer it(shape, kind); it.More(); it.Next())
      BRepBndLib::AddOptimal(it.Current(), box, false, true);
  if (box.IsVoid() || box.IsOpen())
    return {};
  box.Enlarge(entry.tolerance + Precision::Confusion());
  const double lo = box.CornerMin().Z(), hi = box.CornerMax().Z();
  if (!std::isfinite(lo) || !std::isfinite(hi))
    return {};
  return std::pair{lo, hi};
}
std::optional<Slab> collar(const Entry &e) {
  BRepAdaptor_Surface surface(e.face, false);
  if (surface.GetType() != GeomAbs_BSplineSurface)
    return {};
  const auto bs = surface.BSpline();
  if (bs->UDegree() != 1 || bs->VDegree() != 1 || bs->NbUPoles() != 2 ||
      bs->NbVPoles() != 2)
    return {};
  for (int u : {1, 2})
    for (int v : {1, 2})
      if (!(bs->Weight(u, v) > 0.) || !std::isfinite(bs->Weight(u, v)))
        return {};
  auto p = bs->Pole(1, 1);
  // A triangular fan has two coincident control corners. Select an
  // oriented slab from the other three regardless of the collapsed side.
  const std::array<gp_Pnt, 4> poles{p, bs->Pole(2, 1), bs->Pole(1, 2),
                                    bs->Pole(2, 2)};
  gp_Vec n;
  for (int i = 1; i < 4; ++i)
    for (int j = i + 1; j < 4; ++j) {
      auto candidate = gp_Vec(p, poles[i]).Crossed(gp_Vec(p, poles[j]));
      if (candidate.SquareMagnitude() > n.SquareMagnitude())
        n = candidate;
    }
  if (n.Magnitude() < Precision::Confusion())
    return {};
  gp_Trsf tr;
  tr.SetTransformation(gp_Ax3(p, gp_Dir(n)));
  auto bounds = physical_z(e, tr);
  if (!bounds)
    return {};
  double lo = bounds->first, hi = bounds->second;
  for (int u : {1, 2})
    for (int v : {1, 2}) {
      double z = bs->Pole(u, v).Transformed(tr).Z();
      if (!std::isfinite(z))
        return {};
      lo = std::min(lo, z - e.tolerance - Precision::Confusion());
      hi = std::max(hi, z + e.tolerance + Precision::Confusion());
    }
  return Slab{tr, lo, hi};
}
struct Result {
  size_t groups = 0, split = 0, calls = 0, skipped = 0, removed = 0;
  double hottest = 0.;
};
} // namespace

int main(int argc, char **argv) try {
  if (argc != 3 && argc != 4)
    throw std::runtime_error(
        "usage: program public.step base|split|slabs|both");
  if (argc == 4 && std::string(argv[3]) != "--roundtrip")
    throw std::runtime_error("unknown transfer mode");
  const std::string mode = argv[2];
  if (mode == "oracle") {
    auto faces = atlas_bench::source(argv[1], argc == 4);
    if (faces.Extent() > 500)
      throw std::runtime_error("Exhaustive public control exceeds 500 faces");
    BRep_Builder builder;
    TopoDS_Compound shape;
    builder.MakeCompound(shape);
    for (int i = 1; i <= faces.Extent(); ++i)
      builder.Add(shape, faces(i));
    atlas_interference::Checker::verify_group(shape, false);
    return 0;
  }
  if (mode != "base" && mode != "split" && mode != "slabs" && mode != "both")
    throw std::runtime_error("unknown mode");
  const bool partition = mode == "split" || mode == "both",
             slabs = mode == "slabs" || mode == "both";
  auto faces = atlas_bench::source(argv[1], argc == 4);
  using Clock = std::chrono::steady_clock;
  auto start = Clock::now();
  atlas_interference::Checker checker;
  checker.prepare(faces);
  std::map<size_t, std::optional<Slab>> candidates;
  if (slabs)
    for (const auto &e : checker.faces)
      candidates.emplace(e.id, collar(e));
  std::vector<std::pair<size_t, size_t>> kept;
  size_t seen = 0;
  Result stats;
  auto separated = [&](const Entry &a, const Entry &b) {
    const auto &bound = candidates.at(a.id);
    if (!bound)
      return false;
    const auto range = physical_z(b, bound->to_local);
    return range && (bound->hi < range->first || range->second < bound->lo);
  };
  auto edge = [&](const Entry &a, const Entry &b) {
    ++seen;
    if (slabs && (separated(a, b) || separated(b, a)))
      ++stats.skipped;
    else
      kept.emplace_back(a.id, b.id);
  };
  auto accept = [&](const TopoDS_Compound &group) {
    ++stats.groups;
    auto begin = Clock::now();
    if (!seen) {
      atlas_interference::Checker::verify_group(group, false);
      ++stats.calls;
    } else {
      std::map<size_t, std::set<size_t>> graph;
      for (auto [a, b] : kept) {
        graph[a].insert(b);
        graph[b].insert(a);
      }
      stats.removed += size_t(group.NbChildren()) - graph.size();
      std::set<size_t> visited;
      std::vector<TopoDS_Compound> components;
      BRep_Builder builder;
      for (const auto &[seed, neighbors] : graph) {
        if (!visited.insert(seed).second)
          continue;
        if (partition || components.empty())
          builder.MakeCompound(components.emplace_back());
        std::vector<size_t> stack{seed};
        while (!stack.empty()) {
          auto n = stack.back();
          stack.pop_back();
          builder.Add(components.back(), faces(int(n) + 1));
          for (auto b : graph.at(n))
            if (visited.insert(b).second)
              stack.push_back(b);
        }
      }
      if (components.size() > 1)
        ++stats.split;
      // A baseline compound keeps exactly the production face order.
      if (mode == "base") {
        atlas_interference::Checker::verify_group(group, false);
        ++stats.calls;
      } else
        for (const auto &c : components) {
          atlas_interference::Checker::verify_group(c, false);
          ++stats.calls;
        }
      seen = 0;
      kept.clear();
    }
    stats.hottest =
        std::max(stats.hottest,
                 std::chrono::duration<double>(Clock::now() - begin).count());
  };
  checker.visit(0, 0, accept, edge);
  std::cout << "groups=" << stats.groups << " split=" << stats.split
            << " calls=" << stats.calls << " separated_pairs=" << stats.skipped
            << " removed_members=" << stats.removed
            << " max_check_s=" << stats.hottest << " total_s="
            << std::chrono::duration<double>(Clock::now() - start).count()
            << "\n";
} catch (const std::exception &e) {
  std::cerr << e.what() << "\n";
  return 1;
}
