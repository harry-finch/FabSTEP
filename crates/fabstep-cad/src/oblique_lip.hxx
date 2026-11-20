// A cylinder with one quarter-round lip, cut through both by an oblique plane.
namespace oblique_lip {
struct Data {
  double origin[3], axis[3], radial[3], outer, major, height, offset, slope;
};
constexpr double eps = 5.e-7;
void need(bool ok, Status s, const char *message) {
  if (!ok)
    throw CadFailure{s, message};
}
struct Frame {
  gp_Pnt origin;
  gp_Dir z, x;
  double outer, major, h, offset, slope;
  double minor() const { return outer - major; }
  double shoulder() const { return h - minor(); }
  double cut(double z) const { return offset + slope * z; }
  gp_Dir y() const { return z.Crossed(x); }
  gp_Pnt point(double u, double v, double w) const {
    return origin.Translated(gp_Vec(x) * u + gp_Vec(y()) * v + gp_Vec(z) * w);
  }
  gp_Pnt branch(int level, int side) const {
    const double w = level == 0 ? 0. : level == 1 ? shoulder() : h;
    const double r = level == 2 ? major : outer, u = cut(w);
    return point(u, side * std::sqrt(r * r - u * u), w);
  }
  void validate(Status stage) const {
    for (double v :
         {origin.X(), origin.Y(), origin.Z(), outer, major, h, offset, slope})
      need(std::isfinite(v) && std::abs(v) < 1.e5, stage,
           "Dimensions de la lèvre hors limites.");
    need(std::abs(x.Dot(z)) < 1e-10 && major > 1e-4 && minor() > 1e-4 &&
             shoulder() > 1e-4 && std::abs(slope) > 1e-4 &&
             std::abs(slope) < 4. &&
             std::max(std::abs(cut(0.)), std::abs(cut(shoulder()))) <
                 .97 * outer &&
             std::max(std::abs(cut(shoulder())), std::abs(cut(h))) <
                 .97 * major,
         stage,
         "La coupe de la lèvre est tangente ou sans domaine transversal "
         "certifié.");
  }
};
Frame input(const Data &d, Status s) {
  for (auto p : {d.axis, d.radial}) {
    const gp_Vec v(p[0], p[1], p[2]);
    need(std::isfinite(v.Magnitude()) && std::abs(v.Magnitude() - 1.) < 1e-10,
         s, "Direction de lèvre invalide.");
  }
  Frame f{point_at(d.origin, 0),
          direction(d.axis),
          direction(d.radial),
          d.outer,
          d.major,
          d.height,
          d.offset,
          d.slope};
  f.validate(s);
  return f;
}
TopoDS_Solid build(const Frame &f) {
  BRepPrimAPI_MakeCylinder plain(gp_Ax2(f.origin, f.z, f.x), f.outer, f.h);
  const auto cylinder = plain.Shape();
  BRepFilletAPI_MakeFillet fillet(cylinder);
  for (TopExp_Explorer it(cylinder, TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    const BRepAdaptor_Curve c(edge);
    if (c.GetType() == GeomAbs_Circle &&
        c.Circle().Location().Distance(f.point(0., 0., f.h)) <= eps) {
      fillet.Add(f.minor(), edge);
      break;
    }
  }
  fillet.Build();
  need(fillet.IsDone(), invalid_model,
       "Construction de l’arrondi de lèvre impossible.");
  const gp_Dir normal(gp_Vec(f.x) - gp_Vec(f.z) * f.slope);
  const double reach = 16. * std::max(f.outer, f.h);
  const gp_Dir y = f.y(), in_plane = normal.Crossed(y);
  const auto corner = f.point(f.offset, 0., 0.)
                          .Translated(-(gp_Vec(y) + gp_Vec(in_plane)) * reach);
  const auto block = BRepPrimAPI_MakeBox(gp_Ax2(corner, normal, y), 2. * reach,
                                         2. * reach, 2. * reach)
                         .Shape();
  BRepAlgoAPI_Cut cut(fillet.Shape(), block);
  need(cut.IsDone(), invalid_model, "La coupe oblique du congé a échoué.");
  cut.SimplifyResult(true, true);
  return TopoDS::Solid(single_solid(cut.Shape(), invalid_model));
}
struct Proof {
  Frame frame;
  std::array<TopoDS_Face, 5> faces;
  std::array<TopoDS_Edge, 2> splines;
};
gp_Dir outward(const TopoDS_Face &face) {
  const auto p = BRepAdaptor_Surface(face).Plane().Position();
  gp_Dir n = p.XDirection().Crossed(p.YDirection());
  if (face.Orientation() == TopAbs_REVERSED)
    n.Reverse();
  return n;
}
void same(const Frame &a, const Frame &b, Status stage) {
  for (int i = 0; i < 3; ++i)
    for (int sign : {-1, 1})
      need(a.branch(i, sign).Distance(b.branch(i, sign)) < eps, stage,
           "Les extrémités de coupe ont changé.");
  need(a.origin.Distance(b.origin) < eps && std::abs(a.major - b.major) < eps &&
           std::abs(a.outer - b.outer) < eps && std::abs(a.h - b.h) < eps &&
           (gp_Vec(a.z) - gp_Vec(b.z)).Magnitude() * a.h < eps &&
           (gp_Vec(a.x) - gp_Vec(b.x)).Magnitude() * a.outer < eps,
       stage, "Les supports de la lèvre ont changé.");
}

void check_trace(const TopoDS_Face &face, const TopoDS_Edge &edge, Status s) {
  const BRepAdaptor_Curve c(edge);
  const BRepAdaptor_Curve2d pc(edge, face);
  need(std::abs(pc.FirstParameter() - c.FirstParameter()) < 1e-9 &&
           std::abs(pc.LastParameter() - c.LastParameter()) < 1e-9,
       s, "Paramètres d’arête et de tracé incohérents.");
  if (pc.GetType() == GeomAbs_BSplineCurve)
    need(pc.Degree() <= 8 && pc.NbPoles() <= 256 && !pc.IsRational(), s,
         "Tracé spline de lèvre hors budget.");
  BRepLib_CheckCurveOnSurface distance(edge, face);
  distance.Perform();
  need(distance.IsDone() && distance.MaxDistance() >= 0. &&
           distance.MaxDistance() < eps,
       s, "Le tracé de la lèvre quitte son support.");
}

void monotone_trace(const TopoDS_Face &face, const TopoDS_Edge &edge,
                    double span, double physical_scale, Status s) {
  const BRepAdaptor_Curve2d c(edge, face);
  const double a = c.Value(c.FirstParameter()).Y(),
               b = c.Value(c.LastParameter()).Y();
  need(std::abs(std::abs(b - a) - span) * physical_scale < eps, s,
       "La branche ne couvre pas l’intervalle entier.");
  if (c.GetType() == GeomAbs_Line)
    return;
  need(c.GetType() == GeomAbs_BSplineCurve, s,
       "Le tracé de branche possède un support inconnu.");
  const auto pc = c.BSpline();
  need(!pc->IsPeriodic() && !pc->IsRational() && pc->Degree() <= 8 &&
           pc->NbPoles() <= 256,
       s, "Tracé spline de lèvre hors domaine.");
  const double sign = std::copysign(1., b - a);
  for (int i = 2; i <= pc->NbPoles(); ++i)
    need(sign * (pc->Pole(i).Y() - pc->Pole(i - 1).Y()) >= 0., s,
         "Le tracé retourne sur la branche.");
}
Proof verify(const TopoDS_Shape &shape, Status s) {
  TopTools_IndexedMapOfShape all;
  TopExp::MapShapes(shape, TopAbs_FACE, all);
  need(all.Extent() == 5, s, "La lèvre doit comporter cinq faces.");
  std::array<TopoDS_Face, 5> faces;
  std::vector<TopoDS_Face> planes;
  for (int i = 1; i <= 5; ++i) {
    const auto f = TopoDS::Face(all(i));
    const auto type = BRepAdaptor_Surface(f, false).GetType();
    TopTools_IndexedMapOfShape loops;
    TopExp::MapShapes(f, TopAbs_WIRE, loops);
    need(loops.Extent() == 1 && BRep_Tool::Tolerance(f) <= eps, s,
         "Contour de lèvre absent ou tolérance excessive.");
    if (type == GeomAbs_Plane)
      planes.push_back(f);
    else if (type == GeomAbs_Cylinder && faces[3].IsNull())
      faces[3] = f;
    else if (type == GeomAbs_Torus && faces[4].IsNull())
      faces[4] = f;
    else
      need(false, s, "Familles de faces de la lèvre incompatibles.");
  }
  need(planes.size() == 3 && !faces[3].IsNull() && !faces[4].IsNull(), s,
       "Supports de lèvre absents.");
  const auto cylinder = BRepAdaptor_Surface(faces[3], false).Cylinder();
  const auto torus = BRepAdaptor_Surface(faces[4], false).Torus();
  for (const auto &f : planes) {
    int id = 2;
    if (outward(f).IsParallel(cylinder.Axis().Direction(), 1e-9)) {
      int arcs = 0;
      for (TopExp_Explorer e(f, TopAbs_EDGE); e.More(); e.Next()) {
        BRepAdaptor_Curve c(TopoDS::Edge(e.Current()));
        if (c.GetType() == GeomAbs_Circle) {
          ++arcs;
          id = std::abs(c.Circle().Radius() - cylinder.Radius()) < eps ? 0 : 1;
        }
      }
      need(arcs == 1, s, "Disque tronqué sans arc unique.");
    }
    need(faces[id].IsNull(), s, "Plan de lèvre ambigu.");
    faces[id] = f;
  }
  for (const auto &f : faces)
    need(!f.IsNull(), s, "Support absent.");
  const gp_Dir axis = outward(faces[0]).Reversed();
  const auto base = BRepAdaptor_Surface(faces[0], false).Plane().Location();
  const auto tc = torus.Location();
  const auto origin = tc.Translated(gp_Vec(axis) * gp_Vec(tc, base).Dot(axis));
  const auto top = BRepAdaptor_Surface(faces[1], false).Plane().Location();
  const auto cut = BRepAdaptor_Surface(faces[2], false).Plane().Location();
  const auto n = outward(faces[2]);
  const double vertical = n.Dot(axis);
  const gp_Vec xy = gp_Vec(n) - gp_Vec(axis) * vertical;
  need(xy.Magnitude() > .2, s, "Coupe de lèvre hors domaine.");
  Frame f{origin,
          axis,
          gp_Dir(xy),
          cylinder.Radius(),
          torus.MajorRadius(),
          gp_Vec(origin, top).Dot(axis),
          gp_Vec(origin, cut).Dot(n) / xy.Magnitude(),
          -vertical / xy.Magnitude()};
  f.validate(s);
  need(outward(faces[1]).IsEqual(axis, 1e-9) &&
           cylinder.Axis().Direction().IsParallel(axis, 1e-9) &&
           torus.Axis().Direction().IsParallel(axis, 1e-9) &&
           gp_Vec(origin, cylinder.Location())
                   .Crossed(gp_Vec(axis))
                   .Magnitude() < eps &&
           tc.Distance(f.point(0., 0., f.shoulder())) < eps &&
           std::abs(torus.MinorRadius() - f.minor()) < eps,
       s, "Cylindre et tore sans raccord tangent commun.");
  TopTools_IndexedDataMapOfShapeListOfShape map;
  TopExp::MapShapesAndUniqueAncestors(shape, TopAbs_EDGE, TopAbs_FACE, map);
  need(map.Extent() == 9, s,
       "La frontière de lèvre ne comporte pas neuf arêtes partagées.");
  auto label = [&](const TopoDS_Shape &face) {
    for (int i = 0; i < 5; ++i)
      if (faces[i].IsSame(face))
        return i;
    throw CadFailure{s, "Propriétaire d’arête inconnu."};
  };
  std::set<std::array<int, 3>> seen;
  std::array<TopoDS_Edge, 2> branches;
  for (int i = 1; i <= map.Extent(); ++i) {
    const auto e = TopoDS::Edge(map.FindKey(i));
    const auto &owners = map.FindFromIndex(i);
    need(owners.Extent() == 2 && !BRep_Tool::Degenerated(e) &&
             BRep_Tool::Tolerance(e) < 4. * eps,
         s, "Incidence ou tolérance d’arête de lèvre invalide.");
    int a = label(owners.First()), b = label(owners.Last());
    if (a > b)
      std::swap(a, b);
    const BRepAdaptor_Curve c(e);
    auto first = c.Value(c.FirstParameter()), last = c.Value(c.LastParameter());
    auto joins = [&](const gp_Pnt &p, const gp_Pnt &q) {
      return (first.Distance(p) < 4. * eps && last.Distance(q) < 4. * eps) ||
             (first.Distance(q) < 4. * eps && last.Distance(p) < 4. * eps);
    };
    int sign = 0;
    if ((a == 0 || a == 1) && b == 2) {
      const int level = 2 * a;
      need(c.GetType() == GeomAbs_Line &&
               joins(f.branch(level, -1), f.branch(level, 1)),
           s, "Corde du disque inadéquate.");
    } else if ((a == 0 && b == 3) || (a == 1 && b == 4) || (a == 3 && b == 4)) {
      const int k = a == 0 ? 0 : a == 1 ? 2 : 1;
      const double radius = k == 2 ? f.major : f.outer, z = k == 2 ? f.h
                                                            : k == 1
                                                                ? f.shoulder()
                                                                : 0.;
      const double span = 2. * std::acos(-f.cut(z) / radius);
      need(c.GetType() == GeomAbs_Circle &&
               joins(f.branch(k, -1), f.branch(k, 1)) &&
               std::abs(c.LastParameter() - c.FirstParameter() - span) *
                       radius <
                   4. * eps &&
               c.Value((c.FirstParameter() + c.LastParameter()) / 2.)
                       .Distance(f.point(-radius, 0., z)) < 4. * eps,
           s, "L’arc ne suit pas la portion conservée du disque.");
    } else if (a == 2 && (b == 3 || b == 4)) {
      sign = gp_Vec(f.origin, first).Dot(f.y()) > 0. ? 1 : -1;
      const int k = b == 3 ? 0 : 1;
      need(joins(f.branch(k, sign), f.branch(k + 1, sign)), s,
           "Extrémités de la branche incompatibles.");
      if (b == 4) {
        need(c.GetType() == GeomAbs_BSplineCurve && c.Degree() <= 8 &&
                 c.NbPoles() <= 256 && !c.IsRational(),
             s, "Intersection plan–tore hors du domaine polynomial.");
        const auto spline = c.BSpline();
        for (int j = 1; j <= spline->NbPoles(); ++j)
          need(sign * gp_Vec(f.origin, spline->Pole(j)).Dot(f.y()) >
                   .05 * f.major,
               s, "Les branches du tore se confondent.");
        branches[sign > 0] = e;
      } else
        need(c.GetType() == GeomAbs_Ellipse, s,
             "Coupe cylindrique sans ellipse.");
      // Compact spline traces have monotone V controls, so every station of
      // the cylinder or torus has one preimage on this side of the cutting
      // plane.
      monotone_trace(faces[b], e, b == 3 ? f.shoulder() : std::acos(-1.) / 2.,
                     b == 3 ? 1. : f.minor(), s);
    } else
      need(false, s, "Faces de lèvre adjacentes incompatibles.");
    need(seen.insert({a, b, sign}).second, s,
         "Une branche de coupe est dupliquée.");
    check_trace(faces[a], e, s);
    check_trace(faces[b], e, s);
  }
  return {f, faces, branches};
}
// Polygon bounds for the first and second derivatives on each Bernstein
// interval guard both chordal deflection and angular refinement before OCCT.
double segments(const TopoDS_Edge &edge, double deflection, Status s) {
  const BRepAdaptor_Curve c(edge);
  const auto spline = c.BSpline();
  GeomConvert_BSplineCurveToBezierCurve pieces(spline, c.FirstParameter(),
                                               c.LastParameter(), 1e-12);
  need(pieces.NbArcs() <= 256, s, "Trop de travées sur la coupe du tore.");
  double total = 0.;
  for (int i = 1; i <= pieces.NbArcs(); ++i) {
    const auto arc = pieces.Arc(i);
    const int degree = arc->Degree();
    const gp_Dir direction(gp_Vec(arc->StartPoint(), arc->EndPoint()));
    double second = 0., minimum = std::numeric_limits<double>::infinity();
    for (int j = 1; j <= degree; ++j) {
      const auto first = gp_Vec(arc->Pole(j), arc->Pole(j + 1)) * degree;
      minimum = std::min(minimum, first.Dot(direction));
      if (j > 1) {
        const auto before = gp_Vec(arc->Pole(j - 1), arc->Pole(j)) * degree;
        second = std::max(second, (degree - 1) * (first - before).Magnitude());
      }
    }
    need(minimum > 0. && std::isfinite(second), s,
         "Vitesse de la branche non prouvée.");
    total += std::ceil(std::max({1., std::sqrt(second / (8. * deflection)),
                                 second / (minimum * mesh_angle)}));
  }
  return total;
}
} // namespace oblique_lip
