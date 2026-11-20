// A single rounded mouth on an orthogonal through bore. The complete
// periodic surface must agree with the net of a regenerated rolling fillet.
namespace rolling_domain {
void need(bool yes, Status stage, const char *reason) {
  if (!yes)
    throw CadFailure{stage, reason};
}

struct Data {
  gp_Pnt origin;
  double height{}, outer{}, bore{}, offset{}, level{}, radius{};
  Handle(Geom_BSplineSurface) net;
};
struct Proof {
  Data data;
  std::array<TopoDS_Face, 5>
      faces; // bottom, top, outside, bore, rolling fillet
  std::vector<TopoDS_Edge> sharp;
  std::array<TopoDS_Edge, 3> blend_edges; // outer rail, bore rail, seam
};
bool candidate(const TopTools_IndexedMapOfShape &faces) {
  if (faces.Extent() != 5)
    return false;
  int planes = 0, cylinders = 0, patches = 0;
  for (int i = 1; i <= 5; ++i) {
    BRepAdaptor_Surface f(TopoDS::Face(faces(i)), false);
    planes += f.GetType() == GeomAbs_Plane;
    cylinders += f.GetType() == GeomAbs_Cylinder;
    patches += f.GetType() == GeomAbs_BSplineSurface && f.UDegree() == 2 &&
               f.NbUPoles() == 3 && !f.IsUPeriodic();
  }
  return planes == 2 && cylinders == 2 && patches == 1;
}

Proof inspect(const TopoDS_Shape &shape, Status stage) {
  Proof p;
  auto &d = p.data;
  for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
    const auto face = TopoDS::Face(it.Current());
    BRepAdaptor_Surface f(face, false);
    need(BRep_Tool::Tolerance(face) < 2e-6, stage,
         "Tolérance de face excessive.");
    int slot = 4;
    if (f.GetType() == GeomAbs_Plane) {
      const auto n = oblique_lip::outward(face);
      need(std::abs(n.Z()) > 1. - 1e-14, stage, "Un fond du fût est incliné.");
      slot = n.Z() < 0. ? 0 : 1;
    } else if (f.GetType() == GeomAbs_Cylinder) {
      auto cylinder = f.Cylinder();
      bool outer = face.Orientation() == TopAbs_FORWARD;
      const auto dir = cylinder.Axis().Direction();
      need((outer ? dir.Z() : dir.X()) > 1. - 1e-14, stage,
           "Axe ou orientation de cylindre inattendu.");
      slot = outer ? 2 : 3;
    } else {
      need(f.GetType() == GeomAbs_BSplineSurface &&
               face.Orientation() == TopAbs_FORWARD && f.UDegree() == 2 &&
               f.NbUPoles() == 3 && f.NbUKnots() == 2 && !f.IsUPeriodic() &&
               f.VDegree() >= 2 && f.VDegree() <= 12 && f.NbVPoles() <= 256 &&
               f.NbVKnots() >= 3 && f.NbVKnots() <= 32,
           stage, "Carreau périodique hors domaine.");
      // STEP can encode the same closed row either clamped with duplicate
      // endpoint poles or periodic. Align both on their existing seam knot.
      d.net = Handle(Geom_BSplineSurface)::DownCast(f.BSpline()->Copy());
      if (d.net->IsVPeriodic()) {
        double u0, u1, v0, v1;
        d.net->Bounds(u0, u1, v0, v1);
        d.net->Segment(u0, u1, v0, v1, 1e-13, 1e-13);
      }
      const int last = d.net->NbVPoles();
      for (int i = 1; i <= 3; ++i)
        need(d.net->Pole(i, 1).Distance(d.net->Pole(i, last)) < 1e-7 &&
                 std::abs(d.net->Weight(i, 1) - d.net->Weight(i, last)) < 1e-8,
             stage, "Le réseau du raccord ne se ferme pas.");
      for (int i = 1; i <= 3; ++i)
        for (int j = 1; j <= d.net->NbVPoles(); ++j)
          need(d.net->Weight(i, j) > .01 && d.net->Weight(i, j) < 100., stage,
               "Poids du raccord périodique invalide.");
    }
    need(p.faces[slot].IsNull(), stage,
         "Deux faces revendiquent le même domaine du fût.");
    p.faces[slot] = face;
  }
  for (size_t i = 0; i < p.faces.size(); ++i) {
    need(!p.faces[i].IsNull(), stage, "Domaine incomplet du perçage arrondi.");
    TopTools_IndexedMapOfShape wires;
    TopExp::MapShapes(p.faces[i], TopAbs_WIRE, wires);
    need(wires.Extent() == (i == 2 ? 2 : 1), stage,
         "Contour matériel du fût inattendu.");
  }
  auto outer = BRepAdaptor_Surface(p.faces[2], false).Cylinder();
  auto bore = BRepAdaptor_Surface(p.faces[3], false).Cylinder();
  double low = BRepAdaptor_Surface(p.faces[0], false).Plane().Location().Z();
  d.origin = {outer.Location().X(), outer.Location().Y(), low};
  d.height =
      BRepAdaptor_Surface(p.faces[1], false).Plane().Location().Z() - low;
  d.outer = outer.Radius();
  d.bore = bore.Radius();
  d.level = bore.Location().Z() - low;
  d.offset = bore.Location().Y() - d.origin.Y();

