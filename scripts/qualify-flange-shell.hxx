// Common STEP and boundary checks for the Flansch open-shell laboratory.
#pragma once
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepTools.hxx>
#include <BRepTools_WireExplorer.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <GProp_GProps.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Shell.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <map>
#include <numeric>
#include <set>
#include <stdexcept>
#include <vector>

namespace {
constexpr double spatial = 5e-8, areaTolerance = 1e-7;
using Adjacency = TopTools_IndexedDataMapOfShapeListOfShape;
void check(bool condition, const char *why) {
    if (!condition)
        throw std::runtime_error(why);
}
TopoDS_Shape readStep(const std::filesystem::path &path) {
    check(std::filesystem::file_size(path) <= 64 * 1024 * 1024, "oversize STEP");
    STEPControl_Reader reader;
    check(reader.ReadFile(path.c_str()) == IFSelect_RetDone && reader.TransferRoots() == 1,
          "cannot read STEP");
    return reader.OneShape();
}
TopoDS_Shape roundtrip(const TopoDS_Shape &shape, const std::filesystem::path &path) {
    STEPControl_Writer writer;
    check(writer.Transfer(shape, STEPControl_ShellBasedSurfaceModel) == IFSelect_RetDone &&
              writer.Write(path.c_str()) == IFSelect_RetDone,
          "cannot write STEP");
    return readStep(path);
}
int count(const TopoDS_Shape &s, TopAbs_ShapeEnum type) {
    TopTools_IndexedMapOfShape items;
    TopExp::MapShapes(s, type, items);
    return items.Extent();
}
Adjacency adjacency(const TopoDS_Shape &shape) {
    Adjacency a;
    TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, a);
    return a;
}
struct Connectivity {
    std::vector<int> parent;
    explicit Connectivity(int size) : parent(size) { std::iota(parent.begin(), parent.end(), 0); }
    int root(int i) {
        while (parent[i] != i) {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        return i;
    }
    void connect(int a, int b) { parent[root(a)] = root(b); }
};
// A ruled triangle has a single vertex at its tip. OCCT represents that
// parameter-space limit by a degenerate edge with no physical free curve.
bool collapsedTip(const TopoDS_Edge &edge) {
    if (!BRep_Tool::Degenerated(edge))
        return false;
    auto a = TopExp::FirstVertex(edge), b = TopExp::LastVertex(edge);
    check(!a.IsNull() && !b.IsNull() && a.IsSame(b), "degenerate edge has two distinct vertices");
    return true;
}
std::pair<int, int> freeCounts(const TopoDS_Shape &shape) {
    const auto adj = adjacency(shape);
    TopTools_IndexedMapOfShape vertices;
    TopExp::MapShapes(shape, TopAbs_VERTEX, vertices);
    std::vector<int> degree(vertices.Extent() + 1, 0);
    Connectivity components(degree.size());
    int free = 0;
    for (int i = 1; i <= adj.Extent(); ++i) {
        const auto &uses = adj(i);
        check(uses.Extent() == 1 || uses.Extent() == 2, "non-manifold shell edge");
        if (uses.Extent() != 1 || collapsedTip(TopoDS::Edge(adj.FindKey(i))))
            continue;
        ++free;
        TopoDS_Vertex a, b;
        TopExp::Vertices(TopoDS::Edge(adj.FindKey(i)), a, b);
        int u = vertices.FindIndex(a), v = vertices.FindIndex(b);
        check(u && v, "missing endpoint");
        ++degree[u];
        ++degree[v];
        components.connect(u, v);
    }
    std::set<int> roots;
    for (size_t i = 1; i < degree.size(); ++i)
        if (degree[i]) {
            check(degree[i] == 2, "branched free contour");
            roots.insert(components.root(i));
        }
    return {free, roots.size()};
}

struct OldFace {
    int type, wires, edges;
    double area;
    gp_Pnt centroid;
    std::vector<std::array<gp_Pnt, 5>> edges3d;
};
OldFace signature(const TopoDS_Face &face) {
    GProp_GProps g;
    BRepGProp::SurfaceProperties(face, g);
    OldFace result{BRepAdaptor_Surface(face).GetType(),
                   count(face, TopAbs_WIRE),
                   count(face, TopAbs_EDGE),
                   g.Mass(),
                   g.CentreOfMass(),
                   {}};
    for (TopExp_Explorer ex(face, TopAbs_EDGE); ex.More(); ex.Next()) {
        auto edge = TopoDS::Edge(ex.Current());
        BRepAdaptor_Curve curve(edge);
        std::array<gp_Pnt, 5> samples;
        for (size_t k = 0; k < samples.size(); ++k) {
            double t = double(k) / (samples.size() - 1);
            if (edge.Orientation() == TopAbs_REVERSED)
                t = 1 - t;
            samples[k] = curve.Value((1 - t) * curve.FirstParameter() + t * curve.LastParameter());
        }
        result.edges3d.push_back(samples);
    }
    return result;
}
bool sameOriginal(const OldFace &expected, const OldFace &actual) {
    if (expected.type != actual.type || expected.wires != actual.wires || expected.edges != actual.edges ||
        std::abs(expected.area - actual.area) > areaTolerance ||
        expected.centroid.Distance(actual.centroid) > spatial ||
        expected.edges3d.size() != actual.edges3d.size())
        return false;
    // Look up entire oriented edges. The carrier alone has over a thousand.
    auto edges = actual.edges3d;
    std::sort(edges.begin(), edges.end(), [](const auto &a, const auto &b) { return a[0].X() < b[0].X(); });
    std::vector<bool> matched(edges.size(), false);
    for (const auto &e : expected.edges3d) {
        auto at = std::lower_bound(edges.begin(), edges.end(), e[0].X() - spatial,
                                   [](const auto &samples, double x) { return samples[0].X() < x; });
        bool found = false;
        for (; at != edges.end() && (*at)[0].X() <= e[0].X() + spatial; ++at) {
            const size_t j = at - edges.begin();
            if (!matched[j] &&
                std::equal(e.begin(), e.end(), at->begin(),
                           [](const gp_Pnt &p, const gp_Pnt &q) { return p.Distance(q) <= spatial; })) {
                matched[j] = true;
                found = true;
                break;
            }
        }
        if (!found)
            return false;
    }
    return true;
}
void jsonPoint(std::ostream &out, const gp_Pnt &p) {
    out << '[' << p.X() << ',' << p.Y() << ',' << p.Z() << ']';
}
void jsonDir(std::ostream &out, const gp_Dir &d) { jsonPoint(out, gp_Pnt(d.XYZ())); }
void axisFields(std::ostream &out, const gp_Ax1 &axis) {
    out << ",\"axis_origin_mm\":";
    jsonPoint(out, axis.Location());
    out << ",\"axis\":";
    jsonDir(out, axis.Direction());
}
void writeOwner(std::ostream &out, int id, const TopoDS_Face &f) {
    BRepAdaptor_Surface s(f);
    GProp_GProps props;
    BRepGProp::SurfaceProperties(f, props, 1e-12);
    out << "{\"id\":" << id << ",\"area_mm2\":" << props.Mass() << ",\"center_mm\":";
    jsonPoint(out, props.CentreOfMass());
    out << ",\"orientation\":\"" << (f.Orientation() == TopAbs_FORWARD ? "forward" : "reversed")
        << "\",\"family\":";
    switch (s.GetType()) {
    case GeomAbs_Plane:
        out << "\"plane\"";
        axisFields(out, s.Plane().Axis());
        break;
    case GeomAbs_Cylinder:
        out << "\"cylinder\",\"radius_mm\":" << s.Cylinder().Radius();
        axisFields(out, s.Cylinder().Axis());
        break;
    case GeomAbs_Cone:
        out << "\"cone\",\"radius_at_origin_mm\":" << s.Cone().RefRadius()
            << ",\"semi_angle_rad\":" << s.Cone().SemiAngle();
        axisFields(out, s.Cone().Axis());
        break;
    case GeomAbs_Torus:
        out << "\"torus\",\"major_radius_mm\":" << s.Torus().MajorRadius()
            << ",\"minor_radius_mm\":" << s.Torus().MinorRadius();
        axisFields(out, s.Torus().Axis());
        break;
    case GeomAbs_BSplineSurface:
        out << "\"bspline\"";
        break;
    default:
        throw std::runtime_error("unknown boundary face support");
    }
    out << '}';
}
struct BoundaryUse {
    TopoDS_Edge edge;
    int face, first, last;
};
void writeBoundaryEdge(std::ostream &out, const BoundaryUse &use) {
    const BRepAdaptor_Curve c(use.edge);
    const bool forward = use.edge.Orientation() == TopAbs_FORWARD;
    const double a = forward ? c.FirstParameter() : c.LastParameter(),
                 b = forward ? c.LastParameter() : c.FirstParameter();
    out << "{\"face_id\":" << use.face << ",\"orientation\":\"" << (forward ? "forward" : "reversed")
        << "\",\"start_mm\":";
    jsonPoint(out, c.Value(a));
    out << ",\"mid_mm\":";
    jsonPoint(out, c.Value((a + b) / 2));
    out << ",\"end_mm\":";
    jsonPoint(out, c.Value(b));
    out << ",\"samples_mm\":[";
    for (int j = 0; j <= 4; ++j) {
        if (j)
            out << ',';
        jsonPoint(out, c.Value(a + (b - a) * (.25 * j)));
    }
    out << ']';
    switch (c.GetType()) {
    case GeomAbs_Line:
        out << ",\"kind\":\"line\"";
        break;
    case GeomAbs_Circle: {
        const auto circle = c.Circle();
        out << ",\"kind\":\"circle\",\"center_mm\":";
        jsonPoint(out, circle.Location());
        out << ",\"axis\":";
        jsonDir(out, circle.Axis().Direction());
        out << ",\"zero_direction\":";
        jsonDir(out, circle.XAxis().Direction());
        out << ",\"radius_mm\":" << circle.Radius() << ",\"start_rad\":" << a
            << ",\"signed_sweep_rad\":" << b - a;
        break;
    }
    case GeomAbs_BSplineCurve:
        out << ",\"kind\":\"bspline\"";
        break;
    case GeomAbs_BezierCurve:
        out << ",\"kind\":\"bezier\"";
        break;
    default:
        throw std::runtime_error("unknown free edge type");
    }
    out << '}';
}
// Walk the only outgoing coedge at each topological vertex. The direction
// comes from its supporting face, including when that face is reversed.
void writeFrontier(const TopoDS_Shape &shell, const std::filesystem::path &dir, int expectedEdges = 1259) {
    check(count(shell, TopAbs_SHELL) == 1 && count(shell, TopAbs_SOLID) == 0 &&
              BRepCheck_Analyzer(shell, true).IsValid(),
          "invalid open STEP witness");
    const auto adj = adjacency(shell);
    TopTools_IndexedMapOfShape faces, vertices;
    TopExp::MapShapes(shell, TopAbs_FACE, faces);
    std::vector<BoundaryUse> boundary;
    std::map<int, int> outgoing;
    std::set<int> arriving, owners;
    for (int i = 1; i <= faces.Extent(); ++i) {
        const auto face = TopoDS::Face(faces(i));
        for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
            const auto edge = TopoDS::Edge(it.Current());
            const auto &uses = adj.FindFromKey(edge);
            if (uses.Extent() == 2 || collapsedTip(edge))
                continue;
            check(uses.Extent() == 1 && uses.First().IsSame(face) &&
                      (edge.Orientation() == TopAbs_FORWARD || edge.Orientation() == TopAbs_REVERSED),
                  "invalid free coedge incidence");
            const auto a = TopExp::FirstVertex(edge, true), b = TopExp::LastVertex(edge, true);
            const BRepAdaptor_Curve c(edge);
            const bool fwd = edge.Orientation() == TopAbs_FORWARD;
            check(c.Value(fwd ? c.FirstParameter() : c.LastParameter()).Distance(BRep_Tool::Pnt(a)) <=
                          spatial &&
                      c.Value(fwd ? c.LastParameter() : c.FirstParameter()).Distance(BRep_Tool::Pnt(b)) <=
                          spatial,
                  "free edge endpoint displaced from its vertex");
            const int first = vertices.Add(a), last = vertices.Add(b);
            check(outgoing.emplace(first, int(boundary.size())).second && arriving.insert(last).second,
                  "oriented free boundary branches");
            boundary.push_back({edge, i, first, last});
            owners.insert(i);
        }
    }
    check(int(boundary.size()) == expectedEdges && freeCounts(shell) == std::make_pair(expectedEdges, 2),
          "unexpected source frontier size");
    std::vector<bool> taken(boundary.size());
    std::vector<std::vector<int>> loops;
    for (size_t seed = 0; seed < boundary.size(); ++seed) {
        if (taken[seed])
            continue;
        int current = int(seed);
        std::vector<int> loop;
        do {
            check(!taken[current], "edge repeated in boundary walk");
            loop.push_back(current);
            taken[current] = true;
            const auto next = outgoing.find(boundary[current].last);
            check(next != outgoing.end(), "oriented boundary ends prematurely");
            current = next->second;
        } while (current != int(seed));
        loops.push_back(std::move(loop));
    }
    check(loops.size() == 2, "wrong number of oriented frontier cycles");
    // Canonical starting point per cycle and stable ordering across runs.
    auto vertex = [&](int i) {
        const auto p = BRep_Tool::Pnt(TopoDS::Vertex(vertices(boundary[i].first)));
        return std::array<double, 3>{p.X(), p.Y(), p.Z()};
    };
    auto less = [&](int a, int b) { return vertex(a) < vertex(b); };
    for (auto &l : loops)
        std::rotate(l.begin(), std::min_element(l.begin(), l.end(), less), l.end());
    std::sort(loops.begin(), loops.end(),
              [&](const auto &a, const auto &b) { return less(a.front(), b.front()); });
    const auto tmp = dir / "frontier.json.tmp";
    std::ofstream out(tmp);
    out << std::setprecision(17) << "{\"free_edges\":" << boundary.size() << ",\"faces\":[";
    bool first = true;
    for (int id : owners) {
        if (!first)
            out << ',';
        first = false;
        writeOwner(out, id, TopoDS::Face(faces(id)));
    }
    out << "],\"loops\":[";
    for (size_t i = 0; i < loops.size(); ++i) {
        if (i)
            out << ',';
        out << "{\"id\":" << i << ",\"edges\":[";
        for (size_t j = 0; j < loops[i].size(); ++j) {
            if (j)
                out << ',';
            writeBoundaryEdge(out, boundary[loops[i][j]]);
        }
        out << "]}";
    }
    out << "]}\n";
    out.close();
    check(out.good(), "cannot write free edge witness");
    std::filesystem::rename(tmp, dir / "frontier.json");
    std::cout << "{\"free_edges\":" << boundary.size() << ",\"free_loops\":2,\"loop_sizes\":["
              << loops[0].size() << ',' << loops[1].size() << "],\"boundary_faces\":" << owners.size()
              << "}\n";
}
} // namespace
