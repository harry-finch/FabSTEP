// Triangulate one simply connected, unwrapped isoparametric torus chart.
// Required boundary vertices keep their positions; only interior samples are added.
#pragma once
#include "trim_edge_samples.hxx"
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Curve2d.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepMesh_DataStructureOfDelaun.hxx>
#include <BRepMesh_Delaun.hxx>
#include <BRepTools_WireExplorer.hxx>
#include <BRep_Tool.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <ElSLib.hxx>
#include <NCollection_IncAllocator.hxx>
#include <Poly_Triangulation.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Wire.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <limits>
#include <map>
#include <stdexcept>
#include <vector>

namespace trimmed_torus_mesh {
using UV = std::array<double, 2>;
using Triangle = std::array<int, 3>;
struct Mesh {
  std::vector<UV> points;
  std::vector<Triangle> triangles;
  double bound = 0.;
};
inline void require(bool b, const char *why) {
  if (!b)
    throw std::runtime_error(why);
}
inline double orient(UV a, UV b, UV c) {
  return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
}
inline double sq(UV a, UV b) { return std::pow(a[0] - b[0], 2) + std::pow(a[1] - b[1], 2); }
inline double disk2(UV a, UV b, UV c) {
  std::array<double, 3> d{sq(a, b), sq(b, c), sq(c, a)};
  std::sort(d.begin(), d.end());
  if (d[2] >= d[0] + d[1])
    return d[2] / 4.;
  double twice = orient(a, b, c);
  return d[0] * d[1] * d[2] / (4. * twice * twice);
}
inline std::vector<Triangle> triangulate(const std::vector<UV> &p, size_t boundary) {
  Handle(BRepMesh_DataStructureOfDelaun) data =
      new BRepMesh_DataStructureOfDelaun(new NCollection_IncAllocator);
  IMeshData::VectorOfInteger nodes;
  for (size_t i = 0; i < p.size(); ++i) {
    auto k = data->AddNode(BRepMesh_Vertex(gp_XY(p[i][0], p[i][1]), int(i),
                                           i < boundary ? BRepMesh_Frontier : BRepMesh_Free));
    require(k == int(i + 1), "coincident nodes");
    nodes.Append(k);
  }
  for (size_t i = 0; i < boundary; ++i)
    data->AddLink(BRepMesh_Edge(int(i + 1), int((i + 1) % boundary + 1), BRepMesh_Frontier));
  BRepMesh_Delaun delaunay(data, nodes);
  std::vector<Triangle> result;
  for (IMeshData::IteratorOfMapOfInteger it(data->ElementsOfDomain()); it.More(); it.Next()) {
    const auto &element = data->GetElement(it.Key());
    if (element.Movability() == BRepMesh_Deleted)
      continue;
    int v[3];
    data->ElementNodes(element, v);
    Triangle tri;
    for (int i = 0; i < 3; ++i) {
      require(v[i] > 0 && size_t(v[i]) <= p.size(), "foreign node");
      tri[i] = v[i] - 1;
    }
    require(orient(p[tri[0]], p[tri[1]], p[tri[2]]) > 0., "flipped triangle");
    result.push_back(tri);
  }
  require(result.size() == 2 * p.size() - boundary - 2, "incomplete domain");
  return result;
}
inline Mesh build(std::vector<UV> wire, double major, double minor, double deflection, size_t limit,
                  double angle = .35) {
  constexpr double tau = 6.2831853071795864769;
  size_t n = wire.size();
  require(n >= 4 && n <= limit + 2 && limit <= 100000 && major > minor && minor > 0 &&
              std::isfinite(major) && std::isfinite(deflection) && deflection > 0 && angle > 0 &&
              angle <= 1.0,
          "torus chart input");
  for (auto uv : wire)
    for (double x : uv)
      require(std::isfinite(x), "nonfinite UV");
  for (size_t i = 1; i < n; ++i)
    for (int k = 0; k < 2; ++k)
      wire[i][k] += tau * std::round((wire[i - 1][k] - wire[i][k]) / tau);
  UV low = wire[0], high = low;
  double area2 = 0.;
  for (size_t i = 0; i < n; ++i) {
    auto a = wire[i], b = wire[(i + 1) % n];
    area2 += orient(wire[0], a, b);
    require(std::min(std::abs(a[0] - b[0]), std::abs(a[1] - b[1])) < 1e-7,
            "non-isoparametric trim");
    for (int k = 0; k < 2; ++k) {
      low[k] = std::min(low[k], a[k]);
      high[k] = std::max(high[k], a[k]);
    }
  }
  require(area2 > 0. && high[0] - low[0] <= tau + 1e-6 && high[1] - low[1] <= tau + 1e-6,
          "unwrapped trim");
  // Put A=R+r cos(v). Squaring the directional second derivative gives
  // A² du⁴ + 2 q du²dv² + r² dv⁴, with q=Rr cos(v)+2r²-r² cos²(v).
  // Bound q at the maximum of this concave quadratic over the actual chart.
  // The resulting diagonal metric bounds the interpolation error in every
  // direction. The same metric also limits angular variation (ds²<=du²+dv²).
  double min_cos = -1., max_cos = 1.;
  if (std::abs(low[1]) < 1000. && std::abs(high[1]) < 1000.) {
    const double a = low[1] - 1e-11, b = high[1] + 1e-11;
    const auto contains_phase = [&](double phase) {
      return std::ceil((a - phase) / tau) <= std::floor((b - phase) / tau);
    };
    if (!contains_phase(0.))
      max_cos = std::min(1., std::max(std::cos(a), std::cos(b)) + 1e-12);
    if (!contains_phase(tau * .5))
      min_cos = std::max(-1., std::min(std::cos(a), std::cos(b)) - 1e-12);
  }
  const double max_major = major + minor * max_cos;
  const double c = std::clamp(major / (2. * minor), min_cos, max_cos);
  const double mixed = minor * (major * c + minor * (2. - c * c));
  const double factor = std::sqrt(std::max(1., mixed / (max_major * minor))) * (1. + 1e-12);
  const double angular_scale2 = 8. * deflection / (angle * angle);
  const UV scales{std::sqrt(std::max(angular_scale2, factor * max_major)),
                  std::sqrt(std::max(angular_scale2, factor * minor))};
  const double radius = std::sqrt(2. * deflection);
  auto scaled = [&](UV a) { return UV{(a[0] - low[0]) * scales[0], (a[1] - low[1]) * scales[1]}; };
  std::vector<UV> pts;
  for (auto a : wire)
    pts.push_back(scaled(a));
  const auto extent = scaled(high);
  const double side = 1.69 * radius, row = std::sqrt(0.75) * side;
  require(extent[0] / side < limit && extent[1] / row < limit, "grid budget");
  require((extent[1] / row + 1.) * n <= 20000000., "scanline work budget");
  // Scanline parity on the bounded orthogonal wire.
  std::vector<double> intersections;
  for (int j = 0; (j + 0.43) * row < extent[1]; ++j) {
    const double y = (j + 0.43) * row;
    intersections.clear();
    for (size_t i = 0; i < n; ++i) {
      auto a = pts[i], b = pts[(i + 1) % n];
      if ((a[1] > y) != (b[1] > y))
        intersections.push_back(a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
    }
    std::sort(intersections.begin(), intersections.end());
    require(intersections.size() % 2 == 0, "odd boundary parity");
    for (size_t i = 0; i < intersections.size(); i += 2) {
      const double off = (j % 2) * .5 + .32;
      for (int k = int(std::ceil((intersections[i] + .20 * radius) / side - off));
           (k + off) * side < intersections[i + 1] - .20 * radius; ++k) {
        pts.push_back({(k + off) * side, y});
        require(2 * pts.size() - n - 2 <= limit, "triangle limit");
      }
    }
  }
  Mesh output;
  for (int pass = 0; pass < 14; ++pass) {
    output.triangles = triangulate(pts, n);
    output.bound = 0.;
    std::vector<UV> extra;
    for (auto tri : output.triangles) {
      auto a = pts[tri[0]], b = pts[tri[1]], c = pts[tri[2]];
      double bound = disk2(a, b, c) * .5;
      output.bound = std::max(output.bound, bound);
      if (bound <= deflection * (1. - 1e-9))
        continue;
      // A long constrained edge alone cannot be repaired by interior samples.
      // The midpoint of the longest internal edge remains inside the disk.
      int i = -1, j = -1;
      double longest = 0.;
      for (int k = 0; k < 3; ++k) {
        int a = tri[k], b = tri[(k + 1) % 3];
        const double length_squared = sq(pts[a], pts[b]);
        const size_t separation = size_t(std::abs(a - b));
        if (size_t(a) < n && size_t(b) < n && (separation == 1 || separation == n - 1)) {
          require(length_squared * .125 <= deflection * (1. - 1e-9), "long prescribed boundary");
        } else if (length_squared > longest) {
          i = a;
          j = b;
          longest = length_squared;
        }
      }
      require(i >= 0, "triangle without refinable edge");
      extra.push_back({(pts[i][0] + pts[j][0]) * .5, (pts[i][1] + pts[j][1]) * .5});
    }
    if (extra.empty()) {
      // Every trim segment occurs once, in wire order. Every internal edge has two
      // opposite uses. Reject omissions even if the Euler face count happens to match.
      std::map<std::pair<int, int>, std::array<int, 2>> incidence;
      for (const auto &tri : output.triangles)
        for (int k = 0; k < 3; ++k) {
          int a = tri[k], b = tri[(k + 1) % 3];
          ++incidence[std::minmax(a, b)][a > b];
        }
      for (size_t k = 0; k < n; ++k) {
        int a = int(k), b = int((k + 1) % n);
        auto it = incidence.find(std::minmax(a, b));
        require(it != incidence.end() && it->second[a > b] == 1 && it->second[a < b] == 0,
                "missing trim edge");
        incidence.erase(it);
      }
      for (const auto &entry : incidence)
        require(entry.second[0] == 1 && entry.second[1] == 1, "nonmanifold chart");
      // Barycentric Taylor error <= half the variance of the scaled vertices.
      // The smallest enclosing disk bounds that variance on the whole triangle.
      output.points = std::move(wire);
      for (size_t i = n; i < pts.size(); ++i)
        output.points.push_back({low[0] + pts[i][0] / scales[0], low[1] + pts[i][1] / scales[1]});
      return output;
    }
    std::sort(extra.begin(), extra.end());
    extra.erase(std::unique(extra.begin(), extra.end()), extra.end());
    require(2 * (pts.size() + extra.size()) - n - 2 <= limit, "refinement limit");
    pts.insert(pts.end(), extra.begin(), extra.end());
  }
  throw std::runtime_error("torus refinement budget");
}

// An iso-edge represented by a STEP spline must retain one coordinate along
// its whole span. Positive rational weights preserve the control hull and
// monotone control coordinates cannot turn back along the edge.
inline bool iso_curve(const BRepAdaptor_Curve2d &curve, const gp_Torus &torus) {
  const double uScale = torus.MajorRadius() + torus.MinorRadius(), vScale = torus.MinorRadius();
  if (curve.GetType() == GeomAbs_Line) {
    auto d = curve.Value(curve.LastParameter()).XY() - curve.Value(curve.FirstParameter()).XY();
    return std::min(std::abs(d.X()) * uScale, std::abs(d.Y()) * vScale) <= 1e-7;
  }
  if (curve.GetType() != GeomAbs_BSplineCurve)
    return false;
  const auto spline = curve.BSpline();
  if (spline->NbPoles() > 4096 || spline->IsPeriodic())
    return false;
  const auto start = spline->Pole(1);
  double du = 0., dv = 0.;
  for (int i = 1; i <= spline->NbPoles(); ++i) {
    const auto p = spline->Pole(i);
    if (!(spline->Weight(i) > 0.))
      return false;
    du = std::max(du, std::abs(p.X() - start.X()));
    dv = std::max(dv, std::abs(p.Y() - start.Y()));
  }
  const int varying = du * uScale > dv * vScale ? 1 : 2;
  const double direction = spline->Pole(spline->NbPoles()).Coord(varying) - start.Coord(varying);
  if (direction == 0. || std::min(du * uScale, dv * vScale) > 1e-7)
    return false;
  for (int i = 2; i <= spline->NbPoles(); ++i)
    if ((spline->Pole(i).Coord(varying) - spline->Pole(i - 1).Coord(varying)) * direction < 0.)
      return false;
  return true;
}

inline bool supports(const TopoDS_Face &face) {
  if (face.Orientation() != TopAbs_FORWARD)
    return false;
  BRepAdaptor_Surface surface(face);
  if (surface.GetType() != GeomAbs_Torus ||
      surface.Torus().MajorRadius() <= surface.Torus().MinorRadius())
    return false;
  TopExp_Explorer wires(face, TopAbs_WIRE);
  if (!wires.More())
    return false;
  auto wire = TopoDS::Wire(wires.Current());
  wires.Next();
  if (wires.More())
    return false;
  size_t edges = 0;
  for (BRepTools_WireExplorer ex(wire, face); ex.More(); ex.Next()) {
    const auto edge = ex.Current();
    if (++edges > 100000 || BRep_Tool::Degenerated(edge) || !BRep_Tool::SameParameter(edge))
      return false;
    if (!iso_curve(BRepAdaptor_Curve2d(edge, face), surface.Torus()))
      return false;
  }
  return edges > 0;
}

// A STEP vertex can have two slightly different UV images on incident
// isoparametric edges. Retain the constant coordinate of each incident edge.
// All boundary nodes in 3D still come from the same shared edge samples.
inline UV corner_uv(UV previous, UV end, UV start, UV next, const gp_Torus &torus, const gp_Pnt &vertex) {
  const auto delta = [](double a, double b) { return std::remainder(a - b, 6.2831853071795864769); };
  constexpr double angular = 1e-7, linear = 1e-7;
  if (std::min(std::abs(delta(previous[0], start[0])), std::abs(delta(previous[1], start[1]))) < angular)
    return start;
  // Only distinct, unambiguous isoparametric directions define a corner.
  for (int fixed = 0; fixed < 2; ++fixed) {
    const int other = 1 - fixed;
    if (std::abs(delta(previous[fixed], end[fixed])) >= angular ||
        std::abs(delta(previous[other], end[other])) < angular ||
        std::abs(delta(start[other], next[other])) >= angular ||
        std::abs(delta(start[fixed], next[fixed])) < angular)
      continue;
    const double du = delta(end[0], start[0]), dv = delta(end[1], start[1]);
    if (std::hypot((torus.MajorRadius() + torus.MinorRadius()) * du, torus.MinorRadius() * dv) > linear)
      return start;
    UV corner = start;
    corner[fixed] += delta(end[fixed], start[fixed]);
    if (ElSLib::Value(corner[0], corner[1], torus).Distance(vertex) <= linear)
      return corner;
  }
  return start;
}

template <class Samples>
Handle(Poly_Triangulation) mesh(const TopoDS_Face &face, double deflection, double angle,
                                size_t limit, const Samples &samples) {
  require(supports(face), "unsupported torus trim");
  BRepAdaptor_Surface surface(face);
  std::vector<UV> chart;
  std::vector<gp_Pnt> prescribed;
  struct Span { size_t first; UV end; };
  std::vector<Span> spans;
  gp_Pnt end;
  TopExp_Explorer wires(face, TopAbs_WIRE);
  for (BRepTools_WireExplorer ex(TopoDS::Wire(wires.Current()), face); ex.More(); ex.Next()) {
    auto edge = ex.Current();
    const auto &chain = samples(TopoDS::Edge(edge.Oriented(TopAbs_FORWARD)));
    require(chain.size() >= 2 && chart.size() + chain.size() - 1 <= limit + 2,
            "boundary sample budget");
    BRepAdaptor_Curve2d pc(edge, face);
    require(std::abs(chain.front().parameter - pc.FirstParameter()) < 1e-7 &&
                std::abs(chain.back().parameter - pc.LastParameter()) < 1e-7,
            "p-curve range");
    bool reverse = edge.Orientation() == TopAbs_REVERSED;
    const size_t first = chart.size();
    for (size_t i = 0; i < chain.size(); ++i) {
      const auto &item = chain[reverse ? chain.size() - 1 - i : i];
      const auto uv = pc.Value(item.parameter);
      require(surface.Value(uv.X(), uv.Y()).Distance(item.point) <= 1e-7, "p-curve location");
      if (i == 0 && !chart.empty())
        require(item.point.Distance(end) <= 1e-7, "open chart boundary");
      if (i + 1 == chain.size()) {
        spans.push_back({first, {uv.X(), uv.Y()}});
        end = item.point;
        break;
      }
      chart.push_back({uv.X(), uv.Y()});
      prescribed.push_back(item.point);
    }
  }
  require(!prescribed.empty() && end.Distance(prescribed.front()) <= 1e-7, "open trim wire");
  const auto torus = surface.Torus();
  for (size_t i = 0; i < spans.size(); ++i) {
    const auto &from = spans[i], &to = spans[(i + 1) % spans.size()];
    const size_t last = to.first ? to.first - 1 : chart.size() - 1;
    chart[to.first] = corner_uv(chart[last], from.end, chart[to.first], to.end, torus, prescribed[to.first]);
  }
  require(deflection > 5e-7, "deflection below edge tolerance");
  auto data = build(chart, surface.Torus().MajorRadius(), surface.Torus().MinorRadius(),
                    deflection - 5e-7, limit, angle);
  Handle(Poly_Triangulation) out =
      new Poly_Triangulation(int(data.points.size()), int(data.triangles.size()), true);
  for (size_t i = 0; i < data.points.size(); ++i) {
    auto uv = data.points[i];
    out->SetNode(int(i + 1), i < prescribed.size() ? prescribed[i] : surface.Value(uv[0], uv[1]));
    out->SetUVNode(int(i + 1), gp_Pnt2d(uv[0], uv[1]));
  }
  for (size_t i = 0; i < data.triangles.size(); ++i) {
    const auto t = data.triangles[i];
    auto uv =
        (out->UVNode(t[0] + 1).XY() + out->UVNode(t[1] + 1).XY() + out->UVNode(t[2] + 1).XY()) / 3.;
    gp_Pnt origin;
    gp_Vec du, dv;
    surface.D1(uv.X(), uv.Y(), origin, du, dv);
    gp_Vec ab(out->Node(t[0] + 1), out->Node(t[1] + 1)),
        ac(out->Node(t[0] + 1), out->Node(t[2] + 1));
    require(ab.Crossed(ac).Dot(du.Crossed(dv)) > 0., "chord triangle reverses material side");
    out->SetTriangle(int(i + 1), Poly_Triangle(t[0] + 1, t[1] + 1, t[2] + 1));
  }
  out->Deflection(data.bound + 5e-7);
  return out;
}
} // namespace trimmed_torus_mesh
