// Uses the same continuous Hermite/pcurve and degree-one cycle certificates
// as cross_drill, with a convex outer wall and a concave bore between mouths.
#include "drilled_shaft.hxx"
#include "drilled_shaft_volume.hxx"
DrilledShaftProof verify_drilled_shaft(const TopoDS_Shape &shape, double deflection) {
  constexpr double tol = 1.e-7;
  const char *bad =
      "Les deux supports, les bouchons ou les branches de l’arbre percé sont incohérents.";
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  require(faces.Extent() == 4, bad);
  TopoDS_Face shaft, bore;
  std::vector<TopoDS_Face> caps;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto f = TopoDS::Face(faces(i));
    const BRepAdaptor_Surface s(f);
    if (s.GetType() == GeomAbs_Plane) {
      caps.push_back(f);
      continue;
    }
    require(s.GetType() == GeomAbs_Cylinder, bad);
    auto &target = exterior_cylinder(f, s) ? shaft : bore;
    require(target.IsNull(), bad);
    target = f;
  }
  require(caps.size() == 2 && !shaft.IsNull() && !bore.IsNull(), bad);
  const auto outer = BRepAdaptor_Surface(shaft).Cylinder(),
             inner = BRepAdaptor_Surface(bore).Cylinder();
  const double R = outer.Radius(), r = inner.Radius();
  const auto x = outer.Axis().Direction(), z = inner.Axis().Direction();
  require(R <= 32. + tol && r > 0.1 && std::abs(x.Dot(z)) < 1.e-9, bad);
  const auto y = z.Crossed(x);
  const gp_Vec v(outer.Location(), inner.Location());
  const double offset = v.Dot(gp_Vec(y));
  const auto middle =
      outer.Location().Translated(gp_Vec(x) * v.Dot(gp_Vec(x)) + gp_Vec(y) * offset);
  require(R > 1.5 * (std::abs(offset) + r), bad);
  std::vector<double> levels;
  for (const auto &f : caps) {
    const auto p = BRepAdaptor_Surface(f).Plane();
    require(x.IsParallel(p.Axis().Direction(), 1.e-9), bad);
    TopExp_Explorer e(f, TopAbs_EDGE);
    require(e.More(), bad);
    const auto edge = TopoDS::Edge(e.Current());
    const BRepAdaptor_Curve c(edge);
    e.Next();
    require(!e.More() && c.GetType() == GeomAbs_Circle && BRep_Tool::IsClosed(edge), bad);
    const auto ring = c.Circle();
    require(std::abs(ring.Radius() - R) < tol && ring.Axis().Direction().IsParallel(x, 1.e-9) &&
                gp_Lin(outer.Axis()).Distance(ring.Location()) < tol &&
                p.Distance(ring.Location()) < tol,
            bad);
    levels.push_back(gp_Vec(middle, ring.Location()).Dot(gp_Vec(x)));
  }
  std::sort(levels.begin(), levels.end());
  const double height = levels[1] - levels[0];
  require(levels[0] < -r - 0.1 + tol && levels[1] > r + 0.1 - tol && height <= 32. + tol, bad);
  using namespace cross_drill_lab::certificate_detail;
  size_t remaining = 10000000, leaves = 16384;
  Counter framing{0, remaining};
  Scope frame_scope(framing);
  double cp[3], ax[3], az[3];
  store(cp, middle.XYZ());
  store(ax, x.XYZ());
  store(az, z.XYZ());
  LocalFrame frame(cp, ax, az);
  remaining -= framing.work;
  DrilledShaftProof result{};
  std::vector<CycleArc> links;
  std::vector<cross_drill_lab::Certificate> certs;
  TopTools_IndexedMapOfShape vertices;
  TopTools_IndexedDataMapOfShapeListOfShape incidence;
  TopExp::MapShapesAndUniqueAncestors(shape, TopAbs_EDGE, TopAbs_FACE, incidence);
  require(incidence.Extent() <= 32, bad);
  for (int j = 1; j <= incidence.Extent(); ++j) {
    const auto edge = TopoDS::Edge(incidence.FindKey(j));
    const auto &adj = incidence.FindFromIndex(j);
    const auto a = TopoDS::Face(adj.First()), b = TopoDS::Face(adj.Last());
    const BRepAdaptor_Curve c(edge);
    if (adj.Extent() == 1) {
      require(BRep_Tool::IsClosed(edge, a) && c.GetType() == GeomAbs_Line, bad);
      continue;
    }
    require(adj.Extent() == 2 &&
                occurrence(edge, a).Orientation() != occurrence(edge, b).Orientation(),
            bad);
    if (a.IsSame(caps[0]) || a.IsSame(caps[1]) || b.IsSame(caps[0]) || b.IsSame(caps[1])) {
      require((a.IsSame(shaft) || b.IsSame(shaft)) && c.GetType() == GeomAbs_Circle, bad);
      continue;
    }
    require((a.IsSame(shaft) && b.IsSame(bore)) || (a.IsSame(bore) && b.IsSame(shaft)), bad);
    cross_drill_lab::Budget budget;
    budget.tolerance_mm = tol;
    budget.max_work = remaining;
    budget.max_leaves = leaves;
    budget.mesh_deflection_mm = deflection;
    auto cert = cross_drill_lab::certifyEdge(edge, bore, shaft, r, R, budget, &frame, offset, true);
    if (!cert.ok)
      throw std::runtime_error("Arbre percé, certificat d’intersection : " + cert.reason);
    remaining -= cert.work;
    leaves -= cert.leaves;
    const auto use = occurrence(edge, bore);
    const bool forward = use.Orientation() == TopAbs_FORWARD;
    require(cert.phase_direction * (forward ? 1 : -1) == cert.sign, bad);
    TopoDS_Vertex from, to;
    TopExp::Vertices(use, from, to, true);
    links.push_back({vertices.Add(from), vertices.Add(to), cert.sign, forward});
    result.mesh_edges.push_back({edge, cert.mesh_segments});
    certs.push_back(std::move(cert));
  }
  for (int sign : {-1, 1}) {
    const auto coverage = budgetedBranch(certs, links, sign, tol, remaining, r, R, offset, 0.);
    if (!coverage.ok)
      throw std::runtime_error("Arbre percé, couverture cyclique : " + coverage.reason);
  }
  BRepClass3d_SolidClassifier classifier(shape);
  classifier.Perform(middle, tol);
  require(classifier.State() == TopAbs_OUT, bad);
  classifier.Perform(middle.Translated(gp_Vec(x) * (levels[1] + r) * 0.5), tol);
  require(classifier.State() == TopAbs_IN, bad);
  Counter volume_work{0, remaining};
  Scope volume_scope(volume_work);
  const auto volume = cross_drill_lab::shaft_volume(height, R, r, offset);
  const gp_Pnt start = middle.Translated(gp_Vec(x) * levels[0] - gp_Vec(y) * offset);
  auto &d = result.domain;
  store(d.origin, start.XYZ());
  store(d.shaft, x.XYZ());
  store(d.bore, z.XYZ());
  d.radius = R;
  d.bore_radius = r;
  d.height = height;
  d.level = -levels[0];
  d.offset = -offset;
  result.volume_low = volume.lo;
  result.volume_high = volume.hi;
  return result;
}

