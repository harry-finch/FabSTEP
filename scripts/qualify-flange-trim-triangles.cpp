// Private OCCT check of single-triangle owners and their measured frontier joins.
#include "qualify-flange-shell.hxx"
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepAdaptor_Curve2d.hxx>
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <BRepTools.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_BSplineSurface.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <set>
#include <stdexcept>
#include <vector>

namespace trimtriangles {
constexpr double eps = 5e-8;
void check(bool good, const char *why) {
    if (!good)
        throw std::runtime_error(why);
}
struct Triangle {
    int owner, loop, index;
    gp_Pnt start, end, apex;
    double sourceBound;
    std::array<gp_Pnt, 3> source;
    TopoDS_Edge base;
};
gp_Pnt readPoint(std::istream &in) {
    double x, y, z;
    check(bool(in >> x >> y >> z) && std::isfinite(x) && std::isfinite(y) && std::isfinite(z), "invalid point");
    return {x, y, z};
}
std::vector<Triangle> readManifest(const std::filesystem::path &path) {
    check(std::filesystem::file_size(path) < 256 * 1024, "oversized manifest");
    std::ifstream in(path);
    int count = 0;
    check(bool(in >> count) && count == 267, "incorrect one-triangle count");
    std::vector<Triangle> triangles;
    std::set<int> owners;
    std::set<std::pair<int, int>> edges;
    for (int i = 0; i < count; ++i) {
        Triangle t{};
        check(bool(in >> t.owner >> t.loop >> t.index) && t.owner >= 0 && t.loop >= 0 && t.index >= 0 &&
                  owners.insert(t.owner).second && edges.emplace(t.loop, t.index).second,
              "invalid or duplicate owner or edge");
        t.start = readPoint(in);
        t.end = readPoint(in);
        t.apex = readPoint(in);
        check(bool(in >> t.sourceBound) && t.sourceBound >= 0 && t.sourceBound < .05,
              "invalid source fidelity bound");
        for (auto &p : t.source)
            p = readPoint(in);
        triangles.push_back(t);
    }
    in >> std::ws;
    check(in.eof(), "trailing manifest data");
    return triangles;
}
TopoDS_Shape readStep(const std::filesystem::path &path) {
    check(std::filesystem::file_size(path) < 64 * 1024 * 1024, "oversized local STEP");
    STEPControl_Reader reader;
    check(reader.ReadFile(path.c_str()) == IFSelect_RetDone && reader.TransferRoots() >= 1, "STEP read");
    return reader.OneShape();
}
TopoDS_Shape roundtrip(const TopoDS_Shape &shape, const std::filesystem::path &path) {
    STEPControl_Writer writer;
    check(writer.Transfer(shape, STEPControl_AsIs) == IFSelect_RetDone &&
              writer.Write(path.c_str()) == IFSelect_RetDone,
          "STEP write");
    return readStep(path);
}
void findBases(const TopoDS_Shape &shell, std::vector<Triangle> &triangles, bool withEnds = false) {
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(shell, TopAbs_FACE, faces);
    check(faces.Extent() == (withEnds ? 1816 : 1814) && BRepCheck_Analyzer(shell, true).IsValid(),
          "invalid input shell");
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shell, TopAbs_EDGE, TopAbs_FACE, adj);
    int free = 0;
    for (int j = 1; j <= faces.Extent(); ++j)
        for (TopExp_Explorer it(faces(j), TopAbs_EDGE); it.More(); it.Next()) {
            const auto edge = TopoDS::Edge(it.Current());
            if (adj.FindFromKey(edge).Extent() != 1)
                continue;
            ++free;
            const auto a = BRep_Tool::Pnt(TopExp::FirstVertex(edge, true)),
                       b = BRep_Tool::Pnt(TopExp::LastVertex(edge, true));
            for (auto &t : triangles)
                if (t.start.Distance(a) < eps && t.end.Distance(b) < eps) {
                    check(t.base.IsNull(), "ambiguous free STEP edge");
                    t.base = edge;
                }
        }
    check(free == (withEnds ? 1251 : 1259) &&
              std::all_of(triangles.begin(), triangles.end(), [](const auto &t) { return !t.base.IsNull(); }),
          "missing oriented free STEP edge");
}
bool from(const TopoDS_Edge &e, const gp_Pnt &p) {
    return BRep_Tool::Pnt(TopExp::FirstVertex(e, true)).Distance(p) < eps;
}
bool to(const TopoDS_Edge &e, const gp_Pnt &p) {
    return BRep_Tool::Pnt(TopExp::LastVertex(e, true)).Distance(p) < eps;
}
TopoDS_Face makeFace(const Triangle &t) {
    TopoDS_Face face;
    if (BRepAdaptor_Curve(t.base).GetType() == GeomAbs_Line) {
        BRepBuilderAPI_MakeWire wire;
        wire.Add(TopoDS::Edge(t.base.Reversed()));
        wire.Add(BRepBuilderAPI_MakeEdge(t.start, t.apex));
        wire.Add(BRepBuilderAPI_MakeEdge(t.apex, t.end));
        BRepBuilderAPI_MakeFace maker(wire.Wire(), true);
        check(maker.IsDone(), "planar triangle failed");
        face = maker.Face();
    } else {
        BRepOffsetAPI_ThruSections loft(false, true, eps);
        loft.CheckCompatibility(false);
        loft.SetMutableInput(false);
        loft.AddWire(BRepBuilderAPI_MakeWire(t.base).Wire());
        loft.AddVertex(BRepBuilderAPI_MakeVertex(t.apex).Vertex());
        loft.Build();
        check(loft.IsDone(), "ruled triangular face failed");
        TopExp_Explorer f(loft.Shape(), TopAbs_FACE);
        check(f.More(), "no ruled face");
        face = TopoDS::Face(f.Current());
        f.Next();
        check(!f.More(), "multiple loft faces");
    }
    for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
        const auto edge = TopoDS::Edge(e.Current());
        if (from(edge, t.end) && to(edge, t.start))
            return face;
        if (from(edge, t.start) && to(edge, t.end))
            return TopoDS::Face(face.Reversed());
    }
    throw std::runtime_error("missing loft base");
}
using Poles = std::array<gp_Pnt, 4>;
Poles cubicPoles(const Handle(Geom_BSplineCurve) & c, double lo, double hi) {
    check(c->Degree() == 3 && !c->IsRational() && c->NbKnots() == 2 && c->NbPoles() == 4 &&
              c->Multiplicity(1) == 4 && c->Multiplicity(2) == 4 && std::abs(lo - c->Knot(1)) < 1e-12 &&
              std::abs(hi - c->Knot(2)) < 1e-12,
          "not a full cubic Bezier span");
    return {c->Pole(1), c->Pole(2), c->Pole(3), c->Pole(4)};
}
double poleDifference(const Poles &a, Poles b) {
    if (a.front().Distance(b.back()) < a.front().Distance(b.front()))
        std::reverse(b.begin(), b.end());
    double drift = 0.;
    for (size_t i = 0; i < a.size(); ++i)
        drift = std::max(drift, a[i].Distance(b[i]));
    return drift;
}
double baseDifference(const BRepAdaptor_Curve &reference, const BRepAdaptor_Curve &c) {
    check(c.GetType() == reference.GetType(), "changed base curve type");
    if (c.GetType() == GeomAbs_BSplineCurve)
        return poleDifference(
            cubicPoles(reference.BSpline(), reference.FirstParameter(), reference.LastParameter()),
            cubicPoles(c.BSpline(), c.FirstParameter(), c.LastParameter()));
    // Two oriented endpoints identify the line. For a circular arc, compare its
    // start frame and signed derivative and bound the error over the full sweep.
    double a = c.FirstParameter(), b = c.LastParameter(), lo = reference.FirstParameter(),
           hi = reference.LastParameter();
    gp_Pnt p = reference.Value(lo);
    if (c.Value(b).Distance(p) < c.Value(a).Distance(p))
        std::swap(a, b);
    if (c.GetType() == GeomAbs_Line)
        return std::max(c.Value(a).Distance(p), c.Value(b).Distance(reference.Value(hi)));
    check(c.GetType() == GeomAbs_Circle, "unsupported input trim");
    const auto ref = reference.Circle(), circle = c.Circle();
    const gp_Vec radial(ref.Location(), p), radial2(circle.Location(), c.Value(a));
    auto tangent = gp_Vec(ref.Axis().Direction()).Crossed(radial);
    auto tangent2 = gp_Vec(circle.Axis().Direction()).Crossed(radial2);
    if (b < a)
        tangent2.Reverse();
    return ref.Location().Distance(circle.Location()) + (radial - radial2).Magnitude() +
           (tangent - tangent2).Magnitude() + ref.Radius() * std::abs(std::abs(b - a) - (hi - lo));
}
// The source triangle fixes the projection plane. A positive angular
// derivative in one half-plane means each ruling meets the boundary once.
// Bound the derivative by convex combinations of derivative control points.
bool increasingAngle(const Poles &poles, const gp_Pnt &apex, const gp_Vec &normal, double reserve,
                     int depth = 0) {
    bool positive = true;
    for (const auto &p : poles)
        for (int j = 0; j < 3; ++j) {
            const auto v = gp_Vec(poles[j], poles[j + 1]) * 3.;
            if (gp_Vec(apex, p).Crossed(v).Dot(normal) <= reserve)
                positive = false;
        }
    if (positive || depth == 8)
        return positive;
    auto mid = [](const gp_Pnt &a, const gp_Pnt &b) { return a.Translated(gp_Vec(a, b) * .5); };
    const auto a = mid(poles[0], poles[1]), b = mid(poles[1], poles[2]), c = mid(poles[2], poles[3]);
    const auto d = mid(a, b), e = mid(b, c), f = mid(d, e);
    return increasingAngle({poles[0], a, d, f}, apex, normal, reserve, depth + 1) &&
           increasingAngle({f, e, c, poles[3]}, apex, normal, reserve, depth + 1);
}
void verifyFan(const Poles &base, const gp_Pnt &apex, const Triangle &t) {
    const auto normal = gp_Vec(t.source[0], t.source[1]).Crossed(gp_Vec(t.source[0], t.source[2])).Normalized();
    auto flatten = [&](const gp_Pnt &p) {
        const gp_Vec v(apex, p);
        return (v - normal * v.Dot(normal)).Normalized();
    };
    const auto ray = (flatten(base.front()) + flatten(base.back())).Normalized();
    const double scale = std::max({1., apex.XYZ().Modulus(), t.start.XYZ().Modulus(), t.end.XYZ().Modulus()});
    const double reserve = 512 * std::numeric_limits<double>::epsilon() * scale * scale;
    for (const auto &p : base)
        check(gp_Vec(apex, p).Dot(ray) > reserve, "ambiguous angular support");
    check(increasingAngle(base, apex, normal, reserve), "native patch may fold against the source plane");
}
double ruledDifference(const Triangle &t, const BRepAdaptor_Surface &support, double &sourceDistance) {
    auto r = support.BSpline(); // BRepAdaptor_Surface returns a transformed copy.
    check(r->VDegree() == 1 && r->NbVKnots() == 2 && r->NbVPoles() == 2 && r->UDegree() <= 3 &&
              r->NbUKnots() == 2 && r->NbUPoles() == r->UDegree() + 1 && !r->IsURational() && !r->IsVRational(),
          "unexpected ruled control net");
    if (r->UDegree() < 3)
        r->IncreaseDegree(3, 1); // Exact Bernstein degree elevation.
    const int apex = r->Pole(1, 1).Distance(t.apex) < r->Pole(1, 2).Distance(t.apex) ? 1 : 2;
    Poles surfaceBase;
    double apexDrift = 0.;
    const gp_Pnt chartApex = r->Pole(1, apex);
    for (int j = 1; j <= 4; ++j) {
        check(r->Pole(j, apex).Distance(chartApex) == 0., "nonconstant apex control row");
        surfaceBase[j - 1] = r->Pole(j, 3 - apex);
        apexDrift = std::max(apexDrift, r->Pole(j, apex).Distance(t.apex));
    }
    check(apexDrift < eps, "ruled apex control row drift");
    if (surfaceBase.front().Distance(t.start) > surfaceBase.back().Distance(t.start))
        std::reverse(surfaceBase.begin(), surfaceBase.end());
    verifyFan(surfaceBase, chartApex, t);
    // Compare the chart directly to the original flat STL triangle. The linear
    // interpolation remainder for a vector-valued C2 curve is at most sup|C''|/8.
    // A cubic's second derivative is affine between its two endpoint values.
    const gp_Vec d0(surfaceBase[0], surfaceBase[1]), d1(surfaceBase[1], surfaceBase[2]),
        d2(surfaceBase[2], surfaceBase[3]);
    const double second = 6. * std::max((d1 - d0).Magnitude(), (d2 - d1).Magnitude());
    const double endpoint =
        std::max(surfaceBase[0].Distance(t.source[0]), surfaceBase[3].Distance(t.source[1]));
    sourceDistance = std::max(endpoint + second / 8., t.apex.Distance(t.source[2]) + apexDrift);
    const BRepAdaptor_Curve c(t.base);
    const double a = c.FirstParameter(), b = c.LastParameter();
    Poles expected;
    double remainder = 0.;
    if (c.GetType() == GeomAbs_BSplineCurve)
        expected = cubicPoles(c.BSpline(), a, b);
    else {
        check(c.GetType() == GeomAbs_Circle, "unsupported curved base");
        gp_Pnt p, q;
        gp_Vec first, last;
        c.D1(a, p, first);
        c.D1(b, q, last);
        expected = {p, p.Translated(first * ((b - a) / 3)), q.Translated(-last * ((b - a) / 3)), q};
        // Componentwise Hermite remainder, invariant under rotation of the circle.
        remainder = std::sqrt(3.) * c.Circle().Radius() * std::pow(b - a, 4) / 384.;
    }
    return std::max(apexDrift, poleDifference(expected, surfaceBase) + remainder);
}
double railDrift(const BRepAdaptor_Curve &curve, const gp_Pnt &a, const gp_Pnt &b) {
    gp_Pnt first, last;
    if (curve.GetType() == GeomAbs_Line) {
        first = curve.Value(curve.FirstParameter());
        last = curve.Value(curve.LastParameter());
    } else {
        check(curve.GetType() == GeomAbs_BSplineCurve, "curved lateral rail");
        const auto c = curve.BSpline();
        check(c->Degree() == 1 && c->NbPoles() == 2 && c->NbKnots() == 2 && !c->IsRational() &&
                  std::abs(curve.FirstParameter() - c->Knot(1)) < 1e-12 &&
                  std::abs(curve.LastParameter() - c->Knot(2)) < 1e-12,
              "wrong rail control net");
        first = c->Pole(1);
        last = c->Pole(2);
    }
    if (first.Distance(b) < first.Distance(a))
        std::swap(first, last);
    return std::max(first.Distance(a), last.Distance(b));
}
struct Metrics {
    int planes = 0, circles = 0, cubics = 0;
    double transfer = 0., combined = 0.;
};
void verifyFace(const TopoDS_Face &face, const Triangle &t, Metrics &m) {
    check(count(face, TopAbs_WIRE) == 1, "triangle needs exactly one boundary wire");
    const BRepAdaptor_Curve base(t.base);
    const BRepAdaptor_Surface support(face);
    const bool plane = base.GetType() == GeomAbs_Line;
    check(support.GetType() == (plane ? GeomAbs_Plane : GeomAbs_BSplineSurface), "wrong face support");
    int bottom = 0, left = 0, right = 0, apex = 0;
    double drift = 0., directSource = 0.;
    for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
        const auto e = TopoDS::Edge(it.Current());
        if (BRep_Tool::Degenerated(e)) {
            check(from(e, t.apex) && to(e, t.apex), "degenerate edge displaced");
            ++apex;
            continue;
        }
        BRepAdaptor_Curve c(e);
        if (from(e, t.end) && to(e, t.start)) {
            ++bottom;
            drift = std::max(drift, baseDifference(base, c));
        } else if (from(e, t.start) && to(e, t.apex)) {
            ++left;
            drift = std::max(drift, railDrift(c, t.start, t.apex));
        } else if (from(e, t.apex) && to(e, t.end)) {
            ++right;
            drift = std::max(drift, railDrift(c, t.apex, t.end));
        } else
            throw std::runtime_error("unexpected oriented triangle edge");
    }
    check(bottom == 1 && left == 1 && right == 1 && apex == (plane ? 0 : 1), "incomplete triangular boundary");
    if (plane) {
        ++m.planes;
        directSource = std::max(
            {t.start.Distance(t.source[0]), t.end.Distance(t.source[1]), t.apex.Distance(t.source[2])});
        for (const auto &p : {t.start, t.end, t.apex})
            drift = std::max(drift, support.Plane().Distance(p));
    } else {
        if (base.GetType() == GeomAbs_Circle)
            ++m.circles;
        else
            ++m.cubics;
        drift = std::max(drift, ruledDifference(t, support, directSource));
    }
    check(drift < eps, "continuous triangle transfer error exceeds tolerance");
    m.transfer = std::max(m.transfer, drift);
    m.combined = std::max(m.combined, directSource + drift);
    check(directSource + drift < .05, "local face exceeds source budget");
}
std::vector<gp_Pnt> vertices(const TopoDS_Face &face) {
    TopTools_IndexedMapOfShape map;
    TopExp::MapShapes(face, TopAbs_VERTEX, map);
    std::vector<gp_Pnt> pts;
    for (int i = 1; i <= map.Extent(); ++i)
        pts.push_back(BRep_Tool::Pnt(TopoDS::Vertex(map(i))));
    return pts;
}
int matchTriangle(const std::vector<gp_Pnt> &vs, const std::vector<Triangle> &triangles) {
    if (vs.size() != 3)
        return -1;
    int matched = -1;
    for (size_t i = 0; i < triangles.size(); ++i) {
        const auto &t = triangles[i];
        auto has = [&](const gp_Pnt &p) {
            return std::any_of(vs.begin(), vs.end(), [&](const auto &q) { return p.Distance(q) < eps; });
        };
        if (has(t.start) && has(t.end) && has(t.apex)) {
            check(matched < 0, "ambiguous triangle owner");
            matched = int(i);
        }
    }
    return matched;
}
Metrics verify(const TopoDS_Shape &shape, const std::vector<Triangle> &triangles) {
    check(BRepCheck_Analyzer(shape, true).IsValid(), "invalid triangle compound");
    TopExp_Explorer solids(shape, TopAbs_SOLID);
    check(!solids.More(), "unexpected solid");
    Metrics m;
    std::vector<int> seen(triangles.size());
    for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
        const auto f = TopoDS::Face(it.Current());
        const auto vs = vertices(f);
        int matched = matchTriangle(vs, triangles);
        check(matched >= 0, "unmatched triangle apex or base");
        ++seen[matched];
        verifyFace(f, triangles[matched], m);
    }
    check(std::all_of(seen.begin(), seen.end(), [](int n) { return n == 1; }),
          "incomplete one-triangle owner set");
    return m;
}
TopoDS_Compound build(const std::vector<Triangle> &triangles) {
    BRep_Builder builder;
    TopoDS_Compound result;
    builder.MakeCompound(result);
    for (const auto &t : triangles)
        builder.Add(result, makeFace(t));
    return result;
}
struct SourceUse {
    std::string kind;
    int peer;
};
using TriangleUses = std::vector<std::array<SourceUse, 3>>;
TriangleUses readUses(const char *path, const std::vector<Triangle> &triangles) {
    check(std::filesystem::file_size(path) < 256 * 1024, "oversized source incidence table");
    std::map<int, size_t> ids;
    for (size_t i = 0; i < triangles.size(); ++i)
        ids.emplace(triangles[i].owner, i);
    std::ifstream in(path);
    int count;
    check(bool(in >> count) && count == int(triangles.size() * 3), "incomplete source incidence table");
    TriangleUses result(triangles.size());
    constexpr int starts[3] = {1, 0, 2}, ends[3] = {0, 2, 1};
    for (int i = 0; i < count; ++i) {
        int owner, k, peer;
        std::string kind;
        check(bool(in >> owner >> k >> kind >> peer) && ids.count(owner) && k >= 0 && k < 3 &&
                  (kind == "existing" || kind == "added" || kind == "free") &&
                  (kind == "added") == bool(ids.count(peer)),
              "invalid source incidence");
        const auto a = readPoint(in), b = readPoint(in);
        const auto id = ids.at(owner);
        const auto &source = triangles[id].source;
        int side = -1;
        for (int j = 0; j < 3; ++j)
            if (a.IsEqual(source[starts[j]], 1e-12) && b.IsEqual(source[ends[j]], 1e-12))
                side = j;
        check(side >= 0 && (kind == "existing") == (side == 0) && result[id][side].kind.empty(),
              "source side mismatch");
        result[id][side] = {kind, peer};
    }
    in >> std::ws;
    check(in.eof(), "trailing neighbor data");
    return result;
}
// Original orientation and coedge identity distinguish two faces on the same
// support, and two arcs shared by the same owners.
struct OldUse {
    std::array<gp_Pnt, 5> points;
    std::vector<int> owners;
    GeomAbs_CurveType kind;
};
using FaceUses = std::vector<OldUse>;
FaceUses oldUses(const TopoDS_Face &face, const Adjacency &adj, const TopTools_IndexedMapOfShape &faces,
                 const std::vector<int> &labels) {
    FaceUses result;
    for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
        const auto e = TopoDS::Edge(it.Current());
        BRepAdaptor_Curve c(e);
        OldUse use{{}, {}, c.GetType()};
        for (int j = 0; j < 5; ++j) {
            const double t = .25 * (e.Orientation() == TopAbs_FORWARD ? j : 4 - j);
            use.points[j] = c.Value(c.FirstParameter() + t * (c.LastParameter() - c.FirstParameter()));
        }
        for (const auto &neighbor : adj.FindFromKey(e)) {
            const int id = labels[faces.FindIndex(neighbor)];
            if (id > 0)
                use.owners.push_back(id);
        }
        std::sort(use.owners.begin(), use.owners.end());
        result.push_back(std::move(use));
    }
    return result;
}
struct Baseline {
    std::vector<OldFace> faces;
    std::vector<FaceUses> uses;
    std::vector<int> carrier;
};
Baseline baseline(const TopoDS_Shape &prior, const std::vector<Triangle> &triangles) {
    Baseline result;
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(prior, TopAbs_FACE, faces);
    std::vector<int> ids(faces.Extent() + 1);
    std::iota(ids.begin(), ids.end(), 0);
    const auto adj = adjacency(prior);
    for (int i = 1; i <= faces.Extent(); ++i) {
        auto f = TopoDS::Face(faces(i));
        result.faces.push_back(signature(f));
        result.uses.push_back(oldUses(f, adj, faces, ids));
    }
    for (const auto &t : triangles)
        result.carrier.push_back(faces.FindIndex(adj.FindFromKey(t.base).First()));
    return result;
}
void preserved(const FaceUses &prior, const FaceUses &now) {
    check(prior.size() == now.size(), "old coedge count changed");
    std::vector<bool> matched(now.size());
    for (const auto &use : prior) {
        int hit = -1;
        for (size_t i = 0; i < now.size(); ++i) {
            if (matched[i] || now[i].owners != use.owners || now[i].kind != use.kind)
                continue;
            if (std::equal(use.points.begin(), use.points.end(), now[i].points.begin(),
                           [](const gp_Pnt &a, const gp_Pnt &b) { return a.Distance(b) < eps; })) {
                check(hit == -1, "ambiguous oriented old edge");
                hit = int(i);
            }
        }
        check(hit >= 0, "old curve, use or neighbor changed");
        matched[hit] = true;
    }
}
void checkPCurve(const TopoDS_Edge &edge, const TopoDS_Face &face) {
    const BRepAdaptor_Curve c(edge);
    const BRepAdaptor_Curve2d uv(edge, face);
    const BRepAdaptor_Surface surface(face);
    for (int j = 0; j <= 4; ++j) {
        const double t = j * .25;
        const auto p = uv.Value(uv.FirstParameter() + t * (uv.LastParameter() - uv.FirstParameter()));
        const auto q = c.Value(c.FirstParameter() + t * (c.LastParameter() - c.FirstParameter()));
        check(q.Distance(surface.Value(p.X(), p.Y())) < eps, "joined pcurve deviates from 3D edge");
    }
}
struct Joined {
    Metrics metrics;
    int bases = 0, inner = 0, exposed = 0, degenerate = 0, free = 0, loops = 0;
};
Joined verifyJoin(const TopoDS_Shape &shape, const Baseline &prior, const std::vector<Triangle> &triangles,
                  const TriangleUses &source) {
    check(BRepCheck_Analyzer(shape, true).IsValid() && count(shape, TopAbs_SHELL) == 1 &&
              count(shape, TopAbs_SOLID) == 0,
          "invalid sewn shell");
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(shape, TopAbs_FACE, faces);
    check(size_t(faces.Extent()) == prior.faces.size() + triangles.size(), "unexpected sewn face count");
    std::vector<int> labels(faces.Extent() + 1);
    std::vector<int> oldSeen(prior.faces.size()), newSeen(triangles.size());
    Joined result;
    for (int i = 1; i <= faces.Extent(); ++i) {
        const auto f = TopoDS::Face(faces(i));
        const int id = matchTriangle(vertices(f), triangles);
        if (id >= 0) {
            labels[i] = -1 - id;
            ++newSeen[id];
            verifyFace(f, triangles[id], result.metrics);
            continue;
        }
        const auto sig = signature(f);
        for (size_t j = 0; j < prior.faces.size(); ++j)
            if (sameOriginal(prior.faces[j], sig)) {
                check(labels[i] == 0, "ambiguous original face");
                labels[i] = int(j + 1);
                ++oldSeen[j];
            }
        check(labels[i] > 0, "old oriented face signature changed");
    }
    auto once = [](int n) { return n == 1; };
    check(std::all_of(oldSeen.begin(), oldSeen.end(), once) &&
              std::all_of(newSeen.begin(), newSeen.end(), once),
          "incomplete or duplicate sewn faces");
    const auto adj = adjacency(shape);
    for (int i = 1; i <= faces.Extent(); ++i) {
        const auto f = TopoDS::Face(faces(i));
        const int label = labels[i];
        if (label > 0) {
            preserved(prior.uses[label - 1], oldUses(f, adj, faces, labels));
            continue;
        }
        const size_t id = -label - 1;
        const auto &t = triangles[id];
        for (TopExp_Explorer it(f, TopAbs_EDGE); it.More(); it.Next()) {
            const auto e = TopoDS::Edge(it.Current());
            const auto &uses = adj.FindFromKey(e);
            if (BRep_Tool::Degenerated(e)) {
                check(uses.Extent() == 1 && from(e, t.apex) && to(e, t.apex), "joined degenerate apex");
                ++result.degenerate;
                continue;
            }
            checkPCurve(e, f);
            int side = from(e, t.end) && to(e, t.start) ? 0 : from(e, t.start) && to(e, t.apex) ? 1 : 2;
            const auto &use = source[id][side];
            if (use.kind == "free") {
                check(uses.Extent() == 1, "unexpected lateral contact");
                ++result.exposed;
                continue;
            }
            check(uses.Extent() == 2, "missing source contact");
            const auto other = TopoDS::Face(uses.First().IsSame(f) ? uses.Last() : uses.First());
            int neighbor = labels[faces.FindIndex(other)];
            if (use.kind == "existing") {
                check(neighbor == prior.carrier[id], "base joined the wrong original face");
                ++result.bases;
            } else {
                check(neighbor < 0 && triangles[-1 - neighbor].owner == use.peer, "lateral owner mismatch");
                ++result.inner;
            }
            int opposite = 0;
            for (TopExp_Explorer oe(other, TopAbs_EDGE); oe.More(); oe.Next())
                if (oe.Current().IsSame(e) && oe.Current().Orientation() != e.Orientation())
                    ++opposite;
            check(opposite == 1, "unopposed triangle join");
            checkPCurve(e, other);
        }
    }
    // Degenerate apex coedges carry no physical contour. Count the remaining
    // oriented cycles; every boundary vertex needs exactly one exit and entry.
    TopTools_IndexedMapOfShape verts;
    std::map<int, int> next;
    std::set<int> arrivals;
    for (int i = 1; i <= faces.Extent(); ++i)
        for (TopExp_Explorer it(faces(i), TopAbs_EDGE); it.More(); it.Next()) {
            const auto e = TopoDS::Edge(it.Current());
            if (adj.FindFromKey(e).Extent() != 1 || BRep_Tool::Degenerated(e))
                continue;
            const int a = verts.Add(TopExp::FirstVertex(e, true)), b = verts.Add(TopExp::LastVertex(e, true));
            check(next.emplace(a, b).second && arrivals.insert(b).second, "branched physical boundary");
            ++result.free;
        }
    std::set<int> visited;
    for (auto [a, b] : next)
        if (!visited.count(a)) {
            ++result.loops;
            int p = a;
            do {
                check(visited.insert(p).second && next.count(p), "unclosed physical boundary");
                p = next.at(p);
            } while (p != a);
        }
    check(result.bases == 267 && result.inner == 324 && result.exposed == 210 && result.degenerate == 264 &&
              result.free == 1251 - result.bases + result.exposed,
          "unexpected sewn graph");
    return result;
}
int qualifyJoin(const char *input, const char *manifest, const char *uses, const std::filesystem::path &dir) {
    auto triangles = readManifest(manifest);
    const auto prior = readStep(input);
    findBases(prior, triangles, true);
    const auto ref = baseline(prior, triangles);
    const auto source = readUses(uses, triangles);
    BRepBuilderAPI_Sewing sew(eps);
    sew.Add(prior);
    sew.Add(build(triangles));
    sew.Perform();
    const auto joined = sew.SewedShape();
    auto before = verifyJoin(joined, ref, triangles, source);
    const auto imported = roundtrip(joined, dir / "trim-shell.step");
    auto after = verifyJoin(imported, ref, triangles, source);
    auto wrong = source;
    bool replaced = false;
    for (auto &sides : wrong)
        for (auto &side : sides)
            if (!replaced && side.kind == "added") {
                side.peer = -1;
                replaced = true;
            }
    bool refused = false;
    try {
        verifyJoin(imported, ref, triangles, wrong);
    } catch (const std::runtime_error &e) {
        refused = std::string(e.what()) == "lateral owner mismatch";
    }
    check(refused, "wrong source owner accepted");
    std::ofstream out(dir / "report.json");
    out << std::setprecision(16) << "{\"input_faces\":" << ref.faces.size()
        << ",\"faces\":" << count(imported, TopAbs_FACE) << ",\"solids\":0,\"shared_bases\":" << after.bases
        << ",\"paired_lateral_edges\":" << after.inner / 2 << ",\"new_free_lateral_edges\":" << after.exposed
        << ",\"degenerate_apices\":" << after.degenerate << ",\"free_material_edges\":" << after.free
        << ",\"free_material_loops\":" << after.loops
        << ",\"max_native_transfer_bound_mm\":" << std::max(before.metrics.transfer, after.metrics.transfer)
        << ",\"max_bound_with_source_mm\":" << std::max(before.metrics.combined, after.metrics.combined)
        << ",\"wrong_source_owner_rejected\":true}\n";
    out.close();
    check(out.good(), "cannot write sewn shell report");
    return 0;
}

