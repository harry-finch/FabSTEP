// One bicubic graph, rectangular horizontal rim, four sides and a base.
// Included inside the adapter's private namespace.
namespace height_patch {
constexpr double eps = 1e-7;

template <typename P> double axis_coord(const P &point, int k) {
  return k == 0 ? point.X() : k == 1 ? point.Y() : point.Z();
}

void need(bool condition, Status stage) {
  if (!condition)
    throw CadFailure{
        stage, "Domaine ou frontières de la surface bicubique non démontrés."};
}

gp_Pnt pole(const HeightPatchData &d, int i, int j) {
  return {d.origin[0] + d.size[0] * i / 3, d.origin[1] + d.size[1] * j / 3,
          d.origin[2] + d.heights[i][j]};
}

void check(const HeightPatchData &d, Status stage) {
  for (double coord : d.origin)
    need(std::isfinite(coord) && std::abs(coord) * 1e-14 < eps, stage);
  for (double length : d.size)
    need(std::isfinite(length) && length > 2 * eps && length * 1e-14 < eps,
         stage);
  for (int i = 0; i < 4; i++)
    for (int j = 0; j < 4; j++) {
      const double z = d.heights[i][j];
      need(std::isfinite(z) && z > 2 * eps && z * 1e-14 < eps, stage);
      if (i == 0 || j == 0 || i == 3 || j == 3)
        need(std::abs(z - d.heights[0][0]) <= eps, stage);
    }
}

TopoDS_Solid build(const HeightPatchData &d, const size_t *tokens,
                   FaceLabels *labels) {
  check(d, invalid_model);
  TColgp_Array2OfPnt poles(1, 4, 1, 4);
  for (int i = 0; i < 4; i++)
    for (int j = 0; j < 4; j++)
      poles.SetValue(i + 1, j + 1, pole(d, i, j));
  TColStd_Array1OfReal knots(1, 2);
  knots.SetValue(1, 0.);
  knots.SetValue(2, 1.);
  TColStd_Array1OfInteger mult(1, 2);
  mult.Init(4);
  Handle(Geom_BSplineSurface) surf =
      new Geom_BSplineSurface(poles, knots, knots, mult, mult, 3, 3);
  BRepBuilderAPI_Sewing sewing(eps);
  auto append = [&](const TopoDS_Face &face, size_t id) {
    sewing.Add(face);
    if (labels)
      labels->emplace_back(face, tokens[id]);
  };
  append(BRepBuilderAPI_MakeFace(surf, 0., 1., 0., 1., eps).Face(), 5);
  std::array<gp_Pnt, 4> top{pole(d, 0, 0), pole(d, 3, 0), pole(d, 3, 3),
                            pole(d, 0, 3)};
  auto base = top;
  for (auto &p : base)
    p.SetZ(d.origin[2]);
  auto plane = [&](gp_Pnt a, gp_Pnt b, gp_Pnt c, gp_Pnt e, size_t id) {
    BRepBuilderAPI_MakePolygon wire(a, b, c, e, true);
    append(BRepBuilderAPI_MakeFace(wire.Wire(), true).Face(), id);
  };
  plane(base[3], base[2], base[1], base[0], 4);
  const size_t order[] = {2, 1, 3, 0};
  for (int i = 0; i < 4; i++) {
    int j = (i + 1) % 4;
    plane(base[i], base[j], top[j], top[i], order[i]);
  }
  const auto result = solid_from_sewing(sewing);
  if (labels) {
    track_sewn_faces(result, sewing, *labels);
    need(labels->size() == 6, invalid_model);
  }
  return result;
}

// Read all 16 poles back. The five planes fix the sides of the box.
// Edge collinearity and the six face wires exclude arbitrary trimming.
HeightPatchData verify(const TopoDS_Shape &shape, Status stage,
                       const HeightPatchData *expected = nullptr) {
  TopTools_IndexedMapOfShape faces, vertices, edges;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  TopExp::MapShapes(shape, TopAbs_VERTEX, vertices);
  TopExp::MapShapes(shape, TopAbs_EDGE, edges);
  need(faces.Extent() == 6 && edges.Extent() == 12 && vertices.Extent() == 8,
       stage);
  const auto first = BRep_Tool::Pnt(TopoDS::Vertex(vertices(1)));
  std::array<double, 3> lo{first.X(), first.Y(), first.Z()}, hi = lo;
  for (int i = 2; i <= vertices.Extent(); ++i) {
    const auto p = BRep_Tool::Pnt(TopoDS::Vertex(vertices(i)));
    for (int k = 0; k < 3; ++k) {
      lo[k] = std::min(lo[k], axis_coord(p, k));
      hi[k] = std::max(hi[k], axis_coord(p, k));
    }
  }
  HeightPatchData d{{lo[0], lo[1], lo[2]}, {hi[0] - lo[0], hi[1] - lo[1]}, {}};
  const double rim = hi[2] - lo[2];
  need(d.size[0] > eps && d.size[1] > eps && rim > eps, stage);
  unsigned corners = 0, planes = 0;
  for (int i = 1; i <= 8; i++) {
    auto p = BRep_Tool::Pnt(TopoDS::Vertex(vertices(i)));
    unsigned corner = 0;
    for (int k = 1; k <= 3; k++) {
      if (std::abs(axis_coord(p, k - 1) - hi[k - 1]) <= eps)
        corner |= 1u << (k - 1);
      else
        need(std::abs(axis_coord(p, k - 1) - lo[k - 1]) <= eps, stage);
    }
    corners |= 1u << corner;
  }
  need(corners == 255, stage);
  bool roof = false;
  for (int i = 1; i <= 6; i++) {
    const auto face = TopoDS::Face(faces(i));
    TopTools_IndexedMapOfShape wires, trim;
    TopExp::MapShapes(face, TopAbs_WIRE, wires);
    TopExp::MapShapes(face, TopAbs_EDGE, trim);
    need(wires.Extent() == 1 && trim.Extent() == 4, stage);
    BRepAdaptor_Surface surface(face);
    if (surface.GetType() == GeomAbs_Plane) {
      auto pl = surface.Plane();
      for (int k = 1; k <= 3; k++) {
        if (std::abs(std::abs(axis_coord(pl.Axis().Direction(), k - 1)) - 1.) >
            1e-12)
          continue;
        const double coord = axis_coord(pl.Location(), k - 1);
        if (std::abs(coord - lo[k - 1]) <= eps)
          planes |= 1u << (2 * (k - 1));
        if (k < 3 && std::abs(coord - hi[k - 1]) <= eps)
          planes |= 2u << (2 * (k - 1));
      }
      continue;
    }
    need(!roof && surface.GetType() == GeomAbs_BSplineSurface, stage);
    roof = true;
    const auto s = surface.BSpline();
    need(s->UDegree() == 3 && s->VDegree() == 3 && s->NbUPoles() == 4 &&
             s->NbVPoles() == 4 && !s->IsURational() && !s->IsVRational() &&
             !s->IsUPeriodic() && !s->IsVPeriodic(),
         stage);
    need(s->NbUKnots() == 2 && s->NbVKnots() == 2 && s->UMultiplicity(1) == 4 &&
             s->UMultiplicity(2) == 4 && s->VMultiplicity(1) == 4 &&
             s->VMultiplicity(2) == 4,
         stage);
    // The face UV box may be expanded by OCCT. Check the four physical
    // trimming vertices against the actual corners of this polynomial span.
    unsigned roof_corners = 0;
    for (TopExp_Explorer it(face, TopAbs_VERTEX); it.More(); it.Next()) {
      const auto p = BRep_Tool::Pnt(TopoDS::Vertex(it.Current()));
      for (int a = 0; a < 2; ++a)
        for (int b = 0; b < 2; ++b)
          if (p.Distance(s->Pole(1 + 3 * a, 1 + 3 * b)) <= eps)
            roof_corners |= 1u << (2 * a + b);
    }
    need(roof_corners == 15, stage);
    const auto start = s->Pole(1, 1), u = s->Pole(4, 1), v = s->Pole(1, 4);
    // Parameter directions can reverse or swap on transfer, but both
    // must span full orthogonal horizontal axes and vary affinely.
    const bool ux = std::abs(u.X() - start.X()) > eps;
    const double du = ux ? u.X() - start.X() : u.Y() - start.Y();
    const double dv = ux ? v.Y() - start.Y() : v.X() - start.X();
    need(std::abs(std::abs(du) - d.size[ux ? 0 : 1]) <= eps &&
             std::abs(std::abs(dv) - d.size[ux ? 1 : 0]) <= eps,
         stage);
    for (int a = 0; a < 4; a++)
      for (int b = 0; b < 4; b++) {
        const int ia = du > 0 ? a : 3 - a, ib = dv > 0 ? b : 3 - b;
        const int x = ux ? ia : ib, y = ux ? ib : ia;
        const gp_Pnt p = s->Pole(a + 1, b + 1);
        need(std::abs(p.X() - d.origin[0] - d.size[0] * x / 3) <= eps &&
                 std::abs(p.Y() - d.origin[1] - d.size[1] * y / 3) <= eps,
             stage);
        d.heights[x][y] = p.Z() - d.origin[2];
      }
  }
  need(planes == 31 && roof && std::abs(d.heights[0][0] - rim) <= eps, stage);
  for (int i = 1; i <= 12; i++) {
    auto edge = TopoDS::Edge(edges(i));
    BRepAdaptor_Curve curve(edge);
    const auto a = curve.Value(curve.FirstParameter()),
               b = curve.Value(curve.LastParameter());
    const gp_Vec delta(a, b);
    const double len = delta.Magnitude();
    need(len > eps, stage);
    int aligned = 0;
    for (int k = 1; k <= 3; k++)
      aligned += std::abs(axis_coord(delta, k - 1)) > eps;
    need(aligned == 1, stage);
    if (curve.GetType() == GeomAbs_Line)
      continue;
    need(curve.GetType() == GeomAbs_BSplineCurve, stage);
    const auto spline = curve.BSpline();
    need(spline->NbPoles() <= 4 && !spline->IsRational(), stage);
    const gp_Vec dir = delta / len;
    double last = -eps;
    for (int j = 1; j <= spline->NbPoles(); j++) {
      const gp_Vec q(a, spline->Pole(j));
      double t = q.Dot(dir);
      need(q.Crossed(dir).Magnitude() <= eps && t >= last - eps &&
               t <= len + eps,
           stage);
      last = t;
    }
  }
  check(d, stage);
  if (expected) {
    for (int i = 0; i < 3; i++)
      need(std::abs(d.origin[i] - expected->origin[i]) <= eps, stage);
    for (int i = 0; i < 2; i++)
      need(std::abs(d.size[i] - expected->size[i]) <= eps, stage);
    for (int i = 0; i < 4; i++)
      for (int j = 0; j < 4; j++)
        need(std::abs(d.heights[i][j] - expected->heights[i][j]) <= eps, stage);
  }
  return d;
}
// Bernstein second differences bound each Hessian component on the full
// unit square. Linear interpolation on a cell of side 1/n has vertical
// error at most (Muu + 2 Muv + Mvv)/(2 n²) by Taylor's theorem.
double mesh_divisions(const HeightPatchData &d, double deflection) {
  double uu = 0., uv = 0., vv = 0.;
  const auto &h = d.heights;
  for (int i = 0; i < 4; i++)
    for (int j = 0; j < 4; j++) {
      if (i < 2)
        uu = std::max(uu,
                      6. * std::abs(h[i + 2][j] - 2. * h[i + 1][j] + h[i][j]));
      if (j < 2)
        vv = std::max(vv,
                      6. * std::abs(h[i][j + 2] - 2. * h[i][j + 1] + h[i][j]));
      if (i < 3 && j < 3)
        uv = std::max(uv, 9. * std::abs(h[i + 1][j + 1] - h[i + 1][j] -
                                        h[i][j + 1] + h[i][j]));
    }
  return std::max(
      1., std::ceil(std::sqrt((uu + 2. * uv + vv) / (2. * deflection))));
}

double volume(const HeightPatchData &d) {
  double sum = 0.;
  for (auto &row : d.heights)
    for (double z : row)
      sum += z;
  return d.size[0] * d.size[1] * sum / 16.;
}
} // namespace height_patch
