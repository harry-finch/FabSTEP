// A measured bore crosses both sides of the shaft. Round just the isolated
// mouth on the negative X side. The other branch is a material sharp edge.
namespace rolling_shape {
struct Stock {
  TopoDS_Shape shaft;
  TopoDS_Edge mouth;
  static void require(bool condition, Status stage) {
    if (!condition)
      throw CadFailure{stage, "Le débouché du perçage cylindrique est ambigu."};
  }
  Stock(gp_Pnt origin, double height, double radius, double bore, double offset,
        double level, Status stage) {
    require(std::isfinite(height) && height > 0. && height <= 64. &&
                radius <= 32. && bore > .1 &&
                radius > 1.5 * (bore + std::abs(offset)) && level > bore &&
                height - level > bore,
            stage);
    const auto blank =
        BRepPrimAPI_MakeCylinder(gp_Ax2(origin, {0, 0, 1}), radius, height)
            .Shape();
    const auto tool =
        BRepPrimAPI_MakeCylinder(
            gp_Ax2(origin.Translated({-2 * radius, offset, level}), {1, 0, 0}),
            bore, 4 * radius)
            .Shape();
    BRepAlgoAPI_Cut cut(blank, tool);
    require(cut.IsDone() && !cut.HasErrors(), stage);
    shaft = cut.Shape();
    TopTools_IndexedMapOfShape edges;
    TopExp::MapShapes(shaft, TopAbs_EDGE, edges);
    for (int i = 1; i <= edges.Extent(); ++i) {
      auto edge = TopoDS::Edge(edges(i));
      BRepAdaptor_Curve curve(edge);
      if (curve.GetType() == GeomAbs_BSplineCurve &&
          curve.Value((curve.FirstParameter() + curve.LastParameter()) / 2)
                  .X() < origin.X() - radius / 2) {
        require(mouth.IsNull(), stage);
        mouth = edge;
      }
    }
    require(!mouth.IsNull(), stage);
  }
  TopoDS_Solid fillet(double radius, Status stage,
                      bool transfer_precision = true) const {
    require(radius > 0. && std::isfinite(radius), stage);
    BRepFilletAPI_MakeFillet builder(shaft);
    // STEP re-evaluates the rail against its analytic cylinder. Request the
    // modelling precision here, on the fitted surface, before transfer.
    if (transfer_precision)
      builder.SetParams(.01, 1.e-7, 1.e-7, 1.e-7, 1.e-7, 1.e-4);
    builder.Add(radius, mouth);
    builder.Build();
    require(builder.IsDone(), stage);
    TopTools_IndexedMapOfShape solids;
    TopExp::MapShapes(builder.Shape(), TopAbs_SOLID, solids);
    require(solids.Extent() == 1, stage);
    return TopoDS::Solid(solids(1));
  }
};
// Keep the same token order as the five source regions.
inline std::array<TopoDS_Face, 5> material_faces(const TopoDS_Shape &solid,
                                                 Status status) {
  std::array<TopoDS_Face, 5> result;
  std::vector<std::pair<double, TopoDS_Face>> caps;
  for (TopExp_Explorer it(solid, TopAbs_FACE); it.More(); it.Next()) {
    auto f = TopoDS::Face(it.Current());
    BRepAdaptor_Surface ad(f, false);
    if (ad.GetType() == GeomAbs_Plane) {
      caps.emplace_back(ad.Plane().Location().Z(), f);
      continue;
    }
    size_t id = 4;
    if (ad.GetType() == GeomAbs_Cylinder)
      id = std::abs(ad.Cylinder().Axis().Direction().Z()) > .99 ? 2 : 3;
    else
      Stock::require(ad.GetType() == GeomAbs_BSplineSurface, status);
    Stock::require(result[id].IsNull(), status);
    result[id] = f;
  }
  Stock::require(caps.size() == 2 && !result[2].IsNull() &&
                     !result[3].IsNull() && !result[4].IsNull(),
                 status);
  if (caps[0].first > caps[1].first)
    std::swap(caps[0], caps[1]);
  result[0] = caps[0].second;
  result[1] = caps[1].second;
  return result;
}

// Reparameterize all material arcs on the opposite sharp mouth with one
// angular variable on the intersection of the two measured cylinders.
// Keep OCCT's edge identities, endpoints and seams (three arcs for this
// domain).
inline void prepare_transfer(const TopoDS_Shape &solid, Status stage) {
  const auto faces = material_faces(solid, stage);
  const auto shaft = BRepAdaptor_Surface(faces[2], false).Cylinder();
  const auto bore = BRepAdaptor_Surface(faces[3], false).Cylinder();
  const gp_Pnt center(shaft.Location().X(), bore.Location().Y(),
                      bore.Location().Z());
  const double offset = bore.Location().Y() - shaft.Location().Y();
  TopTools_IndexedDataMapOfShapeListOfShape neighbors;
  TopExp::MapShapesAndUniqueAncestors(solid, TopAbs_EDGE, TopAbs_FACE,
                                      neighbors);
  const CadFailure invalid{
      stage, "La courbe commune aux deux cylindres n’est pas vérifiée."};
  int count = 0;
  for (int i = 1; i <= neighbors.Extent(); ++i) {
    const auto &pair = neighbors.FindFromIndex(i);
    if (pair.Extent() != 2)
      continue;
    if (!((pair.First().IsSame(faces[2]) && pair.Last().IsSame(faces[3])) ||
          (pair.Last().IsSame(faces[2]) && pair.First().IsSame(faces[3]))))
      continue;
    const auto e = TopoDS::Edge(neighbors.FindKey(i));
    set_cross_drill_junction(e, faces[3], faces[2], center, {0, 0, 1},
                             {0, -1, 0}, {1, 0, 0}, 1., 0., -offset,
                             bore.Radius(), shaft.Radius(), invalid);
    ++count;
  }
  if (count != 3)
    throw invalid;
}

} // namespace rolling_shape