  const auto &s = d.net;
  double u0, u1, v0, v1;
  s->Bounds(u0, u1, v0, v1);
  for (int k = 1; k <= s->NbVKnots(); ++k) {
    // Each section at a knot must give the same circular radius. Interior
    // sections and tangency are diagnostics in addition to the net identity.
    for (int mid = 0; mid <= int(k < s->NbVKnots()); ++mid) {
      double v = mid ? (s->VKnot(k) + s->VKnot(k + 1)) * .5 : s->VKnot(k);
      gp_Pnt a = s->Value(u0, v), b = s->Value((u0 + u1) * .5, v),
             c = s->Value(u1, v);
      gp_Vec ab(a, b), ac(a, c);
      double r = .5 * ab.Magnitude() * ac.Magnitude() * b.Distance(c) /
                 ab.Crossed(ac).Magnitude();
      if (k == 1 && mid == 0)
        d.radius = r;
      need(r > 0. && r < d.bore * .5 &&
               std::abs(r - d.radius) < (mid ? 3e-5 : 3e-7),
           stage, "Rayon variable sur le raccord du perçage.");
      for (int i = 0; i < 2; ++i) {
        gp_Pnt x;
        gp_Vec du, dv;
        s->D1(i == 0 ? u0 : u1, v, x, du, dv);
        const auto &cylinder = i == 0 ? outer : bore;
        const gp_Vec axis(cylinder.Axis().Direction()),
            shift(cylinder.Location(), x);
        auto radial = shift - axis * axis.Dot(shift);
        auto n = du.Crossed(dv);
        need(x.X() < d.origin.X() && n.Magnitude() > 1e-14 &&
                 std::abs(radial.Magnitude() - cylinder.Radius()) < 2e-5 &&
                 n.Normalized().Crossed(radial.Normalized()).Magnitude() < 1e-4,
             stage,
             "Le raccord ne rejoint pas tangentiellement les deux cylindres.");
      }
    }
  }
  return p;
}