// A change of parameter signs leaves the same physical axes and both cap
// centers. An axial shift of the bore preserves the volume and outer bounds
// but changes the two piercing locations.
bool same_shaft_geometry(const DrilledShaftData &a, const DrilledShaftData &b) {
  constexpr double tol = 1.e-7;
  if (std::abs(a.height - b.height) > tol || std::abs(a.radius - b.radius) > tol ||
      std::abs(a.bore_radius - b.bore_radius) > tol ||
      !dir(a.shaft).IsParallel(dir(b.shaft), 1.e-9) || !dir(a.bore).IsParallel(dir(b.bore), 1.e-9))
    return false;
  const auto caps = [](const DrilledShaftData &d) {
    const auto p = point(d.origin);
    return std::array<gp_Pnt, 2>{p, p.Translated(gp_Vec(dir(d.shaft)) * d.height)};
  };
  const auto left = caps(a), right = caps(b);
  if (!((left[0].Distance(right[0]) <= tol && left[1].Distance(right[1]) <= tol) ||
        (left[0].Distance(right[1]) <= tol && left[1].Distance(right[0]) <= tol)))
    return false;
  const auto bore_center = [](const DrilledShaftData &d) {
    const auto z = dir(d.shaft), x = dir(d.bore);
    return point(d.origin).Translated(gp_Vec(z) * d.level + gp_Vec(z.Crossed(x)) * d.offset);
  };
  return bore_center(a).Distance(bore_center(b)) <= tol;
}
