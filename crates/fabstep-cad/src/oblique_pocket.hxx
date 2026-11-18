// Included inside the private OCCT implementation namespace.
namespace pocket {
using ArcPoints = std::array<gp_Pnt, 3>; // start, midpoint, end

struct Geometry {
  gp_Pnt near_center, far_center;
  ArcPoints circle, ellipse;
  double radius, major;
};

struct Domain {
  std::array<Geometry, 2> walls;
  size_t count;
};

void require(bool condition, Status status) {
  if (!condition)
    throw CadFailure{status, "Le congé cylindrique ne correspond pas à ses butées et à ses deux plans tangents."};
}

Geometry geometry(const MixedArcData& circle, const EllipticArcData& ellipse, const double* points) {
  const auto origin = point_at(circle.center, 0);
  const auto axis = direction(circle.axis);
  const gp_Circ c(gp_Ax2(origin, axis, gp_Dir(gp_Vec(origin, point_at(points, circle.vertices[0])))),
                  circle.radius);
  Geometry result{origin, point_at(ellipse.center, 0), {}, {}, circle.radius, ellipse.major};
  const gp_Vec x(c.Position().XDirection()), y(c.Position().YDirection());
  for (size_t i = 0; i < 3; ++i) {
    const double theta = circle.sweep * i / 2.;
    result.circle[i] = origin.Translated((x * std::cos(theta) + y * std::sin(theta)) * circle.radius);
    result.ellipse[i] = ellipse_at(ellipse, ellipse.start + ellipse.sweep * i / 2.);
  }
  return result;
}

// Axial offset from the ellipse center is a*cos(t) + b*sin(t). Rotate the
// phase to measure from the first endpoint, including an interior minimum
// only if that generator belongs to the retained part of the cylinder.
double minimum_offset(double a, double b, double start, double sweep) {
  const double x = a * std::cos(start) + b * std::sin(start);
  const double y = b * std::cos(start) - a * std::sin(start);
  double minimum_angle = std::atan2(-y, -x);
  if (minimum_angle < 0.) minimum_angle += 2. * std::acos(-1.);
  if (minimum_angle <= sweep) return -std::hypot(a, b);
  return std::min(x, x * std::cos(sweep) + y * std::sin(sweep));
}

TopoDS_Face wall(const MixedArcData& near, const EllipticArcData& far, const double* points,
                 IndexedWires& wires, TopoDS_Edge& circle_edge, TopoDS_Edge& ellipse_edge, bool adjacent) {
  const double tol = Precision::Confusion();
  const auto g = geometry(near, far, points);
  const auto axis = direction(near.axis);
  const gp_Vec z(axis);
  const gp_Ax3 frame(g.near_center, axis, gp_Dir(gp_Vec(g.near_center, g.circle.front())));
  const auto far_axis = direction(far.axis);
  const double alignment = axis.Dot(far_axis);
  // The same edge has positive angular speed on one perpendicular support
  // and negative angular speed on the other. Keep its original 3D parameter.
  const bool reversed = adjacent && alignment < 0.;
  const auto far_index = [reversed](size_t end) { return reversed ? 1 - end : end; };
  const double cosine = reversed ? -alignment : alignment;
  const gp_Vec separation(g.near_center, g.far_center);
  const auto major_dir = direction(far.major_direction);
  const double a = far.major * major_dir.Dot(axis);
  const double b = far.minor * far_axis.Crossed(major_dir).Dot(axis);
  const double amplitude = std::hypot(a, b);
  const double min_offset = adjacent ? minimum_offset(a, b, far.start, far.sweep) : -amplitude;
  require(std::abs(near.sweep - std::acos(-1.) / 2.) < 1.e-5 &&
          std::abs(far.sweep - near.sweep) * near.radius < tol &&
          cosine > 0.1 && cosine < 0.99999 &&
          std::abs(far.major * cosine - near.radius) < tol &&
          std::abs(far.minor - near.radius) < tol &&
          std::abs(b) < tol && separation.Crossed(z).Magnitude() < tol &&
          separation.Dot(z) + min_offset > tol, invalid_model);
  for (size_t i = 0; i < 3; ++i) {
    require(gp_Vec(g.circle[i], g.ellipse[reversed ? 2 - i : i]).Crossed(z).Magnitude() < tol, invalid_model);
  }
  for (size_t i = 0; i < 2; ++i) {
    require(point_at(points, near.vertices[i]).Distance(g.circle[i * 2]) < tol &&
            point_at(points, far.vertices[i]).Distance(g.ellipse[i * 2]) < tol, invalid_model);
  }

  const gp_Circ circle(frame.Ax2(), near.radius);
  const gp_Elips ellipse(gp_Ax2(g.far_center, far_axis, direction(far.major_direction)),
                         far.major, far.minor);
  BRepBuilderAPI_MakeEdge make_near(circle, 0., near.sweep);
  require(make_near.IsDone(), invalid_model);
  circle_edge = make_near.Edge();
  if (ellipse_edge.IsNull()) {
    BRepBuilderAPI_MakeEdge make_far(ellipse, far.start, far.start + far.sweep);
    require(make_far.IsDone(), invalid_model);
    ellipse_edge = make_far.Edge();
  }
  wires.bind_vertices(near.vertices[0], near.vertices[1], circle_edge);
  wires.bind_vertices(far.vertices[0], far.vertices[1], ellipse_edge);

  TopoDS_Face face;
  BRep_Builder builder;
  builder.MakeFace(face, new Geom_CylindricalSurface(frame, near.radius), tol);
  builder.UpdateEdge(circle_edge, new Geom2d_Line(gp_Pnt2d(0., 0.), gp_Dir2d(1., 0.)), face, 0.);
  builder.Range(circle_edge, face, 0., near.sweep);
  set_elliptic_pcurve(face, ellipse_edge, frame, 1., near.sweep / 2., amplitude);

  std::array<TopoDS_Edge, 2> rails;
  for (size_t i = 0; i < rails.size(); ++i) {
    const auto lower = g.circle[i * 2];
    const gp_Pln support(lower, gp_Dir(gp_Vec(g.near_center, lower)));
    rails[i] = wires.line(support, near.vertices[i], far.vertices[far_index(i)]);
    const BRepAdaptor_Curve curve(rails[i]);
    const auto first = curve.FirstParameter(), last = curve.LastParameter();
    const auto z0 = gp_Vec(g.near_center, curve.Value(first)).Dot(z);
    const auto rate = curve.Line().Direction().Dot(axis);
    require(std::abs(std::abs(rate) - 1.) < 1.e-10, invalid_model);
    const double sign = std::copysign(1., rate);
    builder.UpdateEdge(rails[i], new Geom2d_Line(gp_Pnt2d(i * near.sweep, z0 - sign * first),
                                                 gp_Dir2d(0., sign)), face, 0.);
    builder.Range(rails[i], face, first, last);
  }
  BRepBuilderAPI_MakeWire loop;
  loop.Add(circle_edge);
  loop.Add(rails[1]);
  loop.Add(reversed ? ellipse_edge : TopoDS::Edge(ellipse_edge.Reversed()));
  loop.Add(TopoDS::Edge(rails[0].Reversed()));
  require(loop.IsDone() && BRep_Tool::IsClosed(loop.Wire()), invalid_model);
  builder.Add(face, loop.Wire());
  require(BRepCheck_Analyzer(face, true, false, true).IsValid(), invalid_model);
  return TopoDS::Face(face.Reversed());
}

bool matches(const ArcPoints& a, const ArcPoints& b, double tolerance) {
  return a[1].Distance(b[1]) < tolerance &&
      ((a[0].Distance(b[0]) < tolerance && a[2].Distance(b[2]) < tolerance) ||
       (a[0].Distance(b[2]) < tolerance && a[2].Distance(b[0]) < tolerance));
}

Geometry verify_wall(const TopoDS_Face& wall, const TopoDS_Face& paired,
                     const TopTools_IndexedDataMapOfShapeListOfShape& incidence,
                     TopoDS_Edge& seam, TopTools_IndexedMapOfShape& tangents, Status status) {
  const double tol = 4. * Precision::Confusion();
  require(wall.Orientation() == TopAbs_REVERSED, status);
  const auto cyl = BRepAdaptor_Surface(wall).Cylinder();
  Geometry result{};
  result.radius = cyl.Radius();
  TopTools_IndexedMapOfShape neighbors;
  bool circle = false, ellipse = false;
  std::vector<TopoDS_Edge> rails;
  for (int i = 1; i <= incidence.Extent(); ++i) {
    const auto edge = TopoDS::Edge(incidence.FindKey(i));
    const auto& owners = incidence.FindFromIndex(i);
    require(!BRep_Tool::Degenerated(edge) && owners.Extent() == 2, status);
    const BRepAdaptor_Curve curve(edge);
    if (!owners.First().IsSame(wall) && !owners.Last().IsSame(wall)) {
      if (paired.IsNull() || (!owners.First().IsSame(paired) && !owners.Last().IsSame(paired)))
        require(curve.GetType() == GeomAbs_Line, status);
      continue;
    }
    const auto other = TopoDS::Face(owners.First().IsSame(wall) ? owners.Last() : owners.First());
    require(!neighbors.Contains(other), status);
    neighbors.Add(other);
    const BRepAdaptor_Surface support(other);
    const bool join = !paired.IsNull() && other.IsSame(paired);
    require(join ? curve.GetType() == GeomAbs_Ellipse : support.GetType() == GeomAbs_Plane, status);
    const double first = curve.FirstParameter(), last = curve.LastParameter();
    ArcPoints points{curve.Value(first), curve.Value((first + last) / 2.), curve.Value(last)};
    if (curve.GetType() == GeomAbs_Line) {
      require(curve.Line().Direction().IsParallel(cyl.Axis().Direction(), 1.e-8), status);
      const auto plane = support.Plane();
      tangents.Add(other);
      // Both tangent supports have the same material-side normal as the concave wall.
      const auto projection = cyl.Location().Translated(gp_Vec(cyl.Axis().Direction()) *
          gp_Vec(cyl.Location(), points[1]).Dot(gp_Vec(cyl.Axis().Direction())));
      const auto outward = other.Orientation() == TopAbs_FORWARD ? plane.Axis().Direction()
                                                                  : plane.Axis().Direction().Reversed();
      require((gp_Vec(points[1], projection) - gp_Vec(outward) * cyl.Radius()).Magnitude() < tol, status);
      rails.push_back(edge);
    } else if (curve.GetType() == GeomAbs_Circle && !circle) {
      circle = true;
      result.circle = points;
      result.near_center = curve.Circle().Location();
      require(cyl.Axis().Direction().IsParallel(support.Plane().Axis().Direction(), 1.e-8) &&
              gp_Lin(cyl.Axis()).Distance(result.near_center) < tol &&
              std::abs(curve.Circle().Radius() - cyl.Radius()) < tol, status);
    } else if (curve.GetType() == GeomAbs_Ellipse && !ellipse) {
      ellipse = true;
      const auto e = curve.Ellipse();
      result.ellipse = points;
      result.far_center = e.Location();
      result.major = e.MajorRadius();
      require(join == !paired.IsNull(), status);
      seam = edge;
      const auto alignment = std::abs(cyl.Axis().Direction().Dot(e.Axis().Direction()));
      require(alignment > 0.1 && alignment < 0.99999 &&
              (join || e.Axis().Direction().IsParallel(support.Plane().Axis().Direction(), 1.e-8)) &&
              std::abs(e.MinorRadius() - cyl.Radius()) < tol &&
              std::abs(result.major * alignment - cyl.Radius()) < tol &&
              gp_Lin(cyl.Axis()).Distance(result.far_center) < tol, status);
    } else require(false, status);
    for (const auto& face : {wall, other}) {
      BRepLib_CheckCurveOnSurface check(edge, face);
      check.Perform();
      require(check.IsDone() && check.MaxDistance() < tol, status);
    }
    if (curve.GetType() != GeomAbs_Line)
      require(std::abs(last - first - std::acos(-1.) / 2.) < 1.e-5, status);
  }
  require(circle && ellipse && rails.size() == 2 && neighbors.Extent() == 4, status);
  for (const auto& rail : rails) {
    size_t matches = 0;
    for (const size_t near : {0u, 2u})
      for (const size_t far : {0u, 2u})
        matches += !oriented_between(rail, result.circle[near], result.ellipse[far]).IsNull();
    require(matches == 1, status);
  }
  return result;
}

bool matches(const Geometry& a, const Geometry& b, double tolerance) {
  return a.near_center.Distance(b.near_center) < tolerance &&
      a.far_center.Distance(b.far_center) < tolerance &&
      std::abs(a.radius - b.radius) < tolerance && std::abs(a.major - b.major) < tolerance &&
      matches(a.circle, b.circle, tolerance) && matches(a.ellipse, b.ellipse, tolerance);
}

Domain verify(const TopoDS_Shape& solid, const TopTools_IndexedMapOfShape& faces,
              size_t count, const Domain* expected, Status status) {
  const double tol = 4. * Precision::Confusion();
  require((count == 1 || count == 2) && faces.Extent() == static_cast<int>(11 + count), status);
  std::array<TopoDS_Face, 2> walls;
  size_t n = 0;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto face = TopoDS::Face(faces(i));
    const auto type = BRepAdaptor_Surface(face).GetType();
    if (type == GeomAbs_Cylinder && n < count) walls[n++] = face;
    else require(type == GeomAbs_Plane, status);
  }
  require(n == count && (!expected || expected->count == count), status);
  TopTools_IndexedDataMapOfShapeListOfShape incidence;
  TopExp::MapShapesAndUniqueAncestors(solid, TopAbs_EDGE, TopAbs_FACE, incidence);
  std::array<TopoDS_Edge, 2> seams;
  std::array<TopTools_IndexedMapOfShape, 2> tangents;
  Domain result{{}, count};
  for (size_t i = 0; i < count; ++i)
    result.walls[i] = verify_wall(walls[i], walls[1-i], incidence, seams[i], tangents[i], status);
  if (count == 2) {
    const auto a = BRepAdaptor_Surface(walls[0]).Cylinder();
    const auto b = BRepAdaptor_Surface(walls[1]).Cylinder();
    const auto ellipse = BRepAdaptor_Curve(seams[0]).Ellipse();
    require(seams[0].IsSame(seams[1]) && a.Axis().Direction().IsNormal(b.Axis().Direction(), 1.e-8) &&
        std::abs(a.Radius() - b.Radius()) < tol &&
        std::abs(ellipse.MajorRadius() - a.Radius()*std::sqrt(2.)) < tol, status);
    size_t common_planes = 0;
    for (int i = 1; i <= tangents[0].Extent(); ++i) common_planes += tangents[1].Contains(tangents[0](i));
    require(common_planes == 1, status);
  }
  if (expected) {
    // STEP may reorder the two equal-radius walls. Match the actual arc
    // positions and require a bijection, including the shared midpoint.
    const bool swap = count == 2 && !matches(result.walls[0], expected->walls[0], tol);
    for (size_t i = 0; i < count; ++i)
      require(matches(result.walls[i], expected->walls[swap ? 1-i : i], tol), status);
  }
  return result;
}
} // namespace pocket
