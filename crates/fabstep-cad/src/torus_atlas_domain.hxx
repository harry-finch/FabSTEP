namespace atlas_domain {
inline std::optional<gp_Pnt> pole(const TopoDS_Face &face, const TopoDS_Edge &edge) {
  if (auto known = trimfan::pole(face, edge))
    return known;
  if (!BRep_Tool::Degenerated(edge))
    return {};
  TopoDS_Vertex a, b;
  TopExp::Vertices(edge, a, b);
  if (a.IsNull() || b.IsNull() || !a.IsSame(b))
    return {};
  const auto apex = BRep_Tool::Pnt(a);
  BRepAdaptor_Surface surface(face, false);
  if (surface.GetType() != GeomAbs_BSplineSurface && surface.GetType() != GeomAbs_BezierSurface)
    return {};
  auto s = surface.GetType() == GeomAbs_BSplineSurface
               ? surface.BSpline()
               : GeomConvert::SurfaceToBSplineSurface(surface.Bezier());
  if (surface.GetType() == GeomAbs_BezierSurface) {
    const auto f = BRepBuilderAPI_MakeFace(s, Precision::Confusion()).Face();
    if (auto chart = trimfan::profile(f, .05, 3.141592653589793 - 1e-5))
      if (chart->apex.Distance(apex) <= Precision::Confusion())
        return apex;
  }
  if (s->IsUPeriodic() || s->IsVPeriodic() || s->NbUKnots() != 2 || s->NbVKnots() != 2 ||
      !((s->UDegree() == 2 && s->VDegree() == 1) || (s->UDegree() == 1 && s->VDegree() == 2)))
    return {};
  if (s->VDegree() != 1) {
    s = Handle(Geom_BSplineSurface)::DownCast(s->Copy());
    s->ExchangeUV();
  }
  if (s->NbUPoles() != 3 || s->NbVPoles() != 2)
    return {};
  for (int row : {1, 2}) {
    const int opposite = 3 - row;
    if (s->Pole(1, row).Distance(apex) > 1e-8 || s->Pole(2, row).Distance(apex) > 1e-8 ||
        s->Pole(3, row).Distance(apex) > 1e-8)
      continue;
    auto p = s->Pole(1, opposite), q = s->Pole(2, opposite), r = s->Pole(3, opposite);
    const gp_Vec chord(p, r), left(p, q), right(q, r);
    const auto normal = left.Crossed(right);
    if (normal.Magnitude() < 1e-15 || left.Dot(chord) <= 0. || right.Dot(chord) <= 0.)
      continue;
    // Positive rational weights and increasing chord projection make the base
    // simple. Every open ruling cuts a plane parallel to it exactly once.
    if (gp_Pln(p, gp_Dir(normal)).Distance(apex) <= 4e-6)
      continue;
    bool weights = true;
    for (int u = 1; u <= 3; ++u)
      for (int v = 1; v <= 2; ++v)
        weights = weights && s->Weight(u, v) > 0.;
    if (weights)
      return apex;
  }
  return {};
}

// Reconstruct the native domain signature after every STEP read. A label or
// product name never grants spline support. A pinched fan must have its
// four complete isoparametric sides, including the collapsed coedge. A
// regular collar instead has three material sides in its triangular trim.
inline bool fan_trim(const TopoDS_Face &face) {
  const BRepAdaptor_Surface s(face, false);
  double lo[2]{s.FirstUParameter(), s.FirstVParameter()}, hi[2]{s.LastUParameter(), s.LastVParameter()};
  std::array<bool, 4> seen{};
  int collapsed = 0;
  for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
    const auto edge = TopoDS::Edge(e.Current());
    BRepAdaptor_Curve2d pc(edge, face);
    const auto a = pc.Value(pc.FirstParameter()), b = pc.Value(pc.LastParameter());
    int side = -1;
    for (int k = 0; k < 2; ++k)
      for (int end = 0; end < 2; ++end) {
        const double level = end ? hi[k] : lo[k];
        const int j = 1 - k;
        const double tol = 1e-8;
        if (std::max(std::abs(a.Coord(k + 1) - level), std::abs(b.Coord(k + 1) - level)) <= tol &&
            std::abs(std::min(a.Coord(j + 1), b.Coord(j + 1)) - lo[j]) <= tol &&
            std::abs(std::max(a.Coord(j + 1), b.Coord(j + 1)) - hi[j]) <= tol) {
          if (side != -1)
            return false;
          side = 2 * k + end;
          if (pc.GetType() != GeomAbs_Line) {
            if (pc.GetType() != GeomAbs_BSplineCurve)
              return false;
            const auto spline = pc.BSpline();
            if (spline->IsPeriodic() || spline->NbPoles() > 256 || spline->Degree() > 3)
              return false;
            const double sign = std::copysign(1., b.Coord(j + 1) - a.Coord(j + 1));
            for (int i = 1; i <= spline->NbPoles(); ++i) {
              if (std::abs(spline->Pole(i).Coord(k + 1) - level) > tol || spline->Weight(i) <= 0. ||
                  (i > 1 && sign * (spline->Pole(i).Coord(j + 1) - spline->Pole(i - 1).Coord(j + 1)) < 0.))
                return false;
            }
          }
        }
      }
    if (side < 0 || seen[side])
      return false;
    seen[side] = true;
    if (BRep_Tool::Degenerated(edge)) {
      if (++collapsed > 1 || !pole(face, edge))
        return false;
    }
  }
  return collapsed == 1 && std::all_of(seen.begin(), seen.end(), [](bool x) { return x; });
}