int run(int argc, char **argv) {
    try {
        if (argc == 6 && std::string(argv[1]) == "--join")
            return qualifyJoin(argv[2], argv[3], argv[4], argv[5]);
        check(argc == 4, "use QUALIFIED_STEP MEASUREMENTS.txt OUTPUT_DIR");
        const std::filesystem::path dir = argv[3];
        auto triangles = readManifest(argv[2]);
        findBases(readStep(argv[1]), triangles);
        const auto patches = build(triangles);
        const auto before = verify(patches, triangles);
        const auto after = verify(roundtrip(patches, dir / "trim-triangles.step"), triangles);
        auto shifted = triangles;
        shifted.front().apex.Translate(gp_Vec(.01, 0, 0));
        const auto bad = roundtrip(build(shifted), dir / "shifted-trim-apex.step");
        bool rejected = false;
        try {
            verify(bad, triangles);
        } catch (const std::runtime_error &e) {
            rejected = std::string(e.what()) == "unmatched triangle apex or base";
        }
        check(rejected, "shifted apex accepted or unexpected rejection");
        std::ofstream out(dir / "report.json");
        out << std::setprecision(16) << "{\"isolated_faces\":" << triangles.size()
            << ",\"planes\":" << after.planes << ",\"arc_ruled\":" << after.circles
            << ",\"cubic_ruled\":" << after.cubics << ",\"solids\":0,"
            << "\"max_native_transfer_bound_mm\":" << std::max(before.transfer, after.transfer)
            << ",\"max_bound_with_source_mm\":" << std::max(before.combined, after.combined)
            << ",\"shifted_apex_rejected\":true}\n";
        out.close();
        check(out.good(), "cannot write triangle report");
        return 0;
    } catch (const Standard_Failure &e) {
        std::cerr << e.GetMessageString() << '\n';
        return 1;
    } catch (const std::exception &e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}

} // namespace trimtriangles
int main(int argc, char **argv) { return trimtriangles::run(argc, argv); }
