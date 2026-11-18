// Exercise both branches at a small measured byte cap with real OCCT transfers.
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <DESTEP_Parameters.hxx>
#include <GProp_GProps.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp_Explorer.hxx>
#include <algorithm>
#include <cmath>
#include <cstring>
#include <iostream>
#include <memory>
#include <sstream>
#include <stdexcept>

namespace {
constexpr size_t step_document_limit = 128 * 1024 * 1024;
enum Status { document_too_large, write_failed };
struct CadFailure {
  Status code;
  const char *message;
};
#include "../crates/fabstep-cad/src/trim_native_step.hxx"
void require(bool condition) {
  if (!condition)
    throw std::runtime_error("compact STEP control failed");
}
void exercise(int count) {
  std::vector<TopoDS_Shape> shapes;
  for (int root = 0; root < count; ++root) {
    gp_Ax2 axis(gp_Pnt(root * 20, 0, 0), gp_Dir(0, 0, 1));
    shapes.push_back(BRepPrimAPI_MakeCylinder(axis, 3 + root, 7).Shape());
  }
  std::vector<bool> attempts;
  auto transfer = [&](STEPControl_Writer &writer, bool compact) {
    attempts.push_back(compact);
    DESTEP_Parameters params;
    params.WriteAssembly = DESTEP_Parameters::WriteMode_Assembly_Off;
    params.WriteSurfaceCurMode = !compact;
    for (int i = 0; i < count; ++i) {
      params.WriteProductName = (std::string("Part-") + std::to_string(i)).c_str();
      require(writer.Transfer(shapes[i], STEPControl_ManifoldSolidBrep, params) == IFSelect_RetDone);
    }
  };
  auto full = write_bounded_step(transfer);
  require(attempts == std::vector<bool>{false});
  attempts.clear();
  auto compact = write_bounded_step(transfer, full->size() - 1);
  require((attempts == std::vector<bool>{false, true}) && compact->size() < full->size());
  std::string bytes(reinterpret_cast<const char *>(compact->data()), compact->size());
  std::istringstream stream(bytes);
  STEPControl_Reader reader;
  require(reader.ReadStream("compressed.step", stream) == IFSelect_RetDone &&
          reader.NbRootsForTransfer() == count);
  for (int i = 0; i < count; ++i) {
    require(bytes.find("Part-" + std::to_string(i)) != std::string::npos);
    require(reader.TransferOneRoot(i + 1) && reader.NbShapes() == i + 1);
    auto restored = reader.Shape(i + 1);
    require(BRepCheck_Analyzer(restored, true).IsValid());
    TopExp_Explorer it(restored, TopAbs_SOLID);
    require(it.More());
    it.Next();
    require(!it.More());
    GProp_GProps props;
    BRepGProp::VolumeProperties(restored, props);
    require(std::abs(props.Mass() - M_PI * std::pow(3 + i, 2) * 7) < 1e-8);
    require(std::abs(props.CentreOfMass().X() - 20 * i) < 1e-8);
  }
  attempts.clear();
  bool refused = false;
  try {
    write_bounded_step(transfer, compact->size() - 1);
  } catch (const CadFailure &e) {
    refused = e.code == document_too_large;
  }
  require(refused && (attempts == std::vector<bool>{false, true}));
  std::cout << "roots=" << count << " full=" << full->size() << " compact=" << compact->size()
            << " reimport=pass undersize=refused\n";
}
} // namespace
int main() {
  try {
    exercise(1);
    exercise(2);
    return 0;
  } catch (const std::exception &e) {
    std::cerr << e.what() << "\n";
  } catch (const CadFailure &e) {
    std::cerr << e.message << "\n";
  }
  return 1;
}
