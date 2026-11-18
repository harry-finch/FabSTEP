// Included inside the private OCCT implementation namespace.
namespace pocket {
using ArcPoints = std::array<gp_Pnt, 3>; // start, midpoint, end

struct Geometry {
  gp_Pnt near_center, far_center;
  ArcPoints circle, ellipse;
  double radius, major;
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

TopoDS_Face wall(const MixedArcData& near, const EllipticArcData& far, const double* points,
                 IndexedWires& wires, std::vector<TopoDS_Edge>& shared) {
  const double tol = Precision::Confusion();
  const auto g = geometry(near, far, points);
  const auto axis = direction(near.axis);
  const gp_Vec z(axis);
  const gp_Ax3 frame(g.near_center, axis, gp_Dir(gp_Vec(g.near_center, g.circle.front())));
  const auto far_axis = direction(far.axis);
  const double alignment = axis.Dot(far_axis);
  const gp_Vec separation(g.near_center, g.far_center);
  const double amplitude = far.major * std::abs(direction(far.major_direction).Dot(axis));
  require(std::abs(near.sweep - std::acos(-1.) / 2.) < 1.e-5 &&
          std::abs(far.sweep - near.sweep) * near.radius < tol &&
          alignment > 0.1 && alignment < 0.99999 &&
          std::abs(far.major * alignment - near.radius) < tol &&
          std::abs(far.minor - near.radius) < tol &&
          std::abs(direction(far.major_direction).Crossed(far_axis).Dot(axis)) * far.minor < tol &&
          separation.Crossed(z).Magnitude() < tol && separation.Dot(z) > amplitude + tol, invalid_model);
  for (size_t i = 0; i < 3; ++i) {
    require(gp_Vec(g.circle[i], g.ellipse[i]).Crossed(z).Magnitude() < tol, invalid_model);
  }
  for (size_t i = 0; i < 2; ++i) {
    require(point_at(points, near.vertices[i]).Distance(g.circle[i * 2]) < tol &&
            point_at(points, far.vertices[i]).Distance(g.ellipse[i * 2]) < tol, invalid_model);
  }

  const gp_Circ circle(frame.Ax2(), near.radius);
  const gp_Elips ellipse(gp_Ax2(g.far_center, far_axis, direction(far.major_direction)),
                         far.major, far.minor);
  BRepBuilderAPI_MakeEdge make_near(circle, 0., near.sweep);
  BRepBuilderAPI_MakeEdge make_far(ellipse, far.start, far.start + far.sweep);
  require(make_near.IsDone() && make_far.IsDone(), invalid_model);
  shared[0] = make_near.Edge();
  shared[1] = make_far.Edge();
  wires.bind_vertices(near.vertices[0], near.vertices[1], shared[0]);
  wires.bind_vertices(far.vertices[0], far.vertices[1], shared[1]);

  TopoDS_Face face;
  BRep_Builder builder;
  builder.MakeFace(face, new Geom_CylindricalSurface(frame, near.radius), tol);
  builder.UpdateEdge(shared[0], new Geom2d_Line(gp_Pnt2d(0., 0.), gp_Dir2d(1., 0.)), face, 0.);
  builder.Range(shared[0], face, 0., near.sweep);
  set_elliptic_pcurve(face, shared[1], frame, 1., near.sweep / 2., amplitude);

  std::array<TopoDS_Edge, 2> rails;
  for (size_t i = 0; i < rails.size(); ++i) {
    const auto lower = g.circle[i * 2];
    const gp_Pln support(lower, gp_Dir(gp_Vec(g.near_center, lower)));
    rails[i] = wires.line(support, near.vertices[i], far.vertices[i]);
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
  loop.Add(shared[0]);
  loop.Add(rails[1]);
  loop.Add(TopoDS::Edge(shared[1].Reversed()));
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

Geometry verify(const TopoDS_Shape& solid, const TopTools_IndexedMapOfShape& faces,
                const Geometry* expected, Status status) {
  const double tol = 4. * Precision::Confusion();
  require(faces.Extent() == 12, status);
  TopoDS_Face wall;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto face = TopoDS::Face(faces(i));
    const auto type = BRepAdaptor_Surface(face).GetType();
    if (type == GeomAbs_Cylinder && wall.IsNull()) wall = face;
    else require(type == GeomAbs_Plane, status);
  }
  require(!wall.IsNull() && wall.Orientation() == TopAbs_REVERSED, status);
  const auto cyl = BRepAdaptor_Surface(wall).Cylinder();
  Geometry result{};
  result.radius = cyl.Radius();
  TopTools_IndexedDataMapOfShapeListOfShape incidence;
  TopExp::MapShapesAndUniqueAncestors(solid, TopAbs_EDGE, TopAbs_FACE, incidence);
  TopTools_IndexedMapOfShape neighbors;
  bool circle = false, ellipse = false;
  std::vector<TopoDS_Edge> rails;
  for (int i = 1; i <= incidence.Extent(); ++i) {
    const auto edge = TopoDS::Edge(incidence.FindKey(i));
    const auto& owners = incidence.FindFromIndex(i);
    require(!BRep_Tool::Degenerated(edge) && owners.Extent() == 2, status);
    const BRepAdaptor_Curve curve(edge);
    if (!owners.First().IsSame(wall) && !owners.Last().IsSame(wall)) {
      require(curve.GetType() == GeomAbs_Line, status);
      continue;
    }
    const auto other = TopoDS::Face(owners.First().IsSame(wall) ? owners.Last() : owners.First());
    require(!neighbors.Contains(other), status);
    neighbors.Add(other);
    const auto plane = BRepAdaptor_Surface(other).Plane();
    const double first = curve.FirstParameter(), last = curve.LastParameter();
    ArcPoints points{curve.Value(first), curve.Value((first + last) / 2.), curve.Value(last)};
    if (curve.GetType() == GeomAbs_Line) {
      require(curve.Line().Direction().IsParallel(cyl.Axis().Direction(), 1.e-8), status);
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
      require(cyl.Axis().Direction().IsParallel(plane.Axis().Direction(), 1.e-8) &&
              gp_Lin(cyl.Axis()).Distance(result.near_center) < tol &&
              std::abs(curve.Circle().Radius() - cyl.Radius()) < tol, status);
    } else if (curve.GetType() == GeomAbs_Ellipse && !ellipse) {
      ellipse = true;
      const auto e = curve.Ellipse();
      result.ellipse = points;
      result.far_center = e.Location();
      result.major = e.MajorRadius();
      const auto alignment = std::abs(cyl.Axis().Direction().Dot(plane.Axis().Direction()));
      require(alignment > 0.1 && alignment < 0.99999 &&
              e.Axis().Direction().IsParallel(plane.Axis().Direction(), 1.e-8) &&
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
  if (expected) {
    require(expected->near_center.Distance(result.near_center) < tol &&
            expected->far_center.Distance(result.far_center) < tol &&
            std::abs(expected->radius - result.radius) < tol &&
            std::abs(expected->major - result.major) < tol &&
            matches(result.circle, expected->circle, tol) && matches(result.ellipse, expected->ellipse, tol), status);
  }
  return result;
}
} // namespace pocket
