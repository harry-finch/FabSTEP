// Local laboratory: add the ten source-marking patches to an already-qualified open shell.
#include "qualify-flange-shell.hxx"

namespace {
using Key = std::pair<int, int>;
using Polygon = std::vector<int>;
using Loops = std::vector<Polygon>;
Key key(int a, int b) { return std::minmax(a, b); }
struct Patch {
    int owner;
    gp_Dir normal;
    double area;
    Loops loops;
};
struct Source {
    std::vector<gp_Pnt> points;
    std::vector<int> byX;
    std::vector<Patch> patches;
    std::map<Key, std::vector<int>> uses;
    int locate(const gp_Pnt &p) const {
        auto i = std::lower_bound(byX.begin(), byX.end(), p.X() - spatial,
                                  [&](int id, double x) { return points[id].X() < x; });
        int found = -1;
        for (; i != byX.end() && points[*i].X() <= p.X() + spatial; ++i)
            if (points[*i].Distance(p) <= spatial) {
                check(found < 0, "ambiguous source vertices");
                found = *i;
            }
        return found;
    }
};
Source readSource(const char *path) {
    check(std::filesystem::file_size(path) <= 8 * 1024 * 1024, "oversize measurements");
    std::ifstream in(path);
    Source s;
    int nv = 0, nf = 0;
    check(bool(in >> nv >> nf) && nv > 0 && nv <= 16000 && nf > 0 && nf <= 8000, "invalid model dimensions");
    for (int i = 0; i < nv; ++i) {
        double x, y, z;
        check(bool(in >> x >> y >> z) && std::isfinite(x) && std::isfinite(y) && std::isfinite(z),
              "invalid vertex");
        s.points.emplace_back(x, y, z);
        s.byX.push_back(i);
    }
    std::sort(s.byX.begin(), s.byX.end(), [&](int a, int b) { return s.points[a].X() < s.points[b].X(); });
    std::set<int> owners;
    for (int j = 0; j < nf; ++j) {
        int owner, n;
        double a, b, c, area;
        check(bool(in >> owner >> a >> b >> c >> area >> n) && owners.insert(owner).second &&
                  std::abs(std::hypot(a, b, c) - 1) < 1e-10 && std::isfinite(area) && area > 0 && n > 0 &&
                  n <= 32,
              "invalid face");
        Patch p{owner, gp_Dir(a, b, c), area, {}};
        for (int k = 0; k < n; ++k) {
            int len;
            check(bool(in >> len) && len >= 3 && len <= 4096, "invalid wire length");
            Polygon poly(len);
            for (int &id : poly)
                check(bool(in >> id) && id >= 0 && id < nv, "invalid vertex id");
            check(std::set<int>(poly.begin(), poly.end()).size() == poly.size(), "duplicate wire vertex");
            for (int i = 0; i < len; ++i) {
                int u = poly[i], v = poly[(i + 1) % len];
                auto &list = s.uses[key(u, v)];
                int signedId = (u < v ? 1 : -1) * (j + 1);
                check(list.empty() || (list.size() == 1 && (list.front() > 0) != (signedId > 0) &&
                                       std::abs(list.front()) != j + 1),
                      "non-manifold source lettering");
                list.push_back(signedId);
            }
            p.loops.push_back(std::move(poly));
        }
        s.patches.push_back(std::move(p));
    }
    in >> std::ws;
    check(in.eof(), "trailing measurements");
    return s;
}
TopoDS_Shape assemble(const TopoDS_Shape &previous, const Source &s) {
    const auto adj = adjacency(previous);
    std::vector<TopoDS_Vertex> vertices(s.points.size());
    std::map<Key, TopoDS_Edge> edges;
    // Reuse actual STEP vertices/edges on every input boundary of the marking.
    for (int i = 1; i <= adj.Extent(); ++i)
        if (adj(i).Extent() == 1) {
            auto e = TopoDS::Edge(adj.FindKey(i));
            e.Orientation(TopAbs_FORWARD);
            TopoDS_Vertex a, b;
            TopExp::Vertices(e, a, b, true);
            int u = s.locate(BRep_Tool::Pnt(a)), v = s.locate(BRep_Tool::Pnt(b));
            if (u < 0 || v < 0)
                continue;
            auto k = key(u, v);
            check(u != v && s.uses.count(k) && s.uses.at(k).size() == 1 &&
                      BRepAdaptor_Curve(e).GetType() == GeomAbs_Line,
                  "unmatched input free edge near lettering");
            for (auto endpoint : {std::make_pair(u, a), std::make_pair(v, b)}) {
                auto &vertex = vertices[endpoint.first];
                check(vertex.IsNull() || vertex.IsSame(endpoint.second), "unshared input boundary vertex");
                vertex = endpoint.second;
            }
            if (v < u)
                e.Reverse();
            check(edges.emplace(k, e).second, "duplicate input boundary edge");
        }
    check(edges.size() == 1004, "incomplete marking contour");
    for (size_t i = 0; i < vertices.size(); ++i)
        if (vertices[i].IsNull())
            vertices[i] = BRepBuilderAPI_MakeVertex(s.points[i]);
    for (const auto &[k, v] : s.uses) {
        if (v.size() == 1)
            check(edges.count(k), "an exterior lettering edge lacks an input edge");
        else
            edges.emplace(k, BRepBuilderAPI_MakeEdge(vertices[k.first], vertices[k.second]).Edge());
    }
    BRep_Builder builder;
    TopoDS_Shell shell;
    builder.MakeShell(shell);
    for (TopExp_Explorer it(previous, TopAbs_FACE); it.More(); it.Next())
        builder.Add(shell, it.Current());
    for (const auto &p : s.patches) {
        std::vector<TopoDS_Wire> wires;
        for (const auto &loop : p.loops) {
            BRepBuilderAPI_MakeWire wire;
            for (size_t i = 0; i < loop.size(); ++i) {
                int u = loop[i], v = loop[(i + 1) % loop.size()];
                auto edge = edges.at(key(u, v));
                if (v < u)
                    edge.Reverse();
                wire.Add(edge);
            }
            check(wire.IsDone() && wire.Wire().Closed(), "invalid lettering wire");
            wires.push_back(wire.Wire());
        }
        BRepBuilderAPI_MakeFace face(gp_Pln(s.points[p.loops.front().front()], p.normal), wires.front(),
                                     true);
        for (size_t i = 1; i < wires.size(); ++i)
            face.Add(wires[i]);
        check(face.IsDone(), "invalid lettering face");
        builder.Add(shell, face.Face());
    }
    return shell;
}
Polygon normalized(Polygon loop) {
    std::rotate(loop.begin(), std::min_element(loop.begin(), loop.end()), loop.end());
    return loop;
}
Polygon measuredWire(const TopoDS_Wire &w, const TopoDS_Face &f, const Source &s) {
    Polygon poly;
    for (BRepTools_WireExplorer it(w, f); it.More(); it.Next()) {
        if (BRepAdaptor_Curve(it.Current()).GetType() != GeomAbs_Line)
            return {};
        int id = s.locate(BRep_Tool::Pnt(it.CurrentVertex()));
        if (id < 0)
            return {};
        poly.push_back(id);
    }
    return poly.empty() ? poly : normalized(poly);
}
struct Metrics {
    int joins = 0, internal = 0, components = 0, contours = 0, free = 0;
    double areaError = 0;
};
Metrics verify(const TopoDS_Shape &result, const Source &s, const std::vector<OldFace> &originals,
               std::pair<int, int> initialFree) {
    check(count(result, TopAbs_SHELL) == 1 && count(result, TopAbs_SOLID) == 0 &&
              BRepCheck_Analyzer(result, true).IsValid(),
          "result is not a valid single open shell");
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(result, TopAbs_FACE, faces);
    check(faces.Extent() == int(s.patches.size() + originals.size()), "face count changed");
    std::map<Polygon, int> expected;
    for (size_t i = 0; i < s.patches.size(); ++i)
        check(expected.emplace(normalized(s.patches[i].loops.front()), i).second, "duplicate source face");
    const auto adj = adjacency(result);
    std::vector<int> owners(faces.Extent() + 1, -1);
    std::set<int> seen;
    std::vector<bool> oldSeen(originals.size(), false);
    Metrics m;
    for (int i = 1; i <= faces.Extent(); ++i) {
        auto f = TopoDS::Face(faces(i));
        auto outer = BRepTools::OuterWire(f);
        auto poly = measuredWire(outer, f, s);
        auto match = expected.find(poly);
        if (match == expected.end()) {
            const auto candidate = signature(f);
            bool found = false;
            for (size_t j = 0; j < originals.size(); ++j)
                if (!oldSeen[j] && sameOriginal(originals[j], candidate)) {
                    found = true;
                    oldSeen[j] = true;
                    break;
                }
            check(found, "face matches neither original shell nor oriented source marking");
            continue;
        }
        int id = match->second;
        const Patch &p = s.patches[id];
        check(seen.insert(id).second, "repeated marking face");
        owners[i] = id;
        BRepAdaptor_Surface surf(f);
        check(surf.GetType() == GeomAbs_Plane, "nonplanar marking face");
        gp_Dir normal = surf.Plane().Axis().Direction();
        if (f.Orientation() == TopAbs_REVERSED)
            normal.Reverse();
        check(normal.IsEqual(p.normal, 1e-8), "marking face orientation changed");
        std::multiset<Polygon> holes, wanted;
        for (TopExp_Explorer w(f, TopAbs_WIRE); w.More(); w.Next())
            if (!w.Current().IsSame(outer))
                holes.insert(measuredWire(TopoDS::Wire(w.Current()), f, s));
        for (size_t h = 1; h < p.loops.size(); ++h)
            wanted.insert(normalized(p.loops[h]));
        check(holes == wanted, "marking inner contours changed");
        GProp_GProps g;
        BRepGProp::SurfaceProperties(f, g);
        m.areaError = std::max(m.areaError, std::abs(p.area - g.Mass()));
        check(m.areaError <= areaTolerance, "marking face area changed");
    }
    check(seen.size() == s.patches.size(), "missing source marking face");
    Connectivity connectivity(s.patches.size());
    std::set<Key> measured;
    for (int i = 1; i <= adj.Extent(); ++i) {
        std::vector<int> ids;
        for (const auto &f : adj(i)) {
            int id = owners.at(faces.FindIndex(f));
            if (id >= 0)
                ids.push_back(id);
        }
        if (ids.empty())
            continue;
        check(adj(i).Extent() == 2, "marking edge lacks two face uses");
        auto edge = TopoDS::Edge(adj.FindKey(i));
        edge.Orientation(TopAbs_FORWARD);
        TopoDS_Vertex a, b;
        TopExp::Vertices(edge, a, b, true);
        int u = s.locate(BRep_Tool::Pnt(a)), v = s.locate(BRep_Tool::Pnt(b));
        auto k = key(u, v);
        check(s.uses.count(k) && measured.insert(k).second, "unexpected or repeated edge");
        std::set<int> expectedIds;
        for (int id : s.uses.at(k))
            expectedIds.insert(std::abs(id) - 1);
        check(expectedIds == std::set<int>(ids.begin(), ids.end()), "opposite owner mismatch");
        int balance = 0, uses = 0;
        for (const auto &face : adj(i)) {
            for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next())
                if (e.Current().IsSame(edge)) {
                    balance += e.Current().Orientation() == TopAbs_FORWARD ? 1 : -1;
                    ++uses;
                }
        }
        check(uses == 2 && balance == 0, "edge uses of marking do not oppose");
        if (ids.size() == 1)
            ++m.joins;
        else {
            ++m.internal;
            connectivity.connect(ids.front(), ids.back());
        }
    }
    check(measured.size() == s.uses.size() && m.joins == 1004, "missing measured marking edges");
    std::set<int> roots;
    for (size_t j = 0; j < s.patches.size(); ++j)
        roots.insert(connectivity.root(j));
    m.components = roots.size();
    check(m.components == 10, "incorrect marking component count");
    auto free = freeCounts(result);
    m.free = free.first;
    m.contours = free.second;
    check(free.first == initialFree.first - m.joins && free.second == initialFree.second - m.components,
          "unrelated free edges or loops changed");
    return m;
}
} // namespace

