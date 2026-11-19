// The detector proposes charts. Here the source vertices, oriented incidence,
// UV coverage, and continuous bounds are checked again before building OCCT faces.
namespace atlas_native {
void verify(bool value, const char *message) {
  if (!value)
    throw CadFailure{invalid_model, message};
}
using Key = std::pair<int, int>;
Key key(int a, int b) { return std::minmax(a, b); }
constexpr double weld = 1.e-7;
#include "torus_atlas_faces.hxx"

struct Measured {
  Model model;
  double bound = 0.;
};

Measured prepare(const double *points, size_t nv, const size_t *triangles, size_t nt, const double *direction,
                 const double *radial, const AtlasChartData *data, size_t nc, size_t result_count,
                 double tolerance) {
  verify(points && triangles && direction && radial && data && nc && nc <= 1024 && nv >= 4 &&
             nv <= source_vertex_limit && nt >= 4 && nt <= source_triangle_limit && result_count >= nc &&
             result_count <= result_face_limit && std::isfinite(tolerance) && tolerance > 0.,
         "Entrées de l’atlas torique hors limites.");
  Measured result;
  auto &m = result.model;
  m.tolerance = tolerance;
  m.source.reserve(nv);
  m.triangles.reserve(nt);
  for (size_t i = 0; i < nv; ++i) {
    auto p = point_at(points, i);
    verify(std::isfinite(p.X()) && std::isfinite(p.Y()) && std::isfinite(p.Z()), "Sommet source non fini.");
    m.source.push_back(p);
  }
  m.adjusted = m.source;
  m.fixed.assign(nv, false);
  m.claimed.assign(nt, false);
  std::map<Key, std::array<int, 2>> incidence;
  for (size_t i = 0; i < nt; ++i) {
    std::array<int, 3> t{};
    for (int j = 0; j < 3; ++j) {
      const auto n = triangles[3 * i + j];
      verify(n < nv, "Indice de triangle invalide.");
      t[j] = int(n);
    }
    for (int j = 0; j < 3; ++j) {
      int a = t[j], b = t[(j + 1) % 3];
      verify(a != b && ++incidence[key(a, b)][a > b] == 1, "Source non-manifold.");
    }
    const gp_Vec a(m.source[t[0]], m.source[t[1]]), b(m.source[t[0]], m.source[t[2]]);
    const double area = a.Crossed(b).Magnitude() / 2;
    verify(std::isfinite(area) && area > 0., "Triangle source dégénéré.");
    m.triangles.push_back(t);
  }
  for (const auto &e : incidence)
    verify(e.second[0] == 1 && e.second[1] == 1, "Arête source ouverte.");
  incidence.clear();
  const gp_Vec z(direction[0], direction[1], direction[2]), x(radial[0], radial[1], radial[2]);
  verify(std::abs(z.Magnitude() - 1.) < 1e-10 && std::abs(x.Magnitude() - 1.) < 1e-10 &&
             std::abs(z.Dot(x)) < 1e-10,
         "Repère de l’atlas torique invalide.");
  size_t claimed_count = 0, trim_count = 0;
  for (size_t ci = 0; ci < nc; ++ci) {
    const auto &d = data[ci];
    const auto center = point_at(d.center, 0);
    verify(std::isfinite(center.X()) && std::isfinite(center.Y()) && std::isfinite(center.Z()) &&
               std::isfinite(d.major) && d.major > d.minor && d.minor > weld && std::isfinite(d.bound) &&
               d.bound >= 0. && d.bound <= tolerance && (d.orientation == 1 || d.orientation == -1) &&
               d.owners && d.owner_count > 0 && d.owner_count <= nt - claimed_count && d.loops &&
               d.loop_count > 0 && d.loop_count <= d.owner_count,
           "Domaine torique invalide.");
    const gp_Ax3 frame(center, gp_Dir(z), gp_Dir(x));
    TorusChart chart{new Geom_ToroidalSurface(frame, d.major, d.minor), d.orientation, {}};
    std::set<int> vertices;
    std::map<Key, int> perimeter;
    for (size_t j = 0; j < d.owner_count; ++j) {
      size_t id = d.owners[j];
      verify(id < nt && !m.claimed[id], "Propriétaire torique dupliqué ou absent.");
      m.claimed[id] = true;
      ++claimed_count;
      const auto t = m.triangles[id];
      for (int k = 0; k < 3; ++k) {
        int a = t[k], b = t[(k + 1) % 3];
        vertices.insert(a);
        perimeter[key(a, b)] += a < b ? 1 : -1;
      }
    }
    std::map<int, gp_Pnt2d> uv;
    std::array<double, 2> low{INFINITY, INFINITY}, high{-INFINITY, -INFINITY};
    for (size_t j = 0; j < d.loop_count; ++j) {
      const auto &input = d.loops[j];
      verify(input.corners && input.count >= 3 && input.count <= vertices.size() &&
                 input.count <= 3 * nt - trim_count,
             "Longueur du contour torique invalide.");
      trim_count += input.count;
      std::vector<TrimVertex> loop;
      for (size_t k = 0; k < input.count; ++k) {
        const auto &corner = input.corners[k];
        verify(corner.vertex < nv && vertices.count(static_cast<int>(corner.vertex)) &&
                   (corner.constant_u == 0 || corner.constant_u == 1),
               "Sommet du contour absent.");
        int v = static_cast<int>(corner.vertex);
        for (int axis = 0; axis < 2; ++axis) {
          const double a = corner.uv[axis];
          verify(std::isfinite(a) && std::abs(a) <= 4 * M_PI, "Angle de contour invalide.");
          low[axis] = std::min(low[axis], a);
          high[axis] = std::max(high[axis], a);
        }
        const gp_Pnt2d p(corner.uv[0], corner.uv[1]);
        verify(uv.emplace(v, p).second && !m.fixed[v], "Contours toriques en contact ou répétés.");
        const auto snapped = chart.support->Value(p.X(), p.Y());
        const double motion = snapped.Distance(m.source[v]);
        verify(motion <= tolerance, "Sommet hors budget.");
        m.vertexMotion = std::max(m.vertexMotion, motion);
        m.adjusted[v] = snapped;
        m.fixed[v] = true;
        loop.push_back({v, corner.constant_u ? 0 : 1, p.X(), p.Y()});
      }
      for (size_t k = 0; k < loop.size(); ++k) {
        const auto a = loop[k], b = loop[(k + 1) % loop.size()];
        const auto e = key(a.id, b.id);
        auto hit = perimeter.find(e);
        verify(hit != perimeter.end() && hit->second == (a.id < b.id ? 1 : -1),
               "Contour hors du bord orienté.");
        perimeter.erase(hit);
        const bool fixed_u = a.axis == 0;
        const double first = fixed_u ? a.v : a.u, last = fixed_u ? b.v : b.u;
        verify((fixed_u ? a.u == b.u : a.v == b.v) && std::abs(last - first) > 1e-12 &&
                   std::abs(last - first) < M_PI,
               "Contour torique non isoparamétrique.");
        auto iso = fixed_u ? chart.support->UIso(a.u) : chart.support->VIso(a.v);
        auto circle = Handle(Geom_Circle)::DownCast(iso);
        verify(!circle.IsNull() && circle->Value(first).Distance(m.adjusted[a.id]) <= weld &&
                   circle->Value(last).Distance(m.adjusted[b.id]) <= weld &&
                   m.arcs.emplace(e, Arc{circle->Circ(), first, last, a.id}).second,
               "Arc torique ambigu.");
      }
      chart.loops.push_back(std::move(loop));
    }
    for (auto &e : perimeter)
      verify(e.second == 0, "Contour torique incomplet.");
    for (int k = 0; k < 2; ++k)
      verify(high[k] - low[k] <= 2 * M_PI + 1e-8, "Chart torique sur plusieurs tours.");
    for (int v : vertices)
      if (!uv.count(v)) {
        const gp_Vec off(center, m.source[v]);
        const double a = off.Dot(gp_Vec(frame.XDirection())), b = off.Dot(gp_Vec(frame.YDirection()));
        double theta[2] = {std::atan2(b, a),
                           std::atan2(off.Dot(gp_Vec(frame.Direction())), std::hypot(a, b) - d.major)};
        for (int k = 0; k < 2; ++k)
          theta[k] += 2 * M_PI * std::round(((low[k] + high[k]) / 2 - theta[k]) / (2 * M_PI));
        uv.emplace(v, gp_Pnt2d(theta[0], theta[1]));
      }
    for (size_t j = 0; j < d.owner_count; ++j) {
      const auto t = m.triangles[d.owners[j]];
      auto a = uv.at(t[0]), b = uv.at(t[1]), c = uv.at(t[2]);
      verify(d.orientation * gp_Vec2d(a, b).Crossed(gp_Vec2d(a, c)) > 1e-12, "Triangle torique UV replié.");
      double du = std::max({a.X(), b.X(), c.X()}) - std::min({a.X(), b.X(), c.X()}),
             dv = std::max({a.Y(), b.Y(), c.Y()}) - std::min({a.Y(), b.Y(), c.Y()});
      double drift = 0.;
      for (int v : t) {
        const auto p = uv.at(v);
        drift = std::max(drift, chart.support->Value(p.X(), p.Y()).Distance(m.source[v]));
      }
      const double proof =
          drift + ((d.major + d.minor) * du * du + d.minor * dv * dv + 2 * d.minor * du * dv) / 2;
      verify(proof <= d.bound + 1e-9 && proof <= tolerance, "Borne torique déclarée insuffisante.");
      result.bound = std::max(result.bound, proof);
    }
    m.charts.push_back(std::move(chart));
  }
  verify(result_count == nc + nt - claimed_count, "Nombre de faces de l’atlas différent du partage source.");
  return result;
}

TopoDS_Solid sew(const std::vector<TopoDS_Face> &originals, const size_t *tokens, FaceLabels &labels) {
  constexpr double limit = 1.e-6;
  BRepBuilderAPI_Sewing sewing(limit, true, true, true, false);
  sewing.SetMaxTolerance(limit);
  BRep_Builder builder;
  TopoDS_Compound patches;
  builder.MakeCompound(patches);
  for (const auto &face : originals)
    builder.Add(patches, face);
  sewing.Add(patches);
  sewing.Perform();
  verify(!sewing.NbDeletedFaces() && !sewing.NbMultipleEdges(), "La couture modifie le partage des faces.");
  const auto shape = sewing.SewedShape();
  TopTools_IndexedMapOfShape shells, faces, used;
  TopExp::MapShapes(shape, TopAbs_SHELL, shells);
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  verify(shells.Extent() == 1 && BRep_Tool::IsClosed(shells(1)) && size_t(faces.Extent()) == originals.size(),
         "Coque de l’atlas incomplète ou non unique.");
  auto solid = BRepBuilderAPI_MakeSolid(TopoDS::Shell(shells(1))).Solid();
  verify(BRepLib::OrientClosedSolid(solid), "Impossible d’orienter le solide torique.");
  for (size_t i = 0; i < originals.size(); ++i) {
    TopoDS_Shape f = originals[i];
    if (!faces.Contains(f) && sewing.IsModified(f))
      f = sewing.Modified(f);
    if (!faces.Contains(f) && sewing.IsModifiedSubShape(originals[i]))
      f = sewing.ModifiedSubShape(originals[i]);
    verify(!f.IsNull() && f.ShapeType() == TopAbs_FACE && faces.Contains(f) && !used.Contains(f),
           "Historique de couture des faces incomplet.");
    if (!originals[i].IsSame(f))
      atlas_transfer::verify(originals[i], TopoDS::Face(f));
    used.Add(f);
    labels.emplace_back(f, tokens[i]);
  }
  return solid;
}
} // namespace atlas_native
