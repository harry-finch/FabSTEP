// Independent public benchmark for R-130. Two serial OCCT checkers never share
// a mutable native shape. The second owns its own import and transfer session.
#include "../crates/fabstep-cad/src/torus_atlas_interference.hxx"
#include "atlas-benchmark-fixtures.hxx"
#include <BRepAdaptor_Surface.hxx>
#include <STEPControl_Reader.hxx>
#include <TopExp.hxx>
#include <atomic>
#include <chrono>
#include <filesystem>
#include <future>
#include <iostream>
#include <numeric>

namespace {
using Checker = atlas_interference::Checker;
using Clock = std::chrono::steady_clock;

struct Work {
  std::vector<TopoDS_Compound> groups;
  std::vector<std::vector<int>> members;
  std::vector<size_t> scores;
};

Work read_groups(const char *path, bool transfer) {
  auto faces = atlas_bench::source(path, transfer);
  Work work;
  Checker checker;
  checker.prepare(faces);
  checker.visit(0, 0, [&](const TopoDS_Compound &group) {
    std::vector<int> members;
    size_t score = 0;
    for (TopoDS_Iterator it(group); it.More(); it.Next()) {
      auto f = TopoDS::Face(it.Value());
      auto id = faces.FindIndex(f);
      if (!id)
        throw std::runtime_error("Unidentified face");
      members.push_back(id);
      auto type = BRepAdaptor_Surface(f, false).GetType();
      score += type == GeomAbs_BSplineSurface ? 4 : 1;
    }
    work.groups.push_back(group);
    work.members.push_back(std::move(members));
    work.scores.push_back(score);
  });
  return work;
}
} // namespace

int main(int argc, char **argv) try {
  if ((argc != 3 && argc != 4))
    throw std::runtime_error(
        "usage: program public.step single|intra|two|two-sorted");
  if (argc == 4 && std::string(argv[3]) != "--roundtrip")
    throw std::runtime_error("unknown transfer mode");
  const std::string mode = argv[2];
  const bool two = mode == "two" || mode == "two-sorted";
  if (!two && mode != "single" && mode != "intra")
    throw std::runtime_error("unknown mode");
  const auto start = Clock::now();
  auto first = read_groups(argv[1], argc == 4);
  Work second;
  if (two) {
    second = read_groups(argv[1], argc == 4);
    if (first.members != second.members)
      throw std::runtime_error("Workers have different face assignments");
  }
  std::vector<size_t> indices(first.groups.size());
  std::iota(indices.begin(), indices.end(), 0);
  if (mode == "two-sorted")
    std::stable_sort(indices.begin(), indices.end(), [&](size_t a, size_t b) {
      return first.scores[a] > first.scores[b];
    });
  std::atomic_size_t next{0}, completed{0};
  std::atomic_bool failed{false};
  if (mode == "intra")
    atlas_interference::prepare_isolated_pool();
  atlas_interference::ScopedCheckPool pool;
  const bool parallel = mode == "intra" && pool.parallel();
  if (mode == "intra" && !parallel)
    throw std::runtime_error("OCCT pool unavailable");
  std::atomic_uint active{0};
  auto run = [&](const Work &work) {
    while (!failed.load()) {
      auto ticket = next.fetch_add(1);
      if (ticket >= indices.size())
        break;
      try {
        if (++active == 2 && std::getenv("FABSTEP_BENCH_READY"))
          std::cout << "two-active" << std::endl;
        Checker::verify_group(work.groups[indices[ticket]], parallel);
      } catch (...) {
        failed.store(true);
        throw;
      }
      --active;
      ++completed;
    }
  };
  const auto planned = Clock::now();
  std::future<void> other;
  if (two)
    other = std::async(std::launch::async, run, std::cref(second));
  run(first);
  if (two)
    other.get();
  if (completed != indices.size())
    throw std::runtime_error("Incomplete verification");
  std::cout << "groups=" << completed << " preparation_s="
            << std::chrono::duration<double>(planned - start).count()
            << " total_s="
            << std::chrono::duration<double>(Clock::now() - start).count()
            << "\n";
  return 0;
} catch (const std::exception &e) {
  std::cerr << e.what() << "\n";
  return 1;
}
