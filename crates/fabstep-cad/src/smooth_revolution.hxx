// A C2 body has exactly one full-turn polynomial wall and two disk caps.
// No other spline face or edge is admitted through this certificate.
constexpr double revolution_precision = 5.e-8;
constexpr double revolution_period = 2. * 3.14159265358979323846;

struct RevolutionProof {
  SmoothRevolutionData domain;
  TopoDS_Edge seam;
  std::array<TopoDS_Face, 2> caps;
  TopoDS_Face wall;
  bool reverse_basis;
  double max_radius, max_mixed_derivative, max_axial_second_derivative;
};

gp_Pnt revolution_center(const SmoothRevolutionData &d, double t) {
  return point_at(d.origin, 0).Translated(gp_Vec(direction(d.axis)) * (d.height * t));
}

// A clamped cubic with an optional internal knot of multiplicity two.
// Expanding that knot once yields the two adjacent Bernstein polygons.
struct MeridianPoles {
  std::vector<gp_Pnt> points;
  double knot = 0.;
  double parameter(size_t i) const {
    if (!knot)
      return i / 3.;
    return i <= 3 ? knot * i / 3. : knot + (1. - knot) * (i - 3) / 3.;
  }
  void reverse() {
    std::reverse(points.begin(), points.end());
    if (knot)
      knot = 1. - knot;
  }
};

MeridianPoles cubic_poles(const Adaptor3d_Curve &adaptor, double first, double last,
                          const CadFailure &invalid) {
  MeridianPoles result;
  auto &poles = result.points;
  const auto kind = adaptor.GetType();
  if ((kind != GeomAbs_BezierCurve && kind != GeomAbs_BSplineCurve) || adaptor.Degree() != 3 ||
      (adaptor.NbPoles() != 4 && adaptor.NbPoles() != 6) || adaptor.IsRational())
    throw invalid;
  double low, high;
  if (kind == GeomAbs_BezierCurve) {
    const auto c = adaptor.Bezier();
    for (int i = 1; i <= 4; ++i)
      poles.push_back(c->Pole(i));
    low = c->FirstParameter();
    high = c->LastParameter();
  } else {
    const int n = adaptor.NbKnots();
    if (n != 2 && n != 3)
      throw invalid;
    const auto c = adaptor.BSpline();
    if (c->IsPeriodic() || c->Multiplicity(1) != 4 || c->Multiplicity(n) != 4 || c->NbPoles() != 2 * n ||
        (n == 3 && c->Multiplicity(2) != 2))
      throw invalid;
    low = c->Knot(1);
    high = c->Knot(n);
    if (n == 3) {
      result.knot = (c->Knot(2) - low) / (high - low);
      if (!(result.knot >= .125 - 1e-12 && result.knot <= .875 + 1e-12))
        throw invalid;
    }
    for (int i = 1; i <= c->NbPoles(); ++i) {
      if (n == 3 && i == 4)
        poles.push_back(c->Pole(3).Translated(gp_Vec(c->Pole(3), c->Pole(4)) * result.knot));
      poles.push_back(c->Pole(i));
    }
  }
  double speed = 1.;
  for (size_t i = 1; i < poles.size(); ++i)
    speed =
        std::max(speed, poles[i].Distance(poles[i - 1]) / (result.parameter(i) - result.parameter(i - 1)));
  const double parameter_error =
      std::max({std::abs(low), std::abs(high - 1.), std::abs(first), std::abs(last - 1.)});
  if (parameter_error > 1.e-12 || parameter_error * speed > revolution_precision)
    throw invalid;
  return result;
}

double radius_control(const SmoothRevolutionData &d, size_t i) {
  return i <= 3 ? d.radii[i] : d.next_radii[i - 4];
}

