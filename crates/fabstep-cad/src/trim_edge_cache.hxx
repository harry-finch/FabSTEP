#pragma once

#include "trim_edge_samples.hxx"
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Geom2d_Curve.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_BSplineSurface.hxx>
#include <gp_Cone.hxx>
#include <gp_Cylinder.hxx>
#include <IMeshData_Status.hxx>
#include <Poly_PolygonOnTriangulation.hxx>
#include <Poly_Triangulation.hxx>
#include <Precision.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Vertex.hxx>
#include <gp_Circ.hxx>
#include <gp_Elips.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <limits>
#include <stdexcept>
#include <vector>

namespace trimcache {
inline void require(bool condition, const char *reason) {
  if (!condition)
    throw std::runtime_error(reason);
}
inline void budget(bool condition) {
  if (!condition)
    throw std::length_error("Budget de tessellation des contours dépassé.");
}
inline bool near(const gp_Pnt &a, const gp_Pnt &b) { return a.Distance(b) <= Precision::Confusion(); }
inline TopoDS_Edge forward(const TopoDS_Edge &edge) { return TopoDS::Edge(edge.Oriented(TopAbs_FORWARD)); }

// Positive rational Bernstein weights put the whole arc in the convex hull
// of its poles. Distance to a segment is convex. The derivative direction
// is a positive combination of forward pole differences (differentiate
// B_i B_j in the rational quotient); each can be written as a sum of
// consecutive differences. Bounding those differences certifies the turn.
struct WeightedPole {
  gp_XYZ numerator;
  double weight;
};

inline void bezier(const std::array<WeightedPole, 4> &poles, int degree, double lo, double hi,
                   double deflection, double cosine, size_t remaining, int depth, TrimEdgeSamples &out) {
  budget(out.size() < remaining);
  std::array<gp_Pnt, 4> p;
  for (int i = 0; i <= degree; ++i)
    p[i] = gp_Pnt(poles[i].numerator / poles[i].weight);
  const gp_Vec chord(p[0], p[degree]);
  const double length = chord.Magnitude();
  bool fits = std::isfinite(length) && length > 0.;
  for (int i = 0; i < degree && fits; ++i) {
    const gp_Vec tangent(p[i], p[i + 1]), offset(p[0], p[i]);
    const double position = std::clamp(offset.Dot(chord) / (length * length), 0., 1.);
    fits = tangent.Magnitude() > 0. && tangent.Dot(chord) >= cosine * tangent.Magnitude() * length &&
           p[0].Translated(chord * position).Distance(p[i]) <= deflection;
  }
  if (fits) {
    out.push_back({hi, p[degree]});
    return;
  }
  require(depth < 24, "Subdivision du contour de Bézier indémontrée.");
  auto row = poles;
  std::array<WeightedPole, 4> left{}, right{};
  for (int k = 0; k <= degree; ++k) {
    left[k] = row[0];
    right[degree - k] = row[degree - k];
    for (int j = 0; j < degree - k; ++j)
      row[j] = {(row[j].numerator + row[j + 1].numerator) * .5, (row[j].weight + row[j + 1].weight) * .5};
  }
  const double t = lo + (hi - lo) * .5;
  bezier(left, degree, lo, t, deflection, cosine, remaining, depth + 1, out);
  bezier(right, degree, t, hi, deflection, cosine, remaining, depth + 1, out);
}

inline TrimEdgeSamples analytic(const TopoDS_Edge &edge, double deflection, double angle, size_t remaining) {
  const BRepAdaptor_Curve curve(edge);
  const double lo = curve.FirstParameter(), hi = curve.LastParameter();
  require(std::isfinite(lo) && std::isfinite(hi) && hi > lo, "Intervalle de courbe 3D invalide.");
  budget(remaining >= 2);
  TrimEdgeSamples out{{lo, curve.Value(lo)}};
  if (curve.GetType() == GeomAbs_BSplineCurve) {
    const auto spline = curve.BSpline();
    const int degree = spline->Degree();
    require(degree >= 1 && degree <= 3 && spline->NbPoles() == degree + 1 && spline->NbKnots() == 2 &&
                spline->Multiplicity(1) == degree + 1 && spline->Multiplicity(2) == degree + 1 &&
                lo == spline->Knot(1) && hi == spline->Knot(2),
            "Contour spline sans travée de Bézier complète.");
    std::array<WeightedPole, 4> poles{};
    for (int i = 0; i <= degree; ++i) {
      const double weight = spline->Weight(i + 1);
      require(std::isfinite(weight) && weight > 0., "Poids de contour rationnel invalide.");
      poles[i] = {spline->Pole(i + 1).XYZ() * weight, weight};
    }
    bezier(poles, degree, lo, hi, deflection, std::cos(angle * .5), remaining, 0, out);
    return out;
  }
  size_t subdivisions = 1;
  if (curve.GetType() != GeomAbs_Line) {
    const bool circle = curve.GetType() == GeomAbs_Circle;
    require(circle || curve.GetType() == GeomAbs_Ellipse, "Courbe commune sans échantillonnage certifié.");
    const double major = circle ? curve.Circle().Radius() : curve.Ellipse().MajorRadius();
    const double minor = circle ? major : curve.Ellipse().MinorRadius();
    require(std::isfinite(major) && minor > 0. && major >= minor, "Rayon de contour invalide.");
    const double step =
        std::min(4. * std::asin(std::sqrt(std::min(.5 * deflection / major, 1.))), angle * minor / major);
    const double count = std::ceil((hi - lo) / step);
    budget(std::isfinite(count) && count >= 1 && count <= remaining - 1);
    subdivisions = static_cast<size_t>(count);
  }
  out.reserve(subdivisions + 1);
  for (size_t i = 1; i <= subdivisions; ++i) {
    const double parameter = i == subdivisions ? hi : lo + (hi - lo) * (double(i) / subdivisions);
    out.push_back({parameter, curve.Value(parameter)});
  }
  return out;
}

// Native meshing is incremental but its allocations precede extraction of
// the final triangle count. Admit the ordinary subset before handing it to
// OCCT. This route is limited to the analytic neighbors and low-degree ruled
// single spans of the certified trim producer.
inline void admit_ordinary(const TopoDS_Shape &ordinary, double deflection, double angle, size_t limit) {
  TopTools_IndexedMapOfShape edges;
  size_t segments = 0;
  double grid_work = 0.;
  for (TopExp_Explorer it(ordinary, TopAbs_FACE); it.More(); it.Next()) {
    const auto face = TopoDS::Face(it.Current());
    const BRepAdaptor_Surface surface(face);
    for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
      const auto edge = forward(TopoDS::Edge(e.Current()));
      if (BRep_Tool::Degenerated(edge) || edges.Contains(edge))
        continue;
      edges.Add(edge);
      budget(segments < 3 * limit);
      segments += analytic(edge, deflection, angle, 3 * limit - segments + 1).size() - 1;
    }
    switch (surface.GetType()) {
    case GeomAbs_Plane:
      grid_work += 1.;
      break;
    case GeomAbs_Cylinder:
    case GeomAbs_Cone: {
      double radius;
      if (surface.GetType() == GeomAbs_Cylinder)
        radius = surface.Cylinder().Radius();
      else {
        const auto cone = surface.Cone();
        const double slope = std::sin(cone.SemiAngle());
        radius = std::max(std::abs(cone.RefRadius() + slope * surface.FirstVParameter()),
                          std::abs(cone.RefRadius() + slope * surface.LastVParameter()));
      }
      require(std::isfinite(radius) && radius > 0., "Rayon de la face voisine invalide.");
      const double step = std::min(angle, 4. * std::asin(std::sqrt(std::min(.5 * deflection / radius, 1.))));
      grid_work += std::max(1., std::ceil((surface.LastUParameter() - surface.FirstUParameter()) / step));
      break;
    }
    case GeomAbs_BSplineSurface: {
      const auto s = surface.BSpline();
      require(!s->IsUPeriodic() && !s->IsVPeriodic() && s->NbUKnots() == 2 && s->NbVKnots() == 2 &&
                  std::min(s->UDegree(), s->VDegree()) == 1 && std::max(s->UDegree(), s->VDegree()) <= 3 &&
                  s->NbUPoles() == s->UDegree() + 1 && s->NbVPoles() == s->VDegree() + 1,
              "La face voisine sort du domaine réglé à une travée.");
      double diameter = 0., min_weight = std::numeric_limits<double>::infinity(), max_weight = 0.;
      const auto origin = s->Pole(1, 1);
      for (int u = 1; u <= s->NbUPoles(); ++u)
        for (int v = 1; v <= s->NbVPoles(); ++v) {
          const double radius = origin.Distance(s->Pole(u, v)), weight = s->Weight(u, v);
          require(std::isfinite(radius) && std::isfinite(weight) && weight > 0.,
                  "Pôle de surface réglée invalide.");
          diameter = std::max(diameter, 2. * radius);
          min_weight = std::min(min_weight, weight);
          max_weight = std::max(max_weight, weight);
        }
      // Scale guard for native work; actual mesh sizes are counted separately.
      grid_work += std::max(1., std::sqrt(diameter / deflection) * max_weight / min_weight);
      break;
    }
    default:
      throw std::runtime_error("Support ordinaire hors du domaine de tessellation des contours.");
    }
    budget(std::isfinite(grid_work) && grid_work <= 3. * limit);
  }
}

