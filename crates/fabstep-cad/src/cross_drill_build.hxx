// Reparameterize the shared curve and both traces together on the proven
// branch. OCCT supplies the seam topology; the scalar domain supplies geometry.
void set_junction(const CrossDrillData& d, const TopoDS_Edge& edge, const TopoDS_Face& small,
                  const TopoDS_Face& big) {
  const CadFailure invalid{
      invalid_model, "La limite commune des deux perçages ne correspond pas à la branche prouvée."};
  const BRepAdaptor_Curve curve(edge);
  const double first = curve.FirstParameter(), last = curve.LastParameter(),
               period = 2. * std::acos(-1.);
  const gp_Vec x(direction(d.axes[0])), y(direction(d.axes[1])), z(direction(d.axes[2]));
  const auto center = point_at(d.center, 0).Translated(x*d.small_offset[0]+y*d.small_offset[1]);
  auto phase = [&](double t) {
    const gp_Vec v(center, curve.Value(t));
    return std::atan2(v.Dot(y), v.Dot(x));
  };
  const double start = phase(first);
  double end = start;
  for (int i = 1; i <= 16; ++i) {
    double v = phase(first + (last - first) * i / 16.);
    double dv = std::remainder(v - end, period);
    if (std::abs(dv) > 1.)
      throw invalid;
    end += dv;
  }
  const double sign = gp_Vec(center, curve.Value((first + last) / 2.)).Dot(z) > 0. ? 1. : -1.;
  const double r = d.radii[0], R = d.radii[1];
  const double dt = end - start;
  if (std::abs(dt) < 1e-10 || std::abs(dt) > period + 1e-7)
    throw invalid;
  const std::array<TopoDS_Face, 2> faces{small, big};
  std::array<gp_Cylinder, 2> cyl;
  std::array<Handle(Geom2d_Curve), 2> old;
  for (int j = 0; j < 2; ++j) {
    double a, b;
    old[j] = BRep_Tool::CurveOnSurface(edge, faces[j], a, b);
    if (old[j].IsNull())
      throw invalid;
    cyl[j] = BRepAdaptor_Surface(faces[j]).Cylinder();
  }
  const int n =
      std::max(2, static_cast<int>(std::ceil(std::abs(dt) / .15))); // max 42 quintic spans
  const double step = dt / n;
  TColgp_Array1OfPnt poles(1, 5 * n + 1);
  TColgp_Array1OfPnt2d pa(1, 5 * n + 1), pb(1, 5 * n + 1);
  TColStd_Array1OfReal knots(1, n + 1);
  TColStd_Array1OfInteger mults(1, n + 1);
  for (int i = 0; i <= n; ++i) {
    const double t = start + i * step, c = std::cos(t), s = std::sin(t),
                 world_y = d.small_offset[1] + r*s,
                 h = std::sqrt((R-world_y)*(R+world_y));
    const double hp = -world_y*r*c/h, hpp = (-r*r*c*c+world_y*r*s-hp*hp)/h;
    const auto p = center.Translated(x * (r * c) + y * (r * s) + z * (sign * h));
    if ((i == 0 || i == n) &&
        p.Distance(curve.Value(i == 0 ? first : last)) > Precision::Confusion())
      throw invalid;
    const gp_Vec v = x * (-r * s) + y * (r * c) + z * (sign * hp),
                 a = x * (-r * c) + y * (-r * s) + z * (sign * hpp);
    auto hermite = [&](auto& array, const auto& point, const auto& velocity, const auto& accel) {
      const int k = 5 * i + 1;
      array(k) = point;
      if (i) {
        array(k - 1) = point.Translated(velocity * (-step / 5.));
        array(k - 2) = point.Translated(velocity * (-2. * step / 5.) + accel * (step * step / 20.));
      }
      if (i < n) {
        array(k + 1) = point.Translated(velocity * (step / 5.));
        array(k + 2) = point.Translated(velocity * (2. * step / 5.) + accel * (step * step / 20.));
      }
    };
    hermite(poles, p, v, a);
    const double native = i == n ? last : first + (last - first) * i / n;
    knots(i + 1) = native;
    mults(i + 1) = i == 0 || i == n ? 6 : 5;
    for (int j = 0; j < 2; ++j) {
      const gp_Vec X(cyl[j].Position().XDirection()), Y(cyl[j].Position().YDirection()),
          Z(cyl[j].Axis().Direction()), off(cyl[j].Location(), p);
      const double px = off.Dot(X), py = off.Dot(Y), vx = v.Dot(X), vy = v.Dot(Y), ax = a.Dot(X),
                   ay = a.Dot(Y);
      const double den = px * px + py * py, up = (px * vy - py * vx) / den,
                   upp = (px * ay - py * ax - 2. * up * (px * vx + py * vy)) / den;
      const double u = std::atan2(py, px), ref = old[j]->Value(native).X();
      const gp_Pnt2d uv(u + period * std::round((ref - u) / period), off.Dot(Z));
      hermite(j ? pb : pa, uv, gp_Vec2d(up, v.Dot(Z)), gp_Vec2d(upp, a.Dot(Z)));
    }
  }
  BRep_Builder builder;
  Handle(Geom_BSplineCurve) space = new Geom_BSplineCurve(poles, knots, mults, 5);
  builder.UpdateEdge(edge, space, 0.);
  builder.Range(edge, first, last);
  for (int j = 0; j < 2; ++j) {
    Handle(Geom2d_BSplineCurve) pc = new Geom2d_BSplineCurve(j ? pb : pa, knots, mults, 5);
    builder.UpdateEdge(edge, pc, faces[j], 0.);
    builder.Range(edge, faces[j], first, last);
  }
}
TopoDS_Solid build_cross_drill(const CrossDrillData& data, const size_t* plane_tokens,
                               const size_t* wall_tokens, FaceLabels* labels) {
  const double r = data.radii[0], R = data.radii[1], hx = data.half[0], hy = data.half[1],
               hz = data.half[2];
  const CadFailure invalid{
      invalid_model,
      "Le découpage des deux supports cylindriques ne correspond pas aux domaines prouvés."};
  const auto center = point_at(data.center, 0);
  const gp_Vec ex(direction(data.axes[0])), ey(direction(data.axes[1])),
      ez(direction(data.axes[2]));
  auto p = [&](double x, double y, double z) {
    return center.Translated(ex * x + ey * y + ez * z);
  };
  // Build in a common local frame for stable cylindrical seam placement.
  const auto small =
      BRepBuilderAPI_MakeFace(gp_Cylinder(gp_Ax3(p(data.small_offset[0], data.small_offset[1], -hz), gp_Dir(ez), gp_Dir(ex)), r), 0.,
                              2. * std::acos(-1.), 0., 2. * hz)
          .Face();
  const auto large =
      BRepBuilderAPI_MakeFace(gp_Cylinder(gp_Ax3(p(-hx, 0, 0), gp_Dir(ex), gp_Dir(ez)), R), 0.,
                              2. * std::acos(-1.), 0., 2. * hx)
          .Face();
  TopTools_ListOfShape args;
  args.Append(small);
  args.Append(large);
  BRepAlgoAPI_Splitter split;
  split.SetArguments(args);
  split.Build();
  if (!split.IsDone() || split.HasErrors())
    throw invalid;
  std::array<TopoDS_Edge, 4> rings;
  std::array<TopoDS_Face, 3> walls;
  const std::array<gp_Pnt, 4> mouths{p(data.small_offset[0], data.small_offset[1], -hz), p(data.small_offset[0], data.small_offset[1], hz), p(-hx, 0, 0), p(hx, 0, 0)};
  for (TopExp_Explorer it(split.Shape(), TopAbs_FACE); it.More(); it.Next()) {
    const auto face = TopoDS::Face(it.Current());
    const auto surface = BRepAdaptor_Surface(face);
    if (surface.GetType() != GeomAbs_Cylinder)
      throw invalid;
    const double rad = surface.Cylinder().Radius();
    const bool small_wall = std::abs(rad - r) <= Precision::Confusion();
    if (!small_wall && std::abs(rad - R) > Precision::Confusion())
      throw invalid;
    unsigned rim_mask = 0;
    for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
      const auto edge = TopoDS::Edge(e.Current());
      BRepAdaptor_Curve curve(edge);
      if (curve.GetType() != GeomAbs_Circle)
        continue;
      const auto circle = curve.Circle();
      for (size_t j = 0; j < 4; ++j)
        if (circle.Location().Distance(mouths[j]) <= Precision::Confusion()) {
          if (!rings[j].IsNull() || (small_wall != (j < 2)) ||
              std::abs(circle.Radius() - rad) > Precision::Confusion() ||
              !BRep_Tool::IsClosed(edge))
            throw invalid;
          rings[j] = edge;
          rim_mask |= 1u << j;
        }
    }
    if (!rim_mask)
      continue;
    const int slot = rim_mask == 1 ? 0 : rim_mask == 2 ? 1 : rim_mask == 12 ? 2 : -1;
    if (slot < 0 || !walls[slot].IsNull())
      throw invalid;
    walls[slot] = TopoDS::Face(face.Reversed());
  }
  if (walls[0].IsNull() || walls[1].IsNull() || walls[2].IsNull())
    throw invalid;
  for (size_t i = 0; i < 2; ++i) {
    for (TopExp_Explorer it(walls[i], TopAbs_EDGE); it.More(); it.Next()) {
      const auto e = TopoDS::Edge(it.Current());
      if (BRepAdaptor_Curve(e).GetType() == GeomAbs_BSplineCurve)
        set_junction(data, e, walls[i], walls[2]);
    }
  }
  BRepBuilderAPI_Sewing sewing(Precision::Confusion());
  sewing.SetMaxTolerance(Precision::Confusion());
  sewing.SetLocalTolerancesMode(false);
  for (size_t i = 0; i < 3; ++i) {
    if (walls[i].IsNull())
      throw invalid;
    sewing.Add(walls[i]);
    if (labels)
      labels->push_back({walls[i], wall_tokens[i]});
  }
  const auto outer = BRepPrimAPI_MakeBox(gp_Ax2(p(-hx, -hy, -hz), gp_Dir(ez), gp_Dir(ex)), 2. * hx,
                                         2. * hy, 2. * hz)
                         .Shape();
  for (TopExp_Explorer it(outer, TopAbs_FACE); it.More(); it.Next()) {
    const auto f = TopoDS::Face(it.Current());
    auto plane = BRepAdaptor_Surface(f).Plane();
    gp_Dir n = plane.Axis().Direction();
    if (f.Orientation() == TopAbs_REVERSED)
      n.Reverse();
    const double px = n.Dot(gp_Dir(ex)), py = n.Dot(gp_Dir(ey)), pz = n.Dot(gp_Dir(ez));
    const int slot = std::abs(px) > .5   ? (px < 0. ? 0 : 3)
                     : std::abs(py) > .5 ? (py < 0. ? 1 : 4)
                                         : (pz < 0. ? 2 : 5);
    const int ring = slot == 0 ? 2 : slot == 3 ? 3 : slot == 2 ? 0 : slot == 5 ? 1 : -1;
    TopoDS_Face result = f;
    if (ring >= 0) {
      if (rings[ring].IsNull())
        throw invalid;
      const auto forward = TopoDS::Face(f.Oriented(TopAbs_FORWARD));
      BRepBuilderAPI_MakeFace cut(plane, BRepTools::OuterWire(forward), true);
      cut.Add(rim_wire(plane, rings[ring], true));
      if (!cut.IsDone() || !BRepCheck_Analyzer(cut.Face(), true, false, true).IsValid())
        throw invalid;
      result = TopoDS::Face(cut.Face().Oriented(f.Orientation()));
    }
    sewing.Add(result);
    if (labels)
      labels->push_back({result, plane_tokens[slot]});
  }
  auto solid = solid_from_sewing(sewing);
  if (labels)
    track_sewn_faces(solid, sewing, *labels);
  return solid;
}
