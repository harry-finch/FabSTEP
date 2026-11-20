// Independent fixture generator for OCCT 7.9.3. It uses the standard fillet
// builder on three box edges; no FabSTEP recognizer or certificate is linked.
// Pass an output directory to create the four binary STL witnesses.
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepTools.hxx>
#include <BRep_Tool.hxx>
#include <StlAPI_Writer.hxx>
#include <TopExp.hxx>
#include <TopoDS.hxx>
#include <array>
#include <string>

int main(int argc, char **argv) {
  if (argc != 2)
    return 1;
  const auto stock = BRepPrimAPI_MakeBox(20, 16, 10).Shape();
  TopTools_IndexedMapOfShape edges;
  TopExp::MapShapes(stock, TopAbs_EDGE, edges);
  const std::array<std::array<double, 3>, 2> radii = {{{2, 2.5, 3}, {3.1, 2.6, 1.8}}};
  for (int piece = 0; piece < 2; ++piece) {
    BRepFilletAPI_MakeFillet builder(stock);
    for (int i = 1; i <= edges.Extent(); ++i) {
      auto e = TopoDS::Edge(edges(i));
      TopoDS_Vertex a, b;
      TopExp::Vertices(e, a, b);
      const auto p = BRep_Tool::Pnt(a), q = BRep_Tool::Pnt(b);
      if (p.Distance(gp::Origin()) > 1e-7 && q.Distance(gp::Origin()) > 1e-7)
        continue;
      auto d = gp_Vec(p, q);
      builder.Add(radii[piece][std::abs(d.X()) > 1 ? 0 : std::abs(d.Y()) > 1 ? 1 : 2], e);
    }
    builder.Build();
    if (!builder.IsDone())
      return 1;
    const auto shape = builder.Shape();
    for (bool fine : {false, true}) {
      BRepTools::Clean(shape);
      BRepMesh_IncrementalMesh mesh(shape, fine ? .008 : .02, false, piece ? .18 : .22, false);
      const std::string file =
          std::string(argv[1]) + "/corner-" + std::to_string(piece) + (fine ? "-fine.stl" : "-coarse.stl");
      StlAPI_Writer writer;
      writer.ASCIIMode() = false;
      if (!writer.Write(shape, file.c_str()))
        return 1;
    }
  }
}
