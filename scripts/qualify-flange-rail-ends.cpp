// The source rectangle spans actual free coedges. Only two duplicated
// topological vertices have to be identified, one per proven source corner.
#include "qualify-flange-shell.hxx"
#include <BRepTools_ReShape.hxx>
#include <BRepAdaptor_Curve2d.hxx>

namespace {
struct RailEnd {
    int owner;
    gp_Dir normal;
    double area;
    std::array<gp_Pnt, 4> corners;
    std::array<int, 4> neighbors;
};
using Ends = std::array<RailEnd, 2>;
using Rails = std::array<TopoDS_Edge, 4>;
Ends loadEnds(const char *path) {
    std::ifstream in(path);
    Ends ends;
    for (auto &p : ends) {
        double x, y, z;
        check(bool(in >> p.owner >> x >> y >> z >> p.area) && std::abs(std::hypot(x, y, z) - 1) < 1e-10 &&
                  p.area > 0,
              "bad end data");
        p.normal = gp_Dir(x, y, z);
        for (int k = 0; k < 4; ++k) {
            check(bool(in >> x >> y >> z >> p.neighbors[k]) && p.neighbors[k] > 0, "bad corner");
            p.corners[k] = gp_Pnt(x, y, z);
        }
    }
    in >> std::ws;
    check(in.eof(), "trailing end data");
    return ends;
}
bool directed(const TopoDS_Edge &e, const gp_Pnt &first, const gp_Pnt &last) {
    return BRep_Tool::Pnt(TopExp::FirstVertex(e, true)).Distance(first) < spatial &&
           BRep_Tool::Pnt(TopExp::LastVertex(e, true)).Distance(last) < spatial;
}
std::array<Rails, 2> inputRails(const TopoDS_Shape &shell, const Ends &ends) {
    const auto adj = adjacency(shell);
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(shell, TopAbs_FACE, faces);
    std::array<Rails, 2> rails;
    for (int i = 0; i < 2; ++i)
        for (int k = 0; k < 4; ++k) {
            const auto &end = ends[i];
            check(end.neighbors[k] <= faces.Extent(), "bad neighbor id");
            const auto face = TopoDS::Face(faces(end.neighbors[k]));
            check(BRepAdaptor_Surface(face).GetType() == GeomAbs_Cylinder, "noncylindrical rail support");
            for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
                auto e = TopoDS::Edge(it.Current());
                if (!directed(e, end.corners[(k + 1) % 4], end.corners[k]))
                    continue;
                check(rails[i][k].IsNull() && adj.FindFromKey(e).Extent() == 1 &&
                          BRepAdaptor_Curve(e).GetType() == GeomAbs_Line,
                      "missing, ambiguous or occupied rail");
                rails[i][k] = TopoDS::Edge(e.Reversed());
            }
            check(!rails[i][k].IsNull(), "required free coedge absent or reversed");
        }
    return rails;
}
TopoDS_Shape attach(const TopoDS_Shape &prior, const Ends &ends) {
    const auto rails = inputRails(prior, ends);
    BRepTools_ReShape canonical;
    for (const auto &contour : rails) {
        int joins = 0;
        for (int k = 0; k < 4; ++k) {
            const auto v = TopExp::LastVertex(contour[k], true);
            const auto w = TopExp::FirstVertex(contour[(k + 1) % 4], true);
            if (v.IsSame(w))
                continue;
            check(BRep_Tool::Pnt(v).Distance(BRep_Tool::Pnt(w)) < spatial, "distinct rail endpoints");
            canonical.Replace(w.Oriented(TopAbs_FORWARD), v.Oriented(TopAbs_FORWARD));
            ++joins;
        }
        check(joins == 1, "unexpected number of coincident end vertices");
    }
    const auto remapped = canonical.Apply(prior);
    BRep_Builder add;
    TopoDS_Shell combined;
    add.MakeShell(combined);
    for (TopExp_Explorer f(remapped, TopAbs_FACE); f.More(); f.Next())
        add.Add(combined, f.Current());
    for (int i = 0; i < 2; ++i) {
        BRepBuilderAPI_MakeWire wire;
        for (const auto &e : rails[i])
            wire.Add(TopoDS::Edge(canonical.Apply(e)));
        check(wire.IsDone() && wire.Wire().Closed(), "rail wire not closed");
        BRepBuilderAPI_MakeFace plane(gp_Pln(ends[i].corners[0], ends[i].normal), wire.Wire(), true);
        check(plane.IsDone(), "cannot create measured plane");
        add.Add(combined, plane.Face());
    }
    return combined;
}
// Track the other 1,251 free coedges on their original face. Counting them
// alone would not detect a free edge exchanged for an unrelated edge.
OldFace freeSignature(OldFace value, const TopoDS_Face &face, const Adjacency &adj,
                      const TopTools_IndexedMapOfShape &consumed = {}) {
    auto all = std::move(value.edges3d);
    value.edges3d.clear();
    size_t i = 0;
    for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next(), ++i)
        if (adj.FindFromKey(e.Current()).Extent() == 1 && !consumed.Contains(e.Current()))
            value.edges3d.push_back(all.at(i));
    return value;
}
struct Reference {
    std::vector<OldFace> originals, remainingFree;
    int free, loops, edges, vertices;
};
Reference baseline(const TopoDS_Shape &s, const Ends &ends) {
    Reference r{};
    std::tie(r.free, r.loops) = freeCounts(s);
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(s, TopAbs_FACE, faces);
    const auto adj = adjacency(s);
    TopTools_IndexedMapOfShape consumed;
    for (const auto &rails : inputRails(s, ends))
        for (const auto &e : rails)
            consumed.Add(e);
    for (int i = 1; i <= faces.Extent(); ++i) {
        const auto face = TopoDS::Face(faces(i));
        r.originals.push_back(signature(face));
        r.remainingFree.push_back(freeSignature(r.originals.back(), face, adj, consumed));
    }
    r.edges = count(s, TopAbs_EDGE);
    r.vertices = count(s, TopAbs_VERTEX);
    check(r.originals.size() == 1814 && r.free == 1259 && r.loops == 2 && count(s, TopAbs_SHELL) == 1 &&
              count(s, TopAbs_SOLID) == 0 && BRepCheck_Analyzer(s, true).IsValid(),
          "unexpected input shell");
    return r;
}
int rectangle(const TopoDS_Face &f, const Ends &ends) {
    if (BRepAdaptor_Surface(f).GetType() != GeomAbs_Plane || count(f, TopAbs_EDGE) != 4)
        return -1;
    for (int i = 0; i < 2; ++i) {
        int matches = 0;
        for (TopExp_Explorer e(f, TopAbs_EDGE); e.More(); e.Next())
            for (int k = 0; k < 4; ++k)
                if (directed(TopoDS::Edge(e.Current()), ends[i].corners[k], ends[i].corners[(k + 1) % 4]))
                    ++matches;
        if (matches == 4)
            return i;
    }
    return -1;
}
double verify(const TopoDS_Shape &result, const Ends &ends, const Reference &r) {
    check(BRepCheck_Analyzer(result, true).IsValid() && count(result, TopAbs_SHELL) == 1 &&
              count(result, TopAbs_SOLID) == 0,
          "invalid extended shell");
    check(count(result, TopAbs_FACE) == int(r.originals.size()) + 2 &&
              count(result, TopAbs_EDGE) == r.edges && count(result, TopAbs_VERTEX) == r.vertices - 2 &&
              freeCounts(result) == std::make_pair(r.free - 8, r.loops),
          "unexpected topology after joining rail ends");
    std::vector<bool> old(r.originals.size());
    std::array<bool, 2> seen{};
    const auto adj = adjacency(result);
    double maxAreaError = 0.;
    for (TopExp_Explorer ex(result, TopAbs_FACE); ex.More(); ex.Next()) {
        const auto f = TopoDS::Face(ex.Current());
        const int i = rectangle(f, ends);
        if (i < 0) {
            const auto measured = signature(f);
            auto match = std::find_if(r.originals.begin(), r.originals.end(),
                                      [&](const auto &a) { return sameOriginal(a, measured); });
            check(match != r.originals.end(), "original oriented face changed");
            const size_t k = std::distance(r.originals.begin(), match);
            check(!old[k], "duplicate original face");
            check(sameOriginal(r.remainingFree[k], freeSignature(measured, f, adj)),
                  "unrelated oriented free edge changed");
            old[k] = true;
            continue;
        }
        check(!seen[i], "duplicate rail end");
        seen[i] = true;
        const auto &end = ends[i];
        const BRepAdaptor_Surface surf(f);
        auto normal = surf.Plane().Axis().Direction();
        if (f.Orientation() == TopAbs_REVERSED)
            normal.Reverse();
        check(normal.IsEqual(end.normal, 1e-8), "rail-end normal changed");
        GProp_GProps props;
        BRepGProp::SurfaceProperties(f, props);
        maxAreaError = std::max(maxAreaError, std::abs(props.Mass() - end.area));
        check(count(f, TopAbs_WIRE) == 1 && maxAreaError < areaTolerance, "rail-end area or wire changed");
        for (TopExp_Explorer e(f, TopAbs_EDGE); e.More(); e.Next()) {
            const auto edge = TopoDS::Edge(e.Current());
            const auto &uses = adj.FindFromKey(edge);
            const BRepAdaptor_Curve c(edge);
            const BRepAdaptor_Curve2d uv(edge, f);
            check(uses.Extent() == 2 && c.GetType() == GeomAbs_Line && uv.GetType() == GeomAbs_Line,
                  "invalid shared trim");
            for (double t : {0., .5, 1.}) {
                auto q = uv.Value(uv.FirstParameter() * (1 - t) + uv.LastParameter() * t);
                check(surf.Value(q.X(), q.Y())
                              .Distance(c.Value(c.FirstParameter() * (1 - t) + c.LastParameter() * t)) <
                          spatial,
                      "invalid rail pcurve");
            }
            int k = 0;
            while (k < 4 && !directed(edge, end.corners[k], end.corners[(k + 1) % 4]))
                ++k;
            check(k < 4, "unidentified rectangle edge");
            const auto other = TopoDS::Face(uses.First().IsSame(f) ? uses.Last() : uses.First());
            check(sameOriginal(r.originals.at(end.neighbors[k] - 1), signature(other)),
                  "wrong adjacent cylinder");
            int opposite = 0;
            for (TopExp_Explorer use(other, TopAbs_EDGE); use.More(); use.Next())
                if (use.Current().IsSame(edge))
                    opposite += use.Current().Orientation() != edge.Orientation();
            check(opposite == 1, "wrong rail coedge orientation");
        }
    }
    check(seen[0] && seen[1] && std::all_of(old.begin(), old.end(), [](bool yes) { return yes; }),
          "missing face");
    return maxAreaError;
}
} // namespace
int main(int argc, char **argv) {
    try {
        check(argc == 4, "use INPUT.step RAIL-ENDS.txt OUTPUT_DIR");
        const auto s = readStep(argv[1]);
        const auto ends = loadEnds(argv[2]);
        const auto ref = baseline(s, ends);
        const auto built = attach(s, ends);
        const double a = verify(built, ends, ref);
        const auto out = std::filesystem::path(argv[3]);
        const auto imported = roundtrip(built, out / "rail-ends.step");
        const double b = verify(imported, ends, ref);
        auto wrong = ends;
        wrong[0].normal.Reverse();
        bool refused = false;
        try {
            verify(imported, wrong, ref);
        } catch (const std::runtime_error &e) {
            refused = std::string(e.what()) == "rail-end normal changed";
        }
        check(refused, "reversed normal accepted");
        wrong = ends;
        std::swap(wrong[0].neighbors[0], wrong[0].neighbors[1]);
        refused = false;
        try {
            (void)inputRails(s, wrong);
        } catch (const std::runtime_error &) {
            refused = true;
        }
        check(refused, "exchanged source neighbors accepted");
        writeFrontier(imported, out, ref.free - 8);
        std::ofstream report(out / "report.json");
        report << std::setprecision(15)
               << "{\"original_faces\":1814,\"faces\":1816,\"solids\":0,\"shells\":1,"
               << "\"joins\":8,\"identified_vertex_pairs\":2,\"free_edges\":1251,\"free_loops\":2,"
               << "\"max_area_error_mm2\":" << std::max(a, b) << ",\"reversed_normal_rejected\":true,"
               << "\"exchanged_neighbors_rejected\":true,\"step_roundtrip\":true}\n";
        check(bool(report), "cannot write report");
        return 0;
    } catch (const Standard_Failure &e) {
        std::cerr << e.GetMessageString() << "\n";
    } catch (const std::exception &e) {
        std::cerr << e.what() << "\n";
    }
    return 1;
}
