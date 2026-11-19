#include "cross_drill.hxx"
#include "cross_drill_certificate.hxx"
#include <BRepAdaptor_Surface.hxx>
#include <BRepAdaptor_Curve.hxx>
#include <BRepTools.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <TopoDS.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <map>
#include <stdexcept>
namespace {
void require(bool ok, const char* msg) {
  if (!ok)
    throw std::runtime_error(msg);
}
TopoDS_Edge occurrence(const TopoDS_Edge& key, const TopoDS_Face& face) {
  TopoDS_Edge result;
  int count = 0;
  for (TopExp_Explorer it(face, TopAbs_EDGE); it.More(); it.Next()) {
    const auto e = TopoDS::Edge(it.Current());
    if (e.IsSame(key)) {
      result = e;
      ++count;
    }
  }
  require(count == 1, "Intersection edge must occur exactly once per incident face");
  require(result.Orientation() == TopAbs_FORWARD || result.Orientation() == TopAbs_REVERSED,
          "Indeterminate edge orientation");
  return result;
}
struct Junction {
  TopoDS_Edge edge;
  TopoDS_Face small, large;
  int sign;
};
struct Coverage {
  bool ok = false;
  double gap = std::numeric_limits<double>::infinity();
  double bound = std::numeric_limits<double>::infinity();
  std::string reason;
};
struct CycleArc {
  int from, to, branch;
  bool forward;
};
Coverage coverBranch(const std::vector<cross_drill_lab::Certificate>& certs,
                     const std::vector<CycleArc>& arcs, int branch, double tolerance, double r,
                     double R, double offset, double cosine) {
  using cross_drill_lab::certificate_detail::I;
  using cross_drill_lab::certificate_detail::magnitude;
  using cross_drill_lab::certificate_detail::middle;
  using cross_drill_lab::certificate_detail::root;
  using cross_drill_lab::certificate_detail::square;
  Coverage result;
  std::map<int, I> incoming, outgoing;
  std::map<int, int> next;
  I winding(0.);
  int direction = 0;
  double worst = 0.;
  if (arcs.size() != certs.size()) {
    result.reason = "missing edge incidence";
    return result;
  }
  for (std::size_t i = 0; i < arcs.size(); ++i) {
    const auto& arc = arcs[i];
    if (arc.branch != branch)
      continue;
    const auto& c = certs[i];
    if (!c.ok || !c.phase_monotone || c.sign != branch || std::abs(c.phase_direction) != 1) {
      result.reason = "missing or inconsistent edge certificate";
      return result;
    }
    const int sign = arc.forward ? c.phase_direction : -c.phase_direction;
    if (direction && direction != sign) {
      result.reason = "inconsistent oriented phase";
      return result;
    }
    direction = sign;
    I first(c.phase_first_lo, c.phase_first_hi), last(c.phase_last_lo, c.phase_last_hi);
    if (!arc.forward)
      std::swap(first, last);
    if (!incoming.emplace(arc.to, last).second || !outgoing.emplace(arc.from, first).second) {
      result.reason = "multiple incoming or outgoing edges at vertex";
      return result;
    }
    next.emplace(arc.from, arc.to);
    winding = winding + (last - first);
    worst = std::max(worst, c.bound3d);
  }
  if (next.empty()) {
    result.reason = "branch absent";
    return result;
  }
  const I halfTurn = cross_drill_lab::certificate_detail::pi(), turn = I(2.) * halfTurn;
  const int start = next.begin()->first;
  int vertex = start;
  double gap = 0.;
  for (std::size_t i = 0; i < next.size(); ++i) {
    if (!next.count(vertex) || !incoming.count(vertex) || (i && vertex == start)) {
      result.reason = "branch disconnected";
      return result;
    }
    // Close each topological vertex along the unique short angular arc
    // between the actual certified endpoints. The candidate integer lift
    // is accepted only if its entire difference lies strictly inside ±pi.
    const I difference = outgoing.at(vertex) - incoming.at(vertex);
    const double k = std::nearbyint(middle(difference) / middle(turn));
    const I bridge = difference - I(k) * turn;
    if (magnitude(bridge) >= halfTurn.lo) {
      result.reason = "ambiguous vertex phase";
      return result;
    }
    gap = std::max(gap, magnitude(bridge));
    winding = winding + bridge;
    vertex = next.at(vertex);
  }
  if (vertex != start) {
    result.reason = "branch not closed";
    return result;
  }
  // The lifted paths plus bridges form a closed angular path, so their sum
  // is an integer turn. Isolate exactly +1 or -1, excluding duplicated turns.
  const I oriented = I(direction) * winding;
  if (oriented.lo <= halfTurn.hi || oriented.hi >= (I(3.) * halfTurn).lo) {
    result.reason = "winding differs from one";
    return result;
  }
  // An ideal point in a bridge is at most half its angular length from a
  // proved endpoint. Bound the speed on the exact cylinder intersection.
  const I extent = I(r)+I(std::abs(offset));
  const I zmin = root(square(I(R)) - square(extent));
  const I dz = offset == 0. ? square(I(r))/(I(2.)*zmin) : extent*I(r)/zmin;
  const I physical_dz=(dz+I(std::abs(cosine))*I(r))/root(I(1.)-square(I(cosine)));
  const I speed = root(square(I(r)) + square(physical_dz));
  result.gap = gap;
  result.bound = (I(worst) + speed * I(gap) / I(2.)).hi;
  result.ok = result.bound <= tolerance;
  result.reason = result.ok ? "one closed degree-one branch, gaps included in bound"
                            : "vertex gap exceeds tolerance";
  return result;
}
Coverage budgetedBranch(const std::vector<cross_drill_lab::Certificate>& certs,
                        const std::vector<CycleArc>& arcs, int branch, double tolerance,
                        std::size_t& remaining, double r, double R, double offset, double cosine) {
  using namespace cross_drill_lab::certificate_detail;
  Counter counter{0, remaining};
  Scope scope(counter);
  Coverage result;
  try {
    result = coverBranch(certs, arcs, branch, tolerance, r, R, offset, cosine);
  } catch (const Failure& error) {
    result.reason = error.reason;
  }
  remaining -= counter.work;
  return result;
}

gp_Pnt point(const double* p) { return {p[0], p[1], p[2]}; }
gp_Dir dir(const double* p) { return {p[0], p[1], p[2]}; }
void store(double* p, gp_XYZ v) {
  for (int i = 0; i < 3; ++i)
    p[i] = v.Coord(i + 1);
}

// Sweeping the same convex profile in opposite wire order can give a
// reversed face on an indirect cylinder. Use its oriented surface normal.
bool exterior_cylinder(const TopoDS_Face& face, const BRepAdaptor_Surface& surface) {
  if (surface.GetType() != GeomAbs_Cylinder) return false;
  gp_Pnt sample;
  gp_Vec du,dv;
  surface.D1((surface.FirstUParameter()+surface.LastUParameter())*.5,
             (surface.FirstVParameter()+surface.LastVParameter())*.5, sample,du,dv);
  auto normal=du.Crossed(dv);
  if (face.Orientation()==TopAbs_REVERSED) normal.Reverse();
  return normal.Dot(gp_Vec(surface.Cylinder().Location(),sample))>0.;
}

// On a multi-root reimport or display inspection derive the coordinate frame
// from the four actual mouths, independently of STEP names / enumeration.
CrossDrillData infer(const TopTools_IndexedMapOfShape& faces) {
  std::vector<gp_Circ> rings;
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto f = TopoDS::Face(faces(i));
    if (BRepAdaptor_Surface(f).GetType() != GeomAbs_Plane)
      continue;
    for (TopExp_Explorer it(f, TopAbs_EDGE); it.More(); it.Next()) {
      const BRepAdaptor_Curve c(TopoDS::Edge(it.Current()));
      if (c.GetType() == GeomAbs_Circle && BRep_Tool::IsClosed(it.Current()))
        rings.push_back(c.Circle());
    }
  }
  require(rings.size() == 4, "Les quatre entrées circulaires doivent être présentes.");
  std::sort(rings.begin(), rings.end(),
            [](const gp_Circ& a, const gp_Circ& b) { return a.Radius() < b.Radius(); });
  CrossDrillData d{};
  const auto center = (rings[2].Location().XYZ() + rings[3].Location().XYZ()) * .5;
  store(d.center, center);
  const auto dx = gp_Vec(rings[2].Location(), rings[3].Location()),
             dz = gp_Vec(rings[0].Location(), rings[1].Location());
  d.half[0] = dx.Magnitude() / 2.;
  d.half[2] = dz.Magnitude() / 2.;
  const gp_Dir x(dx), z(dz), y(z.Crossed(x));
  const auto delta = (rings[0].Location().XYZ() + rings[1].Location().XYZ())*.5 - center;
  const double axis_cosine=x.Dot(z);
  if (std::abs(axis_cosine)>1.e-9) {
    const auto lateral = delta.Dot(y.XYZ());
    require(std::abs(axis_cosine)<=.51 && (delta-y.XYZ()*lateral).Modulus()<Precision::Confusion(),
            "Décalage oblique hors du domaine de la normale commune.");
    d.large_axis_cosine=axis_cosine;
    d.small_offset[1]=std::abs(lateral)>Precision::Confusion() ? lateral : 0.;
    store(d.axes[0],y.Crossed(z).XYZ());
    store(d.axes[2],z.XYZ());
  } else {
    d.small_offset[0] = delta.Dot(x.XYZ());
    d.small_offset[1] = delta.Dot(y.XYZ());
    store(d.axes[0], x.XYZ());
    store(d.axes[2], x.Crossed(y).XYZ());
  }
  store(d.axes[1], y.XYZ());
  for (int i = 1; i <= faces.Extent(); ++i) {
    const BRepAdaptor_Surface s(TopoDS::Face(faces(i)));
    if (s.GetType() == GeomAbs_Plane && s.Plane().Axis().Direction().IsParallel(y, 1.e-8))
      d.half[1] = s.Plane().Distance(gp_Pnt(center));
  }
  d.radii[0] = (rings[0].Radius() + rings[1].Radius()) / 2.;
  d.radii[1] = (rings[2].Radius() + rings[3].Radius()) / 2.;
  if (faces.Extent() == 10) {
    for (int i = 1; i <= faces.Extent(); ++i) {
      const auto face = TopoDS::Face(faces(i));
      const BRepAdaptor_Surface surface(face);
      if (!exterior_cylinder(face,surface)) continue;
      require(d.fillet_radius == 0., "Plusieurs congés extérieurs.");
      const auto cyl = surface.Cylinder();
      const gp_Vec v(gp_Pnt(center), cyl.Location());
      d.fillet_radius = cyl.Radius();
      d.fillet_y_sign = v.Dot(gp_Vec(y)) > 0. ? 1 : -1;
      d.fillet_z_sign = v.Dot(gp_Vec(z)) > 0. ? 1 : -1;
    }
  }
  return d;
}
} // namespace