double revolution_volume(const SmoothRevolutionData &d) {
  constexpr double c3[]{1, 3, 3, 1}, c6[]{1, 6, 15, 20, 15, 6, 1};
  double square_integral = 0.;
  const int spans = d.knot ? 2 : 1;
  for (int span = 0; span < spans; ++span) {
    const double width = spans == 1 ? 1. : (span ? 1. - d.knot : d.knot);
    for (size_t i = 0; i < 4; ++i)
      for (size_t j = 0; j < 4; ++j)
        square_integral += width * radius_control(d, 3 * span + i) * radius_control(d, 3 * span + j) *
                           c3[i] * c3[j] / (7. * c6[i + j]);
  }
  return (revolution_period / 2.) * d.height * square_integral;
}

RevolutionProof verify_revolution(const TopoDS_Shape &solid, const SmoothRevolutionData *expected,
                                  Status status) {
  const CadFailure invalid{
      status,
      "La révolution cubique, sa couture ou ses deux disques ne correspondent pas au domaine prouvé."};
  RevolutionProof proof{};
  std::vector<TopoDS_Face> planes;
  for (TopExp_Explorer it(solid, TopAbs_FACE); it.More(); it.Next()) {
    auto face = TopoDS::Face(it.Current());
    switch (BRepAdaptor_Surface(face).GetType()) {
    case GeomAbs_Plane:
      if (planes.size() == 2)
        throw invalid;
      planes.push_back(face);
      break;
    case GeomAbs_SurfaceOfRevolution:
      if (!proof.wall.IsNull())
        throw invalid;
      proof.wall = face;
      break;
    default:
      throw invalid;
    }
    TopExp_Explorer wire(face, TopAbs_WIRE);
    if (!wire.More())
      throw invalid;
    wire.Next();
    if (wire.More())
      throw invalid;
  }
  if (planes.size() != 2 || proof.wall.IsNull())
    throw invalid;
  const BRepAdaptor_Surface wall(proof.wall);
  const auto native_axis = wall.AxeOfRevolution();
  auto meridian = cubic_poles(*wall.BasisCurve(), wall.FirstVParameter(), wall.LastVParameter(), invalid);
  // Anchor the certificate to the physical axis and endpoint. STEP may reverse
  // the basis or the axis; neither changes the cap-to-pole correspondence.
  auto &poles = meridian.points;
  gp_Vec axis(native_axis.Direction());
  if (expected && axis.Dot(gp_Vec(direction(expected->axis))) < 0.)
    axis.Reverse();
  const bool reverse_basis = gp_Vec(poles.front(), poles.back()).Dot(axis) < 0.;
  proof.reverse_basis = reverse_basis;
  if (reverse_basis)
    meridian.reverse();
  const auto origin =
      native_axis.Location().Translated(axis * gp_Vec(native_axis.Location(), poles[0]).Dot(axis));
  auto &d = proof.domain;
  d.height = gp_Vec(poles.front(), poles.back()).Dot(axis);
  origin.Coord(d.origin[0], d.origin[1], d.origin[2]);
  axis.Coord(d.axis[0], d.axis[1], d.axis[2]);
  if (!(d.height > revolution_precision))
    throw invalid;
  const gp_Vec radial = gp_Vec(origin, poles[0]).Normalized();
  d.knot = meridian.knot;
  if (expected && (bool(d.knot) != bool(expected->knot) ||
                   std::abs(d.knot - expected->knot) * d.height > revolution_precision))
    throw invalid;
  constexpr double derivative_margin = 1. + 1.e-11;
  for (size_t i = 0; i < poles.size(); ++i) {
    const double t = meridian.parameter(i);
    const auto center = revolution_center(d, t);
    const double r = gp_Vec(center, poles[i]).Dot(radial);
    if (!(r > revolution_precision) ||
        center.Translated(radial * r).Distance(poles[i]) > revolution_precision)
      throw invalid;
    if (i <= 3)
      d.radii[i] = r;
    else
      d.next_radii[i - 4] = r;
    proof.max_radius = std::max(proof.max_radius, gp_Vec(origin, poles[i]).Crossed(axis).Magnitude());
    if (expected) {
      const double expected_t =
          !expected->knot
              ? i / 3.
              : (i <= 3 ? expected->knot * i / 3. : expected->knot + (1. - expected->knot) * (i - 3) / 3.);
      const double expected_r = radius_control(*expected, i);
      if (std::abs(r - expected_r) > revolution_precision ||
          revolution_center(*expected, expected_t).Translated(radial * expected_r).Distance(poles[i]) >
              revolution_precision)
        throw invalid;
    }
  }
  // The double internal knot makes the derivative continuous, including
  // after STEP transfer. On the complete chart its Lipschitz constant is at
  // most the larger span Hessian scaled to the global parameter.
  for (size_t off = 0; off + 3 < poles.size(); off += 3) {
    const double inv = derivative_margin / (meridian.parameter(off + 3) - meridian.parameter(off));
    for (size_t i = off + 1; i <= off + 3; ++i) {
      const gp_Vec first(poles[i - 1], poles[i]);
      proof.max_mixed_derivative =
          std::max(proof.max_mixed_derivative, 3. * inv * first.Crossed(axis).Magnitude());
      if (i >= off + 2)
        proof.max_axial_second_derivative =
            std::max(proof.max_axial_second_derivative,
                     6. * inv * inv * (first - gp_Vec(poles[i - 2], poles[i - 1])).Magnitude());
    }
    const double a = radius_control(d, off), b = radius_control(d, off + 1), c = radius_control(d, off + 2),
                 e = radius_control(d, off + 3);
    if (std::max(std::abs(3 * b - 2 * a - e), std::abs(3 * c - a - 2 * e)) <= revolution_precision)
      throw invalid;
  }
  const double max_radius = proof.max_radius;
  if (std::abs(wall.LastUParameter() - wall.FirstUParameter() - revolution_period) *
          std::max(1., max_radius) >
      revolution_precision)
    throw invalid;
  if (expected) {
    if (axis.Crossed(gp_Vec(direction(expected->axis))).Magnitude() * std::max(1., max_radius) >
            revolution_precision ||
        std::abs(d.knot - expected->knot) * proof.max_mixed_derivative > revolution_precision)
      throw invalid;
    if (origin.Distance(revolution_center(*expected, 0)) > revolution_precision ||
        revolution_center(d, 1).Distance(revolution_center(*expected, 1)) > revolution_precision)
      throw invalid;
  }
  // Both circle edges must be shared with the wall, with a single outward
  // disk on each end. Its other two coedges are the opposite uses of one seam.
  std::array<TopoDS_Edge, 2> rims;
  for (const auto &cap : planes) {
    TopExp_Explorer it(cap, TopAbs_EDGE);
    if (!it.More())
      throw invalid;
    auto edge = TopoDS::Edge(it.Current());
    it.Next();
    if (it.More() || BRep_Tool::Degenerated(edge))
      throw invalid;
    BRepAdaptor_Curve curve(edge);
    if (curve.GetType() != GeomAbs_Circle)
      throw invalid;
    auto circle = curve.Circle();
    BRepAdaptor_Curve2d trim(edge, cap);
    if (trim.GetType() != GeomAbs_Circle)
      throw invalid;
    const auto cap_surface = BRepAdaptor_Surface(cap);
    const auto pc = trim.Circle();
    const auto c = pc.Location();
    if (cap_surface.Value(c.X(), c.Y()).Distance(circle.Location()) > revolution_precision ||
        std::abs(pc.Radius() - circle.Radius()) > revolution_precision ||
        std::abs(trim.FirstParameter() - curve.FirstParameter()) * circle.Radius() > revolution_precision ||
        std::abs(trim.LastParameter() - curve.LastParameter()) * circle.Radius() > revolution_precision)
      throw invalid;
    for (double fraction : {0., .25, .5, .75}) {
      const double t = curve.FirstParameter() + fraction * (curve.LastParameter() - curve.FirstParameter());
      const auto xy = trim.Value(t);
      if (cap_surface.Value(xy.X(), xy.Y()).Distance(curve.Value(t)) > revolution_precision)
        throw invalid;
    }
    int end =
        circle.Location().Distance(origin) < circle.Location().Distance(revolution_center(d, 1)) ? 0 : 1;
    const auto plane = BRepAdaptor_Surface(cap).Plane();
    // A revolved planar face can have an indirect local frame before STEP.
    auto normal = gp_Vec(plane.Position().XDirection()).Crossed(gp_Vec(plane.Position().YDirection()));
    if (cap.Orientation() == TopAbs_REVERSED)
      normal.Reverse();
    if (!rims[end].IsNull() || normal.Dot(axis) * (end ? 1. : -1.) <= 0. ||
        normal.Crossed(axis).Magnitude() * max_radius > revolution_precision ||
        gp_Vec(circle.Axis().Direction()).Crossed(axis).Magnitude() * max_radius > revolution_precision ||
        plane.Distance(circle.Location()) > revolution_precision ||
        circle.Location().Distance(revolution_center(d, end)) > revolution_precision ||
        std::abs(circle.Radius() - radius_control(d, end ? poles.size() - 1 : 0)) > revolution_precision ||
        std::abs(curve.LastParameter() - curve.FirstParameter() - revolution_period) *
                std::max(1., max_radius) >
            revolution_precision)
      throw invalid;
    rims[end] = edge;
    proof.caps[end] = cap;
  }
  // Every wall coedge must be a straight UV side of the full parameter
  // rectangle. Scaling tolerances by derivative bounds charges the trim in
  // millimetres, including levels near a steep portion of the meridian.
  const double u_scale = std::max(1., max_radius);
  const double slope = proof.max_mixed_derivative;
  const double v_scale = std::max(1., std::hypot(d.height, slope));
  const auto u0 = wall.FirstUParameter(), u1 = wall.LastUParameter();
  const auto full_span = [](double a, double b, double low, double high, double scale) {
    return std::max(std::abs(std::min(a, b) - low), std::abs(std::max(a, b) - high)) * scale <=
           revolution_precision;
  };
  int uses[3]{};
  std::array<bool, 2> seam_side{};
  for (TopExp_Explorer it(proof.wall, TopAbs_EDGE); it.More(); it.Next()) {
    const auto edge = TopoDS::Edge(it.Current());
    BRepAdaptor_Curve2d trim(edge, proof.wall);
    if (trim.GetType() != GeomAbs_Line)
      throw invalid;
    const auto a = trim.Value(trim.FirstParameter()), b = trim.Value(trim.LastParameter());
    const BRepAdaptor_Curve curve(edge);
    const double first = curve.FirstParameter(), last = curve.LastParameter();
    if (std::abs(trim.FirstParameter() - first) > 1.e-12 || std::abs(trim.LastParameter() - last) > 1.e-12)
      throw invalid;
    // At one interior point the physical curve selects the angular direction
    // (opposite directions have identical full-circle endpoints).
    for (double t : {first, first + (last - first) * .25, last}) {
      const auto uv = trim.Value(t);
      if (wall.Value(uv.X(), uv.Y()).Distance(curve.Value(t)) > revolution_precision)
        throw invalid;
    }
    const int end = edge.IsSame(rims[0]) ? 0 : edge.IsSame(rims[1]) ? 1 : -1;
    if (end >= 0) {
      const double v = reverse_basis ? 1. - end : end;
      if (++uses[end] != 1 || edge.Orientation() == rims[end].Orientation() ||
          std::max(std::abs(a.Y() - v), std::abs(b.Y() - v)) * v_scale > revolution_precision ||
          !full_span(a.X(), b.X(), u0, u1, u_scale))
        throw invalid;
      continue;
    }
    const int side = std::abs(a.X() - u0) < std::abs(a.X() - u1) ? 0 : 1;
    const double u = side ? u1 : u0;
    if (seam_side[side] ||
        std::max(std::abs(a.X() - u), std::abs(b.X() - u)) * u_scale > revolution_precision ||
        !full_span(a.Y(), b.Y(), 0., 1., v_scale))
      throw invalid;
    seam_side[side] = true;
    if (++uses[2] > 2 || BRep_Tool::Degenerated(edge) || !BRep_Tool::IsClosed(edge, proof.wall))
      throw invalid;
    if (!proof.seam.IsNull()) {
      if (!edge.IsSame(proof.seam) || edge.Orientation() == proof.seam.Orientation())
        throw invalid;
      continue;
    }
    proof.seam = edge;
    auto seam = cubic_poles(curve, curve.FirstParameter(), curve.LastParameter(), invalid);
    if (gp_Vec(seam.points.front(), seam.points.back()).Dot(axis) < 0.)
      seam.reverse();
    if (seam.points.size() != poles.size() || std::abs(seam.knot - d.knot) * v_scale > revolution_precision)
      throw invalid;
    const auto seam_radial = gp_Vec(origin, seam.points.front()).Normalized();
    if (std::abs(seam_radial.Dot(axis)) * max_radius > revolution_precision)
      throw invalid;
    for (size_t i = 0; i < poles.size(); ++i)
      if (revolution_center(d, meridian.parameter(i))
              .Translated(seam_radial * radius_control(d, i))
              .Distance(seam.points[i]) > revolution_precision)
        throw invalid;
  }
  if (uses[0] != 1 || uses[1] != 1 || uses[2] != 2)
    throw invalid;
  return proof;
}

