// Undirected neighboring region tokens and their number of material edges.
// Geometry is certified per edge by atlas_transfer; this companion certificate
// preserves which two source regions actually share those edges.
using Interfaces = std::map<std::pair<size_t, size_t>, size_t>;
inline Interfaces source_interfaces(const Model &m, const size_t *tokens, size_t count) {
  struct Use { size_t token; bool forward; };
  std::map<Key, Use> pending;
  Interfaces graph;
  size_t region = 0;
  auto ring = [&](const auto &vertices, auto vertex_id) {
    verify(region < count, "Région hors de la partition source.");
    const auto token = tokens[region];
    for (size_t j = 0; j < vertices.size(); ++j) {
      const int a = vertex_id(vertices[j]), b = vertex_id(vertices[(j + 1) % vertices.size()]);
      const auto [it, inserted] = pending.emplace(key(a, b), Use{token, a < b});
      if (!inserted) {
        verify(token != it->second.token && it->second.forward != (a < b), "Interface source ambiguë.");
        ++graph[std::minmax(token, it->second.token)];
        pending.erase(it);
      }
    }
  };
  for (const auto &chart : m.charts) {
    for (const auto &loop : chart.loops)
      ring(loop, [](const TrimVertex &v) { return v.id; });
    ++region;
  }
  const auto id = [](int vertex) { return vertex; };
  for (const auto &plane : m.planes) {
    ring(plane.second, id);
    ++region;
  }
  for (size_t i = 0; i < m.triangles.size(); ++i)
    if (!m.claimed[i]) {
      ring(m.triangles[i], id);
      ++region;
    }
  verify(pending.empty() && region == count, "Un bord source n'a pas ses deux régions.");
  return graph;
}

inline void verify_interfaces(const Interfaces &expected, const FaceLabels &labels) {
  Interfaces actual;
  TopTools_IndexedMapOfShape material;
  std::vector<size_t> first;
  for (const auto &[shape, token] : labels) {
    for (TopExp_Explorer it(shape, TopAbs_EDGE); it.More(); it.Next()) {
      const auto edge = TopoDS::Edge(it.Current());
      if (BRep_Tool::Degenerated(edge)) continue;
      const auto id = size_t(material.Add(edge)) - 1;
      if (id == first.size()) {
        first.push_back(token);
      } else {
        if (first[id] == 0 || first[id] == token)
          throw CadFailure{mismatch, "La couture ou le transfert a changé les voisins d'une arête."};
        ++actual[std::minmax(token, first[id])];
        first[id] = 0;
      }
    }
  }
  if (actual != expected || std::any_of(first.begin(), first.end(), [](size_t t) { return t != 0; }))
    throw CadFailure{mismatch, "La couture ou le transfert a changé les interfaces des régions."};
}
