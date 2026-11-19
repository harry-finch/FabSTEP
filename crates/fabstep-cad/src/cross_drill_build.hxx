#include "cross_drill_junction.hxx"

void set_junction(const CrossDrillData& d, const TopoDS_Edge& edge, const TopoDS_Face& small,
                  const TopoDS_Face& big) {
  const CadFailure invalid{invalid_model,
      "La limite commune des deux perçages ne correspond pas à la branche prouvée."};
  const gp_Vec x(direction(d.axes[0])), y(direction(d.axes[1])), z(direction(d.axes[2]));
  const auto center = point_at(d.center, 0).Translated(x*d.small_offset[0]+y*d.small_offset[1]);
  const double c=d.large_axis_cosine;
  set_cross_drill_junction(edge,small,big,center,x,y,z,std::sqrt((1.-c)*(1.+c)),c,d.small_offset[1],d.radii[0],d.radii[1],invalid);
}
TopoDS_Solid build_cross_drill(const CrossDrillData& data, const size_t* plane_tokens,
                               const size_t* wall_tokens, FaceLabels* labels) {
  const double r = data.radii[0], R = data.radii[1], hx = data.half[0], hy = data.half[1],
               hz = data.half[2];
  const CadFailure invalid{
      invalid_model,
      "Le découpage des deux supports cylindriques ne correspond pas aux domaines prouvés."};
  const auto center = point_at(data.center, 0);
  const gp_Vec ex(direction(data.axes[0])), ey(direction(data.axes[1])),
      ez(direction(data.axes[2]));
  auto p = [&](double x, double y, double z) {
    return center.Translated(ex * x + ey * y + ez * z);
  };
  const double cosine=data.large_axis_cosine, sine=std::sqrt((1.-cosine)*(1.+cosine));
  const gp_Vec large_axis=ex*sine+ez*cosine, large_radial=ez*sine-ex*cosine;
  auto large_mouth=[&](double distance){return center.Translated(large_axis*distance);};
  // Build in a common local frame for stable cylindrical seam placement.
  const auto small =
      BRepBuilderAPI_MakeFace(gp_Cylinder(gp_Ax3(p(data.small_offset[0], data.small_offset[1], -hz), gp_Dir(ez), gp_Dir(ex)), r), 0.,
                              2. * std::acos(-1.), 0., 2. * hz)
          .Face();
  const auto large =
      BRepBuilderAPI_MakeFace(gp_Cylinder(gp_Ax3(large_mouth(-hx), gp_Dir(large_axis), gp_Dir(large_radial)), R), 0.,
                              2. * std::acos(-1.), 0., 2. * hx)
          .Face();
  TopTools_ListOfShape args;
  args.Append(small);
  args.Append(large);
  BRepAlgoAPI_Splitter split;
  split.SetArguments(args);
  split.Build();
  if (!split.IsDone() || split.HasErrors())
    throw invalid;
  std::array<TopoDS_Edge, 4> rings;
  std::array<TopoDS_Face, 3> walls;
  const std::array<gp_Pnt, 4> mouths{p(data.small_offset[0], data.small_offset[1], -hz), p(data.small_offset[0], data.small_offset[1], hz), large_mouth(-hx), large_mouth(hx)};
  for (TopExp_Explorer it(split.Shape(), TopAbs_FACE); it.More(); it.Next()) {
    const auto face = TopoDS::Face(it.Current());
    const auto surface = BRepAdaptor_Surface(face);
    if (surface.GetType() != GeomAbs_Cylinder)
      throw invalid;
    const double rad = surface.Cylinder().Radius();
    const bool small_wall = std::abs(rad - r) <= Precision::Confusion();
    if (!small_wall && std::abs(rad - R) > Precision::Confusion())
      throw invalid;
    unsigned rim_mask = 0;
    for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
      const auto edge = TopoDS::Edge(e.Current());
      BRepAdaptor_Curve curve(edge);
      if (curve.GetType() != GeomAbs_Circle)
        continue;
      const auto circle = curve.Circle();
      for (size_t j = 0; j < 4; ++j)
        if (circle.Location().Distance(mouths[j]) <= Precision::Confusion()) {
          if (!rings[j].IsNull() || (small_wall != (j < 2)) ||
              std::abs(circle.Radius() - rad) > Precision::Confusion() ||
              !BRep_Tool::IsClosed(edge))
            throw invalid;
          rings[j] = edge;
          rim_mask |= 1u << j;
        }
    }
    if (!rim_mask)
      continue;
    const int slot = rim_mask == 1 ? 0 : rim_mask == 2 ? 1 : rim_mask == 12 ? 2 : -1;
    if (slot < 0 || !walls[slot].IsNull())
      throw invalid;
    walls[slot] = TopoDS::Face(face.Reversed());
  }
  if (walls[0].IsNull() || walls[1].IsNull() || walls[2].IsNull())
    throw invalid;
  for (size_t i = 0; i < 2; ++i) {
    for (TopExp_Explorer it(walls[i], TopAbs_EDGE); it.More(); it.Next()) {
      const auto e = TopoDS::Edge(it.Current());
      if (BRepAdaptor_Curve(e).GetType() == GeomAbs_BSplineCurve)
        set_junction(data, e, walls[i], walls[2]);
    }
  }
  BRepBuilderAPI_Sewing sewing(Precision::Confusion());
  sewing.SetMaxTolerance(Precision::Confusion());
  sewing.SetLocalTolerancesMode(false);
  for (size_t i = 0; i < 3; ++i) {
    if (walls[i].IsNull())
      throw invalid;
    sewing.Add(walls[i]);
    if (labels)
      labels->push_back({walls[i], wall_tokens[i]});
  }
  // Six planes at signed distances ±hx, ±hy and ±hz. A skew block
  // preserves circular mouths normal to both of its measured bore axes.
  TopoDS_Shape outer;
  if (data.fillet_radius != 0.) {
    const double radius = data.fillet_radius;
    const int sy = data.fillet_y_sign, sz = data.fillet_z_sign;
    if (cosine != 0. || !std::isfinite(radius) || radius <= Precision::Confusion() ||
        (sy != -1 && sy != 1) || (sz != -1 && sz != 1) || radius >= hz ||
        hy - radius <= std::max(R, std::abs(data.small_offset[1]) + r) + Precision::Confusion())
      throw invalid;
    // Extrude the measured YZ profile. Its single circular edge supplies both
    // end arcs and the adjacent planes share the two tangent generators.
    auto end = [&](double y, double z) { return p(-hx, sy*y, sz*z); };
    const auto a=end(-hy,-hz), b=end(hy,-hz), c=end(hy,hz-radius);
    const auto d=end(hy-radius,hz), e=end(-hy,hz);
    const double inset = radius * (1.-std::sqrt(.5));
    GC_MakeArcOfCircle arc(c, end(hy-inset,hz-inset), d);
    if (!arc.IsDone()) throw invalid;
    BRepBuilderAPI_MakeWire wire;
    wire.Add(BRepBuilderAPI_MakeEdge(a,b));
    wire.Add(BRepBuilderAPI_MakeEdge(b,c));
    wire.Add(BRepBuilderAPI_MakeEdge(arc.Value()));
    wire.Add(BRepBuilderAPI_MakeEdge(d,e));
    wire.Add(BRepBuilderAPI_MakeEdge(e,a));
    if (!wire.IsDone()) throw invalid;
    BRepBuilderAPI_MakeFace end_face(wire.Wire(), true);
    if (!end_face.IsDone()) throw invalid;
    BRepPrimAPI_MakePrism prism(end_face.Face(), ex*(2.*hx), true, true);
    if (!prism.IsDone()) throw invalid;
    outer = prism.Shape();
  } else if(cosine==0.) {
    outer=BRepPrimAPI_MakeBox(gp_Ax2(p(-hx,-hy,-hz),gp_Dir(ez),gp_Dir(ex)),2.*hx,2.*hy,2.*hz).Shape();
  } else {
    BRepBuilderAPI_MakePolygon loop;
    for(const auto& signs : std::array<std::array<int,2>,4>{{{-1,-1},{1,-1},{1,1},{-1,1}}}) {
      const double z=signs[1]*hz;
      loop.Add(p((signs[0]*hx-cosine*z)/sine,-hy,z));
    }
    loop.Close();
    if(!loop.IsDone()) throw invalid;
    BRepBuilderAPI_MakeFace cap(loop.Wire(),true);
    if(!cap.IsDone()) throw invalid;
    BRepPrimAPI_MakePrism prism(cap.Face(),ey*(2.*hy),true,true);
    if(!prism.IsDone()) throw invalid;
    outer=prism.Shape();
  }
  for (TopExp_Explorer it(outer, TopAbs_FACE); it.More(); it.Next()) {
    const auto f = TopoDS::Face(it.Current());
    const BRepAdaptor_Surface surface(f);
    if (data.fillet_radius && surface.GetType() == GeomAbs_Cylinder) {
      sewing.Add(f);
      if (labels) labels->push_back({f,wall_tokens[3]});
      continue;
    }
    if (surface.GetType() != GeomAbs_Plane) throw invalid;
    auto plane = surface.Plane();
    gp_Dir n = plane.Axis().Direction();
    if (f.Orientation() == TopAbs_REVERSED)
      n.Reverse();
    const std::array<double,3> direction{n.Dot(gp_Dir(large_axis)),n.Dot(gp_Dir(ey)),n.Dot(gp_Dir(ez))};
    const auto it_max=std::max_element(direction.begin(), direction.end(),[](double a,double b){return std::abs(a)<std::abs(b);});
    if (std::abs(*it_max)<1.-1e-10) throw invalid;
    const int slot = int(it_max-direction.begin())+(*it_max>0. ? 3 : 0);
    const int ring = slot == 0 ? 2 : slot == 3 ? 3 : slot == 2 ? 0 : slot == 5 ? 1 : -1;
    TopoDS_Face result = f;
    if (ring >= 0) {
      if (rings[ring].IsNull())
        throw invalid;
      const auto forward = TopoDS::Face(f.Oriented(TopAbs_FORWARD));
      BRepBuilderAPI_MakeFace cut(plane, BRepTools::OuterWire(forward), true);
      cut.Add(rim_wire(plane, rings[ring], true));
      if (!cut.IsDone() || !BRepCheck_Analyzer(cut.Face(), true, false, true).IsValid())
        throw invalid;
      result = TopoDS::Face(cut.Face().Oriented(f.Orientation()));
    }
    sewing.Add(result);
    if (labels)
      labels->push_back({result, plane_tokens[slot]});
  }
  auto solid = solid_from_sewing(sewing);
  if (labels)
    track_sewn_faces(solid, sewing, *labels);
  return solid;
}
