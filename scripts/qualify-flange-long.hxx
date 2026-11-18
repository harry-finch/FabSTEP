// Private STEP laboratory. The variable-height wall is bounded by its measured
// polyline in (azimuth, axial height). Each sloping interval lifts to a helix.
struct LongWall {
    int sector;
    double cy, cz, radius, bottom, lo, hi, area;
    bool outward;
    std::vector<gp_Pnt2d> top;

    gp_Pnt at(const gp_Pnt2d &p) const {
        return {p.Y(), cy + radius * std::cos(p.X()), cz + radius * std::sin(p.X())};
    }
    gp_Vec tangent(double u, double du, double dx) const {
        return {dx, -radius * du * std::sin(u), radius * du * std::cos(u)};
    }
    gp_Ax3 frame() const { return {gp_Pnt(0., cy, cz), gp::DX(), gp_Dir(0., std::cos(lo), std::sin(lo))}; }
    // Start with the lower arc in increasing azimuth. On this chart the
    // winding is outward; the measured triangle orientation sets face use.
    std::vector<gp_Pnt2d> polygon() const {
        std::vector<gp_Pnt2d> loop{{lo, bottom}, {hi, bottom}};
        loop.insert(loop.end(), top.rbegin(), top.rend());
        return loop;
    }
};
using LongWalls = std::vector<LongWall>;

LongWalls readLongWalls(const Sectors &sectors) {
    LongWalls result;
    for (int k = 0; k < 3; ++k) {
        LongWall w{};
        int sign = 0, count = 0;
        check(bool(std::cin >> w.sector >> w.cy >> w.cz >> w.radius >> w.bottom >> w.lo >> w.hi >> sign >>
                   w.area >> count) &&
                  w.sector >= 0 && size_t(w.sector) < sectors.size() && count >= 3 && count <= 513 &&
                  std::all_of(result.begin(), result.end(),
                              [&](const LongWall &other) { return w.sector != other.sector; }),
              "invalid long flank reference");
        w.outward = sign == 1;
        check((sign == 0 || sign == 1) && w.radius > 0 && w.hi > w.lo && w.hi - w.lo < pi,
              "invalid cylindrical interval");
        const auto &base = sectors[w.sector];
        // The meridian of the base fillet may use either end of its interval.
        const int side = std::abs(std::sin(base.v0)) < std::abs(std::sin(base.v1)) ? 2 : 3;
        const auto arc = boundary(base, side);
        check(arc.center.Distance(gp_Pnt(w.bottom, w.cy, w.cz)) < eps &&
                  std::abs(w.radius - arc.radius) < eps && w.at({w.lo, w.bottom}).Distance(arc.a) < eps &&
                  w.at({w.hi, w.bottom}).Distance(arc.b) < eps &&
                  std::abs(w.hi - w.lo - arc.span) * w.radius < eps,
              "wrong long flank reference arc");
        double area = 0.;
        int jumps = 0;
        for (int j = 0; j < count; ++j) {
            double u, x;
            check(bool(std::cin >> u >> x) && std::isfinite(x) && u >= w.lo && u <= w.hi && x > w.bottom,
                  "upper trim point outside chart");
            if (j) {
                const auto &p = w.top.back();
                check(u >= p.X() && (u > p.X() || x != p.Y()), "upper trim backtracks or repeats a point");
                if (u == p.X())
                    ++jumps;
                area += w.radius * (u - p.X()) * ((x + p.Y()) * .5 - w.bottom);
            }
            w.top.emplace_back(u, x);
        }
        check(w.top.front().X() == w.lo && w.top.back().X() == w.hi && jumps == 2 &&
                  std::abs(area - w.area) < 1.e-7,
              "incomplete long flank upper trim or area");
        result.push_back(w);
    }
    std::cin >> std::ws;
    check(std::cin.eof(), "unexpected long flank measurements");
    return result;
}

