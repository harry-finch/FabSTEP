// Independently qualify the unlabelled STEP root after reimport as well as
// the original solid. Each planar sector meets one quarter-circle and two
// distinct radii, and each nondegenerate edge has exactly two face owners.
void check_cut_octant(const TopoDS_Shape& solid, const TopTools_IndexedMapOfShape& faces,
                      double volume, Status status) {
  const CadFailure invalid{status, "L’octant doit partager trois quarts de cercle avec les plans de coupe et trois rayons entre ces plans."};
  const double tolerance = 4. * Precision::Confusion();
  gp_Pnt center;
  double radius = 0.;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const BRepAdaptor_Surface surface(TopoDS::Face(faces(i)));
    if (surface.GetType() == GeomAbs_Sphere) {
      center = surface.Sphere().Location();
      radius = surface.Sphere().Radius();
    }
  }
  const double expected_volume = std::acos(-1.) * radius * radius * radius / 6.;
  if (radius <= tolerance || std::abs(volume - expected_volume) > std::max(1e-9, 1e-8 * volume)) throw invalid;

  std::vector<gp_Dir> normals;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const BRepAdaptor_Surface surface(TopoDS::Face(faces(i)));
    if (surface.GetType() == GeomAbs_Sphere) continue;
    if (surface.GetType() != GeomAbs_Plane || surface.Plane().Distance(center) > tolerance) throw invalid;
    const auto normal = surface.Plane().Axis().Direction();
    for (const auto& previous : normals) {
      if (!normal.IsNormal(previous, std::min(1e-8, tolerance / radius))) throw invalid;
    }
    normals.push_back(normal);
  }

  TopTools_IndexedDataMapOfShapeListOfShape incidence;
  TopExp::MapShapesAndUniqueAncestors(solid, TopAbs_EDGE, TopAbs_FACE, incidence);
  std::set<int> planes_with_arc;
  std::set<std::pair<int, int>> planes_sharing_radius;
  for (int i = 1; i <= incidence.Extent(); ++i) {
    const auto edge = TopoDS::Edge(incidence.FindKey(i));
    if (BRep_Tool::Degenerated(edge)) continue;
    const auto& owners = incidence(i);
    if (owners.Extent() != 2) throw invalid;
    const int a = faces.FindIndex(owners.First()), b = faces.FindIndex(owners.Last());
    const BRepAdaptor_Surface first(TopoDS::Face(faces(a))), second(TopoDS::Face(faces(b)));
    const BRepAdaptor_Curve curve(edge);
    if (first.GetType() == GeomAbs_Sphere || second.GetType() == GeomAbs_Sphere) {
      const int plane = first.GetType() == GeomAbs_Plane ? a : b;
      if (curve.GetType() != GeomAbs_Circle || curve.Circle().Location().Distance(center) > tolerance ||
          std::abs(curve.Circle().Radius() - radius) > tolerance ||
          !quarter_turn(curve.LastParameter() - curve.FirstParameter()) ||
          !planes_with_arc.insert(plane).second) throw invalid;
    } else {
      if (curve.GetType() != GeomAbs_Line || !planes_sharing_radius.emplace(std::min(a, b), std::max(a, b)).second) {
        throw invalid;
      }
      const double start = curve.Value(curve.FirstParameter()).Distance(center);
      const double end = curve.Value(curve.LastParameter()).Distance(center);
      if (std::min(start, end) > tolerance || std::abs(std::max(start, end) - radius) > tolerance) throw invalid;
    }
  }
  if (normals.size() != 3 || planes_with_arc.size() != 3 || planes_sharing_radius.size() != 3) throw invalid;
}
