// One shared chord per side of a certified, regular triangular collar.
// Included alongside the atlas policy; inputs and output nodes are in world coordinates.
#pragma once
#include "torus_atlas_triangle.hxx"
#include "trim_edge_samples.hxx"
#include <Poly_Triangulation.hxx>

namespace atlas_triangle_mesh {
// Bound the actual reimported rational net against the affine triangle L=A+u(B-A)+v(C-A).
// Normalize O's weight to z relative to A. First take B,C of unit weight. Then
// D(S-L)=uv[z(O-A)-(B-A)-(C-A)+(1-z)(L-A)] and 0<=uv<=1/4 on the unit triangle.
// Perturbing B,C's weights moves a positive weighted mean by at most the net diameter
// times the range of the relative weight multipliers divided by their minimum.
inline double chord_bound(const Handle(Geom_BSplineSurface) &net, const std::array<gp_Pnt, 4> &p) {
  double diameter = 0.;
  for (const auto &a : p)
    for (const auto &b : p)
      diameter = std::max(diameter, a.Distance(b));
  const double w = net->Weight(1, 1), z = net->Weight(2, 2) / w;
  const double wb = net->Weight(2, 1) / w, wc = net->Weight(1, 2) / w;
  const double lo = std::min({1., wb, wc}), hi = std::max({1., wb, wc});
  // This also bounds the homogeneous denominator in the thin trim collar.
  if (!(std::min(lo, z) >= .5 && std::max(hi, z) <= 2.))
    return std::numeric_limits<double>::infinity();
  const gp_Vec b(p[0], p[1]), c(p[0], p[3]), o(p[0], p[2]);
  const double bound = ((o * z - b - c).Magnitude() + std::abs(1. - z) * std::max(b.Magnitude(), c.Magnitude())) /
                       std::min(4., 3. + z);
  // matches() certifies three monotone trims within 1e-8 of their UV sides
  // (and any collapsed corner inside the same radius). The simple wire and
  // ideal triangle enclose the same points outside that boundary strip.
  // Matching boundary points, or projecting from the strip, costs <=8e-8
  // in UV L1 distance, both ways. With weights in [.5,2], differentiating
  // the homogeneous net on [-2e-8,1+2e-8]^2 bounds each partial by 32 diameters.
  return bound + diameter * ((hi - lo) / lo + 32. * 8e-8 + 256. * std::numeric_limits<double>::epsilon());
}

inline bool normal_cone(const Handle(Geom_BSplineSurface) &net, const std::array<gp_Pnt, 4> &p,
                        const gp_Vec &normal, double angle) {
  const double length = normal.Magnitude();
  if (!(length > 0. && angle > 0. && angle <= 1.))
    return false;
  // The homogeneous normal numerator is bilinear. Its controls are the
  // corner turns multiplied by their three incident weights. Evaluate at
  // the corners of the slightly larger box allowed by the actual pcurves.
  const double w = net->Weight(1, 1);
  const std::array<double, 4> weights{1., net->Weight(2, 1) / w, net->Weight(2, 2) / w, net->Weight(1, 2) / w};
  std::array<gp_Vec, 4> turns;
  std::array<double, 4> error;
  for (size_t i = 0; i < turns.size(); ++i) {
    const auto prev = (i + 3) % 4, next = (i + 1) % 4;
    const gp_Vec a(p[prev], p[i]), b(p[i], p[next]);
    const double scale = weights[prev] * weights[i] * weights[next];
    turns[i] = a.Crossed(b) * scale;
    error[i] = 128. * std::numeric_limits<double>::epsilon() * a.Magnitude() * b.Magnitude() * scale;
  }
  const double cosine = std::cos(.5 * angle);
  for (const double u : {-2e-8, 1. + 2e-8}) {
    for (const double v : {-2e-8, 1. + 2e-8}) {
      const std::array<double, 4> coefficients{(1 - u) * (1 - v), u * (1 - v), u * v, (1 - u) * v};
      gp_Vec turn(0., 0., 0.);
      double rounding = 0.;
      for (size_t i = 0; i < turns.size(); ++i) {
        turn += turns[i] * coefficients[i];
        rounding += error[i] * std::abs(coefficients[i]);
      }
      // The cone is convex; every interpolated normal lies inside it.
      if (!(turn.Dot(normal) > length * (cosine * turn.Magnitude() + rounding)))
        return false;
    }
  }
  return true;
}

template <class Samples>
Handle(Poly_Triangulation) mesh(const TopoDS_Face &face, size_t remaining, double deflection, double angle,
                                const Samples &samples) {
  if (remaining == 0 || face.Orientation() != TopAbs_FORWARD || !atlas_triangle::matches(face))
    return {};
  const auto net = BRepAdaptor_Surface(face, false).BSpline();
  const std::array<gp_Pnt, 4> poles{net->Pole(1, 1), net->Pole(2, 1), net->Pole(2, 2), net->Pole(1, 2)};
  const std::array<gp_Pnt2d, 3> uv{gp_Pnt2d(0, 0), gp_Pnt2d(1, 0), gp_Pnt2d(0, 1)};
  std::array<TopoDS_Vertex, 3> vertices;
  std::array<int, 3> uses{};
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current().Oriented(TopAbs_FORWARD));
    if (BRep_Tool::Degenerated(edge))
      continue; // matches() certified the point and its incidences.
    const auto &path = samples(edge);
    if (path.size() != 2)
      return {};
    BRepAdaptor_Curve2d pc(edge, face);
    const std::array<TopoDS_Vertex, 2> ends{TopExp::FirstVertex(edge), TopExp::LastVertex(edge)};
    std::array<size_t, 2> corners{};
    for (size_t j = 0; j < 2; ++j) {
      const auto point = pc.Value(path[j].parameter);
      size_t k = 0;
      while (k < uv.size() && !(point.Distance(uv[k]) <= 3e-8))
        ++k;
      // An analytic curve endpoint and its neighboring mesh sample can
      // differ within native confusion. Both incidences must agree with the
      // same topological vertex, which supplies the emitted coordinate.
      if (k == uv.size() || ends[j].IsNull() ||
          !(path[j].point.Distance(BRep_Tool::Pnt(ends[j])) <= Precision::Confusion()) ||
          (!vertices[k].IsNull() && !vertices[k].IsSame(ends[j])))
        return {};
      vertices[k] = ends[j];
      ++uses[k];
      corners[j] = k;
    }
    if (corners[0] == corners[1])
      return {};
  }
  if (uses != std::array<int, 3>{2, 2, 2})
    return {};
  const std::array<gp_Pnt, 3> nodes{
      BRep_Tool::Pnt(vertices[0]), BRep_Tool::Pnt(vertices[1]), BRep_Tool::Pnt(vertices[2])};
  const auto normal = gp_Vec(nodes[0], nodes[1]).Crossed(gp_Vec(nodes[0], nodes[2]));
  const double anchor = std::max({nodes[0].Distance(poles[0]), nodes[1].Distance(poles[1]), nodes[2].Distance(poles[3])});
  const double bound = chord_bound(net, poles) + anchor + Precision::Confusion();
  if (!(bound < deflection) || !normal_cone(net, poles, normal, angle))
    return {};
  Handle(Poly_Triangulation) result = new Poly_Triangulation(3, 1, true);
  for (int k = 0; k < 3; ++k) {
    result->SetNode(k + 1, nodes[k]);
    result->SetUVNode(k + 1, uv[k]);
  }
  result->SetTriangle(1, Poly_Triangle(1, 2, 3));
  result->Deflection(bound);
  return result;
}
} // namespace atlas_triangle_mesh