std::array<gp_Pnt, 4> hermitePoles(const LongWall &w, const gp_Pnt2d &a, const gp_Pnt2d &b) {
    const double du = b.X() - a.X(), dx = b.Y() - a.Y();
    const auto p = w.at(a), q = w.at(b);
    return {p, p.Translated(w.tangent(a.X(), du, dx) / 3.), q.Translated(-w.tangent(b.X(), du, dx) / 3.), q};
}

double hermiteBound(const LongWall &w, double du) {
    // Cubic Hermite remainder t^2(1-t)^2/24 for t in [0,1].
    // x is affine; both transverse components have |f''''| <= R |du|^4.
    return std::sqrt(2.) * w.radius * std::pow(du, 4) / 384.;
}

TopoDS_Edge longEdge(const LongWall &w, const Handle(Geom_CylindricalSurface) & surf, const gp_Pnt2d &a,
                     const gp_Pnt2d &b) {
    const double du = b.X() - a.X(), dx = b.Y() - a.Y();
    const gp_Pnt2d start(a.X() - w.lo, a.Y()), end(b.X() - w.lo, b.Y());
    Handle(Geom_Curve) curve;
    Handle(Geom2d_Curve) uv;
    double extent = 0., error = 0.;
    if (dx == 0.) {
        const gp_Ax2 plane(gp_Pnt(a.Y(), w.cy, w.cz), du > 0 ? gp::DX() : -gp::DX(),
                           gp_Dir(0., std::cos(a.X()), std::sin(a.X())));
        curve = new Geom_Circle(plane, w.radius);
    } else if (du == 0.) {
        curve = new Geom_Line(w.at(a), dx > 0 ? gp::DX() : -gp::DX());
    } else {
        const auto poles = hermitePoles(w, a, b);
        TColgp_Array1OfPnt points(1, 4);
        for (int j = 0; j < 4; ++j)
            points.SetValue(j + 1, poles[j]);
        curve = new Geom_BezierCurve(points);
        TColgp_Array1OfPnt2d linear(1, 2);
        linear.SetValue(1, start);
        linear.SetValue(2, end);
        uv = new Geom2d_BezierCurve(linear);
        extent = 1.;
        error = hermiteBound(w, du);
        check(error < eps / 4, "unresolved helix interpolation");
    }
    if (uv.IsNull()) {
        extent = start.Distance(end);
        uv = new Geom2d_Line(start, gp_Dir2d(gp_Vec2d(start, end)));
    }
    const auto edge = BRepBuilderAPI_MakeEdge(curve, 0., extent).Edge();
    BRep_Builder builder;
    builder.UpdateEdge(edge, uv, surf, TopLoc_Location(), std::max(error, Precision::Confusion()));
    builder.Range(edge, surf, TopLoc_Location(), 0., extent);
    return edge;
}

TopoDS_Shape attachLong(const TopoDS_Shape &old, const LongWalls &walls) {
    BRepBuilderAPI_Sewing sewing(eps);
    sewing.Add(old);
    for (const auto &w : walls) {
        Handle(Geom_CylindricalSurface) surface = new Geom_CylindricalSurface(w.frame(), w.radius);
        const auto loop = w.polygon();
        BRepBuilderAPI_MakeWire wire;
        for (size_t j = 0; j < loop.size(); ++j)
            wire.Add(longEdge(w, surface, loop[j], loop[(j + 1) % loop.size()]));
        check(wire.IsDone(), "long flank wire construction");
        BRepBuilderAPI_MakeFace maker(surface, wire.Wire(), true);
        check(maker.IsDone(), "long flank face construction");
        auto face = maker.Face();
        if (!w.outward)
            face.Reverse();
        check(BRepCheck_Analyzer(face, true).IsValid(), "invalid standalone long flank");
        sewing.Add(face);
    }
    sewing.Perform();
    return sewing.SewedShape();
}

struct WallErrors {
    double area = 0., boundary = 0.;
    int freeEdges = 0;
};

