// Private open-shell experiment. Coordinates are measured in the YZ plane.
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Vertex.hxx>
#include <TopoDS_Wire.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <vector>

namespace {
constexpr double eps = 5e-8, pi = 3.14159265358979323846;
void check(bool valid, const char *why) {
    if (!valid)
        throw std::runtime_error(why);
}
struct Ring {
    gp_Pnt center;
    double radius;
};
struct Segment {
    bool arc, shared;
    gp_Pnt start, end, center;
    double sweep;
};
struct Shoulder {
    int owner;
    double x, area;
    std::array<Ring, 2> rings;
    std::vector<Segment> outer;
};
using Shoulders = std::array<Shoulder, 2>;

Shoulders readShoulders(std::istream &in) {
    Shoulders result{};
    for (auto &s : result) {
        int n;
        check(bool(in >> s.owner >> s.x >> s.area >> n) && std::isfinite(s.x) && n >= 5 && n <= 4096 &&
                  s.area > 0,
              "invalid shoulder profile header");
        for (auto &r : s.rings) {
            double y, z;
            check(bool(in >> y >> z >> r.radius) && r.radius > 0, "invalid shoulder ring");
            r.center = gp_Pnt(s.x, y, z);
        }
        double integral = 0.;
        int arcs = 0, rails = 0;
        for (int i = 0; i < n; ++i) {
            int kind, shared;
            double y, z, yy, zz, cy, cz, sweep;
            check(bool(in >> kind >> shared >> y >> z >> yy >> zz >> cy >> cz >> sweep) &&
                      (kind == 0 || kind == 1) && (shared == 0 || shared == 1),
                  "invalid measured shoulder edge");
            Segment e{kind == 1, shared == 1, {s.x, y, z}, {s.x, yy, zz}, {s.x, cy, cz}, sweep};
            check(e.start.Distance(e.end) > eps, "degenerate shoulder edge");
            integral += y * zz - yy * z;
            if (e.arc) {
                const double r = e.center.Distance(e.start), a = std::atan2(z - cz, y - cy) + sweep;
                check(r > eps && std::abs(sweep) < 2 * pi && std::abs(sweep) > 0 &&
                          gp_Pnt(s.x, cy + r * std::cos(a), cz + r * std::sin(a)).Distance(e.end) < eps,
                      "unproved circular trim");
                integral += r * r * (sweep - std::sin(sweep));
            } else
                check(sweep == 0, "nonzero line sweep");
            if (e.shared) {
                if (e.arc)
                    ++arcs;
                else
                    ++rails;
            }
            s.outer.push_back(e);
        }
        check(arcs == 2 && rails == 3, "unproved external shoulder joins");
        for (size_t i = 0; i < s.outer.size(); ++i)
            check(s.outer[i].end.Distance(s.outer[(i + 1) % s.outer.size()].start) < eps,
                  "discontinuous shoulder wire");
        double area = std::abs(integral) * .5;
        for (auto &r : s.rings)
            area -= pi * r.radius * r.radius;
        check(std::abs(area - s.area) < 1e-7, "uncertified shoulder area");
        if (integral < 0.) {
            std::reverse(s.outer.begin(), s.outer.end());
            for (auto &e : s.outer) {
                std::swap(e.start, e.end);
                e.sweep = -e.sweep;
            }
        }
    }
    in >> std::ws;
    check(in.eof() && result[0].owner != result[1].owner, "extra or duplicate shoulder data");
    return result;
}
TopoDS_Shape readStep(const std::filesystem::path &path) {
    check(std::filesystem::file_size(path) < 64 * 1024 * 1024, "oversized local STEP fixture");
    STEPControl_Reader r;
    check(r.ReadFile(path.c_str()) == IFSelect_RetDone && r.TransferRoots() == 1, "STEP read");
    return r.OneShape();
}
TopoDS_Shape roundtrip(const TopoDS_Shape &shape, const std::filesystem::path &path) {
    STEPControl_Writer w;
    check(w.Transfer(shape, STEPControl_ShellBasedSurfaceModel) == IFSelect_RetDone &&
              w.Write(path.c_str()) == IFSelect_RetDone,
          "STEP write");
    return readStep(path);
}
TopoDS_Edge makeEdge(const Segment &e) {
    if (!e.arc)
        return BRepBuilderAPI_MakeEdge(e.start, e.end).Edge();
    const gp_Ax2 frame(e.center, e.sweep > 0 ? gp::DX() : -gp::DX(), gp_Dir(gp_Vec(e.center, e.start)));
    return BRepBuilderAPI_MakeEdge(gp_Circ(frame, e.center.Distance(e.start)), 0., std::abs(e.sweep)).Edge();
}
TopoDS_Face makeFace(const Shoulder &s) {
    BRepBuilderAPI_MakeWire wire;
    for (const auto &edge : s.outer)
        wire.Add(makeEdge(edge));
    check(wire.IsDone(), "cannot build shoulder contour");
    BRepBuilderAPI_MakeFace face(gp_Pln(gp_Ax3(gp_Pnt(s.x, 0., 0.), gp::DX(), gp::DY())), wire.Wire(), true);
    for (const auto &ring : s.rings) {
        const gp_Circ circle(gp_Ax2(ring.center, -gp::DX(), gp::DY()), ring.radius);
        face.Add(BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(circle).Edge()).Wire());
    }
    check(face.IsDone() && BRepCheck_Analyzer(face.Face(), true).IsValid(), "invalid shoulder face");
    return face.Face();
}
TopoDS_Shape sew(const TopoDS_Shape &input, const Shoulders &shoulders) {
    BRepBuilderAPI_Sewing sewing(eps);
    sewing.Add(input);
    for (const auto &s : shoulders)
        sewing.Add(makeFace(s));
    sewing.Perform();
    return sewing.SewedShape();
}
struct Signature {
    GeomAbs_SurfaceType type;
    gp_Pnt center;
    double area;
    int wires;
};
int count(const TopoDS_Shape &shape, TopAbs_ShapeEnum kind) {
    TopTools_IndexedMapOfShape map;
    TopExp::MapShapes(shape, kind, map);
    return map.Extent();
}
Signature signature(const TopoDS_Face &face) {
    GProp_GProps props;
    BRepGProp::SurfaceProperties(face, props, 1e-12);
    return {BRepAdaptor_Surface(face).GetType(), props.CentreOfMass(), props.Mass(),
            count(face, TopAbs_WIRE)};
}
bool same(const Signature &a, const Signature &b) {
    return a.type == b.type && a.wires == b.wires && a.center.Distance(b.center) < eps &&
           std::abs(a.area - b.area) < 1e-6;
}
// Each original coedge is compared including the order of its physical
// points on the oriented face and identities of adjacent original faces.
struct Curve {
    GeomAbs_CurveType type;
    std::array<gp_Pnt, 5> points;
    std::vector<int> uses;
    double radius = 0, span = 0;
    gp_Pnt center;
    gp_Dir axis;
};
Curve describe(const TopoDS_Edge &edge) {
    BRepAdaptor_Curve c(edge);
    Curve result{};
    result.type = c.GetType();
    const double lo = c.FirstParameter(), hi = c.LastParameter();
    for (int i = 0; i < 5; ++i) {
        const double t = (edge.Orientation() == TopAbs_REVERSED ? 4 - i : i) * .25;
        result.points[i] = c.Value(lo + (hi - lo) * t);
    }
    if (result.type == GeomAbs_Circle) {
        result.radius = c.Circle().Radius();
        result.center = c.Circle().Location();
        result.axis = c.Circle().Axis().Direction();
        result.span = hi - lo;
    }
    return result;
}
bool onRing(const Curve &c, const Ring &r) {
    return c.type == GeomAbs_Circle && c.center.Distance(r.center) < eps &&
           std::abs(c.radius - r.radius) < eps && c.axis.IsParallel(gp::DX(), eps / r.radius) &&
           std::abs(c.span - 2 * pi) * r.radius < eps;
}
bool sameCircle(const Curve &a, const Curve &b) {
    return a.center.Distance(b.center) < eps && std::abs(a.radius - b.radius) < eps &&
           a.axis.IsParallel(b.axis, eps / a.radius) && std::abs(a.span - b.span) * a.radius < eps;
}
// Subdivision is allowed only for a previously free line bearing a measured
// shoulder rail. Compare the entire oriented parameter interval in that case.
bool interval(const Curve &ref, const Curve &c, bool split, double &start, double &end) {
    if (ref.type != c.type)
        return false;
    if (split && ref.type == GeomAbs_Line) {
        const auto &origin = ref.points.front();
        gp_Vec delta(origin, ref.points.back());
        const double length = delta.Magnitude();
        delta.Normalize();
        start = gp_Vec(origin, c.points.front()).Dot(delta);
        end = gp_Vec(origin, c.points.back()).Dot(delta);
        if (start < -eps || end > length + eps || end <= start)
            return false;
        return origin.Translated(delta * start).Distance(c.points.front()) < eps &&
               origin.Translated(delta * end).Distance(c.points.back()) < eps;
    }
    if (ref.type == GeomAbs_Circle && !sameCircle(ref, c))
        return false;
    for (int i = 0; i < 5; ++i)
        if (ref.points[i].Distance(c.points[i]) >= eps)
            return false;
    start = 0.;
    end = ref.points.front().Distance(ref.points.back());
    return true;
}
using Adj = TopTools_IndexedDataMapOfShapeListOfShape;
struct FaceInfo {
    Signature sig;
    std::vector<Curve> edges;
};
struct Model {
    std::vector<FaceInfo> faces;
    int free = 0, freeLoops = 0;
};
Model describe(const TopoDS_Shape &shape) {
    check(count(shape, TopAbs_SOLID) == 0 && count(shape, TopAbs_SHELL) == 1 &&
              BRepCheck_Analyzer(shape, true).IsValid(),
          "invalid open shell");
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(shape, TopAbs_FACE, faces);
    Adj adj;
    TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, adj);
    Model model;
    TopTools_IndexedMapOfShape vertices;
    std::vector<std::vector<int>> neighbors;
    for (int i = 1; i <= adj.Extent(); ++i)
        if (adj(i).Extent() == 1) {
            ++model.free;
            const auto edge = TopoDS::Edge(adj.FindKey(i));
            const int a = vertices.Add(TopExp::FirstVertex(edge)) - 1,
                      b = vertices.Add(TopExp::LastVertex(edge)) - 1;
            neighbors.resize(vertices.Extent());
            neighbors[a].push_back(b);
            neighbors[b].push_back(a);
        }
    std::vector<bool> visited(neighbors.size());
    for (size_t i = 0; i < neighbors.size(); ++i) {
        check(neighbors[i].size() == 2, "free boundary is not a cycle");
        if (visited[i])
            continue;
        ++model.freeLoops;
        std::vector<int> todo{int(i)};
        visited[i] = true;
        while (!todo.empty()) {
            const int point = todo.back();
            todo.pop_back();
            for (int near : neighbors[point])
                if (!visited[near]) {
                    visited[near] = true;
                    todo.push_back(near);
                }
        }
    }
    for (int i = 1; i <= faces.Extent(); ++i) {
        const auto f = TopoDS::Face(faces(i));
        FaceInfo info{signature(f), {}};
        for (TopExp_Explorer it(f, TopAbs_EDGE); it.More(); it.Next()) {
            auto edge = TopoDS::Edge(it.Current());
            auto c = describe(edge);
            for (const auto &q : adj.FindFromKey(edge))
                c.uses.push_back(faces.FindIndex(q) - 1);
            check(!c.uses.empty() && c.uses.size() <= 2, "unexpected edge incidence");
            info.edges.push_back(std::move(c));
        }
        model.faces.push_back(std::move(info));
    }
    return model;
}
bool supportsRail(const Curve &ref, const Shoulders &shoulders) {
    if (ref.type != GeomAbs_Line || ref.uses.size() != 1)
        return false;
    for (const auto &shoulder : shoulders)
        for (const auto &e : shoulder.outer)
            if (!e.arc && e.shared) {
                Curve c{};
                c.type = GeomAbs_Line;
                c.points.front() = e.start;
                c.points.back() = e.end;
                double a, b;
                if (interval(ref, c, true, a, b))
                    return true;
                std::swap(c.points.front(), c.points.back());
                if (interval(ref, c, true, a, b))
                    return true;
            }
    return false;
}
int verifyOriginal(const FaceInfo &old, const FaceInfo &current, const std::vector<int> &labels,
                   const Shoulders &shoulders) {
    using Intervals = std::vector<std::pair<double, double>>;
    std::vector<Intervals> segments(old.edges.size());
    std::vector<bool> split;
    for (const auto &e : old.edges)
        split.push_back(supportsRail(e, shoulders));
    for (const auto &e : current.edges) {
        std::vector<int> owners;
        for (auto index : e.uses)
            if (labels[index] >= 0)
                owners.push_back(labels[index]);
        std::sort(owners.begin(), owners.end());
        int hit = -1;
        double first = 0., last = 0.;
        for (size_t i = 0; i < old.edges.size(); ++i) {
            const auto &o = old.edges[i];
            double a, b;
            if (!interval(o, e, split[i], a, b))
                continue;
            auto originalOwners = o.uses;
            std::sort(originalOwners.begin(), originalOwners.end());
            if (owners != originalOwners)
                continue;
            check(hit < 0, "ambiguous original coedge");
            hit = int(i);
            first = a;
            last = b;
        }
        check(hit >= 0, "unmatched original oriented coedge or neighbor");
        segments[hit].emplace_back(first, last);
    }
    int subdivisions = 0;
    for (size_t i = 0; i < segments.size(); ++i) {
        auto &spans = segments[i];
        check(!spans.empty(), "missing original curve");
        if (!split[i]) {
            check(spans.size() == 1, "unexpected original edge subdivision");
            continue;
        }
        check(spans.size() == 1 || spans.size() == 3, "a shoulder rail was only partly split");
        std::sort(spans.begin(), spans.end());
        double last = 0.;
        for (auto span : spans) {
            check(std::abs(last - span.first) < eps, "noncontiguous shoulder rail");
            last = span.second;
        }
        check(std::abs(last - old.edges[i].points.front().Distance(old.edges[i].points.back())) < eps,
              "original rail extent changed");
        subdivisions += int(spans.size()) - 1;
    }
    return subdivisions;
}
struct Stats {
    int subdivisions = 0, rings = 0, rails = 0, arcs = 0;
    double areaError = 0.;
};
void verifyShoulder(const TopoDS_Face &face, const FaceInfo &info, const Shoulder &s,
                    const std::vector<int> &labels, const Model &original, Stats &stats) {
    const BRepAdaptor_Surface support(face);
    check(support.GetType() == GeomAbs_Plane && info.sig.wires == 3 &&
              std::abs(support.Plane().Location().X() - s.x) < eps,
          "wrong shoulder support");
    auto dir = support.Plane().Axis().Direction();
    if (face.Orientation() == TopAbs_REVERSED)
        dir.Reverse();
    check(dir.IsEqual(gp::DX(), eps / 100), "shoulder normal reversed");
    std::vector<Curve> boundary;
    for (const auto &trim : s.outer)
        boundary.push_back(describe(makeEdge(trim)));
    std::vector<int> matched(s.outer.size());
    std::array<int, 2> holes{};
    for (const auto &e : info.edges) {
        int neighbor = -1;
        for (int index : e.uses)
            if (labels[index] >= 0)
                neighbor = labels[index];
        check((e.uses.size() == 1 && neighbor < 0) || (e.uses.size() == 2 && neighbor >= 0),
              "unexpected shoulder neighbor");
        bool ring = false;
        for (size_t j = 0; j < s.rings.size(); ++j)
            if (onRing(e, s.rings[j])) {
                check(!ring && neighbor >= 0 && original.faces[neighbor].sig.type == GeomAbs_Cone,
                      "missing conical shoulder neighbor");
                const gp_Vec radial(s.rings[j].center, e.points[0]), quarter(s.rings[j].center, e.points[1]);
                check(radial.Crossed(quarter).Dot(gp_Vec(gp::DX())) < 0., "shoulder hole winding");
                ++holes[j];
                ++stats.rings;
                ring = true;
            }
        if (ring)
            continue;
        int hit = -1;
        for (size_t i = 0; i < s.outer.size(); ++i) {
            double a, b;
            if (!interval(boundary[i], e, false, a, b))
                continue;
            check(hit < 0, "ambiguous shoulder edge");
            hit = int(i);
        }
        check(hit >= 0, "unmatched shoulder trim");
        const auto &trim = s.outer[hit];
        check(trim.shared == (neighbor >= 0), "measured trim incidence mismatch");
        if (trim.shared) {
            if (trim.arc)
                ++stats.arcs;
            else
                ++stats.rails;
        }
        ++matched[hit];
    }
    check(std::all_of(matched.begin(), matched.end(), [](int n) { return n == 1; }) &&
              holes == std::array<int, 2>{1, 1},
          "incomplete shoulder boundary");
    const double error = std::abs(info.sig.area - s.area);
    check(error < 1e-7, "shoulder area loss");
    stats.areaError = std::max(stats.areaError, error);
}
Stats verify(const TopoDS_Shape &shape, const Model &original, const Shoulders &shoulders) {
    const auto model = describe(shape);
    check(model.faces.size() == original.faces.size() + 2, "wrong shoulder face count");
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(shape, TopAbs_FACE, faces);
    std::vector<int> labels(model.faces.size(), -3), seen(original.faces.size());
    std::array<int, 2> added{};
    for (size_t i = 0; i < model.faces.size(); ++i) {
        const auto &face = model.faces[i];
        for (size_t j = 0; j < original.faces.size(); ++j)
            if (same(original.faces[j].sig, face.sig)) {
                check(labels[i] == -3, "ambiguous existing face");
                labels[i] = int(j);
                ++seen[j];
            }
        if (labels[i] != -3)
            continue;
        for (int j = 0; j < 2; ++j) {
            const auto &r = shoulders[j].rings;
            if (std::any_of(face.edges.begin(), face.edges.end(),
                            [&](const Curve &e) { return onRing(e, r[0]) || onRing(e, r[1]); })) {
                check(labels[i] == -3, "ambiguous shoulder");
                labels[i] = -j - 1;
                ++added[j];
            }
        }
        check(labels[i] != -3, "unexpected new face");
    }
    check(std::all_of(seen.begin(), seen.end(), [](int n) { return n == 1; }) &&
              added == std::array<int, 2>{1, 1},
          "incomplete original/added faces");
    Stats stats;
    for (size_t i = 0; i < labels.size(); ++i) {
        if (labels[i] >= 0)
            stats.subdivisions +=
                verifyOriginal(original.faces[labels[i]], model.faces[i], labels, shoulders);
        else
            verifyShoulder(TopoDS::Face(faces(int(i) + 1)), model.faces[i], shoulders[-1 - labels[i]], labels,
                           original, stats);
    }
    // Explicitly compare both coedge orientations at the fourteen new interfaces.
    Adj adj;
    TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, adj);
    for (int i = 1; i <= adj.Extent(); ++i) {
        const auto &owners = adj(i);
        if (owners.Extent() != 2)
            continue;
        if (labels[faces.FindIndex(owners.First()) - 1] >= 0 &&
            labels[faces.FindIndex(owners.Last()) - 1] >= 0)
            continue;
        int sum = 0, uses = 0;
        for (const auto &face : owners)
            for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next())
                if (it.Current().IsSame(adj.FindKey(i))) {
                    ++uses;
                    sum += it.Current().Orientation() == TopAbs_FORWARD ? 1 : -1;
                }
        check(uses == 2 && sum == 0, "uncancelled shared shoulder coedges");
    }
    check(stats.subdivisions == 2 && stats.rings == 4 && stats.arcs == 4 && stats.rails == 6,
          "unexpected shoulder split or incidence count");
    const int addedFree = int(shoulders[0].outer.size() + shoulders[1].outer.size()) - 10;
    check(model.free == original.free + stats.subdivisions - 14 + addedFree,
          "unexpected free boundary count");
    return stats;
}
} // namespace

