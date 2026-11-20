// Shared I/O for the bounded public OCCT benchmarks R-130 and R-131.
#pragma once
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <GeomConvert.hxx>
#include <Geom_BSplineSurface.hxx>
#include <Geom_BezierSurface.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <filesystem>
#include <sstream>

namespace atlas_bench {
inline TopoDS_Face triangle(gp_Pnt a, gp_Pnt b, gp_Pnt c) {
  return BRepBuilderAPI_MakeFace(
             BRepBuilderAPI_MakePolygon(a, b, c, true).Wire())
      .Face();
}
inline TopTools_IndexedMapOfShape source(const std::string &path,
                                         bool transfer) {
  TopoDS_Shape shape;
  if (path == "angled-slabs" || path == "near-slabs") {
    BRep_Builder builder;
    TopoDS_Compound comp;
    builder.MakeCompound(comp);
    for (int side : {0, 1}) {
      TColgp_Array2OfPnt poles(1, 2, 1, 2);
      TColStd_Array2OfReal weights(1, 2, 1, 2);
      for (int i : {1, 2})
        for (int j : {1, 2}) {
          double x = (i == 1 ? -200. : 200.) + (side ? 75. : -75.);
          double y = j == 1 ? -1. : 1.;
          poles.SetValue(
              i, j,
              {x, y,
               .3 * x + (i == j ? .0001 : 0.) +
                   (side ? (path == "angled-slabs" ? .04 : 1e-8) : 0.)});
          weights.SetValue(i, j, i == j ? 1.01 : 1.);
        }
      Handle(Geom_BezierSurface) bez = new Geom_BezierSurface(poles, weights);
      auto face = BRepBuilderAPI_MakeFace(
                      GeomConvert::SurfaceToBSplineSurface(bez), 1.e-7)
                      .Face();
      builder.Add(comp, face);
    }
    for (int i = -160; i <= 160; ++i)
      builder.Add(comp, triangle({i * 6., 20, 0}, {i * 6. + 1, 20, 0},
                                 {i * 6., 21, 0}));
    shape = comp;
  } else if (path == "crossing" || path == "disjoint" || path == "open-wire") {
    BRep_Builder builder;
    TopoDS_Compound comp;
    builder.MakeCompound(comp);
    if (path == "open-wire") {
      auto original = triangle({0, 0, 0}, {2, 0, 0}, {0, 2, 0});
      auto open = TopoDS::Face(original.EmptyCopied());
      TopoDS_Wire wire;
      builder.MakeWire(wire);
      TopExp_Explorer it(original, TopAbs_EDGE);
      builder.Add(wire, it.Current());
      it.Next();
      builder.Add(wire, it.Current());
      builder.Add(open, wire);
      builder.Add(comp, open);
    } else {
      double h = path == "crossing" ? 0. : 4.;
      builder.Add(comp, triangle({-900, -5, 0}, {2, -5, 0}, {2, 5, 0}));
      builder.Add(comp, triangle({-2, 0, h - 1}, {-2, 0, h + 1}, {900, 0, h}));
      for (int i = -160; i <= 160; ++i)
        builder.Add(comp, triangle({i * 6., 20, 0}, {i * 6. + 1, 20, 0},
                                   {i * 6., 21, 0}));
      builder.Add(comp,
                  BRepPrimAPI_MakeBox(gp_Pnt(40, 30, 0), 2, 2, 2).Shape());
    }
    shape = comp;
  } else {
    if (std::filesystem::file_size(path) > 8 * 1024 * 1024)
      throw std::runtime_error("Public reference exceeds 8 MiB");
    STEPControl_Reader reader;
    if (reader.ReadFile(path.c_str()) != IFSelect_RetDone ||
        reader.TransferRoots() != 1)
      throw std::runtime_error("Expected one STEP root");
    shape = reader.OneShape();
  }
  if (transfer) {
    STEPControl_Writer writer;
    std::stringstream buffer;
    if (writer.Transfer(shape, STEPControl_AsIs) != IFSelect_RetDone ||
        writer.WriteStream(buffer) != IFSelect_RetDone)
      throw std::runtime_error("Benchmark STEP transfer failed");
    STEPControl_Reader reader;
    if (reader.ReadStream("benchmark.step", buffer) != IFSelect_RetDone ||
        reader.TransferRoots() != 1)
      throw std::runtime_error("Benchmark STEP reimport failed");
    shape = reader.OneShape();
  }
  TopTools_IndexedMapOfShape faces;
  TopExp::MapShapes(shape, TopAbs_FACE, faces);
  if (faces.Extent() < 1 || faces.Extent() > 1500)
    throw std::runtime_error("Benchmark accepts 1..1500 faces");
  return faces;
}
} // namespace atlas_bench
