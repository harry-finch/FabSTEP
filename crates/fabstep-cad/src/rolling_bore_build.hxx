// Only called after the mesh identified both plates, both cylinders and
// their five disjoint interface cycles. Recheck the fit on the native result.
extern "C" int fabstep_build_rolling_bore(
    const double *data, const double *xyz, size_t nv, const size_t *ids,
    const size_t *owners, size_t nt, size_t first, double fit, double limit,
    StepOutput *output, char *error, size_t capacity) noexcept {
  if (!output || !data || !xyz || !ids || !owners || !nv || nv > 24576 ||
      nt < 80 || nt > 8192 || !first || first > face_token_limit - 4 ||
      !(fit > 0. && fit <= 1.e-5 && limit > fit && std::isfinite(limit)))
    return missing_step_argument(output, error, capacity);
  SurfaceCounts expected{2, 2};
  expected.height_patches = 1;
  FaceLabels labels;
  rolling_bore::SourceProof witness{{}, {}, fit, limit};
  return validated_step(
      [&] {
        const auto reject = [](const char *message) {
          throw CadFailure{rolling_bore_unproven, message};
        };
        for (int i = 0; i < 10; ++i)
          if (!std::isfinite(data[i]) || std::abs(data[i]) > 1024.)
            reject("Paramètres du raccord hors plage.");
        const double quantum = std::min(1.e-6, fit / 10.);
        const auto grid = [&](double v) {
          return std::round(v / quantum) * quantum;
        };
        const gp_Pnt origin(grid(data[0]), grid(data[1]), data[2]);
        const double h = data[3], R = grid(data[4]), b = grid(data[5]),
                     dy = grid(data[6]), dz = grid(data[7]);
        const double radius = data[8], spread = std::max(data[9], quantum);
        if (!(h > 0. && h <= 64. && R <= 32. && b > .1 &&
              R > 1.5 * (b + std::abs(dy)) && dz > b + radius &&
              h - dz > b + radius && radius > 2 * spread &&
              radius + 2 * spread < b / 2. && spread <= .01 * radius))
          reject("Dimensions du raccord non vérifiées.");
        auto &points = witness.vertices;
        points.resize(nv);
        for (size_t i = 0; i < nv; ++i)
          for (size_t j = 0; j < 3; ++j) {
            auto v = xyz[3 * i + j];
            if (!std::isfinite(v) || std::abs(v) > 1024.)
              reject("Coordonnée du raccord hors plage.");
            points[i][j] = v;
          }
        auto &blend = witness.triangles;
        std::set<size_t> vertices;
        std::array<size_t, 5> counts{};
        for (size_t i = 0; i < nt; ++i) {
          rolling_bore::Tri tri;
          std::copy_n(ids + 3 * i, 3, tri.begin());
          const auto owner = owners[i];
          if (owner >= 5)
            reject("Face source du raccord inconnue.");
          ++counts[owner];
          for (auto id : tri) {
            if (id >= nv)
              reject("Sommet source du raccord inconnu.");
            auto v = points[id];
            const double x = v[0] - origin.X(), y = v[1] - origin.Y(),
                         z = v[2] - origin.Z();
            if ((owner == 0 && std::abs(z) > fit) ||
                (owner == 1 && std::abs(z - h) > fit) ||
                (owner == 2 && std::abs(std::hypot(x, y) - R) > fit) ||
                (owner == 3 &&
                 std::abs(std::hypot(y - dy, z - dz) - b) > fit) ||
                (owner == 4 && x >= 0.))
              reject("Une facette dépasse la tolérance de son support "
                     "cylindrique ou plan.");
          }
          if (owner == 4) {
            blend.push_back(tri);
            vertices.insert(tri.begin(), tri.end());
          }
        }
        if (std::find(counts.begin(), counts.end(), 0) != counts.end() ||
            vertices.size() < 12)
          reject("La partition des surfaces est incomplète.");
        rolling_shape::Stock stock(origin, h, R, b, dy, dz,
                                   rolling_bore_unproven);
        auto build = [&](double r) {
          return stock.fillet(r, rolling_bore_unproven, false);
        };
        std::vector<size_t> samples(vertices.begin(), vertices.end());
        auto residual = [&](double r) {
          auto solid = build(r);
          Handle(Geom_BSplineSurface) surface;
          for (TopExp_Explorer it(solid, TopAbs_FACE); it.More(); it.Next()) {
            BRepAdaptor_Surface s(TopoDS::Face(it.Current()), false);
            if (s.GetType() == GeomAbs_BSplineSurface) {
              if (!surface.IsNull())
                reject("Plusieurs surfaces courbes proposées pour le raccord.");
              surface = s.BSpline();
            }
          }
          if (surface.IsNull())
            reject("Surface courbe de raccord absente.");
          double result = 0.;
          for (size_t i = 0; i < samples.size();
               i += std::max<size_t>(1, samples.size() / 64)) {
            GeomAPI_ProjectPointOnSurf p(
                unequal_corner::point(points[samples[i]]), surface);
            if (!p.NbPoints())
              return std::numeric_limits<double>::infinity();
            result = std::max(result, p.LowerDistance());
          }
          return result;
        };
        // The projected points only choose a candidate. Admission requires
        // every source triangle and both material boundaries in the periodic
        // certificate.
        double lo = radius - 2 * spread, hi = radius + 2 * spread,
               best = radius, best_loss = residual(radius);
        for (int i = 0; i < 24; ++i) {
          double a = (2 * lo + hi) / 3, c = (lo + 2 * hi) / 3, fa = residual(a),
                 fc = residual(c);
          if (fa < best_loss) {
            best = a;
            best_loss = fa;
          }
          if (fc < best_loss) {
            best = c;
            best_loss = fc;
          }
          if (fa < fc)
            hi = c;
          else
            lo = a;
        }
        double radius_mm = grid(best);
        // The mesh must identify the measured radius at the user's strict
        // fitting tolerance, including vertices inside the blend. The precise
        // transfer evaluates that same rolling ball with tighter OCCT rails.
        // Its complete annulus, not just the fitted vertices, must then stay
        // inside the remaining source deviation budget in both directions.
        bool identified = false;
        unequal_corner::Certificate last;
        for (int delta : {0, -1, 1, -2, 2, -3, 3, -4, 4}) {
          const double proposal = grid(best) + delta * quantum;
          if (proposal < radius - 2 * spread || proposal > radius + 2 * spread)
            continue;
          auto nominal = build(proposal);
          last = {};
          if (rolling_bore::certify(rolling_shape::material_faces(
                                        nominal, rolling_bore_unproven)[4],
                                    points, blend, last, fit, limit)) {
            radius_mm = proposal;
            identified = true;
            break;
          }
        }
        if (!identified)
          throw CadFailure{
              rolling_bore_unproven,
              std::string("Le rayon mesuré ne reproduit pas le maillage (") +
                  (last.reason ? last.reason : "couture") + ", triangle " +
                  std::to_string(last.triangles) + ", rayon " +
                  std::to_string(grid(best)) + ")."};
        auto solid = stock.fillet(radius_mm, rolling_bore_unproven);
        witness.fit = limit;
        rolling_shape::prepare_transfer(solid, rolling_bore_unproven);
        const auto faces =
            rolling_shape::material_faces(solid, rolling_bore_unproven);
        unequal_corner::Certificate cert;
        if (!rolling_bore::certify(faces[4], points, blend, cert, witness.fit,
                                   limit))
          throw CadFailure{
              rolling_bore_unproven,
              std::string("Le raccord dépasse sa preuve de fidélité (") +
                  (cert.reason ? cert.reason : "paramétrage de la paroi") +
                  ", triangle " + std::to_string(cert.triangles) + ", rayon " +
                  std::to_string(grid(best)) + ")."};
        for (size_t i = 0; i < 5; ++i)
          labels.emplace_back(faces[i], first + i);
        return solid;
      },
      expected, output, error, capacity, nullptr, &labels, nullptr, nullptr,
      nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, &witness);
}
