// Prove a planar disk against the oriented source. All owners project with
// positive area, internal edges cancel and the remaining single wire must be
// simple. Hence the projected triangles cover its interior with degree one.
size_t add_plane(Model &m, const AtlasPlaneData &input, size_t available) {
  verify(input.owners && input.owner_count >= 2 && input.owner_count <= available &&
             input.boundary && input.boundary_count >= 3 && input.boundary_count <= 4096,
         "Domaine plan hors limites.");
  const gp_Vec n(input.normal[0], input.normal[1], input.normal[2]);
  verify(std::isfinite(n.Magnitude()) && std::abs(n.Magnitude() - 1.) < 1e-8 &&
             input.boundary[0] < m.source.size(),
         "Repère de plage plane invalide.");
  const gp_Pln support(m.source[input.boundary[0]], gp_Dir(n));
  std::map<Key, int> boundary;
  for (size_t j = 0; j < input.owner_count; ++j) {
    const auto owner = input.owners[j];
    verify(owner < m.triangles.size() && !m.claimed[owner], "Triangle plan revendiqué plusieurs fois.");
    const auto t = m.triangles[owner];
    m.claimed[owner] = true;
    bool curved = false;
    double motion = 0., sag = 0.;
    for (int k = 0; k < 3; ++k) {
      const int a = t[k], b = t[(k + 1) % 3];
      const auto &p = m.adjusted[a];
      const double distance = std::max(support.Distance(m.source[a]), support.Distance(p));
      plane_proof(distance <= 1.1e-10, "Plage plane pliée ou frontière déplacée.");
      motion = std::max(motion, m.source[a].Distance(p));
      if (auto it = m.arcs.find(key(a, b)); it != m.arcs.end()) {
        const auto &arc = it->second;
        const double sweep = arc.b - arc.a;
        const double weight = std::cos(sweep / 2);
        const auto pole = weightedArcPole(arc, support.Location(), weight);
        // Positive rational weights keep the complete arc in the slab of its
        // three poles. The fan orientation is checked over the whole ruling.
        plane_proof(!curved && weight > 0. && std::abs(pole.Dot(gp_Vec(support.Axis().Direction()))) <= 1.1e-10 * weight &&
                   positiveArcFan(arc, p, m.adjusted[b], m.adjusted[t[(k + 2) % 3]], n),
               "Un arc partagé ne borde pas le plan source.");
        curved = true;
        sag = arc.circle.Radius() * sweep * sweep / 8;
      }
      boundary[key(a, b)] += a < b ? 1 : -1;
    }
    const double bound = motion + sag + 1.1e-10;
    plane_proof(bound <= m.tolerance, "Écart du plan et de ses raccords hors tolérance.");
    m.patchBound = std::max(m.patchBound, bound);
    plane_proof(gp_Vec(m.source[t[0]], m.source[t[1]]).Crossed(gp_Vec(m.source[t[0]], m.source[t[2]])).Dot(n) > 0. &&
           gp_Vec(m.adjusted[t[0]], m.adjusted[t[1]]).Crossed(gp_Vec(m.adjusted[t[0]], m.adjusted[t[2]])).Dot(n) > 0.,
           "Triangle plan replié.");
  }
  std::set<size_t> corners;
  std::vector<int> ring;
  for (size_t j = 0; j < input.boundary_count; ++j) {
    const size_t a = input.boundary[j], b = input.boundary[(j + 1) % input.boundary_count];
    verify(a < m.source.size() && b < m.source.size() && corners.insert(a).second,
           "Contour plan ouvert ou pincé.");
    const auto it = boundary.find(key(int(a), int(b)));
    verify(it != boundary.end() && it->second == (a < b ? 1 : -1), "Contour hors de la plage plane.");
    boundary.erase(it);
    ring.push_back(int(a));
  }
  for (const auto &edge : boundary)
    verify(edge.second == 0, "Contour plan ou perçage absent.");
  m.planes.emplace_back(support, std::move(ring));
  return input.owner_count;
}
