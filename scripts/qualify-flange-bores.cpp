// Join five measured chains of analytic faces to the five existing free cone rings.
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Curve2d.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <gp_Pln.hxx>
#include <gp_Cone.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRep_Tool.hxx>
#include <GProp_GProps.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>
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
constexpr double eps = 5.e-8, pi = 3.14159265358979323846;
void check(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}

struct Ring {
    gp_Pnt center;
    double radius;
};
struct Wall {
    std::string kind;
    int previous, inlet;
    bool inner;
    gp_Pnt center;
    double major, minor, low, high;
    std::array<Ring, 2> rims;
    std::vector<gp_Pnt> polygon;
    double netArea = 0.;
    bool planar() const { return kind == "plane" || kind == "collar"; }
    GeomAbs_SurfaceType type() const {
        if (planar())
            return GeomAbs_Plane;
        if (kind == "cone")
            return GeomAbs_Cone;
        if (kind == "torus")
            return GeomAbs_Torus;
        return GeomAbs_Cylinder;
    }
    double area() const {
        auto ra = rims[0].radius, rb = rims[1].radius;
        if (kind == "collar")
            return netArea;
        if (kind == "plane")
            return pi * (rb * rb - ra * ra);
        if (kind == "torus")
            return 2 * pi * minor * (major * (high - low) + minor * (std::sin(high) - std::sin(low)));
        return pi * (ra + rb) * std::hypot(high - low, rb - ra);
    }
};
using Walls = std::vector<Wall>;
gp_Pnt readPoint(std::istream &s) {
    double x, y, z;
    check(bool(s >> x >> y >> z) && std::isfinite(x) && std::isfinite(y) && std::isfinite(z),
          "invalid point");
    return {x, y, z};
}
Walls readWalls(std::istream &stream) {
    int n = 0;
    check(bool(stream >> n) && n == 22, "wrong chain size");
    Walls result(n);
    for (int i = 0; i < n; ++i) {
        auto &w = result[i];
        int inner;
        check(bool(stream >> w.kind >> w.previous >> w.inlet >> inner) && (inner == 0 || inner == 1) &&
                  (w.inlet == 0 || w.inlet == 1) && w.previous >= -1 && w.previous < i,
              "invalid chain data");
        check(w.kind == "cylinder" || w.kind == "cone" || w.kind == "torus" || w.kind == "plane",
              "unknown surface");
        w.inner = inner == 1;
        w.center = readPoint(stream);
        check(bool(stream >> w.major >> w.minor >> w.low >> w.high) && w.major > 0 && w.minor >= 0 &&
                  w.low < w.high,
              "wrong domain");
        for (auto &r : w.rims) {
            r.center = readPoint(stream);
            check(bool(stream >> r.radius) && std::isfinite(r.radius) && r.radius > 0, "bad ring");
        }
        check(std::isfinite(w.area()) && w.area() > 0, "invalid area");
    }
    stream >> std::ws;
    check(stream.eof(), "trailing chain data");
    return result;
}
void appendCollar(Walls &walls, const char *path) {
    std::ifstream s(path);
    Wall lip{};
    lip.kind = "collar";
    lip.inlet = 0;
    lip.inner = true;
    lip.center = readPoint(s);
    int n = 0;
    check(bool(s >> lip.rims[0].radius >> lip.netArea >> n) && n >= 3 && n <= 4096 && lip.netArea > 0,
          "bad collar");
    lip.rims[0].center = lip.center;
    lip.previous = -1;
    for (size_t i = 0; i < walls.size(); ++i) {
        const auto &w = walls[i];
        const auto &end = w.rims[1 - w.inlet];
        if (w.kind == "torus" && end.center.Distance(lip.center) < eps &&
            std::abs(end.radius - lip.rims[0].radius) < eps) {
            check(lip.previous < 0, "ambiguous lip inlet");
            lip.previous = static_cast<int>(i);
        }
    }
    check(lip.previous >= 0, "unknown lip inlet");
    for (int i = 0; i < n; ++i) {
        double y, z;
        check(bool(s >> y >> z) && std::isfinite(y) && std::isfinite(z), "invalid lip point");
        lip.polygon.emplace_back(lip.center.X(), y, z);
    }
    s >> std::ws;
    check(s.eof(), "trailing lip data");
    walls.push_back(lip);
}
TopoDS_Shape readStep(const std::filesystem::path &path) {
    STEPControl_Reader reader;
    check(reader.ReadFile(path.c_str()) == IFSelect_RetDone && reader.TransferRoots() == 1, "STEP read");
    return reader.OneShape();
}
TopoDS_Shape roundtrip(const TopoDS_Shape &shape, const std::filesystem::path &path) {
    STEPControl_Writer writer;
    check(writer.Transfer(shape, STEPControl_ShellBasedSurfaceModel) == IFSelect_RetDone &&
              writer.Write(path.c_str()) == IFSelect_RetDone,
          "STEP write");
    return readStep(path);
}
int count(const TopoDS_Shape &shape, TopAbs_ShapeEnum kind) {
    TopTools_IndexedMapOfShape map;
    TopExp::MapShapes(shape, kind, map);
    return map.Extent();
}
bool onRing(const TopoDS_Edge &edge, const Ring &ring) {
    BRepAdaptor_Curve c(edge);
    if (c.GetType() != GeomAbs_Circle)
        return false;
    const auto circle = c.Circle();
    return circle.Location().Distance(ring.center) < eps && std::abs(circle.Radius() - ring.radius) < eps &&
           std::abs(c.LastParameter() - c.FirstParameter() - 2 * pi) * ring.radius < eps &&
           circle.Axis().Direction().IsParallel(gp::DX(), eps / ring.radius);
}
TopoDS_Wire circleWire(const Ring &r) {
    auto circle = gp_Circ(gp_Ax2(r.center, gp::DX(), gp::DY()), r.radius);
    return BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(circle).Edge()).Wire();
}
TopoDS_Face makeFace(const Wall &w) {
    gp_Ax3 axis(w.center, gp::DX(), gp::DY());
    TopoDS_Face f;
    if (w.planar()) {
        TopoDS_Wire outer;
        if (w.kind == "collar") {
            BRepBuilderAPI_MakePolygon p;
            for (const auto &vertex : w.polygon)
                p.Add(vertex);
            p.Close();
            outer = p.Wire();
        } else
            outer = circleWire(w.rims[1]);
        BRepBuilderAPI_MakeFace maker(gp_Pln(axis), outer, true);
        auto hole = circleWire(w.rims[0]);
        hole.Reverse();
        maker.Add(hole);
        f = maker.Face();
        if (!w.inner)
            f.Reverse();
    } else {
        if (w.kind == "torus")
            f = BRepBuilderAPI_MakeFace(gp_Torus(axis, w.major, w.minor), 0., 2 * pi, w.low, w.high).Face();
        else if (w.kind == "cylinder")
            f = BRepBuilderAPI_MakeFace(gp_Cylinder(axis, w.major), 0., 2 * pi, w.low, w.high).Face();
        else {
            const double delta = w.rims[1].radius - w.rims[0].radius;
            f = BRepBuilderAPI_MakeFace(gp_Cone(axis, std::atan2(delta, w.high), w.major), 0., 2 * pi, 0.,
                                        std::hypot(delta, w.high))
                    .Face();
        }
        if (w.inner)
            f.Reverse();
    }
    return f;
}
TopoDS_Shape build(const TopoDS_Shape &initial, const Walls &walls) {
    BRepBuilderAPI_Sewing sewing(eps);
    sewing.Add(initial);
    for (const auto &w : walls)
        sewing.Add(makeFace(w));
    sewing.Perform();
    return sewing.SewedShape();
}
struct Signature {
    GeomAbs_SurfaceType type;
    gp_Pnt centroid;
    double area;
    int edges;
};
Signature signature(const TopoDS_Face &face) {
    GProp_GProps prop;
    BRepGProp::SurfaceProperties(face, prop, 1e-12);
    return {BRepAdaptor_Surface(face).GetType(), prop.CentreOfMass(), prop.Mass(), count(face, TopAbs_EDGE)};
}
struct Reference {
    int freeEdges = 0;
    std::vector<Signature> originals;
    std::vector<Signature> inletCones;
};
bool sameFace(const Signature &a, const Signature &b) {
    return a.type == b.type && a.edges == b.edges && a.centroid.Distance(b.centroid) < eps &&
           std::abs(a.area - b.area) < 1.e-6;
}
Reference reference(const TopoDS_Shape &shape, const Walls &walls) {
    check(count(shape, TopAbs_SHELL) == 1 && count(shape, TopAbs_SOLID) == 0 &&
              BRepCheck_Analyzer(shape, true).IsValid(),
          "invalid input shell");
    Reference ref{};
    for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next())
        ref.originals.push_back(signature(TopoDS::Face(it.Current())));
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, adj);
    std::vector<int> found(walls.size());
    ref.inletCones.resize(walls.size());
    for (int i = 1; i <= adj.Extent(); ++i) {
        const auto &uses = adj(i);
        if (uses.Extent() != 1)
            continue;
        ++ref.freeEdges;
        auto edge = TopoDS::Edge(adj.FindKey(i));
        for (size_t j = 0; j < walls.size(); ++j) {
            if (walls[j].previous != -1 || !onRing(edge, walls[j].rims[walls[j].inlet]))
                continue;
            auto sig = signature(TopoDS::Face(uses.First()));
            check(sig.type == GeomAbs_Cone, "wrong inlet neighbor");
            ref.inletCones[j] = sig;
            ++found[j];
        }
    }
    check(std::count(found.begin(), found.end(), 1) == 5, "missing free cone exits");
    for (size_t i = 0; i < walls.size(); ++i)
        check(found[i] == (walls[i].previous == -1), "invalid inlet count");
    return ref;
}
int orientation(const TopoDS_Face &face, const TopoDS_Edge &edge) {
    int result = 0;
    for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next())
        if (it.Current().IsSame(edge))
            result += it.Current().Orientation() == TopAbs_FORWARD ? 1 : -1;
    return result;
}
void validateWall(const TopoDS_Face &face, const Wall &w,
                  const TopTools_IndexedDataMapOfShapeListOfShape &adj, const Signature &previous,
                  const Signature *next, double &err) {
    const BRepAdaptor_Surface surf(face);
    check(surf.GetType() == w.type(), "incorrect new surface family");
    const bool torus = w.kind == "torus";
    if (torus)
        check(surf.Torus().Location().Distance(w.center) < eps &&
                  std::abs(surf.Torus().MajorRadius() - w.major) < eps &&
                  std::abs(surf.Torus().MinorRadius() - w.minor) < eps,
              "torus support drift");
    else if (w.kind == "cylinder")
        check(std::abs(surf.Cylinder().Radius() - w.major) < eps, "cylinder radius drift");
    else if (w.kind == "cone")
        check(std::abs(std::abs(surf.Cone().SemiAngle()) -
                       std::abs(std::atan2(w.rims[1].radius - w.rims[0].radius, w.high))) < 1e-10,
              "cone angle drift");
    check(count(face, TopAbs_WIRE) == (w.planar() ? 2 : 1) &&
              count(face, TopAbs_EDGE) == (w.kind == "collar" ? int(w.polygon.size()) + 1
                                           : w.planar()       ? 2
                                                              : 3),
          "invalid wall topology");
    const double u = (surf.FirstUParameter() + surf.LastUParameter()) / 2;
    const double v = (surf.FirstVParameter() + surf.LastVParameter()) / 2;
    gp_Pnt p;
    gp_Vec du, dv;
    surf.D1(u, v, p, du, dv);
    gp_Vec expected = gp_Vec(gp::DX());
    if (!w.planar()) {
        gp_Vec radial(0., p.Y() - w.center.Y(), p.Z() - w.center.Z());
        radial.Normalize();
        expected = radial;
        if (torus)
            expected = gp_Vec(w.center.Translated(radial * w.major), p).Normalized();
        if (w.kind == "cone") {
            expected.SetX(-(w.rims[1].radius - w.rims[0].radius) / w.high);
            expected.Normalize();
        }
        if (w.inner)
            expected.Reverse();
    } else if (!w.inner)
        expected.Reverse();
    gp_Vec normal = du.Crossed(dv).Normalized();
    if (face.Orientation() == TopAbs_REVERSED)
        normal.Reverse();
    check(normal.Dot(expected) > 1. - 1.e-10, "wrong wall orientation");
    std::array<int, 2> found{};
    int seam = 0;
    std::vector<bool> segments(w.polygon.size());
    for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
        auto edge = TopoDS::Edge(it.Current());
        const auto &uses = adj.FindFromKey(edge);
        BRepAdaptor_Curve curve(edge);
        BRepAdaptor_Curve2d uv(edge, face);
        check(uv.GetType() == (w.planar() ? curve.GetType() : GeomAbs_Line), "unexpected wall pcurve");
        for (double f : {0., .25, .5, .75, 1.}) {
            const auto q = uv.Value(uv.FirstParameter() + f * (uv.LastParameter() - uv.FirstParameter()));
            const auto P = surf.Value(q.X(), q.Y()),
                       Q = curve.Value(curve.FirstParameter() +
                                       f * (curve.LastParameter() - curve.FirstParameter()));
            check(P.Distance(Q) < eps, "displaced wall pcurve");
        }
        int end = -1;
        if (onRing(edge, w.rims[0]))
            end = 0;
        else if (w.kind != "collar" && onRing(edge, w.rims[1]))
            end = 1;
        if (end >= 0) {
            ++found[end];
            const Signature *neighbor = end == w.inlet ? &previous : next;
            check(uses.Extent() == (neighbor ? 2 : 1), "wrong ring incidence");
            if (neighbor) {
                auto other = TopoDS::Face(uses.First().IsSame(face) ? uses.Last() : uses.First());
                check(sameFace(signature(other), *neighbor) &&
                          orientation(face, edge) * orientation(other, edge) == -1,
                      "wrong adjacent face or orientation");
            }
        } else if (w.kind == "collar") {
            check(curve.GetType() == GeomAbs_Line && uses.Extent() == 1, "invalid lip segment");
            const auto A = curve.Value(curve.FirstParameter()), B = curve.Value(curve.LastParameter());
            int k = -1;
            for (size_t i = 0; i < w.polygon.size(); ++i) {
                const auto &a = w.polygon[i];
                const auto &b = w.polygon[(i + 1) % w.polygon.size()];
                if ((A.Distance(a) < eps && B.Distance(b) < eps) ||
                    (B.Distance(a) < eps && A.Distance(b) < eps)) {
                    check(k < 0, "ambiguous lip edge");
                    k = static_cast<int>(i);
                }
            }
            check(k >= 0 && !segments[k], "unseen or repeated lip edge");
            segments[k] = true;
        } else {
            check(BRep_Tool::IsClosed(edge, face) && uses.Extent() == 2 && uses.First().IsSame(face) &&
                      uses.Last().IsSame(face) && curve.GetType() == (torus ? GeomAbs_Circle : GeomAbs_Line),
                  "invalid periodic seam");
            ++seam;
        }
    }
    check(found[0] == 1 && found[1] == (w.kind != "collar") && seam == (w.planar() ? 0 : 2) &&
              std::all_of(segments.begin(), segments.end(), [](bool v) { return v; }),
          "incomplete wall");
    const double delta = std::abs(signature(face).area - w.area());
    check(delta < 1.e-7, "wrong wall area");
    err = std::max(err, delta);
}
double verify(const TopoDS_Shape &shape, const Walls &walls, const Reference &ref) {
    check(count(shape, TopAbs_SHELL) == 1 && count(shape, TopAbs_SOLID) == 0 &&
              size_t(count(shape, TopAbs_FACE)) == ref.originals.size() + walls.size() &&
              BRepCheck_Analyzer(shape, true).IsValid(),
          "invalid extended shell");
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, adj);
    int freeEdges = 0;
    for (int i = 1; i <= adj.Extent(); ++i)
        freeEdges += adj(i).Extent() == 1;
    const auto &last = walls.back();
    const int addedFree = last.kind == "collar" ? int(last.polygon.size()) - 1 : 0;
    check(freeEdges == ref.freeEdges + addedFree, "unexpected free edge count");
    std::vector<bool> recovered(ref.originals.size());
    std::vector<TopoDS_Face> observed(walls.size());
    for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
        const auto face = TopoDS::Face(it.Current());
        Signature sig = signature(face);
        auto old = std::find_if(ref.originals.begin(), ref.originals.end(),
                                [&](const auto &s) { return sameFace(s, sig); });
        if (old != ref.originals.end()) {
            size_t i = std::distance(ref.originals.begin(), old);
            check(!recovered[i], "duplicated input face");
            recovered[i] = true;
            continue;
        }
        bool known = false;
        for (size_t j = 0; j < walls.size(); ++j) {
            const auto &w = walls[j];
            if (sig.type != w.type())
                continue;
            bool ra = false, rb = w.kind == "collar";
            for (TopExp_Explorer e(face, TopAbs_EDGE); e.More(); e.Next()) {
                const auto edge = TopoDS::Edge(e.Current());
                ra |= onRing(edge, w.rims[0]);
                if (!rb)
                    rb = onRing(edge, w.rims[1]);
            }
            if (!ra || !rb)
                continue;
            check(!known && observed[j].IsNull(), "ambiguous new face");
            observed[j] = face;
            known = true;
        }
        check(known, "unrecognized new wall");
    }
    check(std::all_of(recovered.begin(), recovered.end(), [](bool v) { return v; }), "missing initial face");
    std::vector<Signature> signatures;
    for (const auto &f : observed) {
        check(!f.IsNull(), "missing face");
        signatures.push_back(signature(f));
    }
    double error = 0;
    for (size_t j = 0; j < walls.size(); ++j) {
        const auto &w = walls[j];
        const Signature *next = nullptr;
        for (size_t i = j + 1; i < walls.size(); ++i)
            if (walls[i].previous == int(j)) {
                check(next == nullptr, "branched chain");
                next = &signatures[i];
            }
        validateWall(observed[j], w, adj, w.previous < 0 ? ref.inletCones[j] : signatures[w.previous], next,
                     error);
    }
    return error;
}
} // namespace

