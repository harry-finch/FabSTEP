// Broad-phase exclusion for ring tori. A failure to certify disjointness
// leaves the original compound check in charge of the pair.
#pragma once
#include <BRepAdaptor_Surface.hxx>
#include <BRepBndLib.hxx>
#include <BRep_Tool.hxx>
#include <Precision.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_MapOfShape.hxx>
#include <TopoDS.hxx>
#include <gp_Torus.hxx>
#include <cmath>
#include <limits>
#include <optional>
#include <utility>
#include <vector>

namespace atlas_interference {
struct TorusFilter {
  struct Budget {
    size_t visits = 2'000'000;        // Includes repeated occurrences at seams.
    size_t torus_boxes = 250'000;
    size_t neighbor_boxes = 500'000;
    size_t comparisons = 32'000'000;
  } remaining;

  struct Cover {
    gp_Torus surface;
    double tolerance;
    std::vector<Bnd_Box> boundary;
  };
  std::vector<Cover> covers;
  // Each slot is evaluated at most once. An unbounded box means that the
  // face could not be enclosed completely with the available budget.
  std::vector<std::optional<Bnd_Box>> neighbors;

  static bool finite(const Bnd_Box &box) {
    if (box.IsVoid() || box.IsOpen())
      return false;
    for (const gp_Pnt &p : {box.CornerMin(), box.CornerMax()})
      if (!std::isfinite(p.X()) || !std::isfinite(p.Y()) || !std::isfinite(p.Z()))
        return false;
    return true;
  }

  template <class Add>
  bool collect(const TopoDS_Face &face, double tolerance, size_t &box_budget, Add add) {
    TopTools_MapOfShape seen;
    for (const auto type : {TopAbs_EDGE, TopAbs_VERTEX})
      for (TopExp_Explorer it(face, type); it.More(); it.Next()) {
        if (!remaining.visits || !box_budget)
          return false;
        --remaining.visits;
        const auto &shape = it.Current();
        if (!seen.Add(shape))
          continue;
        --box_budget;
        Bnd_Box box;
        BRepBndLib::AddOptimal(shape, box, false, true);
        box.Enlarge(tolerance + Precision::Confusion());
        if (!finite(box))
          return false;
        add(box);
      }
    return !seen.IsEmpty();
  }

  // A nonzero handle is valid only for this instance of the checker.
  size_t record(const TopoDS_Face &face, double tolerance) {
    if (!remaining.torus_boxes || !remaining.visits || !std::isfinite(tolerance))
      return 0;
    try {
      const BRepAdaptor_Surface surface(face, false);
      if (surface.GetType() != GeomAbs_Torus)
        return 0;
      const auto torus = surface.Torus();
      if (!(torus.MinorRadius() > 0. && torus.MajorRadius() > torus.MinorRadius() &&
            std::isfinite(torus.MajorRadius())))
        return 0;
      Cover cover{torus, tolerance, {}};
      const bool complete = collect(face, tolerance, remaining.torus_boxes,
                                    [&](const Bnd_Box &b) { cover.boundary.push_back(b); });
      if (complete) {
        covers.push_back(std::move(cover));
        return covers.size();
      }
    } catch (const Standard_Failure &) {
      // Optional broad phase. The native checker will examine the face.
    }
    return 0;
  }

  // Distance to the full ring torus is 1-Lipschitz, hence the distance at
  // the box centre minus its half diagonal bounds every point from below.
  // Include shape tolerance, kernel confusion and a scale-dependent reserve
  // for the translation, projection onto the orthonormal axes and norms.
  static bool outside_support(const Cover &cover, const Bnd_Box &box) {
    if (!finite(box))
      return false;
    const auto lo = box.CornerMin(), hi = box.CornerMax();
    const gp_Pnt mid((lo.XYZ() + hi.XYZ()) * .5);
    const double half = .5 * hi.Distance(lo);
    const auto &t = cover.surface;
    const auto origin = t.Location();
    const gp_Vec p(origin, mid);
    const auto &ax = t.Position();
    const double rho = std::hypot(p.Dot(ax.XDirection()), p.Dot(ax.YDirection()));
    const double distance = std::abs(std::hypot(rho - t.MajorRadius(), p.Dot(ax.Direction())) - t.MinorRadius());
    double scale = 1 + t.MajorRadius() + t.MinorRadius() + half;
    for (int k = 1; k <= 3; ++k)
      scale += std::abs(origin.Coord(k)) + std::abs(lo.Coord(k)) + std::abs(hi.Coord(k));
    const double threshold = half + cover.tolerance + Precision::Confusion() +
                             64 * std::numeric_limits<double>::epsilon() * scale;
    return std::isfinite(threshold) && std::isfinite(distance) && distance > threshold;
  }

  bool excludes(size_t torus, size_t neighbor, const TopoDS_Face &face, double tolerance, const Bnd_Box &bounds) {
    if (!torus || !remaining.comparisons || !std::isfinite(tolerance))
      return false;
    const auto &cover = covers[torus - 1];
    if (!outside_support(cover, bounds))
      return false;
    auto &whole = neighbors[neighbor];
    if (!whole) {
      whole = bounds;
      try {
        if (!collect(face, tolerance, remaining.neighbor_boxes, [&](const Bnd_Box &box) { whole->Add(box); }))
          whole->SetWhole();
      } catch (const Standard_Failure &) {
        whole->SetWhole();
      }
    }
    if (!outside_support(cover, *whole))
      return false;
    // A stored edge or vertex can deviate from its supporting surface.
    // Test their physical boxes on both sides before excluding the pair.
    for (const auto &edge_or_vertex : cover.boundary) {
      if (!remaining.comparisons)
        return false;
      --remaining.comparisons;
      if (!edge_or_vertex.IsOut(*whole))
        return false;
    }
    return true;
  }
};
} // namespace atlas_interference