bool matches(const TopTools_IndexedMapOfShape &faces) {
  if (faces.Extent() < 4 || size_t(faces.Extent()) > result_face_limit)
    return false;
  bool torus = false, neighbor = false;
  for (int i = 1; i <= faces.Extent(); ++i) {
    auto type = BRepAdaptor_Surface(TopoDS::Face(faces(i)), false).GetType();
    if (type == GeomAbs_Torus)
      torus = true;
    else if (type == GeomAbs_BSplineSurface || type == GeomAbs_BezierSurface || type == GeomAbs_Plane)
      neighbor = true;
    else
      return false;
  }
  if (!torus || !neighbor)
    return false;
  std::optional<gp_Dir> axis;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto face = TopoDS::Face(faces(i).Oriented(TopAbs_FORWARD));
    TopExp_Explorer wires(face, TopAbs_WIRE);
    if (!wires.More())
      return false;
    wires.Next();
    if (wires.More())
      return false;
    const auto s = BRepAdaptor_Surface(face, false);
    if (s.GetType() == GeomAbs_Torus) {
      auto direction = s.Torus().Axis().Direction();
      if (axis && gp_Vec(*axis).Crossed(gp_Vec(direction)).Magnitude() > 1e-8)
        return false;
      axis = direction;
      if (!trimmed_torus_mesh::supports(face))
        return false;
    } else if (s.GetType() == GeomAbs_Plane) {
      int count = 0;
      for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
        const auto material = TopoDS::Edge(e.Current());
        if (++count > 3 || BRep_Tool::Degenerated(material) || !atlas_edge::trace(material))
          return false;
      }
      if (count != 3)
        return false;
    } else if (!atlas_triangle::matches(face) && !fan_trim(face))
      return false;
  }
  return true;
}
struct Policy {
  double deflection;
  bool candidate(const TopoDS_Face &f) const {
    return trimmed_torus_mesh::supports(f) || bool(trimfan::profile(f, deflection, mesh_angle));
  }
  template <class Samples>
  Handle(Poly_Triangulation) mesh(const TopoDS_Face &f, size_t remaining, const Samples &samples) const {
    if (BRepAdaptor_Surface(f, false).GetType() == GeomAbs_Torus)
      return trimmed_torus_mesh::mesh(f, deflection, mesh_angle, remaining, samples);
    return trimfan::mesh(f, deflection, mesh_angle, samples);
  }
  std::optional<gp_Pnt> pole(const TopoDS_Face &f, const TopoDS_Edge &e) const {
    return atlas_domain::pole(f, e);
  }
};
} // namespace atlas_domain
