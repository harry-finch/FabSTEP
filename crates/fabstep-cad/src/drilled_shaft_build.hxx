// Build the two source-witnessed cylinders in the measured right-handed frame.
// Keep each branch off the periodic seams; replace its pcurves and 3D curve
// together and let the transfer verifier certify both complete loops.
extern "C" int fabstep_build_drilled_shaft(const DrilledShaftData *d, const size_t *tokens,
                                           StepOutput *output, char *error,
                                           size_t capacity) noexcept {
  FaceLabels labels;
  auto *tracked = tokens ? &labels : nullptr;
  SurfaceCounts expected{2, 2};
  return validated_step(
      [&] {
        const gp_Pnt origin = point_at(d->origin, 0);
        const gp_Dir z = direction(d->shaft), x = direction(d->bore), y = z.Crossed(x);
        const gp_Pnt middle = origin.Translated(gp_Vec(z) * d->level + gp_Vec(y) * d->offset);
        // Put the longitudinal seam of the shaft on the side between both mouths.
        const auto blank =
            BRepPrimAPI_MakeCylinder(gp_Ax2(origin, z, y), d->radius, d->height).Shape();
        const auto tool =
            BRepPrimAPI_MakeCylinder(gp_Ax2(middle.Translated(-gp_Vec(x) * (2 * d->radius)), x, z),
                                     d->bore_radius, 4 * d->radius)
                .Shape();
        BRepAlgoAPI_Cut cut(blank, tool);
        const CadFailure invalid{invalid_model,
                                 "Le perçage transversal de l’arbre est hors du domaine certifié."};
        if (!cut.IsDone() || cut.HasErrors())
          throw invalid;
        TopTools_IndexedMapOfShape solids, faces;
        TopExp::MapShapes(cut.Shape(), TopAbs_SOLID, solids);
        if (solids.Extent() != 1)
          throw invalid;
        const auto solid = TopoDS::Solid(solids(1));
        TopExp::MapShapes(solid, TopAbs_FACE, faces);
        std::array<TopoDS_Face, 4> found;
        for (int i = 1; i <= faces.Extent(); ++i) {
          auto f = TopoDS::Face(faces(i));
          BRepAdaptor_Surface s(f);
          int slot = -1;
          if (s.GetType() == GeomAbs_Plane) {
            const auto pl = s.Plane();
            if (!pl.Axis().Direction().IsParallel(z, 1.e-9))
              throw invalid;
            const double h = gp_Vec(origin, pl.Location()).Dot(gp_Vec(z));
            if (std::abs(h) < 1.e-7)
              slot = 0;
            if (std::abs(h - d->height) < 1.e-7)
              slot = 1;
          } else if (s.GetType() == GeomAbs_Cylinder) {
            const auto cyl = s.Cylinder();
            if (std::abs(cyl.Radius() - d->radius) < 1.e-7 &&
                cyl.Axis().Direction().IsParallel(z, 1.e-9))
              slot = 2;
            if (std::abs(cyl.Radius() - d->bore_radius) < 1.e-7 &&
                cyl.Axis().Direction().IsParallel(x, 1.e-9))
              slot = 3;
          }
          if (slot < 0 || !found[slot].IsNull())
            throw invalid;
          found[slot] = f;
          if (tracked)
            labels.push_back({f, tokens[slot]});
        }
        for (const auto &face : found)
          if (face.IsNull())
            throw invalid;
        for (TopExp_Explorer it(found[3], TopAbs_EDGE); it.More(); it.Next()) {
          auto e = TopoDS::Edge(it.Current());
          if (BRepAdaptor_Curve(e).GetType() == GeomAbs_BSplineCurve)
            set_cross_drill_junction(e, found[3], found[2], middle, gp_Vec(z), -gp_Vec(y),
                                     gp_Vec(x), 1., 0., -d->offset, d->bore_radius, d->radius,
                                     invalid);
        }
        return solid;
      },
      expected, output, error, capacity, nullptr, tracked, nullptr, nullptr, nullptr, nullptr,
      nullptr, nullptr, d);
}