TopoDS_Solid build_smooth_revolution(const SmoothRevolutionData &d, const size_t *caps, const size_t *wall,
                                     FaceLabels *labels) {
  const gp_Vec radial(gp_Ax2(point_at(d.origin, 0), direction(d.axis)).XDirection());
  TColgp_Array1OfPnt poles(1, d.knot ? 6 : 4);
  Handle(Geom_Curve) curve;
  if (d.knot) {
    // Removing the shared Bernstein control yields a six-control B-spline;
    // its double interior knot enforces a single C1 wall by construction.
    for (int i = 0; i < 6; ++i) {
      const int control = i < 3 ? i : i + 1;
      const double t = i < 3 ? d.knot * i / 3. : d.knot + (1. - d.knot) * (i - 2) / 3.;
      poles(i + 1) = revolution_center(d, t).Translated(radial * radius_control(d, control));
    }
    TColStd_Array1OfReal knots(1, 3);
    knots(1) = 0.;
    knots(2) = d.knot;
    knots(3) = 1.;
    TColStd_Array1OfInteger mult(1, 3);
    mult(1) = 4;
    mult(2) = 2;
    mult(3) = 4;
    curve = new Geom_BSplineCurve(poles, knots, mult, 3);
  } else {
    for (int i = 0; i < 4; ++i)
      poles(i + 1) = revolution_center(d, i / 3.).Translated(radial * d.radii[i]);
    curve = new Geom_BezierCurve(poles);
  }
  const auto bottom = revolution_center(d, 0), top = revolution_center(d, 1);
  BRepBuilderAPI_MakeWire wire;
  wire.Add(BRepBuilderAPI_MakeEdge(bottom, poles(1)).Edge());
  wire.Add(BRepBuilderAPI_MakeEdge(curve).Edge());
  wire.Add(BRepBuilderAPI_MakeEdge(poles(poles.Upper()), top).Edge());
  wire.Add(BRepBuilderAPI_MakeEdge(top, bottom).Edge());
  BRepBuilderAPI_MakeFace face(wire.Wire(), true);
  BRepPrimAPI_MakeRevol revol(face.Face(), gp_Ax1(bottom, direction(d.axis)), revolution_period, true);
  const auto solid = TopoDS::Solid(single_solid(revol.Shape(), invalid_solid));
  if (labels) {
    const auto proof = verify_revolution(solid, &d, invalid_solid);
    for (int i = 0; i < 2; ++i)
      labels->emplace_back(proof.caps[i], caps[i]);
    labels->emplace_back(proof.wall, *wall);
  }
  return solid;
}
