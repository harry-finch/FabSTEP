extern "C" int fabstep_build_unequal_corner(const double *parameters, const double *points, size_t nv,
                                            const size_t *indices, size_t nt, size_t *owners,
                                            const size_t *tokens, double fit, double tolerance,
                                            double *bound, StepOutput *output, char *error,
                                            size_t capacity) noexcept {
  if (!parameters || !points || !indices || !owners || !bound || !output || !nv || nv > 49152 || nt < 10 ||
      nt > 16384)
    return missing_step_argument(output, error, capacity);
  *bound = 0.;
  FaceLabels labels;
  SurfaceCounts counts{6, 3, 3};
  counts.height_patches = 1;
  return validated_step(
      [&] {
        corner_domain::Data data;
        for (size_t i = 0; i < 3; ++i)
          std::copy_n(parameters + 3 * i, 3, data[i].begin());
        const auto solid = corner_domain::build(data, corner_unproven, true);
        const auto proof = corner_domain::inspect(solid, corner_unproven);
        corner_domain::sides(proof, solid, corner_unproven);
        std::vector<unequal_corner::Point> vertices(nv);
        std::vector<unequal_corner::Tri> triangles(nt);
        for (size_t i = 0; i < nv; ++i)
          std::copy_n(points + 3 * i, 3, vertices[i].begin());
        for (size_t i = 0; i < nt; ++i)
          std::copy_n(indices + 3 * i, 3, triangles[i].begin());
        std::vector<size_t> refined(owners, owners + nt);
        *bound = corner_domain::certify_source(proof, vertices, triangles, refined, fit, tolerance);
        std::copy(refined.begin(), refined.end(), owners);
        if (tokens)
          for (size_t i = 0; i < 10; ++i) {
            corner_domain::need(tokens[i] && tokens[i] <= face_token_limit &&
                                    std::find(tokens, tokens + i, tokens[i]) == tokens + i,
                                invalid_model);
            labels.emplace_back(proof.faces[i], tokens[i]);
          }
        return solid;
      },
      counts, output, error, capacity, nullptr, tokens ? &labels : nullptr);
}