void checkLongCurve(const BRepAdaptor_Curve &curve, const BRepAdaptor_Curve2d &uv,
                    const BRepAdaptor_Surface &surf, const LongWall &w, const gp_Pnt2d &a, const gp_Pnt2d &b,
                    WallErrors &error) {
    const double du = b.X() - a.X(), dx = b.Y() - a.Y();
    const double lo = curve.FirstParameter(), hi = curve.LastParameter();
    const double ul = uv.FirstParameter(), uh = uv.LastParameter();
    if (du == 0.)
        check(curve.GetType() == GeomAbs_Line, "nonlinear axial trim");
    else if (dx == 0.) {
        check(curve.GetType() == GeomAbs_Circle && std::abs(curve.Circle().Radius() - w.radius) < eps &&
                  curve.Circle().Location().Distance(gp_Pnt(a.Y(), w.cy, w.cz)) < eps &&
                  curve.Circle().Axis().Direction().IsParallel(gp::DX(), eps / w.radius) &&
                  std::abs((hi - lo) - std::abs(du)) * w.radius < eps,
              "noncircular station trim");
    } else {
        const auto poles = hermitePoles(w, a, b);
        double hull = 0.;
        if (curve.GetType() == GeomAbs_BezierCurve) {
            const auto c = curve.Bezier();
            check(!c->IsRational() && c->Degree() == 3 && std::abs(lo) < 1e-12 && std::abs(hi - 1.) < 1e-12,
                  "noncubic helical witness");
            for (int j = 0; j < 4; ++j)
                hull = std::max(hull, poles[j].Distance(c->Pole(j + 1)));
        } else {
            check(curve.GetType() == GeomAbs_BSplineCurve, "missing cubic helical trim");
            const auto c = curve.BSpline();
            check(!c->IsRational() && c->Degree() == 3 && c->NbPoles() == 4 && c->NbKnots() == 2 &&
                      c->Multiplicity(1) == 4 && c->Multiplicity(2) == 4 &&
                      std::abs(c->Knot(1) - lo) < 1.e-12 && std::abs(c->Knot(2) - hi) < 1.e-12,
                  "unresolved helical spline interval");
            for (int j = 0; j < 4; ++j)
                hull = std::max(hull, poles[j].Distance(c->Pole(j + 1)));
        }
        const double bound = hull + hermiteBound(w, du);
        check(bound < eps, "helical trim exceeds certified deviation");
        error.boundary = std::max(error.boundary, bound);
    }
    // On an analytic cylinder an affine UV segment has a unique lift once
    // its endpoints and unwrapped angular displacement are established.
    const auto p = uv.Value(ul), q = uv.Value(uh);
    if (uv.GetType() == GeomAbs_BSplineCurve || uv.GetType() == GeomAbs_BezierCurve) {
        auto checkLinear = [&](const auto &c) {
            check(!c->IsRational() && c->Degree() == 1 && c->NbPoles() == 2, "nonaffine cylindrical pcurve");
        };
        if (uv.GetType() == GeomAbs_BSplineCurve)
            checkLinear(uv.BSpline());
        else
            checkLinear(uv.Bezier());
    } else
        check(uv.GetType() == GeomAbs_Line, "unexpected cylindrical pcurve");
    const auto xyz = surf.Cylinder();
    const double sign = xyz.XAxis().Direction().Crossed(xyz.YAxis().Direction()).Dot(gp::DX());
    check(std::abs((q.X() - p.X()) * sign - du) * w.radius < eps &&
              std::abs((q.Y() - p.Y()) * xyz.Axis().Direction().Dot(gp::DX()) - dx) < eps,
          "wrong cylindrical UV itinerary");
    for (double f : {0., .5, 1.}) {
        const gp_Pnt2d ref(a.X() + f * du, a.Y() + f * dx), mapped = uv.Value(ul + f * (uh - ul));
        const auto point = w.at(ref);
        const double d = std::max(point.Distance(curve.Value(lo + f * (hi - lo))),
                                  point.Distance(surf.Value(mapped.X(), mapped.Y())));
        check(d < eps, "misplaced long flank boundary");
        error.boundary = std::max(error.boundary, d);
    }
}