int main(int argc, char **argv) {
    try {
        check(argc == 4, "expected INPUT.step MARKING.txt OUTPUT_DIR or --frontier INPUT.step OUTPUT_DIR");
        if (std::string(argv[1]) == "--frontier") {
            const auto shell = readStep(argv[2]);
            const int faces = count(shell, TopAbs_FACE);
            check(faces == 1814 || faces == 1816 || faces == 2083,
                  "unexpected shell for frontier extraction");
            const int edges = faces == 2083 ? 1194 : (faces == 1816 ? 1251 : 1259);
            writeFrontier(shell, argv[3], edges);
            return 0;
        }
        const auto prior = readStep(argv[1]);
        check(count(prior, TopAbs_SHELL) == 1 && count(prior, TopAbs_SOLID) == 0 &&
                  BRepCheck_Analyzer(prior, true).IsValid(),
              "invalid prior shell");
        auto source = readSource(argv[2]);
        const auto initialFree = freeCounts(prior);
        std::vector<OldFace> originals;
        for (TopExp_Explorer f(prior, TopAbs_FACE); f.More(); f.Next())
            originals.push_back(signature(TopoDS::Face(f.Current())));
        const auto combined = assemble(prior, source);
        const auto built = verify(combined, source, originals, initialFree);
        const auto dir = std::filesystem::path(argv[3]);
        auto imported = roundtrip(combined, dir / "lettering.step");
        auto read = verify(imported, source, originals, initialFree);
        // Move the whole recessed layer by 0.01 mm along Z. The base retains
        // all 1004 joins, the cap areas and edge counts still match, and OCCT
        // still accepts the shell. Its measured cap locations must reject it.
        Source shallow = source;
        auto levels = std::minmax_element(shallow.points.begin(), shallow.points.end(),
                                          [](const gp_Pnt &a, const gp_Pnt &b) { return a.Z() < b.Z(); });
        double split = (levels.first->Z() + levels.second->Z()) / 2;
        check(levels.second->Z() - levels.first->Z() > 0.02, "insufficient relief depth for control");
        size_t moved = 0;
        for (auto &p : shallow.points)
            if (p.Z() > split) {
                p.SetZ(p.Z() - 0.01);
                ++moved;
            }
        check(moved && moved < shallow.points.size(), "depth control needs two levels");
        auto shallowStep = roundtrip(assemble(prior, shallow), dir / "shallow-marking.step");
        check(BRepCheck_Analyzer(shallowStep, true).IsValid() &&
                  freeCounts(shallowStep) == freeCounts(imported) &&
                  count(shallowStep, TopAbs_FACE) == count(imported, TopAbs_FACE),
              "depth control changed shell topology");
        bool depthRejected = false;
        try {
            verify(shallowStep, source, originals, initialFree);
        } catch (const std::runtime_error &error) {
            depthRejected = std::string(error.what()) ==
                            "face matches neither original shell nor oriented source marking";
        }
        check(depthRejected, "unfaithful relief depth passed");
        // A reversed shallow wall retains area, contour and ownership.
        source.patches.front().normal.Reverse();
        bool rejected = false;
        try {
            verify(imported, source, originals, initialFree);
        } catch (const std::runtime_error &error) {
            rejected = std::string(error.what()) == "marking face orientation changed";
        }
        check(rejected, "orientation negative passed");
        std::ofstream report(dir / "report.json");
        report << std::setprecision(12) << "{\"original_faces\":" << originals.size()
               << ",\"marking_faces\":" << source.patches.size()
               << ",\"result_faces\":" << originals.size() + source.patches.size()
               << ",\"components\":" << read.components << ",\"joins\":" << read.joins
               << ",\"internal_edges\":" << read.internal << ",\"free_edges\":" << read.free
               << ",\"free_loops\":" << read.contours
               << ",\"max_area_error_mm2\":" << std::max(read.areaError, built.areaError)
               << ",\"orientation_negative_rejected\":true,\"depth_negative_rejected\":true,\"step_"
                  "roundtrip\":true}\n";
        check(bool(report), "cannot write report");
        return 0;
    } catch (const Standard_Failure &e) {
        std::cerr << e.GetMessageString() << "\n";
    } catch (const std::exception &e) {
        std::cerr << e.what() << "\n";
    }
    return 1;
}
