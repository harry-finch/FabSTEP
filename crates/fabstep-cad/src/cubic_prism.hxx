// A straight sweep of a four-pole graph and three lines, with at most one
// circular hole.
namespace cubic_prism {
struct Data {
  double origin[3], x[3], axis[3];
  double width, depth, height, ordinates[2], bore[3];
};

constexpr double eps = 1.e-7;
void need(bool ok, Status stage) {
  if (!ok)
    throw CadFailure{stage, "Le profil cubique, ses flancs ou son extrusion ne "
                            "correspondent pas au domaine certifié."};
}
struct Frame {
  gp_Pnt origin;
  gp_Dir x, y, axis;
  double w, h, depth, a, b;
  std::optional<gp_Circ> bore;
  gp_Pnt point(double u, double v, double z) const {
    return origin.Translated(gp_Vec(x) * u + gp_Vec(y) * v + gp_Vec(axis) * z);
  }
  std::array<gp_Pnt, 4> poles(double z) const {
    return {point(0, 0, z), point(w / 3., a, z), point(2. * w / 3., b, z),
            point(w, 0, z)};
  }
  double volume() const {
    const double r = bore ? bore->Radius() : 0.;
    return h * (w * (depth - (a + b) / 4.) - std::acos(-1.) * r * r);
  }
  void check(Status s) const {
    for (double d : {w, h, depth})
      need(std::isfinite(d) && d > 16 * eps && d < 1.e7, s);
    need(std::abs(a) < 1.e7 && std::abs(b) < 1.e7 && std::abs(a - b) > eps &&
             std::max({0., a, b}) + eps < depth,
         s);
    if (bore) {
      const gp_Vec offset(origin, bore->Location());
      const double u = offset.Dot(x), v = offset.Dot(y), r = bore->Radius();
      need(r > eps && std::abs(offset.Dot(axis)) < eps && u > r + eps &&
               u < w - r - eps && v > std::max({a, b, 0.}) + r + eps &&
               v < depth - r - eps,
           s);
    }
  }
};

Frame input(const Data &d, Status s) {
  for (int i = 0; i < 3; ++i)
    need(std::isfinite(d.origin[i]) && std::abs(d.origin[i]) < 1.e7 &&
             std::isfinite(d.axis[i]) && std::isfinite(d.x[i]) &&
             std::isfinite(d.bore[i]),
         s);
  const gp_Vec xv(d.x[0], d.x[1], d.x[2]), zv(d.axis[0], d.axis[1], d.axis[2]);
  need(std::abs(xv.Magnitude() - 1.) < 1e-12 &&
           std::abs(zv.Magnitude() - 1.) < 1e-12 &&
           std::abs(xv.Dot(zv)) < 1e-12 && d.bore[2] >= 0.,
       s);
  Frame f{point_at(d.origin, 0),
          gp_Dir(xv),
          gp_Dir(zv.Crossed(xv)),
          gp_Dir(zv),
          d.width,
          d.height,
          d.depth,
          d.ordinates[0],
          d.ordinates[1],
          {}};
  if (d.bore[2] > 0.)
    f.bore =
        gp_Circ(gp_Ax2(f.point(d.bore[0], d.bore[1], 0.), f.axis), d.bore[2]);
  f.check(s);
  return f;
}

TopoDS_Solid build(const Frame &f, const size_t *tokens, FaceLabels *labels) {
  const auto p = f.poles(0.);
  TColgp_Array1OfPnt poles(1, 4);
  for (int i = 0; i < 4; ++i)
    poles.SetValue(i + 1, p[i]);
  Handle(Geom_BezierCurve) curve = new Geom_BezierCurve(poles);
  const auto c = f.point(f.w, f.depth, 0.), d = f.point(0., f.depth, 0.);
  std::vector<TopoDS_Edge> profile{
      BRepBuilderAPI_MakeEdge(GeomConvert::CurveToBSplineCurve(curve)),
      BRepBuilderAPI_MakeEdge(p[3], c), BRepBuilderAPI_MakeEdge(c, d),
      BRepBuilderAPI_MakeEdge(d, p[0])};
  BRepBuilderAPI_MakeWire outer;
  for (const auto &e : profile)
    outer.Add(e);
  BRepBuilderAPI_MakeFace base(gp_Pln(f.origin, f.axis), outer.Wire(), true);
  if (f.bore) {
    const TopoDS_Edge e = BRepBuilderAPI_MakeEdge(*f.bore);
    profile.push_back(e);
    base.Add(TopoDS::Wire(BRepBuilderAPI_MakeWire(e).Wire().Reversed()));
  }
  BRepPrimAPI_MakePrism prism(base.Face(), gp_Vec(f.axis) * f.h, false, true);
  const auto solid = TopoDS::Solid(prism.Shape());
  if (labels) {
    labels->emplace_back(prism.FirstShape(), tokens[0]);
    labels->emplace_back(prism.LastShape(), tokens[1]);
    // MakeWire can replace coincident vertices and return new edges. The
    // extrusion history is keyed by the edges of the assembled cap.
    std::set<int> seen;
    for (TopExp_Explorer edge(base.Face(), TopAbs_EDGE); edge.More();
         edge.Next()) {
      const BRepAdaptor_Curve curve(TopoDS::Edge(edge.Current()));
      int owner = 2;
      if (curve.GetType() == GeomAbs_Circle)
        owner = 6;
      else if (curve.GetType() == GeomAbs_Line) {
        const auto center =
            curve.Value((curve.FirstParameter() + curve.LastParameter()) / 2.);
        const auto u = gp_Vec(f.origin, center).Dot(f.x);
        owner = std::abs(u - f.w) < eps ? 3 : std::abs(u) < eps ? 5 : 4;
      }
      auto &generated = prism.Generated(edge.Current());
      need(seen.insert(owner).second && generated.Extent() == 1 &&
               generated.First().ShapeType() == TopAbs_FACE,
           invalid_model);
      labels->emplace_back(generated.First(), tokens[owner]);
    }
    need(seen.size() == profile.size(), invalid_model);
  }
  return solid;
}

struct Proof {
  Frame frame;
  std::array<TopoDS_Edge, 2> rails;
};

bool parallel(const gp_Dir &a, const gp_Dir &b) {
  return a.IsParallel(b, 1.e-9);
}
bool close(const gp_Pnt &a, const gp_Pnt &b) { return a.Distance(b) <= eps; }

Proof verify(const TopoDS_Shape &solid, Status s) {
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(solid, TopAbs_FACE, faces);
  need(faces.Extent() == 6 || faces.Extent() == 7, s);
  TopoDS_Face wall;
  std::optional<gp_Cylinder> cylinder;
  int planes = 0;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto face = TopoDS::Face(faces(i));
    const BRepAdaptor_Surface surface(face, false);
    switch (surface.GetType()) {
    case GeomAbs_Plane:
      ++planes;
      break;
    case GeomAbs_SurfaceOfExtrusion:
      need(wall.IsNull(), s);
      wall = face;
      break;
    case GeomAbs_Cylinder:
      need(!cylinder, s);
      cylinder = surface.Cylinder();
      break;
    default:
      need(false, s);
    }
  }
  need(planes == 5 && !wall.IsNull(), s);
  const BRepAdaptor_Surface surface(wall);
  const gp_Dir axis = surface.Direction();
  const auto basis = cubic_poles(
      *surface.BasisCurve(), surface.FirstUParameter(),
      surface.LastUParameter(), {s, "Profil polynomial incomplet."});
  need(basis.points.size() == 4, s);
  TopTools_IndexedMapOfShape wall_edges;
  TopExp::MapShapes(wall, TopAbs_EDGE, wall_edges);
  need(wall_edges.Extent() == 4, s);
  std::array<TopoDS_Edge, 2> rails;
  std::array<std::vector<gp_Pnt>, 2> p;
  for (int i = 1; i <= 4; ++i) {
    const auto e = TopoDS::Edge(wall_edges(i));
    const BRepAdaptor_Curve curve(e);
    if (curve.GetType() == GeomAbs_Line) {
      need(parallel(curve.Line().Direction(), axis), s);
      continue;
    }
    auto v = cubic_poles(curve, curve.FirstParameter(), curve.LastParameter(),
                         {s, "Arête cubique incomplète."})
                 .points;
    need(v.size() == 4, s);
    const int k = p[0].empty() ? 0 : 1;
    need(p[k].empty(), s);
    p[k] = std::move(v);
    rails[k] = e;
  }
  need(!p[0].empty() && !p[1].empty(), s);
  // Check the untrimmed generating curve too. Both variations consist of
  // the same four Bernstein poles translated along the sweep direction.
  bool hit[2] = {false, false};
  for (double v : {surface.FirstVParameter(), surface.LastVParameter()}) {
    bool matched = false;
    for (int k = 0; k < 2; ++k) {
      if (hit[k])
        continue;
      for (int reverse = 0; reverse < 2; ++reverse) {
        bool same = true;
        for (int j = 0; j < 4; ++j)
          same &= close(basis.points[j].Translated(gp_Vec(axis) * v),
                        p[k][reverse ? 3 - j : j]);
        if (same) {
          hit[k] = true;
          matched = true;
          break;
        }
      }
      if (matched)
        break;
    }
    need(matched, s);
  }
  if (gp_Vec(p[0][0], p[0][3]).Dot(gp_Vec(p[1][0], p[1][3])) < 0.)
    std::reverse(p[1].begin(), p[1].end());
  if (gp_Vec(p[0][0], p[1][0]).Dot(axis) < 0.) {
    std::swap(p[0], p[1]);
    std::swap(rails[0], rails[1]);
  }
  const gp_Vec shift(p[0][0], p[1][0]);
  const double h = shift.Dot(axis);
  const gp_Dir x(gp_Vec(p[0][0], p[0][3]));
  const gp_Dir y = axis.Crossed(x);
  need(std::abs(x.Dot(axis)) < 1e-9 && h > eps, s);
  // Locate the back of the section by the actual transverse edge at this
  // corner.
  double depth = 0.;
  TopTools_IndexedMapOfShape edges;
  TopExp::MapShapes(solid, TopAbs_EDGE, edges);
  for (int i = 1; i <= edges.Extent(); ++i) {
    const BRepAdaptor_Curve c(TopoDS::Edge(edges(i)));
    if (c.GetType() != GeomAbs_Line || !parallel(c.Line().Direction(), y))
      continue;
    auto a = c.Value(c.FirstParameter()), b = c.Value(c.LastParameter());
    if (close(p[0][0], b))
      std::swap(a, b);
    if (close(p[0][0], a)) {
      need(depth == 0., s);
      depth = gp_Vec(a, b).Dot(y);
    }
  }
  if (depth < 0.)
    for (auto &row : p)
      std::reverse(row.begin(), row.end());
  const auto &row = p[0];
  const gp_Dir chord(gp_Vec(row[0], row[3]));
  Frame f{row[0],
          chord,
          axis.Crossed(chord),
          axis,
          row[0].Distance(row[3]),
          h,
          std::abs(depth),
          0.,
          0.,
          {}};
  f.a = gp_Vec(row[0], row[1]).Dot(f.y);
  f.b = gp_Vec(row[0], row[2]).Dot(f.y);
  for (int k = 0; k < 2; ++k) {
    const auto ideal = f.poles(k * h);
    for (int i = 0; i < 4; ++i)
      need(close(ideal[i], p[k][i]), s);
  }
  // Each cap and lateral face must use the complete expected edge cycle.
  // Rail endpoints, back corners and actual circle trims fix all other wires.
  if (cylinder) {
    need(parallel(cylinder->Axis().Direction(), axis), s);
    const auto c = cylinder->Location();
    const auto center =
        c.Translated(gp_Vec(axis) * -gp_Vec(f.origin, c).Dot(axis));
    f.bore = gp_Circ(gp_Ax2(center, axis), cylinder->Radius());
  }
  f.check(s);
  std::set<std::set<int>> expected{{0, 1, 2, 3},   {4, 5, 6, 7},
                                   {0, 4, 8, 9},   {1, 5, 9, 10},
                                   {2, 6, 10, 11}, {3, 7, 8, 11}};
  if (f.bore) {
    expected.erase({0, 1, 2, 3});
    expected.erase({4, 5, 6, 7});
    expected.insert({0, 1, 2, 3, 12});
    expected.insert({4, 5, 6, 7, 13});
    expected.insert({12, 13, 14});
  }
  const auto corner = [&](int i, int k) {
    return f.point((i == 1 || i == 2) ? f.w : 0., i >= 2 ? f.depth : 0., k * h);
  };
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto face = TopoDS::Face(faces(i));
    TopTools_IndexedMapOfShape local;
    TopExp::MapShapes(face, TopAbs_EDGE, local);
    std::set<int> ids;
    for (int j = 1; j <= local.Extent(); ++j) {
      const auto e = TopoDS::Edge(local(j));
      const BRepAdaptor_Curve c(e);
      const auto a = c.Value(c.FirstParameter()),
                 b = c.Value(c.LastParameter());
      const auto joins = [&](gp_Pnt p, gp_Pnt q) {
        return (close(a, p) && close(b, q)) || (close(b, p) && close(a, q));
      };
      int id = -1;
      if (e.IsSame(rails[0]))
        id = 0;
      else if (e.IsSame(rails[1]))
        id = 4;
      else if (c.GetType() == GeomAbs_Line) {
        for (int k = 0; k < 2; ++k)
          for (int m = 1; m < 4; ++m)
            if (joins(corner(m, k), corner((m + 1) % 4, k)))
              id = 4 * k + m;
        for (int m = 0; m < 4; ++m)
          if (joins(corner(m, 0), corner(m, 1)))
            id = 8 + m;
        if (f.bore && id < 0 && BRep_Tool::IsClosed(e, face) &&
            parallel(c.Line().Direction(), axis))
          id = 14;
      } else if (c.GetType() == GeomAbs_Circle && f.bore) {
        const auto circ = c.Circle();
        need(std::abs(c.LastParameter() - c.FirstParameter() -
                      2. * std::acos(-1.)) < 1e-9 &&
                 parallel(circ.Axis().Direction(), axis) &&
                 std::abs(circ.Radius() - f.bore->Radius()) <= eps,
             s);
        for (int k = 0; k < 2; ++k)
          if (close(circ.Location(),
                    f.bore->Location().Translated(gp_Vec(axis) * (h * k))))
            id = 12 + k;
      }
      need(id >= 0 && ids.insert(id).second, s);
    }
    need(expected.erase(ids) == 1, s);
  }
  need(expected.empty(), s);
  return {f, rails};
}

void same(const Frame &a, const Frame &b, Status s) {
  // Cad transfer may reverse both parametrizations. Compare the control hulls
  // in world coordinates.
  std::vector<gp_Pnt> expected, found;
  auto gather = [](const Frame &f, std::vector<gp_Pnt> &pts) {
    for (double z : {0., f.h}) {
      for (auto p : f.poles(z))
        pts.push_back(p);
      pts.push_back(f.point(0., f.depth, z));
      pts.push_back(f.point(f.w, f.depth, z));
      if (f.bore)
        pts.push_back(f.bore->Location().Translated(gp_Vec(f.axis) * z));
    }
  };
  gather(a, expected);
  gather(b, found);
  need(expected.size() == found.size() &&
           (!a.bore || std::abs(a.bore->Radius() - b.bore->Radius()) <= eps),
       s);
  for (const auto &p : expected) {
    const auto i = std::find_if(found.begin(), found.end(),
                                [&](const gp_Pnt &q) { return close(p, q); });
    need(i != found.end(), s);
    found.erase(i);
  }
}
} // namespace cubic_prism
