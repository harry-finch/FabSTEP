// After sewing and STEP transfer compare actual support controls, not only the
// surface enum or overall dimensions. Rational Bernstein functions form a
// convex partition. If paired weights differ by dw after normalization, the
// Cartesian error is bounded by max pole motion + diameter*dw/min(weight).
namespace atlas_transfer {
constexpr double tolerance = 1.e-6;
struct Net {
  int u, v;
  std::array<gp_Pnt, 8> points;
  std::array<double, 8> weights;
};
inline Net net(const BRepAdaptor_Surface &s) {
  const auto b =
      s.GetType() == GeomAbs_BSplineSurface ? s.BSpline() : GeomConvert::SurfaceToBSplineSurface(s.Bezier());
  if (b->NbUKnots() != 2 || b->NbVKnots() != 2 || b->NbUPoles() != b->UDegree() + 1 ||
      b->NbVPoles() != b->VDegree() + 1 || b->NbUPoles() * b->NbVPoles() > 8)
    throw CadFailure{mismatch, "Travée rationnelle de raccord modifiée."};
  Net a{b->NbUPoles(), b->NbVPoles(), {}, {}};
  double norm = 0.;
  for (int i = 0; i < a.u; ++i)
    for (int j = 0; j < a.v; ++j) {
      const int k = i * a.v + j;
      a.points[k] = b->Pole(i + 1, j + 1);
      a.weights[k] = b->Weight(i + 1, j + 1);
      if (!(a.weights[k] > 0.) || !std::isfinite(a.weights[k]))
        throw CadFailure{mismatch, "Poids rationnel invalide."};
      norm = std::max(norm, a.weights[k]);
    }
  for (double &w : a.weights)
    w /= norm;
  return a;
}
inline bool same_net(const Net &a, const Net &b) {
  for (int swap = 0; swap < 2; ++swap)
    for (int rev_u = 0; rev_u < 2; ++rev_u)
      for (int rev_v = 0; rev_v < 2; ++rev_v) {
        if (a.u != (swap ? b.v : b.u) || a.v != (swap ? b.u : b.v))
          continue;
        double pos = 0., weights = 0., min_w = 1., diam = 0.;
        for (int i = 0; i < a.u; ++i)
          for (int j = 0; j < a.v; ++j) {
            const int p = i * a.v + j, x = rev_u ? a.u - 1 - i : i, y = rev_v ? a.v - 1 - j : j;
            const int q = swap ? y * b.v + x : x * b.v + y;
            pos = std::max(pos, a.points[p].Distance(b.points[q]));
            weights = std::max(weights, std::abs(a.weights[p] - b.weights[q]));
            min_w = std::min(min_w, a.weights[p]);
            diam = std::max(diam, 2 * b.points[q].Distance(b.points[0]));
          }
        if (pos + diam * weights / min_w <= tolerance)
          return true;
      }
  return false;
}
inline bool spline(GeomAbs_SurfaceType type) {
  return type == GeomAbs_BezierSurface || type == GeomAbs_BSplineSurface;
}
inline bool same_support(const TopoDS_Face &a, const TopoDS_Face &b) {
  const BRepAdaptor_Surface x(a), y(b);
  if (spline(x.GetType()) && spline(y.GetType()))
    return same_net(net(x), net(y));
  if (x.GetType() != y.GetType())
    return false;
  if (x.GetType() == GeomAbs_Torus) {
    const auto p = x.Torus(), q = y.Torus();
    const double scale = p.MajorRadius() + p.MinorRadius();
    return p.Location().Distance(q.Location()) + std::abs(p.MajorRadius() - q.MajorRadius()) +
               std::abs(p.MinorRadius() - q.MinorRadius()) +
               scale * gp_Vec(p.Axis().Direction()).Crossed(gp_Vec(q.Axis().Direction())).Magnitude() <=
           tolerance;
  }
  if (x.GetType() != GeomAbs_Plane)
    return false;
  const auto q = y.Plane();
  for (TopExp_Explorer v(a, TopAbs_VERTEX); v.More(); v.Next())
    if (q.Distance(BRep_Tool::Pnt(TopoDS::Vertex(v.Current()))) > tolerance)
      return false;
  return true;
}
using EndPair = atlas_edge::Trace;
inline EndPair endpoints(const TopoDS_Edge &edge) {
  auto trace = atlas_edge::trace(edge);
  if (!trace)
    throw CadFailure{mismatch, "Le bord du raccord n’est ni droit ni un arc circulaire mesuré."};
  return *trace;
}
using Cell = std::array<long long, 3>;
inline Cell cell(gp_Pnt p) {
  Cell c{};
  for (int k = 0; k < 3; ++k) {
    const double v = std::floor(p.Coord(k + 1) / tolerance);
    if (!std::isfinite(v) || std::abs(v) > 9.e18)
      throw CadFailure{mismatch, "Position de contour imprécise."};
    c[k] = static_cast<long long>(v);
  }
  return c;
}
// Each toroidal trim is one witnessed minor iso arc. Its endpoints and
// support select a unique arc. Retain its midpoint too to reject traversal
// around the opposite side. A complete ruled chart has its four material or
// collapsed isoparametric sides; their geometry follows the checked net.
inline bool same_edges(const TopoDS_Face &before, const TopoDS_Face &after) {
  std::vector<EndPair> reference;
  std::map<Cell, std::vector<size_t>> bins;
  for (TopExp_Explorer it(before, TopAbs_EDGE); it.More(); it.Next()) {
    const auto e = TopoDS::Edge(it.Current());
    if (BRep_Tool::Degenerated(e))
      continue;
    const auto curve = endpoints(e);
    bins[cell(curve.a)].push_back(reference.size());
    if (cell(curve.a) != cell(curve.b))
      bins[cell(curve.b)].push_back(reference.size());
    reference.push_back(curve);
  }
  std::vector<bool> seen(reference.size());
  size_t count = 0;
  for (TopExp_Explorer it(after, TopAbs_EDGE); it.More(); it.Next()) {
    const auto e = TopoDS::Edge(it.Current());
    if (BRep_Tool::Degenerated(e))
      continue;
    const auto q = endpoints(e);
    const auto c = cell(q.a);
    std::optional<size_t> found;
    for (int x = -1; x <= 1; ++x)
      for (int y = -1; y <= 1; ++y)
        for (int z = -1; z <= 1; ++z) {
          const auto bucket = bins.find({c[0] + x, c[1] + y, c[2] + z});
          if (bucket == bins.end())
            continue;
          for (size_t i : bucket->second) {
            const auto &p = reference[i];
            // At Bernstein weight w, the middle homogeneous numerator is
            // (1+w)M-(A+B)/2. Both denominators exceed 1/2 on a minor arc.
            const double ends = std::min(std::max(p.a.Distance(q.a), p.b.Distance(q.b)),
                                         std::max(p.a.Distance(q.b), p.b.Distance(q.a)));
            const double extent = std::max(p.a.Distance(p.b), q.a.Distance(q.b));
            const double bound =
                2. * (ends + 2. * p.middle.Distance(q.middle) + 2. * extent * std::abs(p.weight - q.weight)) +
                p.bound + q.bound;
            if (seen[i] || bound > tolerance)
              continue;
            if ((p.a.Distance(q.a) <= tolerance && p.b.Distance(q.b) <= tolerance) ||
                (p.a.Distance(q.b) <= tolerance && p.b.Distance(q.a) <= tolerance)) {
              if (found && *found != i)
                return false;
              found = i;
            }
          }
        }
    if (!found)
      return false;
    seen[*found] = true;
    ++count;
  }
  return count == reference.size();
}
inline void verify(const TopoDS_Face &a, const TopoDS_Face &b) {
  if (!same_support(a, b) || !same_edges(a, b))
    throw CadFailure{mismatch,
                     "Un support ou ses contours ont changé lors de la couture ou du transfert STEP."};
}
} // namespace atlas_transfer