int longIndex(const TopoDS_Face &face, const LongWalls &walls) {
    const BRepAdaptor_Surface surf(face);
    if (surf.GetType() != GeomAbs_Cylinder)
        return -1;
    const auto c = surf.Cylinder();
    for (size_t i = 0; i < walls.size(); ++i) {
        const auto &w = walls[i];
        if (c.Axis().Direction().IsParallel(gp::DX(), eps / 100.) && std::abs(c.Radius() - w.radius) < eps &&
            gp_Lin(c.Axis()).Distance(gp_Pnt(0., w.cy, w.cz)) < eps)
            return int(i);
    }
    return -1;
}

template <class VerifyLower>
WallErrors verifyLong(const TopoDS_Shape &shape, const Sectors &sectors, const LongWalls &walls,
                      VerifyLower verifyLower) {
    check(number(shape, TopAbs_SOLID) == 0 && number(shape, TopAbs_SHELL) == 1 &&
              number(shape, TopAbs_FACE) == 37 && BRepCheck_Analyzer(shape, true).IsValid(),
          "invalid 37-face shell");
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, adj);
    WallErrors errors;
    TopoDS_Shell old;
    BRep_Builder builder;
    builder.MakeShell(old);
    std::array<int, 3> seen{}, bases{}, generators{};
    for (TopExp_Explorer fit(shape, TopAbs_FACE); fit.More(); fit.Next()) {
        const auto face = TopoDS::Face(fit.Current());
        const int index = longIndex(face, walls);
        if (index < 0) {
            builder.Add(old, face);
            continue;
        }
        const auto &w = walls[index];
        ++seen[index];
        const auto loop = w.polygon();
        std::vector<int> edges(loop.size(), 0);
        check(number(face, TopAbs_WIRE) == 1 && number(face, TopAbs_EDGE) == int(loop.size()),
              "long flank edge count differs from source");
        const BRepAdaptor_Surface surf(face);
        const double mid = (w.lo + w.hi) * .5;
        const auto frame = surf.Cylinder().Position();
        const gp_Dir radial(0., std::cos(mid), std::sin(mid));
        const double az = std::atan2(radial.Dot(frame.YDirection()), radial.Dot(frame.XDirection()));
        gp_Pnt pos;
        gp_Vec tu, tv;
        surf.D1(az, 0., pos, tu, tv);
        gp_Dir normal(tu.Crossed(tv));
        if (face.Orientation() == TopAbs_REVERSED)
            normal.Reverse();
        check(normal.IsEqual(w.outward ? radial : -radial, eps / w.radius), "wrong long flank normal");
        for (BRepTools_WireExplorer it(BRepTools::OuterWire(face), face); it.More(); it.Next()) {
            const auto edge = it.Current();
            const BRepAdaptor_Curve c(edge);
            const auto p = c.Value(c.FirstParameter()), q = c.Value(c.LastParameter());
            int label = -1;
            bool reverse = false;
            for (size_t j = 0; j < loop.size(); ++j) {
                const auto a = w.at(loop[j]), b = w.at(loop[(j + 1) % loop.size()]);
                const bool forward = p.Distance(a) < eps && q.Distance(b) < eps;
                const bool backward = p.Distance(b) < eps && q.Distance(a) < eps;
                if (forward || backward) {
                    check(label == -1 && forward != backward, "ambiguous long flank edge");
                    label = int(j);
                    reverse = backward;
                }
            }
            check(label >= 0, "unmatched measured trim");
            ++edges[label];
            const auto &a = loop[label];
            const auto &b = loop[(label + 1) % loop.size()];
            checkLongCurve(c, BRepAdaptor_Curve2d(edge, face), surf, w, reverse ? b : a, reverse ? a : b,
                           errors);
            const auto &uses = adj.FindFromKey(edge);
            if (uses.Extent() == 1) {
                check(label != 0, "missing lower arc join");
                continue;
            }
            check(uses.Extent() == 2, "ambiguous long flank join");
            const auto other = TopoDS::Face(uses.First().IsSame(face) ? uses.Last() : uses.First());
            check(!other.IsSame(face) && use(other, edge) != use(face, edge),
                  "inconsistent long flank coedges");
            if (label == 0) {
                check(BRepAdaptor_Surface(other).GetType() == GeomAbs_Torus &&
                          sectorIndex(other, sectors) == w.sector,
                      "wrong sector on shared lower arc");
                ++bases[index];
            } else {
                const int neighbor = longIndex(other, walls);
                check((label == 1 || label == int(loop.size()) - 1) && c.GetType() == GeomAbs_Line &&
                          neighbor >= 0 && neighbor != index,
                      "wrong longitudinal neighbor");
                // Source incidence joins the two high-angle ends of 4 and 5
                // and the two low-angle ends of 4 and 6.
                const auto pair = std::minmax(w.sector, walls[neighbor].sector);
                check(pair.first == 4 && pair.second == (label == 1 ? 5 : 6), "unexpected shared generator");
                ++generators[index];
            }
        }
        check(std::all_of(edges.begin(), edges.end(), [](int n) { return n == 1; }),
              "incomplete long flank wire");
        GProp_GProps props;
        BRepGProp::SurfaceProperties(face, props, 1e-12);
        errors.area = std::max(errors.area, std::abs(props.Mass() - w.area));
    }
    check(bases == std::array<int, 3>{1, 1, 1} && seen == bases, "wrong long flank count");
    std::sort(generators.begin(), generators.end());
    check(generators == std::array<int, 3>{1, 1, 2}, "wrong generator incidence");
    check(errors.area < 1.e-7, "wrong trimmed cylindrical area");
    verifyLower(old);
    std::vector<TopoDS_Edge> free;
    int bores = 0;
    for (int i = 1; i <= adj.Extent(); ++i)
        if (adj(i).Extent() == 1) {
            const auto face = TopoDS::Face(adj(i).First());
            if (BRepAdaptor_Surface(face).GetType() == GeomAbs_Cone)
                ++bores;
            else
                free.push_back(TopoDS::Edge(adj.FindKey(i)));
        }
    int topEdges = 0;
    for (const auto &w : walls)
        topEdges += int(w.top.size()) - 1;
    errors.freeEdges = int(free.size()) + bores;
    check(bores == 5 && errors.freeEdges == 1029 - 3 + topEdges + 2 && boundaryLoops(free, false) == 11,
          "changed free boundary topology");
    return errors;
}

