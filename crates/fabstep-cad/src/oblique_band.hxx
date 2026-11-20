// Plane-trimmed conical and spherical zones with two distinct circular caps.
namespace oblique_band {
using oblique_lip::Frame;
using oblique_lip::need;
constexpr double eps = 5.e-7;
struct Data {
  oblique_lip::Data frame;
  size_t sphere;
};
struct Domain {
  Frame f;
  bool sphere;
};
double zcenter(const Frame &f) {
  return (f.h * f.h + f.major * f.major - f.outer * f.outer) / (2. * f.h);
}
double radius(const Domain &d, double z) {
  const auto &f = d.f;
  if (!d.sphere)
    return f.outer + (f.major - f.outer) * z / f.h;
  const double c = zcenter(f);
  return std::sqrt(f.outer * f.outer + c * c - (z - c) * (z - c));
}
void validate(const Domain &d, Status s) {
  const auto &f = d.f;
  for (double v : {f.origin.X(), f.origin.Y(), f.origin.Z(), f.outer, f.major,
                   f.h, f.offset, f.slope})
    need(std::isfinite(v) && std::abs(v) < 1e5, s,
         "Dimensions du domaine oblique hors limites.");
  need(f.major > 1e-4 && f.outer > f.major + 1e-5 && f.h > 1e-4 &&
           std::abs(f.x.Dot(f.z)) < 1e-10 && std::abs(f.slope) > 1e-3 &&
           std::abs(f.slope) < 4. &&
           std::max(std::abs(f.cut(0.)), std::abs(f.cut(f.h))) < .97 * f.major,
       s, "Coupe tangente ou calottes ambiguës.");
  if (!d.sphere)
    need(std::abs(f.slope) < std::abs((f.outer - f.major) / f.h) - 1e-4, s,
         "Le type de conique change sur cette coupe.");
}
Domain input(const Data &d, Status s) {
  const auto &a = d.frame;
  need(d.sphere <= 1, s, "Famille de bande inconnue.");
  for (auto v : {a.axis, a.radial})
    need(std::abs(gp_Vec(v[0], v[1], v[2]).Magnitude() - 1.) < 1e-10, s,
         "Repère de bande invalide.");
  Domain out{{point_at(a.origin, 0), direction(a.axis), direction(a.radial),
              a.outer, a.major, a.height, a.offset, a.slope},
             bool(d.sphere)};
  validate(out, s);
  return out;
}
TopoDS_Solid build(const Domain &d) {
  const auto &f = d.f;
  TopoDS_Shape blank;
  if (d.sphere) {
    const double c = zcenter(f), r = std::hypot(f.outer, c);
    const auto center = f.point(0., 0., c);
    blank = BRepPrimAPI_MakeSphere(gp_Ax2(center, f.z, f.x), r,
                                   std::asin(-c / r), std::asin((f.h - c) / r))
                .Shape();
  } else {
    // STEP represents a conical support by a positive semi-angle. Building it
    // in that direction preserves the same face through the native transfer.
    blank =
        BRepPrimAPI_MakeCone(gp_Ax2(f.point(0., 0., f.h), f.z.Reversed(), f.x),
                             f.major, f.outer, f.h)
            .Shape();
  }
  const gp_Dir n(gp_Vec(f.x) - gp_Vec(f.z) * f.slope);
  const gp_Dir y = f.y(), in_plane = n.Crossed(y);
  const double reach = 16. * std::max(f.outer, f.h);
  const auto corner = f.point(f.offset, 0., 0.)
                          .Translated(-(gp_Vec(y) + gp_Vec(in_plane)) * reach);
  const auto block = BRepPrimAPI_MakeBox(gp_Ax2(corner, n, y), 2. * reach,
                                         2. * reach, 2. * reach)
                         .Shape();
  BRepAlgoAPI_Cut cut(blank, block);
  need(cut.IsDone(), invalid_model, "La coupe du support a échoué.");
  cut.SimplifyResult(true, true);
  return TopoDS::Solid(single_solid(cut.Shape(), invalid_model));
}
bool candidate(const TopTools_IndexedMapOfShape &map) {
  if (map.Extent() != 4)
    return false;
  std::vector<gp_Dir> normals;
  int count = 0;
  bool sphere = false;
  for (int i = 1; i <= 4; ++i) {
    const auto f = TopoDS::Face(map(i));
    const BRepAdaptor_Surface s(f, false);
    if (s.GetType() == GeomAbs_Plane)
      normals.push_back(oblique_lip::outward(f));
    if (s.GetType() == GeomAbs_Cone || s.GetType() == GeomAbs_Sphere) {
      ++count;
      sphere = s.GetType() == GeomAbs_Sphere;
    }
  }
  if (count != 1 || normals.size() != 3)
    return false;
  TopTools_IndexedMapOfShape edges;
  for (int i = 1; i <= 4; ++i)
    TopExp::MapShapes(map(i), TopAbs_EDGE, edges);
  if (edges.Extent() != 6)
    return false;
  int lines = 0, circles = 0, hyperbolas = 0;
  for (int i = 1; i <= 6; ++i) {
    const auto kind = BRepAdaptor_Curve(TopoDS::Edge(edges(i))).GetType();
    lines += kind == GeomAbs_Line;
    circles += kind == GeomAbs_Circle;
    hyperbolas += kind == GeomAbs_Hyperbola;
  }
  if (lines != 2 || circles != (sphere ? 4 : 2) ||
      hyperbolas != (sphere ? 0 : 2))
    return false;
  for (int i = 0; i < 3; ++i)
    if (normals[i].IsOpposite(normals[(i + 1) % 3], 1e-8)) {
      const double k = std::abs(normals[i].Dot(normals[(i + 2) % 3]));
      return k > 1e-3 && k < .97;
    }
  return false;
}
struct Proof {
  Domain domain;
  std::array<TopoDS_Face, 4> faces;
  std::vector<TopoDS_Edge> branches;
};
Proof verify(const TopoDS_Shape &shape, Status stage) {
  TopTools_IndexedMapOfShape map;
  TopExp::MapShapes(shape, TopAbs_FACE, map);
  need(candidate(map), stage, "Signature de bande oblique absente.");
  std::array<TopoDS_Face, 4> faces;
  std::vector<std::pair<gp_Circ, TopoDS_Face>> caps;
  for (int i = 1; i <= 4; ++i) {
    const auto f = TopoDS::Face(map(i));
    BRepAdaptor_Surface surf(f, false);
    TopTools_IndexedMapOfShape wires;
    TopExp::MapShapes(f, TopAbs_WIRE, wires);
    need(wires.Extent() == 1 && BRep_Tool::Tolerance(f) <= eps, stage,
         "Le contour de la bande a changé.");
    if (surf.GetType() != GeomAbs_Plane) {
      faces[3] = f;
      continue;
    }
    std::vector<gp_Circ> circles;
    size_t lines = 0;
    for (TopExp_Explorer it(f, TopAbs_EDGE); it.More(); it.Next()) {
      BRepAdaptor_Curve curve(TopoDS::Edge(it.Current()));
      if (curve.GetType() == GeomAbs_Circle)
        circles.push_back(curve.Circle());
      lines += curve.GetType() == GeomAbs_Line;
    }
    if (circles.size() == 1 && lines == 1)
      caps.emplace_back(circles[0], f);
    else {
      need(faces[2].IsNull(), stage, "Coupe oblique ambiguë.");
      faces[2] = f;
    }
  }
  need(caps.size() == 2 && !faces[2].IsNull(), stage,
       "Les deux calottes circulaires sont nécessaires.");
  if (caps[1].first.Radius() > caps[0].first.Radius())
    std::swap(caps[0], caps[1]);
  faces[0] = caps[0].second;
  faces[1] = caps[1].second;
  const auto origin = caps[0].first.Location(), top = caps[1].first.Location();
  const auto z = oblique_lip::outward(faces[1]),
             out = oblique_lip::outward(faces[2]);
  need(z.IsOpposite(oblique_lip::outward(faces[0]), 1e-9) &&
           gp_Vec(origin, top).Crossed(gp_Vec(z)).Magnitude() < eps,
       stage, "Les calottes ne sont pas coaxiales.");
  const double nz = out.Dot(z);
  const auto xy = gp_Vec(out) - gp_Vec(z) * nz;
  const auto cut = BRepAdaptor_Surface(faces[2], false).Plane().Location();
  const Frame f{origin,
                z,
                gp_Dir(xy),
                caps[0].first.Radius(),
                caps[1].first.Radius(),
                gp_Vec(origin, top).Dot(z),
                gp_Vec(origin, cut).Dot(out) / xy.Magnitude(),
                -nz / xy.Magnitude()};
  const BRepAdaptor_Surface support(faces[3], false);
  Domain d{f, support.GetType() == GeomAbs_Sphere};
  validate(d, stage);
  if (d.sphere) {
    const double c = zcenter(f);
    const auto s = support.Sphere();
    need(s.Location().Distance(f.point(0., 0., c)) < eps &&
             std::abs(s.Radius() - std::hypot(f.outer, c)) < eps,
         stage, "Centre ou rayon sphérique incohérent.");
  } else {
    const auto cone = support.Cone();
    need(cone.Axis().Direction().IsParallel(z, 1e-9) &&
             gp_Vec(origin, cone.Location()).Crossed(gp_Vec(z)).Magnitude() <
                 eps,
         stage, "Axe conique incohérent.");
    for (double zz : {0., f.h * .5, f.h}) {
      const double axial = gp_Vec(cone.Location(), f.point(0., 0., zz))
                               .Dot(cone.Axis().Direction());
      need(std::abs(cone.RefRadius() + axial * std::tan(cone.SemiAngle()) -
                    radius(d, zz)) < eps,
           stage, "Rayon de conicité incohérent.");
    }
  }
  TopTools_IndexedDataMapOfShapeListOfShape shared;
  TopExp::MapShapesAndUniqueAncestors(shape, TopAbs_EDGE, TopAbs_FACE, shared);
  need(shared.Extent() == 6, stage,
       "La bande doit conserver six arêtes communes.");
  std::set<std::tuple<int, int, int>> seen;
  std::vector<TopoDS_Edge> branches;
  for (int i = 1; i <= shared.Extent(); ++i) {
    const auto e = TopoDS::Edge(shared.FindKey(i));
    const auto &owners = shared(i);
    need(owners.Extent() == 2 && BRep_Tool::Tolerance(e) <= eps, stage,
         "Arête de bande libre ou tolérance excessive.");
    auto owner = [&](const TopoDS_Shape &q) {
      for (int k = 0; k < 4; ++k)
        if (q.IsSame(faces[k]))
          return k;
      return -1;
    };
    int a = owner(owners.First()), b = owner(owners.Last());
    if (b < a)
      std::swap(a, b);
    need(a >= 0 && b > a, stage, "Propriété d’arête ambiguë.");
    const BRepAdaptor_Curve c(e);
    const double l = c.FirstParameter(), h = c.LastParameter();
    auto branch = [&](int cap, int sign) {
      const double zz = cap ? f.h : 0., r = cap ? f.major : f.outer,
                   x = f.cut(zz);
      return f.point(x, sign * std::sqrt(r * r - x * x), zz);
    };
    auto joins = [&](const gp_Pnt &p, const gp_Pnt &q) {
      return (p.Distance(c.Value(l)) < eps && q.Distance(c.Value(h)) < eps) ||
             (q.Distance(c.Value(l)) < eps && p.Distance(c.Value(h)) < eps);
    };
    int sign = 0;
    if (a < 2 && b == 2)
      need(c.GetType() == GeomAbs_Line && joins(branch(a, -1), branch(a, 1)),
           stage, "Corde de calotte inattendue.");
    else if (a < 2 && b == 3) {
      const double zz = a ? f.h : 0., r = radius(d, zz);
      need(
          c.GetType() == GeomAbs_Circle && joins(branch(a, -1), branch(a, 1)) &&
              std::abs(h - l - 2. * std::acos(-f.cut(zz) / r)) * r < 4. * eps &&
              c.Value((l + h) / 2.).Distance(f.point(-r, 0., zz)) < 4. * eps,
          stage, "Arc de calotte incorrect.");
    } else if (a == 2 && b == 3) {
      sign = gp_Vec(origin, c.Value(l)).Dot(f.y()) > 0 ? 1 : -1;
      need(joins(branch(0, sign), branch(1, sign)), stage,
           "Extrémités de la branche de coupe incohérentes.");
      need(d.sphere ? c.GetType() == GeomAbs_Circle
                    : c.GetType() == GeomAbs_Hyperbola,
           stage, "Section conique imprévue.");
      // The derivative of axial height on a hyperbola is cosh(t)*(A*tanh(t)+B).
      // On a circle its consecutive zeros are pi apart.
      gp_Pnt ignored;
      gp_Vec d1, d2;
      c.D1(l, ignored, d1);
      c.D1(h, ignored, d2);
      need(d1.Dot(z) * d2.Dot(z) > 0. &&
               (!d.sphere || (h - l) < std::acos(-1.)),
           stage, "Branche non monotone.");
      branches.push_back(e);
    } else
      need(false, stage, "Deux surfaces non adjacentes se touchent.");
    need(seen.emplace(a, b, sign).second, stage, "Branche dupliquée.");
    oblique_lip::check_trace(faces[a], e, stage);
    oblique_lip::check_trace(faces[b], e, stage);
  }
  return {d, faces, branches};
}
void same(const Domain &a, const Domain &b, Status stage) {
  need(a.sphere == b.sphere, stage, "Famille de bande changée.");
  const auto &f = a.f;
  const auto &g = b.f;
  need(f.origin.Distance(g.origin) < eps && std::abs(f.h - g.h) < eps &&
           std::abs(f.major - g.major) < eps &&
           std::abs(f.outer - g.outer) < eps &&
           std::abs(f.cut(0.) - g.cut(0.)) < eps &&
           std::abs(f.cut(f.h) - g.cut(g.h)) < eps &&
           (gp_Vec(f.x) - gp_Vec(g.x)).Magnitude() * f.outer < eps &&
           (gp_Vec(f.z) - gp_Vec(g.z)).Magnitude() * f.h < eps,
       stage, "Les paramètres de la bande diffèrent après transfert.");
}
double hyperbola_segments(const TopoDS_Edge &e, double tolerance) {
  const BRepAdaptor_Curve c(e);
  const auto h = c.Hyperbola();
  const double a = c.FirstParameter(), b = c.LastParameter(), span = b - a,
               t = std::max(std::abs(a), std::abs(b));
  const double second = std::hypot(h.MajorRadius() * std::cosh(t),
                                   h.MinorRadius() * std::sinh(t));
  return std::ceil(std::max({1., span * std::sqrt(second / (8. * tolerance)),
                             span * second / (h.MinorRadius() * mesh_angle)}));
}
} // namespace oblique_band
