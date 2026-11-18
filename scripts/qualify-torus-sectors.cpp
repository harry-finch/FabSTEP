// Independent reference for a partially swept, rounded annular sector.
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepTools.hxx>
#include <BRep_Tool.hxx>
#include <GProp_GProps.hxx>
#include <Geom2dAdaptor_Curve.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>

namespace {
constexpr double pi = 3.14159265358979323846;
constexpr double sweep = 80 * pi / 180;
constexpr double inner = 3, major = 8, minor = .5, height = 6;
constexpr double outer = major + minor, shoulder = height - minor;
constexpr double tol = 1e-7;
void require(bool ok, const char* detail) {
    if (!ok) throw std::runtime_error(detail);
}
bool near(double a, double b, double tolerance = tol) {
    return std::isfinite(a) && std::abs(a - b) <= tolerance;
}
gp_Pnt polar(double radius, double angle, double z) {
    return {radius * std::cos(angle), radius * std::sin(angle), z};
}
TopoDS_Shape reference() {
    const gp_Pnt a(inner,0,0), b(outer,0,0), c(outer,0,shoulder),
                 d(major,0,height), e(inner,0,height);
    const gp_Circ arc(gp_Ax2(gp_Pnt(major,0,shoulder), gp_Dir(0,-1,0), gp::DX()), minor);
    BRepBuilderAPI_MakeWire wire;
    wire.Add(BRepBuilderAPI_MakeEdge(a,b).Edge());
    wire.Add(BRepBuilderAPI_MakeEdge(b,c).Edge());
    wire.Add(BRepBuilderAPI_MakeEdge(arc,0,pi/2).Edge());
    wire.Add(BRepBuilderAPI_MakeEdge(d,e).Edge());
    wire.Add(BRepBuilderAPI_MakeEdge(e,a).Edge());
    require(wire.IsDone(), "meridian wire");
    return BRepPrimAPI_MakeRevol(BRepBuilderAPI_MakeFace(wire.Wire(),true).Face(),
                                gp_Ax1(gp::Origin(),gp::DZ()), sweep).Shape();
}
double referenceVolume() {
    // Integrate (outer_radius(z)^2 - inner^2) * sweep/2 over height.
    return sweep/2 * (shoulder*(outer*outer-inner*inner) + minor*(major*major-inner*inner)
                    + major*pi*minor*minor/2 + 2*minor*minor*minor/3);
}
int count(const TopoDS_Shape& shape, TopAbs_ShapeEnum type) {
    TopTools_IndexedMapOfShape map;
    TopExp::MapShapes(shape,type,map);
    return map.Extent();
}
TopAbs_Orientation use(const TopoDS_Face& face, const TopoDS_Edge& edge) {
    int n=0;
    auto result=TopAbs_EXTERNAL;
    for(TopExp_Explorer it(face,TopAbs_EDGE); it.More(); it.Next()) {
        if(it.Current().IsSame(edge)) { ++n; result=it.Current().Orientation(); }
    }
    require(n==1 && (result==TopAbs_FORWARD || result==TopAbs_REVERSED), "edge use");
    return result;
}
TopoDS_Shape roundTrip(const TopoDS_Shape& solid, const std::filesystem::path& path) {
    STEPControl_Writer writer;
    require(writer.Transfer(solid,STEPControl_ManifoldSolidBrep)==IFSelect_RetDone,"transfer");
    require(writer.Write(path.c_str())==IFSelect_RetDone,"write");
    require(std::filesystem::file_size(path)<1024*1024,"reference file size");
    STEPControl_Reader reader;
    require(reader.ReadFile(path.c_str())==IFSelect_RetDone && reader.TransferRoots()==1,"read");
    return reader.OneShape();
}
void chart(const TopoDS_Face& face, const TopoDS_Edge& edge, std::array<int,4>& sides) {
    double u0,u1,v0,v1,first,last;
    BRepTools::UVBounds(face,u0,u1,v0,v1);
    require(near(u1-u0,sweep) && near(v1-v0,pi/2), "torus rectangular parameter spans");
    auto curve=BRep_Tool::CurveOnSurface(edge,face,first,last);
    require(!curve.IsNull() && std::isfinite(first) && std::isfinite(last) && first<last,"missing pcurve");
    require(Geom2dAdaptor_Curve(curve,first,last).GetType()==GeomAbs_Line,"nonlinear trim");
    const auto a=curve->Value(first), b=curve->Value(last);
    const auto full=[](double a,double b,double lo,double hi) {
        return near(std::min(a,b),lo) && near(std::max(a,b),hi);
    };
    if (near(a.X(),b.X()) && full(a.Y(),b.Y(),v0,v1)) {
        if (near(a.X(),u0)) { ++sides[0]; return; }
        if (near(a.X(),u1)) { ++sides[1]; return; }
    }
    if (near(a.Y(),b.Y()) && full(a.X(),b.X(),u0,u1)) {
        if (near(a.Y(),v0)) { ++sides[2]; return; }
        if (near(a.Y(),v1)) { ++sides[3]; return; }
    }
    throw std::runtime_error("torus trim outside rectangle");
}
struct Result { double volume, area; };
Result verify(const TopoDS_Shape& shape, const gp_Trsf& placement) {
    require(count(shape,TopAbs_SOLID)==1 && count(shape,TopAbs_SHELL)==1 &&
            count(shape,TopAbs_FACE)==7 && BRepCheck_Analyzer(shape,true).IsValid(),"invalid reference solid");
    auto pos=[&](gp_Pnt p) {return p.Transformed(placement);};
    auto dir=[&](gp_Dir d) {return d.Transformed(placement);};
    TopoDS_Face face;
    for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
        auto f=TopoDS::Face(it.Current());
        if (BRepAdaptor_Surface(f).GetType()==GeomAbs_Torus) {
            require(face.IsNull(),"multiple toroidal faces"); face=f;
        }
    }
    require(!face.IsNull() && count(face,TopAbs_WIRE)==1 && count(face,TopAbs_EDGE)==4, "sector topology");
    const auto torus=BRepAdaptor_Surface(face).Torus();
    require(near(torus.MajorRadius(),major) && near(torus.MinorRadius(),minor)
        && torus.Location().Distance(pos({0,0,shoulder}))<tol
        && torus.Axis().Direction().IsParallel(dir(gp::DZ()),tol), "torus placement/radii");
    TopTools_IndexedDataMapOfShapeListOfShape adjacency;
    TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,adjacency);
    std::array<int,4> chartSides{}, physicalSides{};
    for(TopExp_Explorer it(face,TopAbs_EDGE);it.More();it.Next()) {
        const auto edge=TopoDS::Edge(it.Current());
        chart(face,edge,chartSides);
        const auto& incident=adjacency.FindFromKey(edge);
        require(incident.Extent()==2, "nonmanifold boundary");
        const auto neighbor=TopoDS::Face(incident.First().IsSame(face) ? incident.Last() : incident.First());
        require(!neighbor.IsSame(face) && use(face,edge)!=use(neighbor,edge), "unshared/opposed incidences");
        BRepAdaptor_Curve curve(edge);
        require(curve.GetType()==GeomAbs_Circle, "noncircular boundary");
        const auto circle=curve.Circle();
        const auto support=BRepAdaptor_Surface(neighbor);
        const double span=curve.LastParameter()-curve.FirstParameter();
        const auto mid=curve.Value((curve.FirstParameter()+curve.LastParameter())/2);
        if (near(circle.Radius(),minor)) {
            int side=circle.Location().Distance(pos(polar(major,0,shoulder)))<tol ? 0 : 1;
            double angle=side*sweep;
            auto normal=dir(gp_Dir(-std::sin(angle),std::cos(angle),0));
            require(near(span,pi/2) && circle.Location().Distance(pos(polar(major,angle,shoulder)))<tol &&
                    circle.Axis().Direction().IsParallel(normal,tol) && support.GetType()==GeomAbs_Plane &&
                    support.Plane().Axis().Direction().IsParallel(normal,tol) &&
                    mid.Distance(pos(polar(major+minor/std::sqrt(2.),angle,shoulder+minor/std::sqrt(2.))))<tol,
                    "meridian circle or radial plane");
            ++physicalSides[side];
        } else {
            const bool upper=near(circle.Radius(),major);
            const double radius=upper ? major : outer, z=upper ? height : shoulder;
            require(near(circle.Radius(),radius) && near(span,sweep) &&
                    circle.Location().Distance(pos({0,0,z}))<tol && mid.Distance(pos(polar(radius,sweep/2,z)))<tol &&
                    circle.Axis().Direction().IsParallel(dir(gp::DZ()),tol), "parallel circle");
            if (upper) require(support.GetType()==GeomAbs_Plane &&
                support.Plane().Axis().Direction().IsParallel(dir(gp::DZ()),tol),"top neighbor");
            else require(support.GetType()==GeomAbs_Cylinder && near(support.Cylinder().Radius(),outer) &&
                support.Cylinder().Axis().Direction().IsParallel(dir(gp::DZ()),tol) &&
                gp_Lin(support.Cylinder().Axis()).Distance(pos(gp::Origin()))<tol,"cylindrical neighbor");
            ++physicalSides[upper ? 2 : 3];
        }
    }
    for (int i=0;i<4;++i) require(chartSides[i]==1 && physicalSides[i]==1,"missing boundary");
    GProp_GProps volume, area;
    BRepGProp::VolumeProperties(shape,volume,1e-12,true);
    BRepGProp::SurfaceProperties(face,area,1e-12);
    require(near(volume.Mass(),referenceVolume()),"volume");
    require(near(area.Mass(),sweep*minor*(major*pi/2+minor)),"area");
    for(auto p : {std::pair{polar(major+.25,sweep/2,height-.25),true},
                  std::pair{polar(major+.45,sweep/2,height-.05),false},
                  std::pair{polar(5.,-sweep/2,2.),false}}) {
        BRepClass3d_SolidClassifier classifier(shape,pos(p.first),tol);
        require(classifier.State()==(p.second?TopAbs_IN:TopAbs_OUT),"material probe");
    }
    return {volume.Mass(),area.Mass()};
}
} // namespace
int main(int argc, char** argv) {
    try {
        require(argc==2,"artifact directory");
        const std::filesystem::path output=argv[1];
        const auto solid=reference();
        gp_Trsf identity, moved;
        moved.SetRotation(gp_Ax1(gp::Origin(), gp_Dir(1,2,3)),.73);
        moved.SetTranslationPart(gp_Vec(12,-7,4));
        double maxError=0;
        for(const auto& item : {std::pair{"nominal",identity},std::pair{"oblique",moved}}) {
            const auto shape=BRepBuilderAPI_Transform(solid,item.second,true).Shape();
            const auto loaded=roundTrip(shape,output/(std::string(item.first)+".step"));
            for(const auto& s : {shape,loaded})
                maxError=std::max(maxError,std::abs(verify(s,item.second).volume-referenceVolume()));
            // Same torus, volume and rectangle widths, wrong angular position.
            gp_Trsf shift; shift.SetRotation(gp_Ax1(gp::Origin(),gp::DZ()),.12);
            bool rejected=false;
            try { verify(loaded,item.second*shift); }
            catch(const std::runtime_error& e) {
                rejected=std::string(e.what())=="meridian circle or radial plane" ||
                         std::string(e.what())=="parallel circle";
            }
            require(rejected,"azimuth adversary");
        }
        std::ofstream report(output/"report.json");
        report<<std::setprecision(16)<<"{\"fixtures\":2,\"before_after_checks\":4,\"wrong_azimuth_refusals\":2,"
              <<"\"reference_volume_mm3\":"<<referenceVolume()<<",\"max_volume_error_mm3\":"<<maxError<<"}\n";
        return report.good()?0:1;
    } catch(const std::exception& e) {std::cerr<<e.what()<<"\n";return 1;}
      catch(const Standard_Failure& e) {std::cerr<<e.GetMessageString()<<"\n";return 1;}
}
