// Compare whole-shape OCCT and the bounded dispatcher on both sides of a leaf boundary.
#include "../crates/fabstep-cad/src/torus_atlas_interference.hxx"
#include "../crates/fabstep-cad/src/torus_atlas_fan.hxx"
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <ElCLib.hxx>
#include <ElSLib.hxx>
#include <gp_Torus.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <TopExp.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <IntTools_FaceFace.hxx>
#include <STEPControl_Reader.hxx>
#include <iostream>

namespace {
TopoDS_Face triangle(gp_Pnt a, gp_Pnt b, gp_Pnt c) {
  return BRepBuilderAPI_MakeFace(BRepBuilderAPI_MakePolygon(a, b, c, true).Wire()).Face();
}

bool bounded_valid(atlas_interference::Checker &checker, const TopTools_IndexedMapOfShape &faces) {
  try {
    checker.run(faces);
    return true;
  } catch (const atlas_interference::Failure &) {
    return false;
  }
}

void open_face_wire() {
  const auto closed = triangle({0, 0, 0}, {3, 0, 0}, {0, 2, 0});
  auto face = TopoDS::Face(closed.EmptyCopied());
  TopExp_Explorer edge(closed, TopAbs_EDGE);
  BRep_Builder builder;
  TopoDS_Wire wire;
  builder.MakeWire(wire);
  builder.Add(wire, edge.Current());
  edge.Next();
  builder.Add(wire, edge.Current());
  builder.Add(face, wire);
  TopTools_IndexedMapOfShape faces;
  faces.Add(face);
  atlas_interference::Checker checker;
  BRepAlgoAPI_Check reference(face, false, true);
  if (reference.HasErrors() || reference.IsValid() || bounded_valid(checker, faces))
    throw std::runtime_error("Face with an open boundary was accepted.");
  std::cout << "open face wire rejected\n";
}

void check_case(bool split, bool intersect) {
  const double far = split ? 900. : 12.;
  const double height = intersect ? 0. : 4.;
  const auto a = triangle({-far, -5, 0}, {2, -5, 0}, {2, 5, 0});
  const auto b = triangle({-2, 0, -1 + height}, {-2, 0, 1 + height}, {far, 0, height});
  BRep_Builder builder;
  TopoDS_Compound shape;
  builder.MakeCompound(shape);
  builder.Add(shape, a);
  builder.Add(shape, b);
  for (int i = -160; i < 161; ++i) {
    const double x = i * 6.;
    builder.Add(shape, triangle({x, 20, 0}, {x + 1, 20, 0}, {x, 21, 0}));
  }
  // A legitimate shared coedge and shared vertices must be accepted.
  builder.Add(shape, BRepPrimAPI_MakeBox(gp_Pnt(40, 30, 0), 2, 2, 2).Shape());
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  BRepAlgoAPI_Check whole(shape, false, true);
  atlas_interference::Checker checker;
  const bool valid = bounded_valid(checker, faces);
  size_t owner_a = 0, owner_b = 0;
  for (const auto &node : checker.tree)
    if (node.left < 0) {
      for (size_t j = node.first; j < node.last; ++j) {
        if (checker.faces[j].face.IsSame(a))
          owner_a = node.first + 1;
        if (checker.faces[j].face.IsSame(b))
          owner_b = node.first + 1;
      }
    }
  if (!owner_a || !owner_b || (split && owner_a == owner_b) || whole.HasErrors() ||
      whole.IsValid() == intersect || valid == intersect)
    throw std::runtime_error("Unexpected whole/bounded interference result or leaf assignment.");
  std::cout << "whole=" << whole.IsValid() << " bounded=" << valid << " leaf-a=" << owner_a
            << " leaf-b=" << owner_b << " faces=" << faces.Extent() << "\n";
}
void check_torus_filter() {
  using atlas_interference::TorusFilter;
  const auto require = [](bool ok, const char *reason) {
    if (!ok)
      throw std::runtime_error(reason);
  };
  for (const auto &axis : {gp_Dir(0, 0, 1), gp_Dir(2, -1, 4)}) {
    const gp_Pnt center(3104., -1093., 209.);
    const gp_Torus support(gp_Ax3(center, axis), 30., 3.);
    const auto torus = BRepBuilderAPI_MakeFace(support, 0., 2 * M_PI, -.9, .9).Face();
    gp_Pnt point;
    gp_Vec du, dv;
    ElSLib::D1(.83, .32, support, point, du, dv);
    const gp_Vec n = du.Crossed(dv).Normalized();
    du.Normalize();
    for (bool piercing : {false, true}) {
      const auto anchor = piercing ? point : center;
      const auto tri = triangle(anchor.Translated(-n), anchor.Translated(n + du), anchor.Translated(n - du));
      TopoDS_Compound compound;
      BRep_Builder builder;
      builder.MakeCompound(compound);
      builder.Add(compound, torus);
      builder.Add(compound, tri);
      BRepAlgoAPI_Check reference(compound, false, true);
      require(!reference.HasErrors() && reference.IsValid() != piercing, "Unexpected torus oracle");

      TopTools_IndexedMapOfShape faces;
      faces.Add(torus);
      faces.Add(tri);
      atlas_interference::Checker checker;
      require(bounded_valid(checker, faces) != piercing, "Invalid diagonal torus verdict");
      if (axis.Z() == 1.)
        require(checker.torus_disjoint(checker.faces[0], checker.faces[1]) != piercing, "Invalid exclusion");
      const int left = checker.build(0, 1), right = checker.build(1, 2);
      bool accepted = true;
      try {
        checker.inspect(left, right);
      } catch (const atlas_interference::Failure &) {
        accepted = false;
      }
      require(accepted != piercing, "Invalid cross-leaf torus verdict");
      require(checker.pairs == 1 && checker.groups == 2, "Filtered pair was not charged");

      if (axis.Z() != 1. || piercing)
        continue;
      const auto &near = checker.faces[1];
      const auto certified = [&](TorusFilter::Budget budget, const TopoDS_Face &owner, const TopoDS_Face &other) {
        TorusFilter filter;
        filter.remaining = budget;
        filter.neighbors.resize(1);
        size_t id = filter.record(owner, 1.e-5);
        return filter.excludes(id, 0, other, 1.e-5, near.bounds);
      };
      require(certified({}, torus, tri), "Expected exclusion in the torus hole");
      // Partial collection or comparison cannot prove the whole pair disjoint.
      for (auto member : {&TorusFilter::Budget::visits, &TorusFilter::Budget::torus_boxes,
                          &TorusFilter::Budget::neighbor_boxes, &TorusFilter::Budget::comparisons}) {
        TorusFilter::Budget budget;
        budget.*member = 1;
        require(!certified(budget, torus, tri), "Budget exhaustion skipped a pair");
      }

      // Keep the physical boundary even if an imported trim does not follow
      // its support. Exercise both operand orders with an undersized face box.
      const auto detached = [&](const TopoDS_Face &owner, const TopoDS_Face &extra) {
        auto result = TopoDS::Face(owner.EmptyCopied());
        for (const auto &f : {owner, extra})
          for (TopExp_Explorer it(f, TopAbs_WIRE); it.More(); it.Next())
            builder.Add(result, it.Current());
        return result;
      };
      require(!certified({}, detached(torus, tri), tri), "Detached torus boundary skipped");
      require(!certified({}, torus, detached(tri, torus)), "Detached neighbor boundary skipped");
      const auto spindle = BRepBuilderAPI_MakeFace(gp_Torus(gp_Ax3(center, axis), 1., 4.), 0., .5, 2.9, 3.2).Face();
      TorusFilter filter;
      require(filter.record(spindle, 1.e-5) == 0, "Spindle torus accepted by ring-distance test");
    }
  }
  std::cout << "Torus exclusions, native penetrations, detached trims and budget fallbacks passed.\n";
}
// Independent near-planar seam in a torus of outer radius 10 mm. Its
// singular loft used to fail the OCCT self-face check at the collapsed pole.
void near_planar_fan() {
  const double angle = 1.044, span = .0175, rad = 10.;
  const gp_Pnt center(2.75, 17.2, -9.1);
  const gp_Ax3 frame(center, gp_Dir(1, 0, 0), gp_Dir(0, 1, 0));
  auto torus = BRepBuilderAPI_MakeFace(gp_Torus(frame, 8., 2.), angle, angle + span, 0., .4).Face();
  TopoDS_Edge rim;
  for (TopExp_Explorer i(torus, TopAbs_EDGE); i.More(); i.Next()) {
    const auto edge = TopoDS::Edge(i.Current());
    auto c = BRepAdaptor_Curve(edge);
    if (c.GetType() == GeomAbs_Circle && std::abs(c.Circle().Radius() - rad) < 1e-12)
      rim = TopoDS::Edge(edge.Oriented(TopAbs_FORWARD));
  }
  if (rim.IsNull())
    throw std::runtime_error("Missing torus rim.");
  const auto edge = BRepAdaptor_Curve(rim);
  const auto m = edge.Value(angle + .9 * span);
  const gp_Pnt pole(center.X() + 3e-8, center.Y() + (m.Y() - center.Y()) * 1.14,
                    center.Z() + (m.Z() - center.Z()) * 1.14);
  auto loft = [&](gp_Pnt apex) {
    BRepOffsetAPI_ThruSections builder(false, true, Precision::Confusion());
    builder.CheckCompatibility(false);
    builder.SetMutableInput(false);
    builder.AddWire(BRepBuilderAPI_MakeWire(rim).Wire());
    builder.AddVertex(BRepBuilderAPI_MakeVertex(apex).Vertex());
    builder.Build();
    TopExp_Explorer f(builder.Shape(), TopAbs_FACE);
    if (!f.More())
      throw std::runtime_error("Missing ruled patch.");
    return TopoDS::Face(f.Current());
  };
  BRepBuilderAPI_Sewing sewing(1e-6, true, true, true, false);
  sewing.Add(torus);
  sewing.Add(loft(pole));
  sewing.Perform();
  TopoDS_Face original;
  for (TopExp_Explorer it(sewing.SewedShape(), TopAbs_FACE); it.More(); it.Next()) {
    auto f = TopoDS::Face(it.Current());
    if (BRepAdaptor_Surface(f).GetType() == GeomAbs_Torus)
      torus = f;
    else
      original = f;
  }
  if (original.IsNull())
    throw std::runtime_error("Missing sewn fan.");
  TopTools_IndexedMapOfShape torus_edges;
  TopExp::MapShapes(torus, TopAbs_EDGE, torus_edges);
  const auto q = atlas_fan::controls(original);
  if (!q || atlas_fan::repair(loft(pole.Translated(gp_Vec(.1, 0, 0)))))
    throw std::runtime_error("Incorrect near-planar eligibility.");
  // A pole between a circular arc and its tangent control reverses the
  // projected angular derivative somewhere inside the span.
  const gp_Pnt folded((q->first.XYZ() + q->last.XYZ() + 2 * q->middle.XYZ()) / 4.);
  if (atlas_fan::circle_injective(folded, q->first, q->middle, q->last, edge.Circle().Axis().Direction(), 1e-7))
    throw std::runtime_error("Folded rational fan was eligible.");
  BRepAlgoAPI_Check old(original, false, true);
  TopTools_IndexedMapOfShape singular;
  singular.Add(original);
  atlas_interference::Checker individual;
  if (old.HasErrors() || bounded_valid(individual, singular) != old.IsValid())
    throw std::runtime_error("Unexpected individual near-planar fan verdict.");
  auto candidate = atlas_fan::repair(original);
  if (old.IsValid() == bool(candidate))
    throw std::runtime_error("Incorrect apex repair routing.");
  if (!candidate)
    candidate = atlas_fan::quadratic(original, *q);
  if (!candidate || atlas_fan::repair(*candidate))
    throw std::runtime_error("Non-idempotent circular reconstruction.");
  TopTools_IndexedMapOfShape material;
  TopExp::MapShapes(*candidate, TopAbs_EDGE, material);
  if (!material.Contains(q->edge) || !torus_edges.Contains(q->edge))
    throw std::runtime_error("Shared circle was replaced.");
  BRep_Builder builder;
  TopoDS_Compound pair;
  builder.MakeCompound(pair);
  builder.Add(pair, torus);
  builder.Add(pair, *candidate);
  BRepAlgoAPI_Check complete(pair, false, true);
  if (complete.HasErrors() || !complete.IsValid() || !BRepCheck_Analyzer(pair, true, false, true).IsValid())
    throw std::runtime_error("Reconstructed torus seam is invalid.");
  TopTools_IndexedMapOfShape adjacent;
  TopExp::MapShapes(pair, TopAbs_FACE, adjacent);
  atlas_interference::Checker checker;
  if (adjacent.Extent() != 2 || !bounded_valid(checker, adjacent))
    throw std::runtime_error("Shared torus seam was refused in one leaf.");
  // Force a cross-leaf check of exactly these two incident faces.
  const int left = checker.build(0, 1);
  const int right = checker.build(1, 2);
  const auto before = checker.groups;
  checker.inspect(left, right);
  if (checker.groups != before + 1)
    throw std::runtime_error("Shared torus seam missed the cross-leaf check.");
  const auto evaluated = BRepAdaptor_Surface(*candidate);
  for (int i = 0; i <= 32; ++i)
    for (int j = 0; j <= 8; ++j) {
      const double u = i / 32., v = j / 8., h = span / 2.;
      const double t = angle + 2 * std::atan2(u * std::sin(h), 1 - u + u * std::cos(h));
      const gp_Pnt expected(pole.XYZ() * (1. - v) + edge.Value(t).XYZ() * v);
      if (evaluated.Value(u, v).Distance(expected) > 1e-11)
        throw std::runtime_error("Ruled surface drift.");
    }
  std::cout << "shared rational torus fan, legacy BOP valid=" << old.IsValid() << "\n";
}

// Mirrors the individual surface test in OCCT's BOPAlgo_CheckerSI 7.9 for
// residual spline faces; the complete solid still requires its inter-face check.
void check_ruled_faces(const char *path) {
  auto shape = [&] {
    STEPControl_Reader reader;
    if (reader.ReadFile(path) != IFSelect_RetDone || reader.TransferRoots() != 1)
      throw std::runtime_error("Cannot read one STEP root.");
    return reader.OneShape();
  }();
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  int checked = 0, invalid = 0, incomplete = 0;
  for (int index = 1; index <= faces.Extent(); ++index) {
    const auto face = TopoDS::Face(faces(index));
    const auto type = BRepAdaptor_Surface(face).GetType();
    if (type != GeomAbs_BSplineSurface && type != GeomAbs_BezierSurface)
      continue;
    IntTools_FaceFace check;
    check.Perform(face, face, false);
    ++checked;
    if (!check.IsDone())
      ++incomplete;
    else if (check.Lines().Length() || check.Points().Length())
      ++invalid;
  }
  std::cout << "spline_faces=" << checked << " self_intersections=" << invalid << " incomplete=" << incomplete
            << "\n";
  if (!checked || invalid || incomplete)
    throw std::runtime_error("A ruled face could not be verified.");
}
} // namespace
int main(int argc, char **argv) try {
  if (argc == 3 && std::string(argv[1]) == "--ruled-faces") {
    check_ruled_faces(argv[2]);
    return 0;
  }
  if (argc != 1)
    throw std::runtime_error("Expected --ruled-faces path.step or no arguments.");
  check_torus_filter();
  open_face_wire();
  near_planar_fan();
  for (bool split : {false, true})
    for (bool intersect : {false, true})
      check_case(split, intersect);
} catch (const std::exception &failure) {
  std::cerr << failure.what() << "\n";
  return 1;
}
