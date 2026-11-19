// Included inside the private OCCT implementation namespace.
namespace pocket {
using ArcPoints = std::array<gp_Pnt, 3>; // start, midpoint, end

struct Geometry {
  gp_Pnt near_center, far_center;
  ArcPoints near_arc, far_arc;
  double radius, major, near_major;
};

struct Domain {
  std::array<Geometry, 3> walls;
  size_t count;
};

void require(bool condition, Status status) {
  if (!condition)
    throw CadFailure{status, "Le congé cylindrique ne correspond pas à ses butées et à ses deux plans tangents."};
}

Geometry geometry(const MixedArcData& circle, const EllipticArcData& ellipse, const double* points,
                  const EllipticArcData* low = nullptr) {
  const auto origin = point_at(circle.center, 0);
  const auto axis = direction(circle.axis);
  const gp_Circ c(gp_Ax2(origin, axis, gp_Dir(gp_Vec(origin, point_at(points, circle.vertices[0])))),
                  circle.radius);
  Geometry result{origin, point_at(ellipse.center, 0), {}, {}, circle.radius, ellipse.major, low ? low->major : 0.};
  const gp_Vec x(c.Position().XDirection()), y(c.Position().YDirection());
  for (size_t i = 0; i < 3; ++i) {
    const double theta = circle.sweep * i / 2.;
    result.near_arc[i] = low ? ellipse_at(*low, low->start + low->sweep * i / 2.)
                           : origin.Translated((x * std::cos(theta) + y * std::sin(theta)) * circle.radius);
    result.far_arc[i] = ellipse_at(ellipse, ellipse.start + ellipse.sweep * i / 2.);
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
                 IndexedWires& wires, TopoDS_Edge& circle_edge, TopoDS_Edge& ellipse_edge, bool adjacent,
                 const EllipticArcData* low = nullptr, bool network_center = false) {
  const double tol = Precision::Confusion();
  const auto g = geometry(near, far, points, low);
  const auto axis = direction(near.axis);
  const gp_Vec z(axis);
  const gp_Ax3 frame(g.near_center, axis, gp_Dir(gp_Vec(g.near_center, g.near_arc.front())));
  const auto far_axis = direction(far.axis);
  const double alignment = axis.Dot(far_axis);
  // The same edge has positive angular speed on one cylindrical support
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
          (low || separation.Dot(z) + min_offset > tol), invalid_model);
  if (low) {
    const auto n0 = direction(low->axis);
    const auto c0 = axis.Dot(n0);
    const auto n1 = direction(far.axis);
    require(c0 > 0.1 && c0 < 0.99999, invalid_model);
    const auto grad = (gp_Vec(n0) / c0 - gp_Vec(n1) / cosine) * near.radius;
    const double gx = grad.Dot(gp_Vec(frame.XDirection())), gy = grad.Dot(gp_Vec(frame.YDirection()));
    const double minimum = network_center ? minimum_offset(gx, gy, 0., near.sweep) : -std::hypot(gx, gy);
    require(low->minor > tol &&
        std::abs(low->minor - near.radius) < tol && std::abs(low->major * c0 - near.radius) < tol &&
        std::abs(low->sweep - near.sweep) * low->major < tol &&
        separation.Dot(z) + minimum > tol, invalid_model);
  }
  for (size_t i = 0; i < 3; ++i) {
    require(gp_Vec(g.near_arc[i], g.far_arc[reversed ? 2 - i : i]).Crossed(z).Magnitude() < tol, invalid_model);
  }
  for (size_t i = 0; i < 2; ++i) {
    require(point_at(points, near.vertices[i]).Distance(g.near_arc[i * 2]) < tol &&
            point_at(points, far.vertices[i]).Distance(g.far_arc[i * 2]) < tol, invalid_model);
  }

  const gp_Circ circle(frame.Ax2(), near.radius);
  const gp_Elips ellipse(gp_Ax2(g.far_center, far_axis, direction(far.major_direction)),
                         far.major, far.minor);
  if (low) {
    const gp_Elips lower(gp_Ax2(g.near_center, direction(low->axis), direction(low->major_direction)),
                         low->major, low->minor);
    BRepBuilderAPI_MakeEdge edge(lower, low->start, low->start + low->sweep);
    require(edge.IsDone(), invalid_model);
    circle_edge = edge.Edge();
  } else {
    BRepBuilderAPI_MakeEdge edge(circle, 0., near.sweep);
    require(edge.IsDone(), invalid_model);
    circle_edge = edge.Edge();
  }
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
  if (low) {
    const double scale = low->major * std::abs(direction(low->major_direction).Dot(axis));
    set_elliptic_pcurve(face, circle_edge, frame, 1., near.sweep / 2., scale);
  } else {
    builder.UpdateEdge(circle_edge, new Geom2d_Line(gp_Pnt2d(0., 0.), gp_Dir2d(1., 0.)), face, 0.);
    builder.Range(circle_edge, face, 0., near.sweep);
  }
  set_elliptic_pcurve(face, ellipse_edge, frame, 1., near.sweep / 2., amplitude);

  std::array<TopoDS_Edge, 2> rails;
  for (size_t i = 0; i < rails.size(); ++i) {
    const auto lower = g.near_arc[i * 2];
    const gp_Vec radius(g.near_center, lower);
    const gp_Pln support(lower, gp_Dir(radius - z * radius.Dot(z)));
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

Geometry verify_wall(const TopoDS_Face& wall, size_t count,
                     const TopTools_IndexedDataMapOfShapeListOfShape& incidence,
                     TopTools_IndexedMapOfShape& seams, TopTools_IndexedMapOfShape& tangents, Status status) {
  const double tol = 4. * Precision::Confusion();
  require(wall.Orientation() == TopAbs_REVERSED, status);
  const auto cyl = BRepAdaptor_Surface(wall).Cylinder();
  Geometry result{};
  result.radius = cyl.Radius();
  TopTools_IndexedMapOfShape neighbors;
  bool have_near = false, have_far = false;
  std::vector<TopoDS_Edge> rails;
  for (int i = 1; i <= incidence.Extent(); ++i) {
    const auto edge = TopoDS::Edge(incidence.FindKey(i));
    const auto& owners = incidence.FindFromIndex(i);
    require(!BRep_Tool::Degenerated(edge) && owners.Extent() == 2, status);
    const BRepAdaptor_Curve curve(edge);
    if (!owners.First().IsSame(wall) && !owners.Last().IsSame(wall)) {
      if (BRepAdaptor_Surface(TopoDS::Face(owners.First())).GetType() == GeomAbs_Plane &&
          BRepAdaptor_Surface(TopoDS::Face(owners.Last())).GetType() == GeomAbs_Plane)
        require(curve.GetType() == GeomAbs_Line, status);
      continue;
    }
    const auto other = TopoDS::Face(owners.First().IsSame(wall) ? owners.Last() : owners.First());
    require(!neighbors.Contains(other), status);
    neighbors.Add(other);
    const BRepAdaptor_Surface support(other);
    const bool join = support.GetType() == GeomAbs_Cylinder;
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
    } else if (curve.GetType() == GeomAbs_Circle && !have_near) {
      have_near = true;
      result.near_arc = points;
      result.near_center = curve.Circle().Location();
      require(cyl.Axis().Direction().IsParallel(support.Plane().Axis().Direction(), 1.e-8) &&
              gp_Lin(cyl.Axis()).Distance(result.near_center) < tol &&
              std::abs(curve.Circle().Radius() - cyl.Radius()) < tol, status);
    } else if (curve.GetType() == GeomAbs_Ellipse && (!have_far || (!have_near && count != 2))) {
      const auto e = curve.Ellipse();
      if (have_far) {
        have_near = true; // First stop is elliptic as well.
        result.near_arc = points;
        result.near_center = e.Location();
        result.near_major = e.MajorRadius();
      } else {
        have_far = true;
        result.far_arc = points;
        result.far_center = e.Location();
        result.major = e.MajorRadius();
      }
      require(join == (count > 1), status);
      if (join) seams.Add(edge);
      const auto alignment = std::abs(cyl.Axis().Direction().Dot(e.Axis().Direction()));
      require(alignment > 0.1 && alignment < 0.99999 &&
              (join || e.Axis().Direction().IsParallel(support.Plane().Axis().Direction(), 1.e-8)) &&
              std::abs(e.MinorRadius() - cyl.Radius()) < tol &&
              std::abs(e.MajorRadius() * alignment - cyl.Radius()) < tol &&
              gp_Lin(cyl.Axis()).Distance(e.Location()) < tol, status);
    } else require(false, status);
    for (const auto& face : {wall, other}) {
      BRepLib_CheckCurveOnSurface check(edge, face);
      check.Perform();
      require(check.IsDone() && check.MaxDistance() < tol, status);
    }
    if (curve.GetType() != GeomAbs_Line)
      require(std::abs(last - first - std::acos(-1.) / 2.) < 1.e-5, status);
  }
  require(have_near && have_far && rails.size() == 2 && neighbors.Extent() == 4, status);
  for (const auto& rail : rails) {
    size_t matches = 0;
    for (const size_t near : {0u, 2u})
      for (const size_t far : {0u, 2u})
        matches += !oriented_between(rail, result.near_arc[near], result.far_arc[far]).IsNull();
    require(matches == 1, status);
  }
  return result;
}

bool matches(const Geometry& a, const Geometry& b, double tolerance) {
  if (std::abs(a.radius - b.radius) >= tolerance) return false;
  const auto same = [tolerance](const gp_Pnt& c1, double r1, const ArcPoints& p1,
                                const gp_Pnt& c2, double r2, const ArcPoints& p2) {
    return c1.Distance(c2) < tolerance && std::abs(r1-r2) < tolerance && matches(p1,p2,tolerance);
  };
  return (same(a.near_center,a.near_major,a.near_arc,b.near_center,b.near_major,b.near_arc) &&
          same(a.far_center,a.major,a.far_arc,b.far_center,b.major,b.far_arc)) ||
         (a.near_major > 0. && b.near_major > 0. &&
          same(a.near_center,a.near_major,a.near_arc,b.far_center,b.major,b.far_arc) &&
          same(a.far_center,a.major,a.far_arc,b.near_center,b.near_major,b.near_arc));
}

Domain verify(const TopoDS_Shape& solid, const TopTools_IndexedMapOfShape& faces,
              size_t count, const Domain* expected, Status status) {
  const double tol = 4. * Precision::Confusion();
  require((count >= 1 && count <= 3) && faces.Extent() == static_cast<int>(11 + count), status);
  std::array<TopoDS_Face, 3> walls;
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
  std::array<TopTools_IndexedMapOfShape, 3> seams, tangents;
  Domain result{{}, count};
  for (size_t i = 0; i < count; ++i)
    result.walls[i] = verify_wall(walls[i], count, incidence, seams[i], tangents[i], status);
  size_t links = 0;
  for (size_t i = 0; i < count; ++i) {
    for (size_t j = i + 1; j < count; ++j) {
      const auto a = BRepAdaptor_Surface(walls[i]).Cylinder();
      const auto b = BRepAdaptor_Surface(walls[j]).Cylinder();
      const auto axis_a = a.Axis().Direction(), axis_b = b.Axis().Direction();
      int common_seams = 0;
      for (int k = 1; k <= seams[i].Extent(); ++k) {
        if (!seams[j].Contains(seams[i](k))) continue;
        ++common_seams;
        const auto ellipse = BRepAdaptor_Curve(TopoDS::Edge(seams[i](k))).Ellipse();
        const auto normal = ellipse.Axis().Direction();
        require(std::abs(axis_a.Dot(axis_b)) < (count == 3 ? 1.e-7 : std::sqrt(0.5)) &&
                std::abs(a.Radius() - b.Radius()) < tol &&
                std::abs(ellipse.MinorRadius() - a.Radius()) < tol &&
                std::abs(ellipse.MajorRadius() * std::abs(axis_a.Dot(normal)) - a.Radius()) < tol &&
                std::abs(ellipse.MajorRadius() * std::abs(axis_b.Dot(normal)) - b.Radius()) < tol, status);
      }
      require(common_seams <= 1, status);
      links += common_seams;
      if (!common_seams)
        require(count == 3 && axis_a.IsParallel(axis_b, 1.e-7), status);
      size_t common_planes = 0;
      for (int k = 1; k <= tangents[i].Extent(); ++k) common_planes += tangents[j].Contains(tangents[i](k));
      require(common_planes == 1, status);
    }
  }
  require(links == count - 1, status);
  if (count == 3) {
    int common_floor = 0;
    for (int k = 1; k <= tangents[0].Extent(); ++k)
      common_floor += tangents[1].Contains(tangents[0](k)) && tangents[2].Contains(tangents[0](k));
    require(common_floor == 1, status);
  }
  if (expected) {
    // STEP may reorder the faces. Every wall must retain both trimmed ends.
    std::array<bool, 3> used{};
    for (size_t i = 0; i < count; ++i) {
      size_t j = 0;
      while (j < count && (used[j] || !matches(result.walls[i], expected->walls[j], tol))) ++j;
      require(j < count, status);
      used[j] = true;
    }
  }
  return result;
}
} // namespace pocket
