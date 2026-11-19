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
#include "../crates/fabstep-cad/src/torus_atlas_faces.hxx"
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
Model load(const std::filesystem::path &path, const TopoDS_Shape &shell, bool large) {
  const int sourceLimit=large?600000:100000;
  verify(std::filesystem::file_size(path) < (large?128:32) * 1024 * 1024, "unbounded mesh witness");
  std::ifstream in(path);
  int n, t, e;
  Model m;
  verify(bool(in >> n >> t >> e >> m.volume >> m.area) && n >= 3 && n <= 3*sourceLimit && t > 0 && t <= sourceLimit &&
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
  if(!large)verify(faces.Extent() + t <= 100000, "result face limit");
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
TopoDS_Shape build(Model &m, const TopoDS_Shape &original) {
  BRep_Builder bb;
  TopoDS_Compound residual;
  bb.MakeCompound(residual);
  for (auto& face:patch_faces(m)) bb.Add(residual,face);
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
    bool large=std::string(argv[1])=="--large-source";
    if (large || std::string(argv[1]) == "--source-only") {
      BRep_Builder builder;
      TopoDS_Compound empty;
      builder.MakeCompound(empty);
      original = empty;
    } else
      original = readCandidate(argv[1]);
    auto m = load(argv[2], original,large);
    if (argc == 5)
      loadCharts(argv[4], m);
    std::cerr << std::setprecision(14) << "vertex_motion_mm=" << m.vertexMotion
              << " spline_straightness_mm=" << m.splineDeviation << "\n";
    auto resultCount=::count(original,TopAbs_FACE)+m.triangles.size()+m.charts.size()-std::count(m.claimed.begin(),m.claimed.end(),true);
    verify(resultCount<=100000,"result face limit");
    auto shape = build(m, original);
    GProp_GProps props;
    BRepGProp::VolumeProperties(shape, props,1e-11,true);
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
    BRepGProp::VolumeProperties(reread, result,1e-11,true);
    std::cerr<<"adaptive_volume_before_mm3="<<props.Mass()<<" after_mm3="<<result.Mass()<<"\n";
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
