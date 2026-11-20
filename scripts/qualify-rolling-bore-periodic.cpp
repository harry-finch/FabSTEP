// Compare a complete annular STL region to its periodic rational face. This
// qualification reads a public STEP witness, not a source of export authority.
#include "../crates/fabstep-cad/src/rolling_bore_certificate.hxx"
#include <STEPControl_Reader.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <fstream>
#include <iostream>
using rolling_bore::Point;
using rolling_bore::Tri;
void need(bool okay, const char *message) {
  if (!okay)
    throw std::runtime_error(message);
}

int main(int argc, char **argv) try {
  if (argc != 3)
    return 2;
  STEPControl_Reader reader;
  need(reader.ReadFile(argv[1]) == IFSelect_RetDone &&
           reader.TransferRoots() == 1,
       "STEP input");
  TopoDS_Face patch;
  for (TopExp_Explorer it(reader.OneShape(), TopAbs_FACE); it.More();
       it.Next()) {
    auto f = TopoDS::Face(it.Current());
    if (BRepAdaptor_Surface(f, false).GetType() == GeomAbs_BSplineSurface) {
      need(patch.IsNull(), "ambiguous reference patch");
      patch = f;
    }
  }
  need(!patch.IsNull(), "missing reference patch");
  std::ifstream stream(argv[2]);
  size_t nv = 0, nt = 0, inside = 0;
  need(bool(stream >> nv >> nt >> inside) && nv <= 30000 && nt <= 10000 &&
           inside < nv,
       "patch sizes");
  std::vector<Point> p(nv);
  std::vector<Tri> t(nt);
  for (auto &pt : p)
    for (auto &v : pt)
      need(bool(stream >> v) && std::isfinite(v), "vertex");
  for (auto &tri : t)
    for (auto &id : tri)
      need(bool(stream >> id) && id < nv, "triangle");
  unequal_corner::Certificate proof;
  if (!rolling_bore::certify(patch, p, t, proof, 1e-5, .049)) {
    std::cerr << "refused at triangle " << proof.triangles
              << " reason=" << (proof.reason ? proof.reason : "chart") << "\n";
    return 1;
  }
  p[inside][0] += .12;
  unequal_corner::Certificate negative;
  need(!rolling_bore::certify(patch, p, t, negative, 1e-5, .049),
       "accepted a displaced interior vertex");
  std::cout << "triangles=" << proof.triangles << " boxes=" << proof.boxes
            << " bound_mm=" << proof.maximum << " changed_interior=rejected\n";
} catch (const Standard_Failure &e) {
  std::cerr << e.GetMessageString() << "\n";
  return 1;
} catch (const std::exception &e) {
  std::cerr << e.what() << "\n";
  return 1;
}