// The two angular seams and both cap edges have distinct material owners.
// Certify the sharp mouth over an entire turn and the iso-trims over the
// entire periodic chart, including the second use of the same seam.
void boundaries(const TopoDS_Shape &shape, Proof &p, Status stage) {
  TopTools_IndexedDataMapOfShapeListOfShape map;
  TopExp::MapShapesAndUniqueAncestors(shape, TopAbs_EDGE, TopAbs_FACE, map);
  need(map.Extent() == 11, stage,
       "Nombre d’arêtes inattendu sur le perçage arrondi.");
  std::array<int, 5> self{}, caps{};
  std::vector<std::pair<double, double>> spans;
  TopTools_IndexedMapOfShape vertices;
  TopExp::MapShapes(shape, TopAbs_VERTEX, vertices);
  need(vertices.Extent() == 7, stage, "Sommets du perçage arrondi incomplets.");
  for (int i = 1; i <= vertices.Extent(); ++i)
    need(BRep_Tool::Tolerance(TopoDS::Vertex(vertices(i))) < 2e-6, stage,
         "Tolérance de sommet excessive.");
  for (int i = 1; i <= map.Extent(); ++i) {
    const auto edge = TopoDS::Edge(map.FindKey(i));
    const auto &owners = map.FindFromIndex(i);
    const auto slot = [&](const TopoDS_Shape &f) {
      for (int j = 0; j < 5; ++j)
        if (p.faces[j].IsSame(f))
          return j;
      return -1;
    };
    int a = slot(owners.First()), b = slot(owners.Last());
    if (a > b)
      std::swap(a, b);
    need(a >= 0 && owners.Extent() <= 2 && !BRep_Tool::Degenerated(edge) &&
             BRep_Tool::Tolerance(edge) < 2e-6,
         stage, "Incidence ou tolérance d’arête du perçage invalide.");
    BRepAdaptor_Curve curve(edge);
    if (a == b) {
      need(BRep_Tool::IsClosed(edge, p.faces[a]) &&
               (a == 4
                    ? curve.GetType() == GeomAbs_BSplineCurve
                    : ((a == 2 || a == 3) && curve.GetType() == GeomAbs_Line)),
           stage, "Couture du perçage non prouvée.");
      ++self[a];
      if (a == 4)
        p.blend_edges[2] = edge;
    } else if (a < 2 && b == 2) {
      auto center =
          p.data.origin.Translated({0., 0., a == 0 ? 0. : p.data.height});
      need(curve.GetType() == GeomAbs_Circle &&
               std::abs(curve.LastParameter() - curve.FirstParameter() -
                        2 * std::acos(-1.)) < 1e-8 &&
               std::abs(curve.Circle().Radius() - p.data.outer) < 1e-7 &&
               curve.Circle().Location().Distance(center) < 1e-7 &&
               std::abs(curve.Circle().Axis().Direction().Z()) > 1. - 1e-14,
           stage, "Cercle du fond de fût incomplet.");
      ++caps[a];
    } else {
      need(curve.GetType() == GeomAbs_BSplineCurve && curve.Degree() <= 12 &&
               curve.NbPoles() <= 256,
           stage, "Courbe de raccord de perçage hors budget.");
      if (a == 2 && b == 3) {
        need(!curve.IsRational(), stage,
             "Branche directe du perçage hors domaine.");
        const auto s = curve.BSpline();
        for (int j = 1; j <= s->NbPoles(); ++j)
          need(s->Pole(j).X() > p.data.origin.X(), stage,
               "Mauvaise branche de perçage.");
        BRepAdaptor_Curve2d trace(edge, p.faces[3]);
        need(trace.GetType() == GeomAbs_BSplineCurve && trace.NbPoles() <= 256,
             stage, "Tracé de branche non vérifié.");
        const auto pc = trace.BSpline();
        auto lo = trace.Value(trace.FirstParameter()).X(),
             hi = trace.Value(trace.LastParameter()).X();
        const double sign = lo < hi ? 1. : -1.;
        for (int j = 2; j <= pc->NbPoles(); ++j)
          need(pc->Weight(j) > 0. &&
                   sign * (pc->Pole(j).X() - pc->Pole(j - 1).X()) >= -1e-8,
               stage, "Retour angulaire du perçage.");
        spans.emplace_back(std::min(lo, hi), std::max(lo, hi));
        p.sharp.push_back(edge);
      } else {
        need((a == 2 || a == 3) && b == 4 && p.blend_edges[a - 2].IsNull(),
             stage, "Mauvais voisin du raccord arrondi.");
        p.blend_edges[a - 2] = edge;
      }
      for (auto id : {a, b}) {
        BRepAdaptor_Curve2d pc(edge, p.faces[id]);
        need(pc.GetType() == GeomAbs_Line ||
                 (pc.GetType() == GeomAbs_BSplineCurve && pc.NbPoles() <= 256 &&
                  pc.Degree() <= 12),
             stage, "Tracé du raccord hors budget.");
        BRepLib_CheckCurveOnSurface dist(edge, p.faces[id]);
        dist.Perform();
        need(dist.IsDone() && dist.MaxDistance() >= 0. &&
                 dist.MaxDistance() < 5e-7,
             stage, "L’arête quitte le support du perçage.");
      }
    }
  }
  need(caps[0] == 1 && caps[1] == 1 && self[2] == 2 && self[3] == 1 &&
           self[4] == 1 && spans.size() == 3 && !p.blend_edges[0].IsNull() &&
           !p.blend_edges[1].IsNull(),
       stage, "Boucles de perçage incomplètes.");
  std::sort(spans.begin(), spans.end());
  double start = spans.front().first, end = start;
  for (auto [lo, hi] : spans) {
    need(std::abs(lo - end) < 1e-8 && hi > lo, stage,
         "Branche de perçage discontinue.");
    end = hi;
  }
  need(std::abs(end - start - 2 * std::acos(-1.)) < 1e-8, stage,
       "Branche de perçage incomplète.");
  double u0, u1, v0, v1;
  p.data.net->Bounds(u0, u1, v0, v1);
  std::array<bool, 4> seen{};
  for (TopExp_Explorer it(p.faces[4], TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    const bool seam = edge.IsSame(p.blend_edges[2]);
    BRepAdaptor_Curve2d pc(edge, p.faces[4]);
    auto level = [&](gp_Pnt2d x) { return seam ? x.Y() : x.X(); };
    auto along = [&](gp_Pnt2d x) { return seam ? x.X() : x.Y(); };
    auto start = pc.Value(pc.FirstParameter()),
         end = pc.Value(pc.LastParameter());
    int index = seam ? (std::abs(level(start) - v0) < 1e-7 ? 2 : 3)
                     : int(edge.IsSame(p.blend_edges[1]));
    double fixed = std::array<double, 4>{u0, u1, v0, v1}[index];
    need(!seen[index] && std::abs(level(start) - fixed) < 1e-7 &&
             std::abs(level(end) - fixed) < 1e-7 &&
             std::abs(std::min(along(start), along(end)) - (seam ? u0 : v0)) <
                 1e-7 &&
             std::abs(std::max(along(start), along(end)) - (seam ? u1 : v1)) <
                 1e-7,
         stage, "Limites du carreau périodique invalides.");
    seen[index] = true;
    if (pc.GetType() == GeomAbs_Line)
      continue;
    need(pc.GetType() == GeomAbs_BSplineCurve && pc.NbPoles() <= 256 &&
             pc.Degree() <= 12,
         stage, "Couture du raccord hors budget.");
    auto s = pc.BSpline();
    const double sign = along(end) > along(start) ? 1. : -1.;
    for (int j = 1; j <= s->NbPoles(); ++j)
      need(s->Weight(j) > 0. && std::abs(level(s->Pole(j)) - fixed) < 1e-7 &&
               (j == 1 ||
                sign * (along(s->Pole(j)) - along(s->Pole(j - 1))) >= -1e-7),
           stage, "Couture périodique non isoparamétrique.");
  }
  need(std::all_of(seen.begin(), seen.end(), [](bool x) { return x; }), stage,
       "Le raccord ne couvre pas le tour complet.");
}

bool same_net(const Handle(Geom_BSplineSurface) & a,
              const Handle(Geom_BSplineSurface) & b, double tol) {
  if (a->UDegree() != b->UDegree() || a->VDegree() != b->VDegree() ||
      a->NbUKnots() != b->NbUKnots() || a->NbVKnots() != b->NbVKnots() ||
      a->NbUPoles() != b->NbUPoles() || a->NbVPoles() != b->NbVPoles())
    return false;
  for (int i = 1; i <= a->NbUKnots(); ++i)
    if (a->UMultiplicity(i) != b->UMultiplicity(i) ||
        !(std::abs(a->UKnot(i) - b->UKnot(i)) <= tol))
      return false;
  for (int i = 1; i <= a->NbVKnots(); ++i)
    if (a->VMultiplicity(i) != b->VMultiplicity(i) ||
        !(std::abs(a->VKnot(i) - b->VKnot(i)) <= tol))
      return false;
  for (int i = 1; i <= a->NbUPoles(); ++i)
    for (int j = 1; j <= a->NbVPoles(); ++j)
      if (!(a->Pole(i, j).Distance(b->Pole(i, j)) <= tol &&
            std::abs(a->Weight(i, j) / a->Weight(1, 1) -
                     b->Weight(i, j) / b->Weight(1, 1)) <= 1e-8))
        return false;
  return true;
}

void same(const Data &a, const Data &b, Status stage) {
  need(a.origin.Distance(b.origin) < 1e-7 &&
           std::abs(a.height - b.height) < 1e-7 &&
           std::abs(a.outer - b.outer) < 1e-7 &&
           std::abs(a.bore - b.bore) < 1e-7 &&
           std::abs(a.offset - b.offset) < 1e-7 &&
           std::abs(a.level - b.level) < 1e-7 &&
           std::abs(a.radius - b.radius) < 1e-7 && same_net(a.net, b.net, 1e-7),
       stage,
       "Les supports ou le réseau périodique ont changé après transfert.");
}

Proof verify(const TopoDS_Shape &shape, Status stage) {
  auto proof = inspect(shape, stage);
  const auto &d = proof.data;
  // The rounded source dimensions are recoverable up to the Part 21 writer's
  // decimal precision. Rounding only proposes a candidate; the whole net,
  // every knot and both actual rails must still agree.
  bool match = false;
  for (double step : {1e-6, 1e-8, 1e-10, 0.}) {
    const auto snap = [&](double x) {
      return step ? std::round(x / step) * step : x;
    };
    const gp_Pnt o(snap(d.origin.X()), snap(d.origin.Y()), d.origin.Z());
    rolling_shape::Stock stock(o, d.height, snap(d.outer), snap(d.bore),
                               snap(d.offset), snap(d.level), stage);
    const auto expected = inspect(stock.fillet(snap(d.radius), stage), stage);
    if (same_net(d.net, expected.data.net, 2e-6)) {
      match = true;
      break;
    }
  }
  need(match, stage,
       "Le réseau périodique diffère du congé à rayon constant reconstruit.");
  boundaries(shape, proof, stage);
  return proof;
}

void mesh(const Proof &proof, double deflection,
          const TopTools_IndexedMapOfShape &edges,
          std::map<int, double> &junctions, size_t &triangles, size_t limit) {
  auto s = proof.data.net;
  double total = 0., maximum = 0.;
  for (int i = 1; i < s->NbVKnots(); ++i) {
    auto span = Handle(Geom_BSplineSurface)::DownCast(s->Copy());
    const auto u0 = s->UKnot(1), u1 = s->UKnot(2), v0 = s->VKnot(i),
               v1 = s->VKnot(i + 1);
    span->Segment(u0, u1, v0, v1, 1e-13, 1e-13);
    need(span->NbVPoles() == span->VDegree() + 1 && span->NbUPoles() == 3,
         mesh_too_complex, "Travée rationnelle non bornée.");
    const auto n =
        corner_domain::mesh_divisions({span, u0, u1, v0, v1}, deflection);
    need(std::isfinite(n) && 2. * n * n <= limit - triangles, mesh_too_complex,
         "Le raccord dépasse le budget de tessellation.");
    triangles += size_t(2. * n * n);
    total += n;
    maximum = std::max(n, maximum);
  }
  for (int i = 0; i < 3; ++i)
    junctions.emplace(edges.FindIndex(proof.blend_edges[i]),
                      i == 2 ? maximum : total);
  for (auto &e : proof.sharp)
    junctions.emplace(edges.FindIndex(e),
                      oblique_lip::segments(e, deflection, mesh_too_complex));
}
} // namespace rolling_domain
