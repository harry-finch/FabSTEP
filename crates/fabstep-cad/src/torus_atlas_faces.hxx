// Shared construction of measured torus charts and their adjacent source triangles.
struct Arc {
  gp_Circ circle;
  double a, b;
  int start;
};
struct TrimVertex {
  int id, axis;
  double u, v;
};
struct TorusChart {
  Handle(Geom_ToroidalSurface) support;
  int sign;
  std::vector<std::vector<TrimVertex>> loops;
};
struct Model {
  std::vector<gp_Pnt> source, adjusted;
  std::vector<bool> fixed;
  std::vector<std::array<int, 3>> triangles;
  std::map<Key, Arc> arcs;
  std::vector<bool> claimed;
  std::vector<TorusChart> charts;
  std::set<Key> boundary;
  std::array<gp_Pnt, 2> bounds;
  double tolerance{.05};
  double volume{}, area{}, vertexMotion{}, splineDeviation{}, patchBound{};
};
TopoDS_Face torusFace(const TorusChart &chart, const std::vector<TopoDS_Vertex> &verts) {
  BRep_Builder builder;
  TopoDS_Face face;
  builder.MakeFace(face, chart.support, weld);
  for (auto &loop : chart.loops) {
    TopoDS_Wire wire;
    builder.MakeWire(wire);
    for (size_t j = 0; j < loop.size(); ++j) {
      const auto a = loop[j], b = loop[(j + 1) % loop.size()];
      bool u = a.axis == 0;
      double s = u ? a.v : a.u, e = u ? b.v : b.u;
      bool forward = s < e;
      auto iso = u ? chart.support->UIso(a.u) : chart.support->VIso(a.v);
      auto edge = BRepBuilderAPI_MakeEdge(iso, verts[forward ? a.id : b.id], verts[forward ? b.id : a.id],
                                          std::min(s, e), std::max(s, e))
                      .Edge();
      BRepAdaptor_Curve geometry(edge);
      double lower = geometry.FirstParameter(), upper = geometry.LastParameter();
      double shift = lower - std::min(s, e);
      verify(std::abs((upper - lower) - std::abs(s - e)) < 1e-9, "unexpected circle trim range");
      Handle(Geom2d_Line) pc = u ? new Geom2d_Line(gp_Pnt2d(a.u, -shift), gp_Dir2d(0, 1))
                                 : new Geom2d_Line(gp_Pnt2d(-shift, a.v), gp_Dir2d(1, 0));
      builder.UpdateEdge(edge, pc, face, weld);
      builder.Range(edge, face, lower, upper);
      if (!forward)
        edge.Reverse();
      builder.Add(wire, edge);
    }
    wire.Closed(true);
    if (chart.sign < 0)
      wire.Reverse();
    builder.Add(face, wire);
  }
  if (chart.sign < 0)
    face.Reverse();
  BRepCheck_Analyzer checker(face, true);
  verify(checker.IsValid(), "invalid trimmed torus");
  return face;
}
int binomial(int n, int k) {
  int c = 1;
  for (int i = 1; i <= k; ++i)
    c = c * (n + 1 - i) / i;
  return c;
}
template <class A, class B, class Op> auto bernsteinProduct(const A &a, const B &b, Op multiply) {
  constexpr auto n = std::tuple_size<A>::value - 1, m = std::tuple_size<B>::value - 1;
  using Value = decltype(multiply(a[0], b[0]));
  std::array<Value, n + m + 1> out{};
  for (unsigned i = 0; i <= n; ++i)
    for (unsigned j = 0; j <= m; ++j)
      out[i + j] += multiply(a[i], b[j]) * (double(binomial(n, i) * binomial(m, j)) / binomial(n + m, i + j));
  return out;
}
bool positiveBezier(std::array<double, 7> p, int depth = 0) {
  if (*std::min_element(p.begin(), p.end()) > 1e-14)
    return true;
  if (depth >= 10 || p.front() <= 1e-14 || p.back() <= 1e-14)
    return false;
  std::array<double, 7> left{}, right{};
  for (unsigned i = 0; i < 7; ++i) {
    left[i] = p[0];
    right[6 - i] = p[6 - i];
    for (unsigned j = 0; j < 6 - i; ++j)
      p[j] = (p[j] + p[j + 1]) / 2;
  }
  return positiveBezier(left, depth + 1) && positiveBezier(right, depth + 1);
}
bool orientedRuledArcs(const TColgp_Array2OfPnt &poles, const TColStd_Array2OfReal &weights,
                       const gp_Vec &normal) {
  // F(t,q) sweeps a segment between rational traces C0,C1. Both endpoint
  // determinants Ck' x (C1-C0)/t must be positive; the intermediate ones
  // are their convex combinations. All omitted denominators are positive.
  std::array<std::array<gp_Vec, 3>, 2> n;
  std::array<std::array<double, 3>, 2> w;
  std::array<std::array<gp_Vec, 4>, 2> derivatives;
  auto weighted = [](gp_Vec v, double x) { return v * x; };
  for (int k = 0; k < 2; ++k) {
    for (int i = 0; i < 3; ++i) {
      w[k][i] = weights(i + 1, k + 1);
      n[k][i] = gp_Vec(poles(1, 1), poles(i + 1, k + 1)) * w[k][i];
    }
    std::array<gp_Vec, 2> dn;
    std::array<double, 2> dw;
    for (int i = 0; i < 2; ++i) {
      dn[i] = 2 * (n[k][i + 1] - n[k][i]);
      dw[i] = 2 * (w[k][i + 1] - w[k][i]);
    }
    auto first = bernsteinProduct(dn, w[k], weighted), second = bernsteinProduct(n[k], dw, weighted);
    for (int i = 0; i < 4; ++i)
      derivatives[k][i] = first[i] - second[i];
  }
  auto first = bernsteinProduct(n[1], w[0], weighted), second = bernsteinProduct(n[0], w[1], weighted);
  std::array<gp_Vec, 4> gap;
  for (int i = 0; i < 4; ++i)
    gap[i] = (first[i + 1] - second[i + 1]) * (4. / (i + 1));
  for (auto &d : derivatives)
    if (!positiveBezier(
            bernsteinProduct(d, gap, [&](gp_Vec a, gp_Vec b) { return a.Crossed(b).Dot(normal); })))
      return false;
  return true;
}
// One triangle can meet two independently measured arcs. Their rational
// quadratic representations form two rows of a ruled Bezier surface, with the
// same apex for both rows. At fixed u, positive weights traverse the straight
// ruling monotonically even when the arc sweeps differ.
TopoDS_Face twoArcs(Model &m, const std::array<int, 3> &tri, int first) {
  const int apex = tri[first], b = tri[(first + 1) % 3], c = tri[(first + 2) % 3];
  TColgp_Array2OfPnt grid(1, 3, 1, 2);
  TColStd_Array2OfReal weights(1, 3, 1, 2);
  double slope[2]{}, motion[2]{};
  for (int k = 0; k < 2; ++k) {
    int end = k == 0 ? b : c;
    auto arc = m.arcs.at(key(apex, end));
    double angle = (arc.a + arc.b) / 2, weight = std::cos((arc.b - arc.a) / 2);
    verify(weight > .5, "unbounded circular trim sweep");
    auto center = arc.circle.Location(), mid = ElCLib::Value(angle, arc.circle);
    gp_Pnt pole = center.Translated(gp_Vec(center, mid) / weight);
    auto a = m.adjusted[apex], z = m.adjusted[end];
    grid(1, k + 1) = a;
    grid(2, k + 1) = pole;
    grid(3, k + 1) = z;
    weights(1, k + 1) = weights(3, k + 1) = 1.;
    weights(2, k + 1) = weight;
    // Bernstein coefficients of N = W * (curve - adjusted chord).
    auto p1 = (pole.XYZ() * (2 * weight) + a.XYZ() - a.XYZ() * (2 * weight) - z.XYZ()) / 3.;
    auto p2 = (pole.XYZ() * (2 * weight) + z.XYZ() - z.XYZ() * (2 * weight) - a.XYZ()) / 3.;
    double maxN = std::max(p1.Modulus(), p2.Modulus());
    double maxD = 3 * std::max({p1.Modulus(), (p2 - p1).Modulus(), p2.Modulus()});
    double ea = a.Distance(m.source[apex]), ez = z.Distance(m.source[end]);
    motion[k] = maxN / weight + std::max(ea, ez);
    slope[k] = maxD / weight + maxN * (2 * (1 - weight)) / (weight * weight) + ea + ez;
  }
  auto x = gp_Vec(m.source[apex], m.source[b]), y = gp_Vec(m.source[apex], m.source[c]);
  double e = std::max(slope[0], slope[1]), f = slope[0] + slope[1];
  // Ft differs from its planar value by <= e. Fv/t is a difference
  // of two averaged derivatives with error <= f because both start at A.
  verify(x.Crossed(y).Magnitude() >
                 e * ((y - x).Magnitude() + f) + std::max(x.Magnitude(), y.Magnitude()) * f ||
             orientedRuledArcs(grid, weights, x.Crossed(y).Normalized()),
         "two-arc triangle could fold");
  double bound = std::max(motion[0], motion[1]);
  verify(bound < m.tolerance, "two-arc triangle exceeds fidelity");
  m.patchBound = std::max(m.patchBound, bound);
  Handle(Geom_BezierSurface) surface = new Geom_BezierSurface(grid, weights);
  auto face = BRepBuilderAPI_MakeFace(surface, 0, 1, 0, 1, weld).Face();
  verify(BRepCheck_Analyzer(face, true).IsValid(), "invalid two-arc face");
  return face;
}
// In homogeneous coordinates the sign of the projected area of the fan
// at parameter t is sign(N(t) x N'(t)), where N is the weighted quadratic
// numerator relative to the actual apex. Evaluate its four Bernstein bounds.
TopoDS_Face anchoredFan(const TopoDS_Edge &base, const TopoDS_Vertex &apex) {
  BRepOffsetAPI_ThruSections loft(false, true, weld);
  loft.CheckCompatibility(false);
  loft.SetMutableInput(false);
  loft.AddWire(BRepBuilderAPI_MakeWire(base).Wire());
  loft.AddVertex(apex);
  loft.Build();
  verify(loft.IsDone(), "cannot build a ruled patch on the measured circular edge");
  TopExp_Explorer it(loft.Shape(), TopAbs_FACE);
  verify(it.More(), "missing edge-anchored patch");
  auto face = TopoDS::Face(it.Current());
  it.Next();
  verify(!it.More(), "unexpected ruled patch split");
  auto first = BRep_Tool::Pnt(TopExp::FirstVertex(base, true)),
       last = BRep_Tool::Pnt(TopExp::LastVertex(base, true));
  for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
    auto edge = TopoDS::Edge(e.Current());
    auto a = BRep_Tool::Pnt(TopExp::FirstVertex(edge, true)),
         b = BRep_Tool::Pnt(TopExp::LastVertex(edge, true));
    if (a.Distance(first) <= weld && b.Distance(last) <= weld)
      return face;
    if (a.Distance(last) <= weld && b.Distance(first) <= weld)
      return TopoDS::Face(face.Reversed());
  }
  throw std::runtime_error("ruled patch omits its circular base");
}
bool positiveArcFan(const Arc &s, gp_Pnt a, gp_Pnt b, gp_Pnt apex, const gp_Vec &normal) {
  double w = std::cos((s.b - s.a) / 2);
  auto center = s.circle.Location(), mid = ElCLib::Value((s.b + s.a) / 2, s.circle);
  gp_Pnt pole = center.Translated(gp_Vec(center, mid) / w);
  std::array<gp_Vec, 3> h{gp_Vec(apex, a), gp_Vec(apex, pole) * w, gp_Vec(apex, b)};
  double c[4] = {};
  constexpr int bin2[3] = {1, 2, 1}, bin3[4] = {1, 3, 3, 1};
  for (int i = 0; i < 3; ++i)
    for (int j = 0; j < 2; ++j)
      c[i + j] += 2. * bin2[i] / bin3[i + j] * h[i].Crossed(h[j + 1] - h[j]).Dot(normal);
  return std::all_of(c, c + 4, [](double v) { return v > 1e-14; });
}
bool circularSegment(Model &m, const std::array<int, 3> &tri, int j) {
  int a = tri[j], b = tri[(j + 1) % 3], c = tri[(j + 2) % 3];
  auto first = m.arcs.at(key(a, b)), second = m.arcs.at(key(b, c));
  // Only merge consecutive uses of the same minor circle in its own plane.
  // Every intermediate point of both curves lies in the circular segment.
  if (first.circle.Location().Distance(second.circle.Location()) > 1e-9 ||
      std::abs(first.circle.Radius() - second.circle.Radius()) > 1e-9 ||
      first.circle.Axis().Direction().Angle(second.circle.Axis().Direction()) > 1e-10)
    return false;
  double angle1 = (first.b - first.a) * (first.start == a ? 1 : -1);
  double angle2 = (second.b - second.a) * (second.start == b ? 1 : -1);
  if (angle1 * angle2 <= 0 || std::abs(angle1 + angle2) >= M_PI)
    return false;
  gp_Vec source = gp_Vec(m.source[a], m.source[b]).Crossed(gp_Vec(m.source[a], m.source[c]));
  gp_Vec native = gp_Vec(m.adjusted[a], m.adjusted[b]).Crossed(gp_Vec(m.adjusted[a], m.adjusted[c]));
  if (source.Dot(native) <= 0)
    return false;
  // Join the middle point to the opposite chord to parameterize both
  // circular arcs by corresponding points in the measured planar triangle.
  double position = std::max({m.source[a].Distance(m.adjusted[a]), m.source[b].Distance(m.adjusted[b]),
                              m.source[c].Distance(m.adjusted[c])});
  double angle = std::max(std::abs(angle1), std::abs(angle2));
  const double bound = position + first.circle.Radius() * angle * angle / 8;
  if (!(bound < m.tolerance))
    return false;
  m.patchBound = std::max(m.patchBound, bound);
  return true;
}
std::vector<TopoDS_Face> patch_faces(Model &m) {
  // Most source vertices can be strictly inside a measured torus. The BRep
  // only needs the trim vertices and the unclaimed source triangle corners.
  std::vector<TopoDS_Vertex> vertices(m.adjusted.size());
  const auto prepare_vertex = [&](int id) {
    if (vertices[id].IsNull())
      vertices[id] = BRepBuilderAPI_MakeVertex(m.adjusted[id]).Vertex();
  };
  for (const auto &chart : m.charts)
    for (const auto &ring : chart.loops)
      for (const auto &corner : ring)
        prepare_vertex(corner.id);
  for (size_t owner = 0; owner < m.triangles.size(); ++owner)
    if (m.claimed.empty() || !m.claimed[owner])
      for (int id : m.triangles[owner])
        prepare_vertex(id);
  std::map<Key, TopoDS_Edge> edges;
  std::vector<TopoDS_Face> faces;
  for (auto &chart : m.charts)
    faces.push_back(torusFace(chart, vertices));
  for (size_t owner = 0; owner < m.triangles.size(); ++owner) {
    if (!m.claimed.empty() && m.claimed[owner])
      continue;
    auto tri = m.triangles[owner];
    std::array<TopoDS_Edge, 3> rim;
    int curved = -1, arcCount = 0;
    for (int j = 0; j < 3; ++j) {
      int a = tri[j], b = tri[(j + 1) % 3];
      Key k = key(a, b);
      auto &edge = edges[k];
      auto arc = m.arcs.find(k);
      if (arc != m.arcs.end()) {
        ++arcCount;
        curved = j;
      }
      if (edge.IsNull()) {
        if (arc == m.arcs.end())
          edge = BRepBuilderAPI_MakeEdge(vertices[k.first], vertices[k.second]).Edge();
        else {
          auto s = arc->second;
          int lo = s.a < s.b ? s.start : (s.start == a ? b : a), hi = lo == a ? b : a;
          edge = BRepBuilderAPI_MakeEdge(new Geom_Circle(s.circle), vertices[lo], vertices[hi],
                                         std::min(s.a, s.b), std::max(s.a, s.b))
                     .Edge();
          if (lo != k.first)
            edge.Reverse();
        }
      }
      rim[j] = a == k.first ? edge : TopoDS::Edge(edge.Reversed());
    }
    if (arcCount > 1) {
      verify(arcCount == 2, "three circular rims on one triangle");
      for (int j = 0; j < 3; ++j)
        if (!m.arcs.count(key(tri[j], tri[(j + 1) % 3]))) {
          if (circularSegment(m, tri, (j + 1) % 3)) {
            auto wire = BRepBuilderAPI_MakeWire(rim[0], rim[1], rim[2]).Wire();
            auto face = BRepBuilderAPI_MakeFace(wire, true).Face();
            verify(BRepCheck_Analyzer(face, true).IsValid(), "invalid circular segment");
            faces.push_back(face);
          } else {
            auto face = twoArcs(m, tri, (j + 2) % 3);
            faces.push_back(face);
          }
          break;
        }
      continue;
    }
    int j = curved >= 0 ? curved : 0;
    int a = tri[j], b = tri[(j + 1) % 3], z = tri[(j + 2) % 3];
    gp_Vec d(m.source[a], m.source[b]), side(m.source[a], m.source[z]);
    double area = d.Crossed(side).Magnitude();
    double eps = std::max({m.adjusted[a].Distance(m.source[a]), m.adjusted[b].Distance(m.source[b]),
                           m.adjusted[z].Distance(m.source[z])});
    double arcError = 0, derivative = 0;
    if (curved >= 0) {
      auto s = m.arcs.at(key(a, b));
      double sweep = std::abs(s.b - s.a);
      // Linear interpolation remainder and derivative remainder for a circle under normalized angle.
      arcError = s.circle.Radius() * sweep * sweep / 8;
      derivative += s.circle.Radius() * sweep * sweep / 2;
    }
    double disp = eps + arcError;
    m.patchBound = std::max(m.patchBound, disp);
    // Ruled patch F(t,v) = (1-v) C(t) + v A. Its normalized
    // projected Jacobian keeps the source sign on the whole triangle.
    gp_Vec actualD(m.adjusted[a], m.adjusted[b]), actualSide(m.adjusted[a], m.adjusted[z]);
    double span = std::max(actualSide.Magnitude(), m.adjusted[b].Distance(m.adjusted[z]));
    double projected = actualD.Crossed(actualSide).Dot(d.Crossed(side)) / area;
    double reserve = derivative * (span + arcError) + actualD.Magnitude() * arcError;
    bool positive = projected > reserve ||
                    (curved >= 0 && positiveArcFan(m.arcs.at(key(a, b)), m.adjusted[a], m.adjusted[b],
                                                   m.adjusted[z], d.Crossed(side).Normalized()));
    if (!positive && curved >= 0 && projected > 1e-14) {
      const auto &arc = m.arcs.at(key(a, b));
      // A nonplanar fan over a simple minor arc is injective: each ruling
      // fraction before the apex lies in a different plane parallel to the
      // arc. Linear interpolation of the arc with its chord also has strictly
      // increasing chord projection for sweeps below pi/2.
      gp_Pln plane(arc.circle.Location(), arc.circle.Axis().Direction());
      positive = plane.Distance(m.adjusted[z]) > 4e-6 && std::abs(arc.b - arc.a) < M_PI / 2;
    }
    verify(disp < m.tolerance && positive, "residual triangle could invert");
    TopoDS_Face face;
    if (curved < 0)
      face = BRepBuilderAPI_MakeFace(BRepBuilderAPI_MakeWire(rim[0], rim[1], rim[2]).Wire(), true).Face();
    else {
      // The planar maker guarantees a planar support only up to kernel tolerance.
      // Use the exact ruled patch if the three edges cannot share one plane.
      auto wire = BRepBuilderAPI_MakeWire(rim[0], rim[1], rim[2]).Wire();
      auto arc = m.arcs.at(key(a, b));
      gp_Pln plane(arc.circle.Location(), arc.circle.Axis().Direction());
      if (plane.Distance(m.adjusted[z]) < 1e-9) {
        face = BRepBuilderAPI_MakeFace(plane, wire, true).Face();
        for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
          if (it.Current().IsSame(rim[j])) {
            if (it.Current().Orientation() != rim[j].Orientation())
              face.Reverse();
            break;
          }
        }
      } else {
        // A shallow collar can use a regular triangular patch with the actual
        // material edges. Keep the loft when the source bound or orientation
        // cannot be certified for that alternative.
        if (const auto triangle = atlas_triangle::geometry(rim[j], m.adjusted[z])) {
          const auto &first = rim[j].Orientation() == TopAbs_FORWARD ? m.source[a] : m.source[b];
          const auto &last = rim[j].Orientation() == TopAbs_FORWARD ? m.source[b] : m.source[a];
          const double drift = std::max({triangle->apex.Distance(m.source[z]), triangle->first.Distance(first),
                                         triangle->last.Distance(last)});
          const double bound = drift + triangle->deviation;
          if (bound < m.tolerance) {
            if (auto regular = atlas_triangle::make(rim, j, *triangle, d.Crossed(side).Normalized())) {
              face = *regular;
              m.patchBound = std::max(m.patchBound, bound);
            }
          }
        }
        if (face.IsNull())
          face = anchoredFan(rim[j], vertices[z]);
      }
    }
    faces.push_back(face);
    verify(BRepCheck_Analyzer(face, true).IsValid(), "invalid residual face before sewing");
  }
  return faces;
}