int main(int argc, char **argv) {
    try {
        check(argc == 4, "use INPUT.step MEASUREMENTS.txt OUTPUT_DIR");
        const std::filesystem::path dir = argv[3];
        std::ifstream input(argv[2]);
        const auto shoulders = readShoulders(input);
        const auto initial = readStep(argv[1]);
        const auto reference = describe(initial);
        const auto joined = sew(initial, shoulders);
        const auto before = verify(joined, reference, shoulders);
        const auto result = roundtrip(joined, dir / "shoulders.step");
        const auto after = verify(result, reference, shoulders);
        auto wrong = shoulders;
        wrong.front().rings.front().radius -= .05;
        const auto negative = roundtrip(sew(initial, wrong), dir / "undersized-hole.step");
        bool rejected = false;
        try {
            (void)verify(negative, reference, shoulders);
        } catch (const std::runtime_error &e) {
            rejected = std::string(e.what()) == "unmatched shoulder trim";
        }
        check(rejected, "wrong shoulder ring accepted or unexpected negative control");
        const auto finalModel = describe(result);
        std::cout << std::setprecision(16) << "{\"input_faces\":" << reference.faces.size()
                  << ",\"faces\":" << count(result, TopAbs_FACE)
                  << ",\"shells\":1,\"solids\":0,\"shared_cone_rings\":" << after.rings
                  << ",\"shared_arcs\":" << after.arcs << ",\"shared_rails\":" << after.rails
                  << ",\"extra_subdivision_edges\":" << after.subdivisions
                  << ",\"free_edges\":" << finalModel.free << ",\"free_loops\":" << finalModel.freeLoops
                  << ",\"max_shoulder_area_error_mm2\":" << std::max(before.areaError, after.areaError)
                  << ",\"undersized_hole_rejected\":true}\n";
    } catch (const Standard_Failure &e) {
        std::cerr << e.GetMessageString() << '\n';
        return 1;
    } catch (const std::exception &e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
