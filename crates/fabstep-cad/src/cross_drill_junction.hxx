// Reparameterize one witnessed branch and both cylindrical traces together.
// x, y span the section normal to the small cylinder, and z follows its axis.
// The large axis is sine*x+cosine*z. OCCT supplies the trim and its endpoints.
template<class Error>
void set_cross_drill_junction(const TopoDS_Edge& edge, const TopoDS_Face& small,
    const TopoDS_Face& big, const gp_Pnt& center, const gp_Vec& x, const gp_Vec& y,
    const gp_Vec& z, double sine, double cosine, double offset, double r, double R,
    const Error& invalid) {
  const BRepAdaptor_Curve curve(edge);
  const double first = curve.FirstParameter(), last = curve.LastParameter(),
               period = 2. * std::acos(-1.);
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
  const gp_Vec middle(center, curve.Value((first + last) / 2.));
  const double sign = sine*middle.Dot(z)-cosine*middle.Dot(x) > 0. ? 1. : -1.;
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
                 world_y = offset + r*s,
                 h = std::sqrt((R-world_y)*(R+world_y));
    const double hp = -world_y*r*c/h, hpp = (-r*r*c*c+world_y*r*s-hp*hp)/h;
    const auto p = center.Translated(x * (r * c) + y * (r * s) + z * ((sign * h + cosine*r*c)/sine));
    if ((i == 0 || i == n) &&
        p.Distance(curve.Value(i == 0 ? first : last)) > Precision::Confusion())
      throw invalid;
    const gp_Vec v = x * (-r * s) + y * (r * c) + z * ((sign * hp - cosine*r*s)/sine),
                 a = x * (-r * c) + y * (-r * s) + z * ((sign * hpp - cosine*r*c)/sine);
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
