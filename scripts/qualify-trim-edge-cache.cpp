// Isolated OCCT regression for canonical contour routing (no meshers required).
#include "../crates/fabstep-cad/src/trim_edge_cache.hxx"
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <iostream>

namespace {
void check(bool condition, const char *message) {
  if (!condition)
    throw std::runtime_error(message);
}

template <class Run> void must_refuse(Run run) {
  try {
    run();
  } catch (const std::exception &) {
    return;
  }
  throw std::runtime_error("Expected refusal for malformed incidence or budget.");
}

void rational_quarter_circle() {
  // Exact rational quadratic in an off-origin plane. Its Bernstein abscissa
  // is not the circle angle, so uniformly assigning parameters by arc length
  // would fail this check at intermediate points.
  TColgp_Array1OfPnt poles(1, 3);
  poles.SetValue(1, gp_Pnt(12, 0, 4));
  poles.SetValue(2, gp_Pnt(12, 2, 4));
  poles.SetValue(3, gp_Pnt(10, 2, 4));
  TColStd_Array1OfReal weights(1, 3), knots(1, 2);
  TColStd_Array1OfInteger mult(1, 2);
  weights.SetValue(1, 1.);
  weights.SetValue(2, std::sqrt(.5));
  weights.SetValue(3, 1.);
  knots.SetValue(1, 2.);
  knots.SetValue(2, 7.);
  mult.SetValue(1, 3);
  mult.SetValue(2, 3);
  Handle(Geom_BSplineCurve) spline = new Geom_BSplineCurve(poles, weights, knots, mult, 2);
  const TopoDS_Edge edge = BRepBuilderAPI_MakeEdge(spline);
  const double tolerance = .005;
  const auto samples = trimcache::analytic(edge, tolerance, .35, 10000);
  check(samples.size() > 3 && samples.front().parameter == 2. && samples.back().parameter == 7.,
        "Rational boundary parameter range changed.");
  for (size_t i = 1; i < samples.size(); ++i) {
    const auto &start = samples[i - 1];
    const auto &end = samples[i];
    check(end.parameter > start.parameter && end.point.Distance(spline->Value(end.parameter)) < 1.e-12,
          "Rational curve sample moved.");
    const gp_Pnt middle((start.point.XYZ() + end.point.XYZ()) * .5);
    check(2. - middle.Distance(gp_Pnt(10, 0, 4)) <= tolerance, "Chord sag exceeds the certified bound.");
  }
  must_refuse([&] { trimcache::analytic(edge, tolerance, .35, 3); });
}

void cylinder(bool moved) {
  TopoDS_Shape solid = BRepPrimAPI_MakeCylinder(5., 10.).Shape();
  if (moved) {
    gp_Trsf pose;
    pose.SetRotation(gp_Ax1(gp_Pnt(), gp_Dir(1, 2, 3)), .72);
    pose.SetTranslationPart(gp_Vec(37., 14., -19.));
    solid.Move(TopLoc_Location(pose));
  }
  TopTools_IndexedMapOfShape all, wall;
  TopExp::MapShapes(solid, TopAbs_FACE, all);
  for (int i = 1; i <= all.Extent(); ++i) {
    const auto face = TopoDS::Face(all(i));
    if (BRepAdaptor_Surface(face).GetType() == GeomAbs_Cylinder)
      wall.Add(face);
  }
  check(wall.Extent() == 1, "Missing cylinder.");
  const double deflection = .005;
  const trimcache::Cache mixed(solid, wall, deflection, .35, 100000);
  check(mixed.counts().ordinary_faces == 2 && mixed.counts().mixed_edges == 2 &&
            mixed.counts().analytic_edges == 1,
        "Unexpected rim/seam incidence.");
  TopLoc_Location wall_location;
  check(BRep_Tool::Triangulation(TopoDS::Face(wall(1)), wall_location).IsNull(),
        "Special face was meshed by OCCT.");
  for (int k = 1; k <= all.Extent(); ++k) {
    const auto face = TopoDS::Face(all(k));
    if (wall.Contains(face))
      continue;
    TopLoc_Location location;
    const auto triangles = BRep_Tool::Triangulation(face, location);
    TopExp_Explorer e(face, TopAbs_EDGE);
    const auto rim = trimcache::forward(TopoDS::Edge(e.Current()));
    const auto &chain = mixed.samples(rim);
    const auto &polygon = BRep_Tool::PolygonOnTriangulation(rim, triangles, location);
    check(chain.size() == static_cast<size_t>(polygon->NbNodes()) && chain.size() > 5, "Missing rim nodes.");
    for (size_t j = 0; j < chain.size(); ++j) {
      const gp_Pnt point = triangles->Node(polygon->Node(int(j) + 1)).Transformed(location.Transformation());
      check(chain[j].point.IsEqual(point, 0.) && chain[j].parameter == polygon->Parameter(int(j) + 1),
            "Mixed boundary did not retain the exact neighboring samples.");
    }
    must_refuse([&] { mixed.samples(TopoDS::Edge(rim.Reversed())); });
  }
  const trimcache::Cache analytic(solid, all, deflection, .35, 100000);
  check(analytic.counts().ordinary_faces == 0 && analytic.counts().analytic_edges == 3 &&
            analytic.counts().mixed_edges == 0,
        "Purely special edges were not shared.");
  must_refuse([&] { trimcache::Cache cache(solid, wall, deflection, .35, 3); });
  must_refuse([&] { trimcache::Cache cache(wall(1), wall, deflection, .35, 100000); });
  BRep_Builder builder;
  TopoDS_Compound inverted;
  builder.MakeCompound(inverted);
  for (int i = 1; i <= all.Extent(); ++i)
    builder.Add(inverted, wall.Contains(all(i)) ? all(i).Reversed() : all(i));
  must_refuse([&] { trimcache::Cache cache(inverted, wall, deflection, .35, 100000); });
}
} // namespace

int main() try {
  rational_quarter_circle();
  cylinder(false);
  cylinder(true);
  std::cout << "PASS rational arc; two placements; mixed rims; two seam p-curves; analytic edges; "
               "refused reversed lookup, missing cap, reversed wall and excess work.\n";
} catch (const std::exception &error) {
  std::cerr << error.what() << '\n';
  return 1;
}