CrossDrillResult verify_cross_drill(const TopoDS_Shape& solid, const CrossDrillData* expected,
                                    double deflection) {
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(solid, TopAbs_FACE, faces);
  require(faces.Extent() == 9 || faces.Extent() == 10, "Nombre de faces du bloc percé inattendu.");
  const auto d = expected ? *expected : infer(faces);
  const auto center = point(d.center);
  const double r = d.radii[0], R = d.radii[1], tol = Precision::Confusion();
  const double dx=d.small_offset[0], dy=d.small_offset[1];
  require(r > tol && std::isfinite(dx) && std::isfinite(dy) && std::abs(dy)<=r &&
              R-1.5*(r+std::abs(dy))>(dy==0. ? 2.5 : 5.5)*tol && d.half[0] > std::abs(dx)+r+tol && d.half[1] > R + tol &&
              d.half[2] > R + tol,
          "Rayons ou portées des perçages invalides.");
  const double cosine=d.large_axis_cosine, tilt_sine=std::sqrt((1.-cosine)*(1.+cosine));
  const gp_Dir u=dir(d.axes[0]),y_axis=dir(d.axes[1]),z_axis=dir(d.axes[2]);
  require(std::isfinite(cosine) && std::abs(cosine)<=.51 && tilt_sine>=.85 &&
          u.IsNormal(z_axis, 1.e-8) && z_axis.Crossed(u).IsEqual(y_axis, 1.e-8),
          "Repère des perçages invalide.");
  const gp_Dir big_axis(gp_Vec(u)*tilt_sine+gp_Vec(z_axis)*cosine);
  const std::array<gp_Dir, 3> axes{big_axis,y_axis,z_axis};
  const double radius = d.fillet_radius;
  const auto sy = d.fillet_y_sign, sz = d.fillet_z_sign;
  require(faces.Extent() == (radius ? 10 : 9) && (radius ?
      (std::isfinite(radius) && radius > tol && cosine == 0. && (sy == -1 || sy == 1) &&
       (sz == -1 || sz == 1) && d.half[2] > radius && d.half[1]-radius > std::max(R, std::abs(dy)+r)+tol) :
      (sy == 0 && sz == 0)), "Domaine du congé extérieur incohérent.");
  const auto fillet_center = center.Translated(gp_Vec(y_axis)*double(sy)*(d.half[1]-radius) +
                                               gp_Vec(z_axis)*double(sz)*(d.half[2]-radius));
  // The two tangent points of each end arc replace one stock corner.
  auto stock_vertex = [&](const gp_Pnt& p) {
    const gp_Vec v(center, p);
    if (std::abs(std::abs(v.Dot(gp_Vec(u))) - d.half[0]) > tol) return false;
    const double y = v.Dot(gp_Vec(y_axis)), z = v.Dot(gp_Vec(z_axis));
    return std::hypot(y-sy*d.half[1], z-sz*(d.half[2]-radius)) <= tol ||
           std::hypot(y-sy*(d.half[1]-radius), z-sz*d.half[2]) <= tol;
  };
  bool fillet_found = false;
  require(cosine==0. || (dx==0. &&
      d.half[0]>std::abs(cosine)*d.half[2]+tilt_sine*r+tol &&
      d.half[2]>std::abs(cosine)*d.half[0]+tilt_sine*R+tol),
      "Les bouches doivent rester distinctes à l’intérieur de leur support.");
  auto at = [&](size_t axis, double v) { return center.Translated(gp_Vec(axes[axis]) * v); };
  const gp_Pnt small_center=center.Translated(gp_Vec(axes[0])*dx+gp_Vec(axes[1])*dy);
  auto mouth=[&](size_t axis, double value) {
    return (axis==2 ? small_center : center).Translated(gp_Vec(axes[axis])*value);
  };
  auto near_angle = [&](double extent) { return std::min(1.e-8, tol / (4. * extent)); };
  std::array<int, 6> planes{};
  std::array<int, 3> walls{};
  std::array<int, 4> bores{};
  std::vector<int> owners(faces.Extent() + 1, -1);
  for (int i = 1; i <= faces.Extent(); ++i) {
    const auto f = TopoDS::Face(faces(i));
    const BRepAdaptor_Surface s(f);
    if (s.GetType() == GeomAbs_Plane) {
      const auto p = s.Plane();
      auto n = p.Axis().Direction();
      if (f.Orientation() == TopAbs_REVERSED)
        n.Reverse();
      int slot = -1;
      for (int j = 0; j < 6; ++j) {
        const int k = j % 3;
        auto out = axes[k];
        if (j < 3)
          out.Reverse();
        if (n.IsEqual(out, near_angle(*std::max_element(d.half, d.half + 3))) &&
            p.Distance(at(k, (j < 3 ? -1. : 1.) * d.half[k])) <= tol) {
          slot = j;
          break;
        }
      }
      if (slot >= 0) {
        const int k = slot % 3;
        const double sine = gp_Vec(n).Crossed(gp_Vec(axes[k])).Magnitude();
        const double reach = cosine==0. ? std::hypot(d.half[(k+1)%3],d.half[(k+2)%3]) :
            2.*(d.half[0]+d.half[1]+d.half[2])/tilt_sine;
        require(p.Distance(at(k, (slot < 3 ? -1. : 1.) * d.half[k])) +
                        reach * sine <=
                    tol,
                "Déplacement du plan après transfert.");
      }
      require(slot >= 0 && !planes[slot], "Les six plans doivent borner le bloc centré.");
      planes[slot] = i;
      owners[i] = slot;
    } else if (radius && exterior_cylinder(f,s)) {
      const auto cyl=s.Cylinder();
      require(!fillet_found && cyl.Axis().Direction().IsParallel(u, near_angle(d.half[0])) &&
              gp_Lin(cyl.Axis()).Distance(fillet_center) + std::abs(cyl.Radius()-radius) +
              gp_Vec(cyl.Axis().Direction()).Crossed(gp_Vec(u)).Magnitude()*(d.half[0]+radius) <= tol,
              "Support du congé extérieur déplacé.");
      owners[i]=9;
      fillet_found=true;
    } else {
      require(s.GetType() == GeomAbs_Cylinder && f.Orientation() == TopAbs_REVERSED,
              "Une paroi doit être un cylindre intérieur.");
      const auto c = s.Cylinder();
      const bool large = std::abs(c.Radius() - R) <= tol;
      const size_t axis = large ? 0 : 2;
      require(std::abs(c.Radius() - (large ? R : r)) <= tol &&
                  c.Axis().Direction().IsParallel(axes[axis], near_angle(d.half[axis])) &&
                  gp_Lin(c.Axis()).Distance(large ? center : small_center) +
                          gp_Vec(c.Axis().Direction()).Crossed(gp_Vec(axes[axis])).Magnitude() *
                              (d.half[axis] + c.Radius()) +
                          std::abs(c.Radius() - (large ? R : r)) <=
                      tol,
              "Axe ou rayon du perçage incohérent.");
      unsigned mask = 0;
      for (TopExp_Explorer it(f, TopAbs_EDGE); it.More(); it.Next()) {
        const auto e = TopoDS::Edge(it.Current());
        const BRepAdaptor_Curve c3(e);
        if (c3.GetType() != GeomAbs_Circle)
          continue;
        const auto circle = c3.Circle();
        const auto pos = circle.Location();
        int k = -1;
        for (int j = 0; j < 2; ++j)
          if (pos.Distance(mouth(axis, (j ? 1. : -1.) * d.half[axis])) <= tol)
            k = j;
        require(
            k >= 0 && circle.Axis().Direction().IsParallel(axes[axis], near_angle(c.Radius())) &&
                std::abs(circle.Radius() - c.Radius()) <= tol && BRep_Tool::IsClosed(e) &&
                std::abs(c3.LastParameter() - c3.FirstParameter() - 2. * std::acos(-1.)) < 1.e-8,
            "Anneau d’entrée incorrect.");
        require(pos.Distance(mouth(axis, (k ? 1. : -1.) * d.half[axis])) +
                        gp_Vec(circle.Axis().Direction()).Crossed(gp_Vec(axes[axis])).Magnitude() *
                            circle.Radius() +
                        std::abs(circle.Radius() - (large ? R : r)) <=
                    tol,
                "Déplacement de l’entrée après transfert.");
        require(++bores[(large ? 2 : 0) + k] == 1, "Entrée cylindrique dupliquée.");
        mask |= 1u << k;
      }
      const int slot = large && mask == 3    ? 2
                       : !large && mask == 1 ? 0
                       : !large && mask == 2 ? 1
                                             : -1;
      require(slot >= 0 && !walls[slot], "Portion de paroi mal délimitée.");
      walls[slot] = i;
      owners[i] = slot + 6;
    }
  }
  for (int n : planes)
    require(n, "Plan de bloc absent.");
  for (int n : walls)
    require(n, "Paroi de perçage absente.");
  TopTools_IndexedDataMapOfShapeListOfShape incidence;
  TopExp::MapShapesAndUniqueAncestors(solid, TopAbs_EDGE, TopAbs_FACE, incidence);
  size_t remaining = 10000000, leaves = 16384;
  int lines = 0, mouths = 0;
  unsigned fillet_planes = 0;
  std::vector<cross_drill_lab::Certificate> certs;
  std::vector<CycleArc> links;
  TopTools_IndexedMapOfShape vertices;
  CrossDrillResult out{};
  out.data = d;
  using namespace cross_drill_lab::certificate_detail;
  Counter framing{0, remaining};
  Scope frame_scope(framing);
  const double actual_large[]{big_axis.X(),big_axis.Y(),big_axis.Z()};
  LocalFrame frame(d.center, actual_large, d.axes[2], cosine!=0.);
  frame.center=plus(frame.center,plus(scale(frame.axes[0],I(dx)),scale(frame.axes[1],I(dy))));
  remaining -= framing.work;
  for (int i = 1; i <= incidence.Extent(); ++i) {
    const auto edge = TopoDS::Edge(incidence.FindKey(i));
    const auto& adj = incidence.FindFromIndex(i);
    const auto a = TopoDS::Face(adj.First()), b = TopoDS::Face(adj.Last());
    const int l = owners[faces.FindIndex(a)], h = owners[faces.FindIndex(b)];
    const BRepAdaptor_Curve c(edge);
    if (adj.Extent() == 1) {
      require(l >= 6 && BRep_Tool::IsClosed(edge, a) && c.GetType() == GeomAbs_Line,
              "Couture cylindrique invalide.");
      continue;
    }
    require(adj.Extent() == 2 &&
                occurrence(edge, a).Orientation() != occurrence(edge, b).Orientation(),
            "Incidence du raccord invalide.");
    if (l == 9 || h == 9) {
      const int plane = l == 9 ? h : l;
      const bool cap = plane == 0 || plane == 3;
      require(plane == (sy > 0 ? 4 : 1) || plane == (sz > 0 ? 5 : 2) || cap,
              "Appui du congé extérieur incorrect.");
      require(!(fillet_planes & (1u << plane)), "Arête du congé dupliquée.");
      fillet_planes |= 1u << plane;
      const double a0=c.FirstParameter(), a1=c.LastParameter();
      require(stock_vertex(c.Value(a0)) && stock_vertex(c.Value(a1)), "Extrémité du congé incorrecte.");
      if (cap) {
        const auto mid=c.Value((a0+a1)*.5);
        const auto target=fillet_center.Translated(gp_Vec(u)*((plane==0?-1.:1.)*d.half[0])+
              (gp_Vec(y_axis)*double(sy)+gp_Vec(z_axis)*double(sz))*(radius*std::sqrt(.5)));
        require(c.GetType()==GeomAbs_Circle && std::abs(a1-a0-std::acos(-1.)*.5)*radius<=tol &&
                mid.Distance(target)<=tol, "Arc du congé incorrect.");
      } else {
        require(c.GetType()==GeomAbs_Line && c.Line().Direction().IsParallel(u,near_angle(d.half[0])) &&
                std::abs(c.Value(a0).Distance(c.Value(a1))-2.*d.half[0])<=tol,
                "Génératrice du congé incohérente.");
      }
      continue;
    }
    if (l < 6 && h < 6) {
      require(c.GetType() == GeomAbs_Line && l % 3 != h % 3, "Arête extérieure hors bloc.");
      for (double t : {c.FirstParameter(), c.LastParameter()}) {
        const gp_Vec v(center, c.Value(t));
        std::array<double, 3> residual;
        for (int j = 0; j < 3; ++j) {
          const double component=v.Dot(gp_Vec(axes[j]));
          residual[j] = component - std::copysign(d.half[j],component);
        }
        const bool old_corner = !radius || sy*v.Dot(gp_Vec(y_axis)) < 0. || sz*v.Dot(gp_Vec(z_axis)) < 0.;
        require((old_corner && std::hypot((residual[0]-cosine*residual[2])/tilt_sine, residual[1], residual[2]) <= tol) ||
                (radius && stock_vertex(c.Value(t))),
                "Sommet du bloc hors domaine.");
      }
      ++lines;
      continue;
    }
    if (std::min(l, h) < 6) {
      require(c.GetType() == GeomAbs_Circle, "Entrée sans cercle.");
      ++mouths;
      continue;
    }
    const int small = std::min(l, h) - 6, branch = small == 0 ? -1 : 1;
    require((small == 0 || small == 1) && std::max(l, h) == 8 && certs.size() < 32,
            "Branche de perçage ambiguë.");
    const auto use = occurrence(edge, TopoDS::Face(faces(walls[small])));
    cross_drill_lab::Budget budget;
    budget.tolerance_mm = 1.e-7;
    budget.max_work = remaining;
    budget.max_leaves = leaves;
    budget.mesh_deflection_mm = deflection;
    auto cert = cross_drill_lab::certifyEdge(edge, a, b, r, R, budget, &frame, dy);
    if (!cert.ok)
      throw std::runtime_error("Certificat continu du raccord refusé : " + cert.reason);
    require(cert.sign == branch &&
                cert.phase_direction * (use.Orientation() == TopAbs_FORWARD ? 1 : -1) == -branch,
            "Orientation de branche incompatible avec la matière.");
    remaining -= cert.work;
    leaves -= cert.leaves;
    TopoDS_Vertex from, to;
    TopExp::Vertices(use, from, to, true);
    links.push_back(
        {vertices.Add(from), vertices.Add(to), branch, use.Orientation() == TopAbs_FORWARD});
    out.mesh_edges.push_back({edge, cert.mesh_segments});
    certs.push_back(std::move(cert));
  }
  const unsigned expected_fillet_planes = radius ?
      ((1u<<0)|(1u<<3)|(1u<<(sy>0 ? 4 : 1))|(1u<<(sz>0 ? 5 : 2))) : 0u;
  require(lines == (radius ? 11 : 12) && mouths == 4 && fillet_planes == expected_fillet_planes &&
          fillet_found == (radius != 0.), "Frontière extérieure ou entrée du bloc incorrecte.");
  for (int sign : {-1, 1}) {
    auto coverage = budgetedBranch(certs, links, sign, 1.e-7, remaining, r, R, dy, cosine);
    if (!coverage.ok)
      throw std::runtime_error("Couverture cyclique du raccord refusée : " + coverage.reason);
  }
  Counter arithmetic{0, remaining};
  Scope volume_scope(arithmetic);
  BRepClass3d_SolidClassifier classify(solid);
  auto probe = [&](const gp_Pnt& p, bool material) {
    classify.Perform(p, tol);
    require(classify.State() == (material ? TopAbs_IN : TopAbs_OUT),
            "Sonde matière/vide des perçages incohérente.");
  };
  probe(center, false);
  probe(mouth(2, (R + d.half[2]) * .5), false);
  probe(at(0, (r + d.half[0]) * .5), false);
  probe(at(1, (R + d.half[1]) * .5), true);
  const double step = .015 * std::min({r, d.half[0] - std::abs(dx) - r, d.half[1] - R, d.half[2] - R});
  for (int branch : {-1, 1})
    for (double t : {.39, 1.71, 3.44, 5.12}) {
      const double x = dx+r*std::cos(t), y = dy+r*std::sin(t), k = branch*std::sqrt(R*R-y*y);
      const double z=(k+cosine*x)/tilt_sine;
      const gp_Pnt p =
          center.Translated(gp_Vec(u) * x + gp_Vec(axes[1]) * y + gp_Vec(axes[2]) * z);
      const gp_Vec a = gp_Vec(u)*((x-dx)/r)+gp_Vec(axes[1])*((y-dy)/r),
                   b = gp_Vec(axes[1]) * (y / R) + (gp_Vec(z_axis)*tilt_sine-gp_Vec(u)*cosine)*(k/R);
      const double dot = a.Dot(b), den = 1. - dot * dot;
      for (int i : {-1, 1})
        for (int j : {-1, 1})
          probe(p.Translated(((a - b * dot) * i + (b - a * dot) * j) * (step / den)),
                i > 0 && j > 0);
    }
  auto volume =
      cross_drill_lab::idealVolume(2. * d.half[0], 2. * d.half[1], 2. * d.half[2], r, R, dy);
  if(cosine!=0.) {
    const I sine=root(I(1.)-square(I(cosine)));
    // The stock and the overlap integral each acquire a 1/sin(alpha)
    // Jacobian. The individual bore lengths remain perpendicular to the mouths.
    const I removed=I(2.)*pi()*(I(d.half[0])*square(I(R))+I(d.half[2])*square(I(r)));
    volume=(volume+removed)/sine-removed;
  }
  if (radius) volume = volume-I(2.)*I(d.half[0])*square(I(radius))*(I(1.)-pi()/I(4.));
  out.volume_low = volume.lo;
  out.volume_high = volume.hi;
  return out;
}

#include "drilled_shaft_verify.hxx"
