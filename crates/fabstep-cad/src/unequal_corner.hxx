// Fixed-radius fillets on the three incident low edges of a Cartesian stock.
// Every accepted spline is checked against the regenerated OCC fillet net,
// its four trimmed sides, and the independently measured source interior.
namespace corner_domain {
using Data = std::array<std::array<double, 3>, 3>; // origin, sizes, radii
using unequal_corner::Patch;

inline void need(bool yes, Status stage) {
  if (!yes)
    throw CadFailure{stage, "Le coin à trois rayons ne respecte pas son domaine vérifié."};
}
int axis(const gp_Dir &d, Status stage) {
  for (int i = 0; i < 3; ++i)
    if (std::abs(d.Coord(i + 1)) > 1. - 1e-13)
      return i;
  need(false, stage);
  return 0;
}
TopoDS_Solid build(const Data &data, Status stage, bool prepare_transfer = false) {
  const auto &[lo, size, radii] = data;
  for (int i = 0; i < 3; ++i) {
    need(std::isfinite(lo[i]) && std::abs(lo[i]) < 1e6 && std::isfinite(size[i]) && size[i] > .001 &&
             size[i] <= 10000. && std::isfinite(radii[i]) && radii[i] > 1e-4 &&
             radii[i] < std::min(size[(i + 1) % 3], size[(i + 2) % 3]),
         stage);
    for (int j = 0; j < i; ++j)
      need(std::abs(radii[i] - radii[j]) > 1e-5, stage);
  }
  const auto origin = unequal_corner::point(lo);
  auto block = BRepPrimAPI_MakeBox(origin, size[0], size[1], size[2]).Shape();
  BRepFilletAPI_MakeFillet fillet(block);
  fillet.SetParams(1e-3, 1e-7, 1e-7, 1e-7, 1e-7, 1e-3);
  TopTools_IndexedMapOfShape edges;
  TopExp::MapShapes(block, TopAbs_EDGE, edges);
  for (int i = 1; i <= edges.Extent(); ++i) {
    const auto e = TopoDS::Edge(edges(i));
    TopoDS_Vertex a, b;
    TopExp::Vertices(e, a, b);
    auto p = BRep_Tool::Pnt(a), q = BRep_Tool::Pnt(b);
    if (std::min(p.Distance(origin), q.Distance(origin)) < 1e-7)
      fillet.Add(radii[axis(gp_Dir(gp_Vec(p, q)), stage)], e);
  }
  fillet.Build();
  need(fillet.IsDone(), stage);
  TopTools_IndexedMapOfShape solids;
  TopExp::MapShapes(fillet.Shape(), TopAbs_SOLID, solids);
  need(solids.Extent() == 1, stage);
  const auto solid = TopoDS::Solid(solids(1));
  if (!prepare_transfer)
    return solid;
  // A one-span quarter circle represented rationally can be reprojected
  // too coarsely by the STEP reader onto its cylinder. Knot insertion leaves
  // the 3D curve identical and gives the transfer 32 bounded intervals.
  TopTools_IndexedMapOfShape blends;
  TopExp::MapShapes(solid, TopAbs_EDGE, blends);
  for (int i = 1; i <= blends.Extent(); ++i) {
    const auto edge = TopoDS::Edge(blends(i));
    double low, high;
    const auto original = Handle(Geom_BSplineCurve)::DownCast(BRep_Tool::Curve(edge, low, high));
    if (original.IsNull())
      continue;
    const auto curve = Handle(Geom_BSplineCurve)::DownCast(original->Copy());
    for (int j = 1; j < 32; ++j)
      curve->InsertKnot(low + (high - low) * j / 32., curve->Degree(), 1.e-12);
    BRep_Builder().UpdateEdge(edge, curve, BRep_Tool::Tolerance(edge));
  }
  return solid;
}

void same(const Data &left, const Data &right, Status stage) {
  for (int i = 0; i < 3; ++i)
    for (int j = 0; j < 3; ++j)
      need(std::abs(left[i][j] - right[i][j]) < 1e-7, stage);
}

struct Proof {
  Data data{};
  std::array<TopoDS_Face, 10> faces;
  Patch patch{};
};

bool candidate(const TopTools_IndexedMapOfShape &faces) {
  if (faces.Extent() != 10)
    return false;
  int planes = 0, cylinders = 0, splines = 0;
  for (int i = 1; i <= 10; ++i) {
    const auto type = BRepAdaptor_Surface(TopoDS::Face(faces(i)), false).GetType();
    planes += type == GeomAbs_Plane;
    cylinders += type == GeomAbs_Cylinder;
    splines += type == GeomAbs_BSplineSurface;
  }
  return planes == 6 && cylinders == 3 && splines == 1;
}

Proof inspect(const TopoDS_Shape &solid, Status stage) {
  Proof proof;
  auto &[lo, size, radii] = proof.data;
  std::array<std::vector<std::pair<double, TopoDS_Face>>, 3> planes;
  for (TopExp_Explorer it(solid, TopAbs_FACE); it.More(); it.Next()) {
    const auto f = TopoDS::Face(it.Current());
    const BRepAdaptor_Surface surf(f, false);
    const auto t = surf.GetType();
    if (t == GeomAbs_Plane) {
      const auto p = surf.Plane();
      const int i = axis(p.Axis().Direction(), stage);
      planes[i].emplace_back(p.Location().Coord(i + 1), f);
    } else if (t == GeomAbs_Cylinder) {
      const auto cylinder = surf.Cylinder();
      const int i = axis(cylinder.Axis().Direction(), stage);
      need(proof.faces[6 + i].IsNull(), stage);
      proof.faces[6 + i] = f;
      radii[i] = cylinder.Radius();
    } else {
      need(t == GeomAbs_BSplineSurface && proof.faces[9].IsNull(), stage);
      proof.faces[9] = f;
      auto &p = proof.patch;
      p.surface = surf.BSpline();
      p.surface->Bounds(p.u0, p.u1, p.v0, p.v1);
      need(p.surface->NbUKnots() == 2 && p.surface->NbVKnots() == 2 && p.surface->UDegree() == 2 &&
               p.surface->VDegree() >= 2 && p.surface->VDegree() <= 9,
           stage);
    }
  }
  for (int i = 0; i < 3; ++i) {
    need(planes[i].size() == 2 && !proof.faces[6 + i].IsNull() && !proof.faces[9].IsNull(), stage);
    if (planes[i][0].first > planes[i][1].first)
      std::swap(planes[i][0], planes[i][1]);
    lo[i] = planes[i][0].first;
    size[i] = planes[i][1].first - lo[i];
    proof.faces[i] = planes[i][0].second;
    proof.faces[i + 3] = planes[i][1].second;
  }
  for (int i = 0; i < 3; ++i) {
    const auto cyl = BRepAdaptor_Surface(proof.faces[6 + i], false).Cylinder();
    for (int j = 0; j < 3; ++j)
      if (i != j)
        need(std::abs(cyl.Location().Coord(j + 1) - lo[j] - radii[i]) <= 2e-6, stage);
  }
  return proof;
}

// Verify iso-trims over the entire parameter interval. Positive rational
// Bernstein weights with ordered poles preserve monotonicity in one coordinate.
void sides(const Proof &p, const TopoDS_Shape &solid, Status stage) {
  const auto &patch = p.patch;
  std::array<bool, 4> seen{};
  std::array<bool, 3> walls{};
  int planes = 0;
  TopTools_IndexedDataMapOfShapeListOfShape adjacent;
  TopExp::MapShapesAndUniqueAncestors(solid, TopAbs_EDGE, TopAbs_FACE, adjacent);
  for (TopExp_Explorer it(p.faces[9], TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    const auto &owners = adjacent.FindFromKey(edge);
    need(owners.Extent() == 2, stage);
    const auto neighbor = TopoDS::Face(owners.First().IsSame(p.faces[9]) ? owners.Last() : owners.First());
    const auto type = BRepAdaptor_Surface(neighbor, false).GetType();
    if (type == GeomAbs_Plane)
      ++planes;
    else {
      need(type == GeomAbs_Cylinder, stage);
      const int i = axis(BRepAdaptor_Surface(neighbor, false).Cylinder().Axis().Direction(), stage);
      need(!walls[i], stage);
      walls[i] = true;
    }
    BRepAdaptor_Curve2d trim(edge, p.faces[9]);
    const auto start = trim.Value(trim.FirstParameter()), finish = trim.Value(trim.LastParameter());
    const double tol = 1e-8;
    const bool iso_u = std::abs(start.X() - finish.X()) <= tol;
    const int i = iso_u ? 0 : 1;
    const double lo = i == 0 ? patch.u0 : patch.v0, hi = i == 0 ? patch.u1 : patch.v1;
    const double opposite_lo = i == 0 ? patch.v0 : patch.u0, opposite_hi = i == 0 ? patch.v1 : patch.u1;
    const auto coord = [&](const gp_Pnt2d &v) { return i == 0 ? v.X() : v.Y(); };
    const auto other = [&](const gp_Pnt2d &v) { return i == 0 ? v.Y() : v.X(); };
    const auto ends = std::minmax({other(start), other(finish)});
    need(std::abs(ends.first - opposite_lo) <= tol && std::abs(ends.second - opposite_hi) <= tol, stage);
    const bool top = std::abs(coord(start) - hi) < tol;
    const auto level = top ? hi : lo;
    need(std::abs(coord(start) - level) < tol && !seen[2 * i + int(top)], stage);
    seen[2 * i + int(top)] = true;
    if (trim.GetType() == GeomAbs_BSplineCurve) {
      const auto bs = trim.BSpline();
      need(bs->NbPoles() <= 96, stage);
      double previous = other(bs->Pole(1));
      const double sign = other(bs->Pole(bs->NbPoles())) >= previous ? 1. : -1.;
      for (int j = 1; j <= bs->NbPoles(); ++j) {
        const auto pt = bs->Pole(j);
        need(std::abs(coord(pt) - level) < tol && bs->Weight(j) > 0. &&
                 sign * (other(pt) - previous) >= -tol,
             stage);
        previous = other(pt);
      }
    } else
      need(trim.GetType() == GeomAbs_Line, stage);
    double a, b, c, d;
    const auto first = BRep_Tool::CurveOnSurface(edge, p.faces[9], a, b);
    const auto second = BRep_Tool::CurveOnSurface(edge, neighbor, c, d);
    BRepAdaptor_Surface fs(p.faces[9], false), gs(neighbor, false);
    for (double t : {.05, .25, .5, .75, .95}) {
      const auto uv = first->Value(a + t * (b - a)), st = second->Value(c + t * (d - c));
      gp_Pnt x, y;
      gp_Vec xu, xv, yu, yv;
      fs.D1(uv.X(), uv.Y(), x, xu, xv);
      gs.D1(st.X(), st.Y(), y, yu, yv);
      need(x.Distance(y) <= 3e-6 &&
               xu.Crossed(xv).Normalized().Crossed(yu.Crossed(yv).Normalized()).Magnitude() <= 3e-5,
           stage);
    }
  }
  need(planes == 1 && std::all_of(seen.begin(), seen.end(), [](bool b) { return b; }) &&
           std::all_of(walls.begin(), walls.end(), [](bool b) { return b; }),
       stage);
}

// Bound each normalized parametric second derivative by the Bernstein
// difference hulls of H = w*(S-origin) and w, using H = w*(S-origin).
// A uniform chart then has chord deviation at most
// (Muu + 2*Muv + Mvv)/(8*n*n). Keep the existing mesh-size cap.
double mesh_divisions(const Patch &patch, double deflection) {
  const auto bs = patch.surface;
  const int p = bs->UDegree(), q = bs->VDegree();
  double min_w = std::numeric_limits<double>::infinity(), extent = 0.;
  const auto origin = bs->Pole(1, 1);
  auto w = [&](int i, int j) { return bs->Weight(i + 1, j + 1); };
  auto h = [&](int i, int j) { return gp_Vec(origin, bs->Pole(i + 1, j + 1)) * w(i, j); };
  std::array<double, 2> first{}, dw{};
  std::array<double, 3> second{}, ddw{};
  for (int i = 0; i <= p; ++i)
    for (int j = 0; j <= q; ++j) {
      min_w = std::min(min_w, w(i, j));
      extent = std::max(extent, origin.Distance(bs->Pole(i + 1, j + 1)));
      for (int k = 0; k < 2; ++k) {
        const int n = k ? q : p, di = k ? 0 : 1, dj = k ? 1 : 0;
        if (i + di > p || j + dj > q)
          continue;
        first[k] = std::max(first[k], n * (h(i + di, j + dj) - h(i, j)).Magnitude());
        dw[k] = std::max(dw[k], n * std::abs(w(i + di, j + dj) - w(i, j)));
        if (i + 2 * di > p || j + 2 * dj > q)
          continue;
        second[2 * k] = std::max(
            second[2 * k],
            n * (n - 1) * (h(i + 2 * di, j + 2 * dj) - 2. * h(i + di, j + dj) + h(i, j)).Magnitude());
        ddw[2 * k] =
            std::max(ddw[2 * k],
                     n * (n - 1) * std::abs(w(i + 2 * di, j + 2 * dj) - 2. * w(i + di, j + dj) + w(i, j)));
      }
      if (i < p && j < q) {
        second[1] = std::max(second[1],
                             p * q * (h(i + 1, j + 1) - h(i + 1, j) - h(i, j + 1) + h(i, j)).Magnitude());
        ddw[1] = std::max(ddw[1], p * q * std::abs(w(i + 1, j + 1) - w(i + 1, j) - w(i, j + 1) + w(i, j)));
      }
    }
  for (int k = 0; k < 2; ++k)
    first[k] = (first[k] + extent * dw[k]) / min_w;
  for (int k = 0; k < 3; ++k) {
    const int a = k == 2, b = k != 0;
    second[k] = (second[k] + ddw[k] * extent + dw[a] * first[b] + dw[b] * first[a]) / min_w;
  }
  return std::ceil(std::max(16., std::sqrt((second[0] + 2. * second[1] + second[2]) / (8. * deflection))));
}

Proof verify(const TopoDS_Shape &solid, Status stage) {
  auto proof = inspect(solid, stage);
  auto canonical = build(proof.data, stage);
  const auto expected = inspect(canonical, stage);
  const auto a = proof.patch.surface, b = expected.patch.surface;
  need(a->NbUPoles() == b->NbUPoles() && a->NbVPoles() == b->NbVPoles(), stage);
  for (int i = 1; i <= a->NbUPoles(); ++i)
    for (int j = 1; j <= a->NbVPoles(); ++j) {
      need(a->Pole(i, j).Distance(b->Pole(i, j)) < 2e-6 &&
               std::abs(a->Weight(i, j) / a->Weight(1, 1) - b->Weight(i, j) / b->Weight(1, 1)) < 1e-8,
           stage);
    }
  sides(proof, solid, stage);
  return proof;
}
} // namespace corner_domain

namespace corner_domain {
void borders(const Proof &p, const std::vector<unequal_corner::Point> &vertices,
             const std::vector<unequal_corner::Tri> &triangles, const std::vector<size_t> &owners,
             double fit) {
  using Edge = std::pair<size_t, size_t>;
  std::map<Edge, size_t> pending;
  std::map<Edge, std::vector<Edge>> boundary;
  for (size_t i = 0; i < triangles.size(); ++i)
    for (int j = 0; j < 3; ++j) {
      const Edge edge = std::minmax(triangles[i][j], triangles[i][(j + 1) % 3]);
      const auto [found, added] = pending.try_emplace(edge, owners[i]);
      if (added)
        continue;
      if (found->second != owners[i])
        boundary[std::minmax(owners[i], found->second)].push_back(edge);
      pending.erase(found);
    }
  need(pending.empty(), corner_unproven);
  // Each pair of neighboring native faces has one shared edge. Its complete
  // source counterpart is a single chain, ordered along that same 3D curve.
  std::map<Edge, TopoDS_Edge> expected;
  TopTools_IndexedMapOfShape edges;
  std::vector<size_t> first;
  for (size_t i = 0; i < p.faces.size(); ++i)
    for (TopExp_Explorer it(p.faces[i], TopAbs_EDGE); it.More(); it.Next()) {
      const int known = edges.FindIndex(it.Current());
      if (!known) {
        edges.Add(it.Current());
        first.push_back(i);
      } else
        need(expected.emplace(std::minmax(i, first.at(known - 1)), TopoDS::Edge(it.Current())).second,
             corner_unproven);
    }
  need(expected.size() == boundary.size(), corner_unproven);
  for (const auto &[pair, edge] : expected) {
    const auto found = boundary.find(pair);
    need(found != boundary.end(), corner_unproven);
    std::map<size_t, std::vector<size_t>> chain;
    for (const auto &[i, j] : found->second) {
      chain[i].push_back(j);
      chain[j].push_back(i);
    }
    std::vector<size_t> ends;
    for (const auto &[id, neighbors] : chain) {
      if (neighbors.size() == 1)
        ends.push_back(id);
      else
        need(neighbors.size() == 2, corner_unproven);
    }
    need(ends.size() == 2, corner_unproven);
    double lo, hi;
    const auto curve = BRep_Tool::Curve(edge, lo, hi);
    need(!curve.IsNull() && std::isfinite(lo) && hi > lo, corner_unproven);
    const auto a = curve->Value(lo), b = curve->Value(hi);
    if (a.Distance(unequal_corner::point(vertices[ends[0]])) >
        a.Distance(unequal_corner::point(vertices[ends[1]])))
      std::swap(ends[0], ends[1]);
    need(a.Distance(unequal_corner::point(vertices[ends[0]])) <= fit &&
             b.Distance(unequal_corner::point(vertices[ends[1]])) <= fit,
         corner_unproven);
    size_t id = ends[0], previous = vertices.size(), visited = 1;
    double parameter = lo;
    while (id != ends[1]) {
      const auto &neighbors = chain.at(id);
      const size_t next = neighbors[0] == previous ? neighbors.back() : neighbors[0];
      previous = id;
      id = next;
      need(++visited <= chain.size(), corner_unproven);
      if (id == ends[1])
        break;
      const auto target = unequal_corner::point(vertices[id]);
      double best = std::numeric_limits<double>::infinity(), t = lo;
      for (int k = 0; k <= 8; ++k) {
        const double seed = lo + (hi - lo) * k / 8., distance = curve->Value(seed).SquareDistance(target);
        if (distance < best) {
          best = distance;
          t = seed;
        }
      }
      for (int k = 0; k < 24; ++k) {
        gp_Pnt v;
        gp_Vec tangent;
        curve->D1(t, v, tangent);
        if (tangent.SquareMagnitude() < 1e-22)
          break;
        const double next =
            std::clamp(t + gp_Vec(v, target).Dot(tangent) / tangent.SquareMagnitude(), lo, hi);
        if (next == t)
          break;
        t = next;
      }
      need(curve->Value(t).Distance(target) <= fit && t > parameter && t < hi, corner_unproven);
      parameter = t;
    }
    need(visited == chain.size(), corner_unproven);
  }
}

double certify_source(const Proof &p, const std::vector<unequal_corner::Point> &vertices,
                      const std::vector<unequal_corner::Tri> &triangles, std::vector<size_t> &owners,
                      double fit, double deviation) {
  const auto reject = [](const char *why) { throw CadFailure{corner_unproven, why}; };
  if (!(fit > 0. && deviation > fit && fit <= 1e-4) || !std::isfinite(deviation))
    reject("Budget du carreau incompatible.");
  const auto &[origin, size, radii] = p.data;
  std::vector<unequal_corner::UV> uv(vertices.size());
  std::vector<bool> on_patch(vertices.size());
  for (size_t i = 0; i < vertices.size(); ++i) {
    const auto &v = vertices[i];
    bool near = true;
    for (int k = 0; k < 3; ++k) {
      if (!std::isfinite(v[k]))
        reject("Sommet non fini.");
      near = near && v[k] <= origin[k] + 2. * std::max({radii[0], radii[1], radii[2]});
    }
    if (near)
      on_patch[i] = p.patch.project(v, fit, uv[i]);
  }
  double bound = 0.;
  std::vector<unequal_corner::Tri> patch_triangles;
  std::array<size_t, 10> count{};
  for (size_t n = 0; n < triangles.size(); ++n) {
    const auto tri = triangles[n];
    size_t &owner = owners[n];
    if (owner >= 10 || *std::max_element(tri.begin(), tri.end()) >= vertices.size())
      reject("Attribution source invalide.");
    if (on_patch[tri[0]] && on_patch[tri[1]] && on_patch[tri[2]]) {
      const auto a = uv[tri[0]], b = uv[tri[1]], c = uv[tri[2]];
      if (std::abs((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) > 1e-13)
        owner = 9;
    }
    ++count[owner];
    if (owner == 9) {
      patch_triangles.push_back(tri);
      continue;
    }
    const int i = int(owner % 3);
    if (owner < 6) {
      for (auto id : tri)
        bound = std::max(bound, std::abs(vertices[id][i] - origin[i] - (owner < 3 ? 0. : size[i])));
    } else {
      const int j = (i + 1) % 3, k = (i + 2) % 3;
      const double r = radii[i];
      std::array<double, 3> angles;
      double residual = 0.;
      for (int v = 0; v < 3; ++v) {
        const double x = vertices[tri[v]][j] - origin[j] - r, y = vertices[tri[v]][k] - origin[k] - r;
        if (x > fit || y > fit)
          reject("Quadrant du congé non reconnu.");
        angles[v] = std::atan2(-y, -x);
        residual = std::max(residual, std::abs(std::hypot(x, y) - r));
      }
      const auto range = std::minmax_element(angles.begin(), angles.end());
      const double span = *range.second - *range.first;
      bound = std::max(bound, residual + r * (1. - std::cos(span / 2.)));
    }
    if (bound > deviation)
      reject("Écart des surfaces d’appui hors tolérance.");
  }
  borders(p, vertices, triangles, owners, fit);
  if (std::find(count.begin(), count.end(), 0) != count.end())
    reject("Partition du coin incomplète.");
  unequal_corner::Certificate certificate;
  if (!p.patch.certify(vertices, patch_triangles, fit, deviation, certificate))
    reject(certificate.reason);
  return std::max(bound, certificate.maximum);
}
} // namespace corner_domain