struct Stats {
  size_t ordinary_triangles = 0, ordinary_faces = 0, mixed_edges = 0, analytic_edges = 0, samples = 0;
};

class Cache {
public:
  // 'special' is selected by the caller's surface certificates. Only its
  // boundaries enter this cache. All ordinary neighbors are meshed together
  // so OCCT also shares polygons between two ordinary faces. Degenerate UV
  // sides have no 3D edge and remain the surface mesher's responsibility.
  Cache(const TopoDS_Shape &shape, const TopTools_IndexedMapOfShape &special, double deflection, double angle,
        size_t triangle_limit) {
    require(special.Extent() > 0 && std::isfinite(deflection) && deflection > 0. && std::isfinite(angle) &&
                angle > 0. && angle < 3.141592653589793,
            "Paramètres de cache invalides.");
    budget(triangle_limit <= std::numeric_limits<size_t>::max() / 6 &&
           static_cast<size_t>(special.Extent()) <= triangle_limit);
    for (int i = 1; i <= special.Extent(); ++i)
      for (TopExp_Explorer e(special(i), TopAbs_EDGE); e.More(); e.Next()) {
        const auto edge = TopoDS::Edge(e.Current());
        if (!BRep_Tool::Degenerated(edge))
          edges.Add(forward(edge));
        budget(static_cast<size_t>(edges.Extent()) <= 3 * triangle_limit);
      }

    struct Use {
      TopoDS_Face face;
      TopoDS_Edge edge;
      bool special = false, reversed = false;
    };
    struct Pair {
      std::array<Use, 2> sides;
      int count = 0;
    };
    std::vector<Pair> pairs(edges.Extent());
    TopoDS_Compound ordinary;
    BRep_Builder builder;
    builder.MakeCompound(ordinary);
    size_t face_count = 0, special_count = 0, edge_uses = 0;
    for (TopExp_Explorer f(shape, TopAbs_FACE); f.More(); f.Next()) {
      const auto oriented = TopoDS::Face(f.Current());
      const auto face = TopoDS::Face(oriented.Oriented(TopAbs_FORWARD));
      const bool is_special = special.Contains(face);
      budget(++face_count <= triangle_limit);
      if (is_special)
        ++special_count;
      else {
        builder.Add(ordinary, oriented);
        ++stats.ordinary_faces;
      }
      for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
        budget(++edge_uses <= 6 * triangle_limit);
        const auto edge = TopoDS::Edge(e.Current());
        const int id = edges.FindIndex(edge);
        if (!id)
          continue;
        auto &pair = pairs[id - 1];
        require(pair.count < 2 &&
                    (edge.Orientation() == TopAbs_FORWARD || edge.Orientation() == TopAbs_REVERSED),
                "Usages non-manifold du contour partagé.");
        pair.sides[pair.count++] = {face, edge, is_special,
                                    (edge.Orientation() == TopAbs_REVERSED) !=
                                        (oriented.Orientation() == TopAbs_REVERSED)};
      }
    }
    require(special_count == static_cast<size_t>(special.Extent()), "Face spéciale absente ou répétée.");
    if (stats.ordinary_faces > 0) {
      admit_ordinary(ordinary, deflection, angle, triangle_limit);
      constexpr int errors = IMeshData_OpenWire | IMeshData_SelfIntersectingWire | IMeshData_Failure |
                             IMeshData_UnorientedWire | IMeshData_TooFewPoints | IMeshData_UserBreak;
      const BRepMesh_IncrementalMesh mesh(ordinary, deflection, false, angle, false);
      require(mesh.IsDone() && !(mesh.GetStatusFlags() & errors),
              "Tessellation des voisins ordinaires impossible.");
      for (TopExp_Explorer f(ordinary, TopAbs_FACE); f.More(); f.Next()) {
        TopLoc_Location location;
        const auto tri = BRep_Tool::Triangulation(TopoDS::Face(f.Current()), location);
        require(!tri.IsNull() && tri->NbTriangles() > 0, "Tessellation ordinaire vide.");
        budget(static_cast<size_t>(tri->NbTriangles()) <= triangle_limit - stats.ordinary_triangles);
        stats.ordinary_triangles += tri->NbTriangles();
      }
    }
    paths.reserve(pairs.size());
    // A triangle has three segment uses. Even without sharing, an edge with
    // at least one segment stores at most two samples per segment.
    const size_t sample_limit = 6 * triangle_limit;
    for (size_t k = 0; k < pairs.size(); ++k) {
      const auto &pair = pairs[k];
      require(pair.count == 2 && pair.sides[0].reversed != pair.sides[1].reversed,
              "Le contour exige deux usages opposés dans la coque.");
      const auto canonical = TopoDS::Edge(edges(static_cast<int>(k + 1)));
      require(BRep_Tool::SameParameter(canonical), "Paramétrages de contour incompatibles.");
      const size_t remaining = sample_limit - stats.samples;
      const Use &other = pair.sides[pair.sides[0].special ? 1 : 0];
      TrimEdgeSamples path;
      if (other.special) {
        ++stats.analytic_edges;
        path = analytic(canonical, deflection, angle, remaining);
      } else {
        ++stats.mixed_edges;
        TopLoc_Location location;
        const auto tri = BRep_Tool::Triangulation(other.face, location);
        const auto &polygon = BRep_Tool::PolygonOnTriangulation(other.edge, tri, location);
        require(!polygon.IsNull() && polygon->HasParameters() && polygon->NbNodes() >= 2,
                "Polygone paramétré absent sur la face voisine.");
        budget(static_cast<size_t>(polygon->NbNodes()) <= remaining);
        path.reserve(polygon->NbNodes());
        for (int i = 1; i <= polygon->NbNodes(); ++i) {
          const int node = polygon->Node(i);
          require(node > 0 && node <= tri->NbNodes(), "Indice de contour hors limites.");
          path.push_back({polygon->Parameter(i), tri->Node(node).Transformed(location.Transformation())});
        }
      }
      const BRepAdaptor_Curve curve(canonical);
      require(std::abs(path.front().parameter - curve.FirstParameter()) <= Precision::PConfusion() &&
                  std::abs(path.back().parameter - curve.LastParameter()) <= Precision::PConfusion() &&
                  near(path.front().point, BRep_Tool::Pnt(TopExp::FirstVertex(canonical))) &&
                  near(path.back().point, BRep_Tool::Pnt(TopExp::LastVertex(canonical))),
              "Extrémités du contour incompatibles.");
      for (size_t i = 0; i < path.size(); ++i) {
        require(std::isfinite(path[i].parameter) && (i == 0 || path[i].parameter > path[i - 1].parameter) &&
                    near(curve.Value(path[i].parameter), path[i].point),
                "Échantillon de contour hors de la courbe 3D.");
      }
      for (const auto &use : pair.sides) {
        double lo, hi;
        const auto pc = BRep_Tool::CurveOnSurface(use.edge, use.face, lo, hi);
        require(!pc.IsNull() && std::abs(path.front().parameter - lo) <= Precision::PConfusion() &&
                    std::abs(path.back().parameter - hi) <= Precision::PConfusion(),
                "Paramètres de p-courbe incompatibles.");
        const BRepAdaptor_Surface surface(use.face);
        for (const auto &sample : path) {
          const auto uv = pc->Value(sample.parameter);
          require(near(sample.point, surface.Value(uv.X(), uv.Y())),
                  "Échantillon de p-courbe hors du support.");
        }
      }
      stats.samples += path.size();
      paths.push_back(std::move(path));
    }
  }

  const TrimEdgeSamples &samples(const TopoDS_Edge &edge) const {
    const int id = edges.FindIndex(edge);
    require(id > 0 && edge.Orientation() == TopAbs_FORWARD, "Contour canonique inconnu ou inversé.");
    return paths[id - 1];
  }
  const Stats &counts() const { return stats; }

private:
  TopTools_IndexedMapOfShape edges;
  std::vector<TrimEdgeSamples> paths;
  Stats stats;
};
} // namespace trimcache
