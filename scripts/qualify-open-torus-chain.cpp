// Qualification only: the measured patch is deliberately an open shell.
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRep_Tool.hxx>
#include <GProp_GProps.hxx>
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

namespace {
constexpr double tol=5e-8, pi=3.14159265358979323846;
void check(bool test,const char* why) {if(!test) throw std::runtime_error(why);}
struct Geometry {
    gp_Pnt center;
    gp_Dir axis, start;
    double major,minor,sweep,v0,v1;
    std::array<gp_Vec,2> extensions;
    gp_Dir radial(double u) const {return start.Rotated(gp_Ax1(center,axis),u);}
    gp_Circ circle(int end) const {
        const auto radialEnd=radial(end*sweep);
        return {gp_Ax2(center.Translated(gp_Vec(radialEnd)*major),radialEnd.Crossed(axis),radialEnd),minor};
    }
    gp_Pnt point(int end,double v) const {
        return center.Translated(gp_Vec(radial(end*sweep))*(major+minor*std::cos(v)) +
                                 gp_Vec(axis)*(minor*std::sin(v)));
    }
    double torusArea() const {
        return sweep*minor*(major*(v1-v0)+minor*(std::sin(v1)-std::sin(v0)));
    }
    double cylinderArea(int end) const {return extensions[end].Magnitude()*minor*(v1-v0);}
};

Geometry measured(std::istream& input) {
    std::array<double,20> v;
    for(auto& value:v) check(bool(input>>value) && std::isfinite(value),"invalid measurement");
    input>>std::ws; check(input.eof(),"extra measurement");
    Geometry g{{v[0],v[1],v[2]},{v[3],v[4],v[5]},{v[6],v[7],v[8]},v[9],v[10],v[11],v[12],v[13],
               {gp_Vec(v[14],v[15],v[16]),gp_Vec(v[17],v[18],v[19])}};
    check(g.major>g.minor && g.minor>0 && g.sweep>0 && g.sweep<pi &&
          std::abs(g.v1-g.v0-pi/2)<1e-9 && g.axis.IsNormal(g.start,1e-10),"sector domain");
    for(int end=0;end<2;++end) {
        const auto direction=g.axis.Crossed(g.radial(end*g.sweep));
        check(g.extensions[end].Magnitude()>tol &&
              g.extensions[end].Crossed(gp_Vec(direction)).Magnitude()<tol &&
              (end ? 1.:-1.)*g.extensions[end].Dot(gp_Vec(direction))>0,"tangent extension");
    }
    return g;
}

TopoDS_Shape build(const Geometry& g,bool shorten) {
    const gp_Torus torus(gp_Ax3(g.center,g.axis,g.start),g.major,g.minor);
    BRepBuilderAPI_Sewing sew(tol);
    sew.Add(BRepBuilderAPI_MakeFace(torus,0,g.sweep,g.v0,g.v1).Face());
    for(int i=0;i<2;++i) {
        auto edge=BRepBuilderAPI_MakeEdge(g.circle(i),g.v0,g.v1).Edge();
        const double fraction=(i==0 && shorten) ? .85 : 1.;
        sew.Add(BRepPrimAPI_MakePrism(edge,g.extensions[i]*fraction).Shape());
    }
    sew.Perform();
    check(sew.NbFreeEdges()==8 && !sew.NbMultipleEdges(),"sewn edge counts");
    return sew.SewedShape();
}

TopAbs_Orientation use(const TopoDS_Face& face,const TopoDS_Edge& edge) {
    for(TopExp_Explorer it(face,TopAbs_EDGE);it.More();it.Next())
        if(it.Current().IsSame(edge)) return it.Current().Orientation();
    throw std::runtime_error("missing coedge");
}

struct Result {double areaError=0.,pointError=0.;};
Result verify(const TopoDS_Shape& shell,const Geometry& g) {
    Result result;
    auto count=[&](TopAbs_ShapeEnum type) {
        TopTools_IndexedMapOfShape items;TopExp::MapShapes(shell,type,items);return items.Extent();
    };
    check(count(TopAbs_SHELL)==1 && count(TopAbs_SOLID)==0 && count(TopAbs_FACE)==3 &&
          count(TopAbs_EDGE)==10 && BRepCheck_Analyzer(shell,true).IsValid(),"open shell validity");
    TopTools_IndexedDataMapOfShapeListOfShape map;
    TopExp::MapShapesAndAncestors(shell,TopAbs_EDGE,TopAbs_FACE,map);
    std::array<TopoDS_Face,2> tubes;
    TopoDS_Face wall;
    std::array<int,2> shared{}, far{};
    int boundary=0;
    for(int i=1;i<=map.Extent();++i) {
        const auto edge=TopoDS::Edge(map.FindKey(i));
        const BRepAdaptor_Curve curve(edge);
        const auto& faces=map.FindFromIndex(i);
        if(faces.Extent()==1) {
            ++boundary;
            const BRepAdaptor_Surface owner(TopoDS::Face(faces.First()));
            if(owner.GetType()!=GeomAbs_Cylinder || curve.GetType()!=GeomAbs_Circle) continue;
            const auto circle=curve.Circle();
            const bool isFirst=circle.Location().Distance(g.circle(0).Location().Translated(g.extensions[0]))<tol;
            const int side=isFirst?0:1;
            const auto expected=g.circle(side).Location().Translated(g.extensions[side]);
            check(circle.Location().Distance(expected)<tol,"distal circle displaced");
            const auto distalMid=g.point(side,(g.v0+g.v1)/2).Translated(g.extensions[side]);
            const double midError=curve.Value((curve.FirstParameter()+curve.LastParameter())/2).Distance(distalMid);
            check(std::abs(circle.Radius()-g.minor)<tol && midError<tol &&
                  std::abs(curve.LastParameter()-curve.FirstParameter()-g.v1+g.v0)*g.minor<tol,
                  "distal arc extent");
            result.pointError=std::max(result.pointError,midError);
            ++far[side]; continue;
        }
        check(faces.Extent()==2 && !faces.First().IsSame(faces.Last()),"boundary incidence");
        auto a=TopoDS::Face(faces.First()), b=TopoDS::Face(faces.Last());
        if(BRepAdaptor_Surface(b).GetType()==GeomAbs_Torus) std::swap(a,b);
        const BRepAdaptor_Surface t(a),c(b);
        check(t.GetType()==GeomAbs_Torus && c.GetType()==GeomAbs_Cylinder &&
              curve.GetType()==GeomAbs_Circle && use(a,edge)!=use(b,edge),"shared torus/cylinder circle");
        check(wall.IsNull() || wall.IsSame(a),"torus identity");
        const auto tor=t.Torus();
        check(tor.Location().Distance(g.center)<tol && tor.Axis().Direction().IsParallel(g.axis,1e-9) &&
              std::abs(tor.MajorRadius()-g.major)<tol && std::abs(tor.MinorRadius()-g.minor)<tol,
              "torus placement and radii");
        wall=a;
        const auto circle=curve.Circle();
        const int side=circle.Location().Distance(g.circle(0).Location())<tol ? 0 : 1;
        const auto cylinder=c.Cylinder();
        check(std::abs(circle.Radius()-g.minor)<tol && std::abs(cylinder.Radius()-g.minor)<tol &&
              circle.Location().Distance(g.circle(side).Location())<tol &&
              cylinder.Axis().Direction().IsParallel(gp_Dir(g.extensions[side]),1e-9) &&
              gp_Lin(cylinder.Axis()).Distance(circle.Location())<tol &&
              circle.Axis().Direction().IsParallel(cylinder.Axis().Direction(),1e-9),"tangent axes/radii");
        const double lo=curve.FirstParameter(), hi=curve.LastParameter();
        auto first=curve.Value(lo),last=curve.Value(hi);
        if(first.Distance(g.point(side,g.v1))<tol) std::swap(first,last);
        const double error=std::max({first.Distance(g.point(side,g.v0)),last.Distance(g.point(side,g.v1)),
            curve.Value((lo+hi)/2).Distance(g.point(side,(g.v0+g.v1)/2))});
        result.pointError=std::max(result.pointError,error);
        check(error<tol && std::abs(hi-lo-g.v1+g.v0)*g.minor<tol,"shared quarter arc");
        tubes[side]=b; ++shared[side];
    }
    check(boundary==8 && shared==std::array<int,2>{1,1} && far==shared &&
          !tubes[0].IsSame(tubes[1]),"open connected torus chain");
    auto area=[](const TopoDS_Face& f) {
        GProp_GProps mass; BRepGProp::SurfaceProperties(f,mass,1e-12);return mass.Mass();
    };
    result.areaError=std::max({std::abs(area(wall)-g.torusArea()),
        std::abs(area(tubes[0])-g.cylinderArea(0)),std::abs(area(tubes[1])-g.cylinderArea(1))});
    check(result.areaError<1e-7,"face area disagreement");
    return result;
}
} // namespace

