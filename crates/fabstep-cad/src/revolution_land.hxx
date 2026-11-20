// Validate the annulus between a cubic terminal circle and a translated disk.
// Keep the orientation of the cylinder-side rim for the cubic wall's opposite
// use.
TopoDS_Edge revolution_land_rim(const TopoDS_Face &face,
                                const TopoDS_Edge &disk, const gp_Pnt &center,
                                const gp_Vec &axis, double radius,
                                double length, const CadFailure &invalid) {
  const BRepAdaptor_Surface surface(face);
  const auto cylinder = surface.Cylinder();
  if (!cylinder.Axis().Direction().IsParallel(gp_Dir(axis), 1.e-9) ||
      gp_Vec(cylinder.Location(), center).Crossed(axis).Magnitude() >
          revolution_precision ||
      std::abs(cylinder.Radius() - radius) > revolution_precision ||
      std::abs(surface.LastUParameter() - surface.FirstUParameter() -
               revolution_period) *
              radius >
          revolution_precision ||
      std::abs(surface.LastVParameter() - surface.FirstVParameter() - length) >
          revolution_precision)
    throw invalid;
  TopoDS_Edge seam, rim;
  int end_uses = 0, seam_uses = 0;
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto e = TopoDS::Edge(it.Current());
    const BRepAdaptor_Curve c(e);
    BRepAdaptor_Curve2d trim(e, face);
    if (trim.GetType() != GeomAbs_Line || BRep_Tool::Degenerated(e))
      throw invalid;
    if (c.GetType() == GeomAbs_Line) {
      if (!BRep_Tool::IsClosed(e, face) ||
          !c.Line().Direction().IsParallel(gp_Dir(axis), 1.e-9) ||
          (!seam.IsNull() &&
           (!seam.IsSame(e) || seam.Orientation() == e.Orientation())))
        throw invalid;
      ++seam_uses;
      seam = e;
    } else if (e.IsSame(disk)) {
      if (e.Orientation() == disk.Orientation())
        throw invalid;
      ++end_uses;
    } else {
      if (!rim.IsNull() || c.GetType() != GeomAbs_Circle ||
          !c.Circle().Axis().Direction().IsParallel(gp_Dir(axis), 1.e-9) ||
          c.Circle().Location().Distance(center) > revolution_precision ||
          std::abs(c.Circle().Radius() - radius) > revolution_precision ||
          std::abs(c.LastParameter() - c.FirstParameter() - revolution_period) *
                  radius >
              revolution_precision)
        throw invalid;
      rim = e;
    }
  }
  if (rim.IsNull() || seam_uses != 2 || end_uses != 1)
    throw invalid;
  return rim;
}
