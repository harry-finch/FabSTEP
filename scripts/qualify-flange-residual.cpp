// Laboratory reconstruction of a measured residual mesh against a qualified open STEP shell.
#include "qualify-flange-shell.hxx"
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepLib.hxx>
#include <BRepBndLib.hxx>
#include <Bnd_Box.hxx>
#include <DESTEP_Parameters.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <Poly_PolygonOnTriangulation.hxx>
#include <BRepTools.hxx>
#include <ElCLib.hxx>
#include <Geom_BSplineCurve.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <Geom_Circle.hxx>
#include <Geom_ToroidalSurface.hxx>
#include <Geom2d_Line.hxx>
#include <gp_Ax3.hxx>
#include <Geom_BezierSurface.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopoDS_Compound.hxx>
#include <fstream>
#include <iomanip>
#include <iostream>

namespace {
void verify(bool condition, const char *message) {
  if (!condition)
    throw std::runtime_error(message);
}
using Key = std::pair<int, int>;
Key key(int a, int b) { return std::minmax(a, b); }
constexpr double weld = 1e-7, fit = 1e-5;
gp_Pnt point(std::istream &in) {
  double x, y, z;
  verify(bool(in >> x >> y >> z) && std::isfinite(x) && std::isfinite(y) && std::isfinite(z),
         "invalid point");
  return {x, y, z};
}
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
  double volume{}, area{}, vertexMotion{}, splineDeviation{}, patchBound{};
};
TopoDS_Shape readCandidate(const std::filesystem::path &p) {
  verify(std::filesystem::file_size(p) <= 128 * 1024 * 1024, "STEP input size limit");
  STEPControl_Reader reader;
  verify(reader.ReadFile(p.c_str()) == IFSelect_RetDone && reader.TransferRoots() == 1, "STEP read failed");
  return reader.OneShape();
}
void adjust(Model &m, int id, gp_Pnt p, double limit) {
  double d = m.source[id].Distance(p);
  if (d > limit)
    std::cerr << "boundary drift=" << d << " admitted=" << limit << " id=" << id << "\n";
  verify(d <= limit, "boundary vertex differs from source");
  m.vertexMotion = std::max(m.vertexMotion, d);
  if (m.fixed[id])
    verify(m.adjusted[id].Distance(p) <= weld, "inconsistent shell occurrences at a source vertex");
  else {
    m.adjusted[id] = p;
    m.fixed[id] = true;
  }
}
// Positive-weight spline poles inside a cylinder enclose the entire curve.
// Ordered axial projections guarantee a one-to-one traversal of the chord.
double straightSpline(const BRepAdaptor_Curve &c, gp_Pnt a, gp_Pnt b) {
  Handle(Geom_BSplineCurve) spline = Handle(Geom_BSplineCurve)::DownCast(c.BSpline()->Copy());
  spline->Segment(c.FirstParameter(), c.LastParameter());
  const gp_Vec d(a, b);
  double end = d.SquareMagnitude(), last = 0, worst = 0;
  verify(end > 0, "zero spline chord");
  if (a.Distance(spline->Pole(1)) > b.Distance(spline->Pole(1)))
    spline->Reverse();
  for (int i = 1; i <= spline->NbPoles(); ++i) {
    gp_Vec v(a, spline->Pole(i));
    double u = v.Dot(d) / end;
    verify(spline->Weight(i) > 0 && u >= last - 1e-10 && u <= 1. + 1e-10, "spline reverses");
    worst = std::max(worst, (v - d * u).Magnitude());
    last = u;
  }
  verify(worst <= weld, "nonlinear spline requires an independent surface proof");
  return worst;
}
Model load(const std::filesystem::path &path, const TopoDS_Shape &shell) {
  verify(std::filesystem::file_size(path) < 32 * 1024 * 1024, "unbounded mesh witness");
  std::ifstream in(path);
  int n, t, e;
  Model m;
  verify(bool(in >> n >> t >> e >> m.volume >> m.area) && n >= 3 && n <= 300000 && t > 0 && t <= 100000 &&
             e >= 0 && e <= 3 * t && std::isfinite(m.volume) && m.volume > 0 && std::isfinite(m.area) &&
             m.area > 0,
         "invalid source counts or volume");
  m.bounds = {point(in), point(in)};
  for (int j = 0; j < n; ++j)
    m.source.push_back(point(in));
  m.adjusted = m.source;
  m.fixed.assign(n, false);
  std::set<int> ids;
  std::map<Key, int> incidence, uses;
  for (int i = 0; i < t; ++i) {
    int id;
    std::array<int, 3> vs{};
    verify(bool(in >> id >> vs[0] >> vs[1] >> vs[2]) && id >= 0 && ids.insert(id).second,
           "invalid source owner");
    for (int j = 0; j < 3; ++j) {
      int a = vs[j], b = vs[(j + 1) % 3];
      verify(a >= 0 && a < n && a != b, "invalid triangle");
      verify(++uses[key(a, b)] <= 2, "non-manifold source edge");
      incidence[key(a, b)] += a < b ? 1 : -1;
    }
    m.triangles.push_back(vs);
  }
  TopTools_IndexedMapOfShape faces, used;
  TopExp::MapShapes(shell, TopAbs_FACE, faces);
  verify(faces.Extent() + t <= 100000, "result face limit");
  const auto adj = ::adjacency(shell);
  int free = 0;
  for (int i = 1; i <= adj.Extent(); ++i)
    if (adj.FindFromIndex(i).Extent() == 1 && !::collapsedTip(TopoDS::Edge(adj.FindKey(i))))
      ++free;
  verify(free == e, "incomplete free edge witness");
  for (int i = 0; i < e; ++i) {
    int face, len;
    verify(bool(in >> face >> len) && face >= 1 && face <= faces.Extent() && len >= 2 && len <= n,
           "invalid edge chain");
    std::vector<int> chain(len);
    for (int &v : chain)
      verify(bool(in >> v) && v >= 0 && v < n, "bad chain vertex");
    gp_Pnt start = point(in), end = point(in);
    TopoDS_Edge edge;
    for (TopExp_Explorer it(faces(face), TopAbs_EDGE); it.More(); it.Next()) {
      auto candidate = TopoDS::Edge(it.Current());
      if (adj.FindFromKey(candidate).Extent() == 1 && !::collapsedTip(candidate) &&
          start.Distance(BRep_Tool::Pnt(TopExp::FirstVertex(candidate, true))) <= weld &&
          end.Distance(BRep_Tool::Pnt(TopExp::LastVertex(candidate, true))) <= weld) {
        verify(edge.IsNull(), "ambiguous shell edge");
        edge = candidate;
      }
    }
    verify(!edge.IsNull() && !used.Contains(edge), "missing or duplicated free shell edge");
    used.Add(edge);
    BRepAdaptor_Curve c(edge);
    double lo = c.FirstParameter(), hi = c.LastParameter();
    if (edge.Orientation() == TopAbs_REVERSED)
      std::swap(lo, hi);
    auto type = c.GetType();
    verify(type == GeomAbs_Circle || (len == 2 && (type == GeomAbs_Line || type == GeomAbs_BSplineCurve)),
           "unsupported trim support");
    if (type == GeomAbs_BSplineCurve)
      m.splineDeviation = std::max(m.splineDeviation, straightSpline(c, start, end));
    double prev = lo;
    for (int j = 0; j < len; ++j) {
      double param = j == 0 ? lo : hi;
      if (j > 0 && j + 1 < len) {
        double u = ElCLib::Parameter(c.Circle(), m.source[chain[j]]);
        double near = lo + (hi - lo) * j / (len - 1);
        param = u + 2 * M_PI * std::round((near - u) / (2 * M_PI));
      }
      adjust(m, chain[j], c.Value(param), j == 0 || j + 1 == len ? fit : .05);
      if (j > 0) {
        int a = chain[j - 1], b = chain[j];
        auto k = key(a, b);
        verify((param - prev) * (hi - lo) > 0 && incidence.at(k) == (a < b ? -1 : 1) &&
                   m.boundary.insert(k).second,
               "unordered or duplicate source trim");
        if (type == GeomAbs_Circle)
          m.arcs.emplace(k, Arc{c.Circle(), prev, param, a});
      }
      prev = param;
    }
  }
  for (auto &[k, v] : incidence)
    verify(v == 0 || m.boundary.count(k) == 1, "undeclared mesh boundary");
  in >> std::ws;
  verify(in.eof(), "trailing mesh data");
  return m;
}
void loadCharts(const std::filesystem::path &path, Model &m) {
  verify(std::filesystem::file_size(path) < 12 * 1024 * 1024, "oversize trimmed chart witness");
  std::ifstream in(path);
  int n;
  verify(bool(in >> n) && n > 0 && n <= 1024, "invalid torus chart count");
  m.claimed.assign(m.triangles.size(), false);
  for (int i = 0; i < n; ++i) {
    auto center = point(in);
    double R, r, bound;
    int sign, owners, cycles;
    verify(bool(in >> R >> r >> sign >> owners >> cycles >> bound) && R > r && r > 0 &&
               (sign == 1 || sign == -1) && owners > 0 && size_t(owners) <= m.triangles.size() &&
               cycles > 0 && cycles <= owners && bound >= 0 && bound <= .05,
           "invalid torus chart");
    std::set<int> verts;
    std::map<Key, int> incidence;
    std::vector<int> selected;
    for (int j = 0; j < owners; ++j) {
      int id;
      verify(bool(in >> id) && id >= 0 && size_t(id) < m.claimed.size() && !m.claimed[id],
             "duplicate chart owner");
      m.claimed[id] = true;
      selected.push_back(id);
      auto tri = m.triangles[id];
      for (int j = 0; j < 3; ++j) {
        verts.insert(tri[j]);
        incidence[key(tri[j], tri[(j + 1) % 3])] += tri[j] < tri[(j + 1) % 3] ? 1 : -1;
      }
    }
    TorusChart chart{
        new Geom_ToroidalSurface(gp_Ax3(center, gp_Dir(1, 0, 0), gp_Dir(0, 1, 0)), R, r), sign, {}};
    std::map<int, gp_Pnt2d> uv;
    for (int j = 0; j < cycles; ++j) {
      int len;
      verify(bool(in >> len) && len >= 3 && size_t(len) <= verts.size(), "bad torus boundary count");
      std::vector<TrimVertex> loop(len);
      for (auto &q : loop) {
        verify(bool(in >> q.id >> q.axis >> q.u >> q.v) && verts.count(q.id) &&
                   (q.axis == 0 || q.axis == 1) && std::isfinite(q.u) && std::isfinite(q.v) &&
                   uv.emplace(q.id, gp_Pnt2d(q.u, q.v)).second,
               "invalid torus corner");
        verify(!m.fixed[q.id], "torus chart touches a previously modified source vertex");
        adjust(m, q.id, chart.support->Value(q.u, q.v), .05);
      }
      for (int k = 0; k < len; ++k) {
        auto a = loop[k], b = loop[(k + 1) % len];
        auto keyId = key(a.id, b.id);
        verify(incidence.at(keyId) == (a.id < b.id ? 1 : -1), "chart trim disagrees with triangle incidence");
        incidence[keyId] = 0;
        bool u = a.axis == 0;
        double start = u ? a.v : a.u, end = u ? b.v : b.u;
        verify((u ? a.u == b.u : a.v == b.v) && std::abs(start - end) > 1e-12 && std::abs(start - end) < M_PI,
               "inconsistent isoparametric trim");
        auto iso = u ? chart.support->UIso(a.u) : chart.support->VIso(a.v);
        auto circle = Handle(Geom_Circle)::DownCast(iso);
        verify(!circle.IsNull() && circle->Value(start).Distance(m.adjusted[a.id]) <= weld &&
                   circle->Value(end).Distance(m.adjusted[b.id]) <= weld,
               "inconsistent torus iso circle");
        verify(m.arcs.emplace(keyId, Arc{circle->Circ(), start, end, a.id}).second,
               "already claimed trim edge");
      }
      chart.loops.push_back(std::move(loop));
    }
    for (auto &[k, v] : incidence)
      verify(v == 0, "incomplete chart trim");
    // Recompute the pointwise two-sided piecewise-affine UV certificate
    // against the source triangles, including every snapped trim vertex.
    for (int id : verts)
      if (!uv.count(id)) {
        auto p = m.source[id];
        double y = p.Y() - center.Y(), z = p.Z() - center.Z();
        auto lift = [](double t) { return t < -.075 ? t + 2 * M_PI : t; };
        uv.emplace(
            id, gp_Pnt2d(lift(std::atan2(z, y)), lift(std::atan2(p.X() - center.X(), std::hypot(y, z) - R))));
      }
    for (int id : selected) {
      auto tri = m.triangles[id];
      gp_Pnt2d a = uv[tri[0]], b = uv[tri[1]], c = uv[tri[2]];
      verify(sign * gp_Vec2d(a, b).Crossed(gp_Vec2d(a, c)) > 1e-12, "folded torus source triangle");
      double du = std::max({a.X(), b.X(), c.X()}) - std::min({a.X(), b.X(), c.X()}),
             dv = std::max({a.Y(), b.Y(), c.Y()}) - std::min({a.Y(), b.Y(), c.Y()});
      double drift = 0;
      for (int v : tri)
        drift = std::max(drift, chart.support->Value(uv[v].X(), uv[v].Y()).Distance(m.source[v]));
      double proof = drift + ((R + r) * du * du + r * dv * dv + 2 * r * du * dv) / 2;
      verify(proof <= bound + 1e-9, "torus witness understates source deviation");
    }
    m.charts.push_back(std::move(chart));
  }
  in >> std::ws;
  verify(in.eof(), "trailing chart data");
}
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
  verify(bound < .05, "two-arc triangle exceeds fidelity");
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
TopoDS_Face ruledFan(const Arc &s, gp_Pnt a, gp_Pnt b, gp_Pnt apex) {
  auto center = s.circle.Location();
  auto mid = ElCLib::Value((s.b + s.a) / 2, s.circle);
  double weight = std::cos((s.b - s.a) / 2);
  gp_Pnt pole = center.Translated(gp_Vec(center, mid) / weight);
  TColgp_Array2OfPnt grid(1, 3, 1, 2);
  TColStd_Array2OfReal w(1, 3, 1, 2);
  grid(1, 1) = a;
  grid(2, 1) = pole;
  grid(3, 1) = b;
  for (int i = 1; i <= 3; ++i) {
    grid(i, 2) = apex;
    w(i, 1) = w(i, 2) = i == 2 ? weight : 1.;
  }
  Handle(Geom_BezierSurface) surface = new Geom_BezierSurface(grid, w);
  return BRepBuilderAPI_MakeFace(surface, 0, 1, 0, 1, weld).Face();
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
bool circularSegment(const Model &m, const std::array<int, 3> &tri, int j) {
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
  return position + first.circle.Radius() * angle * angle / 8 < .05;
}
TopoDS_Shape build(Model &m, const TopoDS_Shape &original) {
  std::vector<TopoDS_Vertex> vertices;
  for (auto &p : m.adjusted)
    vertices.push_back(BRepBuilderAPI_MakeVertex(p).Vertex());
  std::map<Key, TopoDS_Edge> edges;
  BRep_Builder bb;
  TopoDS_Compound residual;
  bb.MakeCompound(residual);
  for (auto &chart : m.charts)
    bb.Add(residual, torusFace(chart, vertices));
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
          try {
            if (circularSegment(m, tri, (j + 1) % 3)) {
              auto wire = BRepBuilderAPI_MakeWire(rim[0], rim[1], rim[2]).Wire();
              auto face = BRepBuilderAPI_MakeFace(wire, true).Face();
              verify(BRepCheck_Analyzer(face, true).IsValid(), "invalid circular segment");
              bb.Add(residual, face);
            } else {
              auto face = twoArcs(m, tri, (j + 2) % 3);
              bb.Add(residual, face);
            }
          } catch (...) {
            std::cerr << "source index=" << owner << " vertices=" << tri[0] << "," << tri[1] << "," << tri[2]
                      << "\n";
            throw;
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
    if (!(disp < .05 && positive))
      std::cerr << "source index=" << owner << " projected=" << projected << " reserve=" << reserve
                << " displacement=" << disp << "\n";
    verify(disp < .05 && positive, "residual triangle could invert");
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
        // A rational circular arc and an angular circular arc trace the same
        // points with different normalized parameters. For long arcs retain
        // the original base TShape in the loft so sewing cannot lose it.
        double start = arc.a, end = arc.b, w = std::cos((end - start) / 2);
        auto p0 = ElCLib::Value(start, arc.circle), p2 = ElCLib::Value(end, arc.circle);
        auto center = arc.circle.Location();
        auto p1 = center.Translated(gp_Vec(center, ElCLib::Value((start + end) / 2, arc.circle)) / w);
        auto quarter = gp_Pnt((p0.XYZ() * 9 + p1.XYZ() * (6 * w) + p2.XYZ()) / (10 + 6 * w));
        if (quarter.Distance(ElCLib::Value((3 * start + end) / 4, arc.circle)) > 1e-6)
          face = anchoredFan(rim[j], vertices[z]);
        else
          face = ruledFan(arc, m.adjusted[a], m.adjusted[b], m.adjusted[z]);
      }
    }
    bb.Add(residual, face);
    verify(BRepCheck_Analyzer(face, true).IsValid(), "invalid residual face before sewing");
  }
  std::cerr << "built residual triangles=" << m.triangles.size() << " patch_bound_mm=" << m.patchBound
            << "\n";
  constexpr double sewingLimit = 1e-6;
  BRepBuilderAPI_Sewing sewing(sewingLimit, true, true, true, false);
  sewing.SetMaxTolerance(sewingLimit);
  if (::count(original, TopAbs_FACE) > 0)
    sewing.Add(original);
  sewing.Add(residual);
  sewing.Perform();
  auto sewn = sewing.SewedShape();
  std::cerr << "sewing free=" << sewing.NbFreeEdges() << " multiple=" << sewing.NbMultipleEdges()
            << " deleted_faces=" << sewing.NbDeletedFaces() << "\n";
  int nf = ::count(original, TopAbs_FACE) + m.triangles.size() + m.charts.size() -
           std::count(m.claimed.begin(), m.claimed.end(), true);
  verify(sewing.NbDeletedFaces() == 0 && sewing.NbMultipleEdges() == 0 && ::count(sewn, TopAbs_FACE) == nf,
         "sewing changed face attribution");
  verify(::count(sewn, TopAbs_SHELL) == 1, "disconnected result");
  const auto adj = ::adjacency(sewn);
  int open = 0;
  for (int i = 1; i <= adj.Extent(); ++i) {
    auto e = TopoDS::Edge(adj.FindKey(i));
    if (!BRep_Tool::Degenerated(e) && adj.FindFromIndex(i).Extent() != 2) {
      ++open;
      BRepAdaptor_Curve curve(e);
      auto a = curve.Value(curve.FirstParameter()), b = curve.Value(curve.LastParameter());
      std::cerr << "unpaired " << curve.GetType() << " from " << a.X() << "," << a.Y() << "," << a.Z()
                << " to " << b.X() << "," << b.Y() << "," << b.Z() << " supports ";
      for (auto &f : adj.FindFromIndex(i))
        std::cerr << BRepAdaptor_Surface(TopoDS::Face(f)).GetType() << " ";
      std::cerr << "\n";
    }
  }
  std::cerr << "unpaired_material_edges=" << open << "\n";
  verify(open == 0, "open material seams");
  TopExp_Explorer it(sewn, TopAbs_SHELL);
  auto shell = TopoDS::Shell(it.Current());
  verify(BRep_Tool::IsClosed(shell), "shell closure");
  auto solid = BRepBuilderAPI_MakeSolid(shell).Solid();
  verify(BRepLib::OrientClosedSolid(solid), "solid orientation");
  BRepCheck_Analyzer validator(solid, true);
  verify(validator.IsValid(), "invalid closed solid");
  return solid;
}
void inspect(const TopoDS_Shape &shape, const std::filesystem::path &path, double deflection) {
  verify(::count(shape, TopAbs_SOLID) == 1 && BRepCheck_Analyzer(shape, true).IsValid(),
         "invalid inspection solid");
  BRepMesh_IncrementalMesh mesh(shape, deflection, false, .35, false);
  verify(mesh.IsDone() && mesh.GetStatusFlags() == 0, "incomplete native mesh");
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  const auto adj = ::adjacency(shape);
  std::ofstream out(path);
  out << std::setprecision(17);
  int total = 0;
  std::map<int, std::array<int, 3>> classes;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto face = TopoDS::Face(faces(i));
    TopLoc_Location loc;
    auto triangulation = BRep_Tool::Triangulation(face, loc);
    verify(!triangulation.IsNull() && triangulation->NbTriangles() > 0, "missing result tessellation");
    total += triangulation->NbTriangles();
    std::set<int> border;
    for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
      auto e = TopoDS::Edge(it.Current());
      auto poly = BRep_Tool::PolygonOnTriangulation(e, triangulation, loc);
      if (poly.IsNull())
        continue;
      for (int k = 1; k <= poly->NbNodes(); ++k)
        border.insert(poly->Node(k));
    }
    auto &stats = classes[BRepAdaptor_Surface(face).GetType()];
    stats[0]++;
    stats[1] += triangulation->NbTriangles();
    stats[2] += border.size();
    GProp_GProps props;
    BRepGProp::SurfaceProperties(face, props);
    auto center = props.CentreOfMass();
    std::set<int> neighbors;
    for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next())
      for (auto &other : adj.FindFromKey(it.Current())) {
        int j = faces.FindIndex(other);
        if (j != i)
          neighbors.insert(j - 1);
      }
    out << (i - 1) << " " << int(BRepAdaptor_Surface(face).GetType()) << " " << triangulation->NbTriangles()
        << " " << props.Mass() << " " << center.X() << " " << center.Y() << " " << center.Z() << " "
        << neighbors.size();
    for (int j : neighbors)
      out << " " << j;
    out << "\n";
  }
  for (auto &[kind, sum] : classes)
    std::cerr << "surface=" << kind << " faces=" << sum[0] << " triangles=" << sum[1]
              << " boundary_nodes=" << sum[2] << "\n";
  verify(out.good(), "face graph write failed");
  std::cerr << "faces=" << faces.Extent() << " mesh_triangles=" << total << " deflection_mm=" << deflection
            << "\n";
}

} // namespace
int main(int argc, char **argv) {
  try {
    const bool assess = argc > 1 && std::string(argv[1]) == "--assess";
    if (assess) {
      --argc;
      ++argv;
    }
    if (argc == 5 && std::string(argv[1]) == "--inspect") {
      double deflection = std::stod(argv[4]);
      verify(deflection >= .001 && deflection <= .01, "invalid deflection");
      inspect(readCandidate(argv[2]), argv[3], deflection);
      return 0;
    }
    verify(argc == 4 || argc == 5, "use OPEN_SHELL.step RESIDUAL.txt OUTPUT.step [TORUS_CHARTS.txt]");
    TopoDS_Shape original;
    if (std::string(argv[1]) == "--source-only") {
      BRep_Builder builder;
      TopoDS_Compound empty;
      builder.MakeCompound(empty);
      original = empty;
    } else
      original = readCandidate(argv[1]);
    auto m = load(argv[2], original);
    if (argc == 5)
      loadCharts(argv[4], m);
    std::cerr << std::setprecision(14) << "vertex_motion_mm=" << m.vertexMotion
              << " spline_straightness_mm=" << m.splineDeviation << "\n";
    auto shape = build(m, original);
    GProp_GProps props;
    BRepGProp::VolumeProperties(shape, props);
    std::cerr << "source_volume_mm3=" << m.volume << " native_volume_mm3=" << props.Mass() << "\n";
    STEPControl_Writer writer;
    DESTEP_Parameters params;
    params.InitFromStatic();
    params.WriteSurfaceCurMode = false;
    verify(writer.Transfer(shape, STEPControl_AsIs, params) == IFSelect_RetDone &&
               writer.Write(argv[3]) == IFSelect_RetDone,
           "solid STEP write failed");
    auto bytes = std::filesystem::file_size(argv[3]);
    std::cerr << "step_bytes=" << bytes << "\n";
    verify(bytes <= 128 * 1024 * 1024, "STEP output exceeds production byte limit");
    auto reread = readCandidate(argv[3]);
    verify(::count(reread, TopAbs_SOLID) == 1 && BRepCheck_Analyzer(reread, true).IsValid(),
           "invalid reimported solid");
    int nf = ::count(shape, TopAbs_FACE);
    verify(::count(reread, TopAbs_FACE) == nf, "changed reimported face count");
    GProp_GProps result;
    BRepGProp::VolumeProperties(reread, result);
    verify(std::isfinite(result.Mass()) && std::abs(result.Mass() - props.Mass()) <= props.Mass() * 1e-8 &&
               std::abs(result.Mass() - m.volume) <= .05 * m.area * 8,
           "volume changed");
    Bnd_Box originalBox, restoredBox;
    BRepBndLib::AddOptimal(shape, originalBox, false, false);
    BRepBndLib::AddOptimal(reread, restoredBox, false, false);
    double before[6], after[6];
    originalBox.Get(before[0], before[1], before[2], before[3], before[4], before[5]);
    restoredBox.Get(after[0], after[1], after[2], after[3], after[4], after[5]);
    for (int i = 0; i < 6; ++i)
      verify(std::abs(before[i] - after[i]) <= 1e-6 &&
                 std::abs(after[i] - m.bounds[i / 3].Coord(i % 3 + 1)) <= .05,
             "model dimensions changed");
    BRepMesh_IncrementalMesh mesh(reread, .01, false, .35, false);
    int triangles = 0;
    for (TopExp_Explorer it(reread, TopAbs_FACE); it.More(); it.Next()) {
      TopLoc_Location loc;
      auto triangulation = BRep_Tool::Triangulation(TopoDS::Face(it.Current()), loc);
      verify(!triangulation.IsNull() && triangulation->NbTriangles() > 0, "missing result face tessellation");
      triangles += triangulation->NbTriangles();
    }
    std::cerr << "faces=" << nf << " mesh_triangles=" << triangles
              << " mesher_flags=" << mesh.GetStatusFlags() << " reimport_volume_mm3=" << result.Mass()
              << "\n";
    verify(mesh.IsDone() && mesh.GetStatusFlags() == 0, "incomplete tessellation");
    std::cerr << "production_tessellation=" << (triangles <= 100000 ? "pass" : "refused") << "\n";
    verify(assess || triangles <= 100000, "production tessellation limits");
    return 0;
  } catch (const Standard_Failure &e) {
    std::cerr << e.GetMessageString() << "\n";
  } catch (const std::exception &e) {
    std::cerr << e.what() << "\n";
  }
  return 1;
}