int main(int argc, char **argv) {
    try {
        check(argc == 4 || argc == 5, "use INPUT.step MEASUREMENTS.txt OUTPUT_DIR [COLLAR.txt]");
        const std::filesystem::path out = argv[3];
        std::ifstream input(argv[2]);
        auto walls = readWalls(input);
        if (argc == 5)
            appendCollar(walls, argv[4]);
        const auto initial = readStep(argv[1]);
        const auto ref = reference(initial, walls);
        const auto joined = build(initial, walls);
        const auto a = verify(joined, walls, ref);
        const auto result = roundtrip(joined, out / "bores.step");
        const auto b = verify(result, walls, ref);
        auto altered = walls;
        auto &cylinder =
            *std::find_if(altered.begin(), altered.end(), [](const Wall &w) { return w.kind == "cylinder"; });
        cylinder.high -= .05;
        const auto wrong = roundtrip(build(initial, altered), out / "short-bore.step");
        bool refused = false;
        try {
            verify(wrong, walls, ref);
        } catch (const std::runtime_error &) {
            refused = true;
        }
        check(refused, "incomplete wall accepted");
        std::cout << std::setprecision(15) << "{\"input_faces\":" << ref.originals.size()
                  << ",\"faces\":" << count(result, TopAbs_FACE)
                  << ",\"shells\":1,\"solids\":0,\"joined_cone_rings\":5,"
                  << "\"free_edges\":"
                  << (ref.freeEdges + (argc == 5 ? int(walls.back().polygon.size()) - 1 : 0))
                  << ",\"new_free_rings\":" << (argc == 5 ? 4 : 5)
                  << ",\"chain_faces\":22,\"collar_faces\":" << (argc == 5)
                  << ",\"max_wall_area_error_mm2\":" << std::max(a, b) << ",\"short_bore_rejected\":true}\n";
    } catch (const Standard_Failure &e) {
        std::cerr << e.GetMessageString() << '\n';
        return 1;
    } catch (const std::exception &e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