template <class VerifyLower>
int qualifyLong(const std::filesystem::path &dir, const TopoDS_Shape &old, const Sectors &sectors,
                const LongWalls &walls, VerifyLower verifyLower) {
    constexpr size_t limit = 24 * 1024 * 1024;
    const auto baseline = roundtrip(old, dir / "reference-34.step", limit);
    verifyLower(baseline);
    const auto assembled = attachLong(baseline, walls);
    const auto before = verifyLong(assembled, sectors, walls, verifyLower);
    const auto imported = roundtrip(assembled, dir / "long-flanks.step", limit);
    const auto after = verifyLong(imported, sectors, walls, verifyLower);
    auto shifted = walls;
    // Move an interior trim vertex without changing the support or the joins.
    auto &witness = shifted.front().top.at(shifted.front().top.size() / 2);
    witness.SetY(witness.Y() + .05);
    const auto bad = roundtrip(attachLong(baseline, shifted), dir / "shifted-trim.step", limit);
    bool rejected = false;
    try {
        verifyLong(bad, sectors, walls, verifyLower);
    } catch (const std::runtime_error &e) {
        rejected = std::string(e.what()) == "unmatched measured trim";
    }
    check(rejected, "wrong upper trim accepted or unexpected rejection");
    std::ofstream out(dir / "long-flanks-report.json");
    out << std::setprecision(16)
        << "{\"faces\":37,\"solids\":0,\"free_loops\":16,\"free_edges\":" << after.freeEdges
        << ",\"shared_bottom_arcs\":3,\"shared_generators\":2,\"max_long_area_error_mm2\":"
        << std::max(before.area, after.area)
        << ",\"max_boundary_error_mm\":" << std::max(before.boundary, after.boundary)
        << ",\"shifted_trim_rejected\":true}\n";
    out.close();
    check(out.good(), "failed to write long flank report");
    return 0;
}