int main(int argc,char** argv) {
    try {
        check(argc==2,"artifact directory");
        const std::filesystem::path dir=argv[1];
        const Geometry geometry=measured(std::cin);
        const auto shell=build(geometry,false);
        const auto before=verify(shell,geometry);
        const auto path=dir/"measured-open-shell.step";
        STEPControl_Writer writer;
        check(writer.Transfer(shell,STEPControl_ShellBasedSurfaceModel)==IFSelect_RetDone &&
              writer.Write(path.c_str())==IFSelect_RetDone && std::filesystem::file_size(path)<1024*1024,
              "write shell STEP");
        STEPControl_Reader reader;
        check(reader.ReadFile(path.c_str())==IFSelect_RetDone && reader.TransferRoots()==1,"read shell STEP");
        const auto after=verify(reader.OneShape(),geometry);
        bool rejected=false;
        try {verify(build(geometry,true),geometry);}
        catch(const std::runtime_error& e) {rejected=std::string(e.what())=="distal circle displaced";}
        check(rejected,"shortened tube must fail");
        std::ofstream report(dir/"open-chain-report.json");
        report<<std::setprecision(16)<<"{\"shells\":1,\"solids\":0,\"faces\":3,\"shared_edges\":2,"
            <<"\"free_edges\":8,\"nominal_area_mm2\":"<<geometry.torusArea()+geometry.cylinderArea(0)+geometry.cylinderArea(1)
            <<",\"max_face_area_error_mm2\":"<<std::max(before.areaError,after.areaError)
            <<",\"max_arc_sample_error_mm\":"<<std::max(before.pointError,after.pointError)
            <<",\"shortened_tube_rejected\":true}\n";
        return report.good()?0:1;
    } catch(const std::exception& e){std::cerr<<e.what()<<'\n';return 1;}
      catch(const Standard_Failure& e){std::cerr<<e.GetMessageString()<<'\n';return 1;}
}
