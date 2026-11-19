namespace atlas_edge {
// A positive-weight quadratic with endpoints A,B is the minor circular arc
// through those points iff its interior control is the tangent intersection
// and w1/sqrt(w0*w2) is the cosine of half the sweep. These invariants
// survive reversal and rational reparameterization by endpoint weights.
struct Trace {
  gp_Pnt a, b, middle;
  double bound, weight;
};
inline std::optional<Trace> trace(const TopoDS_Edge &edge) {
  const BRepAdaptor_Curve curve(edge);
  const double lo = curve.FirstParameter(), hi = curve.LastParameter();
  if (!std::isfinite(lo) || !std::isfinite(hi) || !(lo < hi))
    return {};
  const auto a = curve.Value(lo), b = curve.Value(hi);
  const gp_Vec chord(a, b);
  const double length = chord.Magnitude();
  if (!(length > 1e-12))
    return {};
  const gp_Pnt mid((a.XYZ() + b.XYZ()) * .5);
  if (curve.GetType() == GeomAbs_Line)
    return Trace{a, b, mid, 0., 1.};
  if (curve.GetType() == GeomAbs_Circle) {
    if (hi - lo >= M_PI + 1e-8)
      return {};
    return Trace{a, b, curve.Value((lo + hi) * .5), 0., std::cos((hi - lo) * .5)};
  }
  Handle(Geom_BSplineCurve) bs;
  if (curve.GetType() == GeomAbs_BSplineCurve)
    bs = curve.BSpline();
  else if (curve.GetType() == GeomAbs_BezierCurve)
    bs = GeomConvert::CurveToBSplineCurve(curve.Bezier());
  else
    return {};
  if (bs->Degree() > 3 || bs->NbPoles() > 256 || bs->IsPeriodic() ||
      std::abs(bs->FirstParameter() - lo) > Precision::PConfusion() ||
      std::abs(bs->LastParameter() - hi) > Precision::PConfusion())
    return {};
  double prev = 0., linear = 0.;
  for (int i = 1; i <= bs->NbPoles(); ++i) {
    if (!(bs->Weight(i) > 0.))
      return {};
    gp_Vec d(a, bs->Pole(i));
    double t = d.Dot(chord) / (length * length);
    if (t < prev - 1e-10 || t > 1. + 1e-10)
      return {};
    linear = std::max(linear, a.Translated(chord * std::clamp(t, 0., 1.)).Distance(bs->Pole(i)));
    prev = t;
  }
  if (linear <= 2e-7)
    return Trace{a, b, mid, linear, 1.};
  if (bs->Degree() != 2 || bs->NbPoles() != 3 || bs->NbKnots() != 2)
    return {};
  const double weight = bs->Weight(2) / std::sqrt(bs->Weight(1) * bs->Weight(3));
  if (!(weight > 0. && weight < 1.))
    return {};
  const auto pole = bs->Pole(2);
  // The canonical symmetric control lies in the plane of A, pole, B, on
  // their perpendicular bisector. Its distance follows from the half angle.
  const gp_Vec offset(mid, pole);
  const auto ortho = offset - chord * (offset.Dot(chord) / (length * length));
  if (ortho.Magnitude() <= 1e-14)
    return {};
  const auto exact =
      mid.Translated(ortho.Normalized() * (.5 * length * std::sqrt((1. - weight) * (1. + weight)) / weight));
  const double drift = pole.Distance(exact);
  if (drift > 2e-7)
    return {};
  return Trace{a, b, mid.Translated(gp_Vec(mid, exact) * (weight / (1. + weight))), drift, weight};
}
} // namespace atlas_edge
