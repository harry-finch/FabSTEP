extern "C" int fabstep_build_cut_octant(
    const double* center, const double* axes, double radius, const size_t* tokens,
    StepOutput* output, char* error, size_t capacity) noexcept {
  FaceLabels labels;
  return validated_step([&] {
    const double quarter = std::acos(-1.) / 2.;
    const gp_Ax2 frame(point_at(center, 0), direction(axes + 6), direction(axes));
    BRepPrimAPI_MakeSphere octant(frame, radius, 0., quarter, quarter);
    const auto solid = octant.Solid();
    if (!octant.IsDone() || solid.IsNull()) {
      throw CadFailure{invalid_solid, "Impossible de construire l’octant sphérique."};
    }
    if (tokens) {
      std::array<bool, 4> seen{};
      for (TopExp_Explorer it(solid, TopAbs_FACE); it.More(); it.Next()) {
        const auto face = TopoDS::Face(it.Current());
        const BRepAdaptor_Surface surface(face);
        size_t owner = 3;
        if (surface.GetType() == GeomAbs_Plane) {
          for (owner = 0; owner < 3; ++owner) {
            const auto normal = direction(axes + 3 * ((owner + 2) % 3));
            if (surface.Plane().Axis().Direction().IsParallel(normal, 1e-8)) break;
          }
          if (owner == 3) throw CadFailure{invalid_model, "Plan de l’octant sans provenance."};
        } else if (surface.GetType() != GeomAbs_Sphere) {
          throw CadFailure{invalid_model, "Surface imprévue dans l’octant."};
        }
        if (seen[owner]) throw CadFailure{invalid_model, "Provenance de l’octant ambiguë."};
        seen[owner] = true;
        labels.push_back({face, tokens[owner]});
      }
      if (seen != std::array<bool, 4>{true, true, true, true}) {
        throw CadFailure{invalid_model, "Provenance incomplète de l’octant."};
      }
    }
    return solid;
  }, {3, 0, 0, 0, 1, 0, 1}, output, error, capacity, nullptr, tokens ? &labels : nullptr);
}
