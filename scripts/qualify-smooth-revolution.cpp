// C2 feasibility laboratory: fixed analytic inputs, no STL recognition or production calls.
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <Geom_BezierCurve.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Adaptor3d_Curve.hxx>
#include <GProp_GProps.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TColgp_Array1OfPnt.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopoDS.hxx>
#include <gp_Ax1.hxx>
#include <gp_Trsf.hxx>
#include <array>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>

namespace {
constexpr double pi=3.14159265358979323846;
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
struct Case { const char* name; std::array<double,4> radius; double height; bool placed; };
// Bernstein product integral: integral B_i^3 B_j^3 = C(3,i)C(3,j)/(7 C(6,i+j)).
double volume(const Case& c){
  const double c3[]={1,3,3,1},c6[]={1,6,15,20,15,6,1};
  double sum=0.;for(int i=0;i<4;++i)for(int j=0;j<4;++j)
    sum+=c.radius[i]*c.radius[j]*c3[i]*c3[j]/(7.*c6[i+j]);
  return pi*c.height*sum;
}
double radius(const Case& c,double t){
  auto p=c.radius;for(int n=3;n>0;--n)for(int i=0;i<n;++i)p[i]=(1.-t)*p[i]+t*p[i+1];
  return p[0];
}
gp_Trsf placement(const Case& c){
  gp_Trsf t;if(c.placed){t.SetRotation(gp_Ax1(gp::Origin(),gp_Dir(1,2,3)),.731);t.SetTranslationPart(gp_Vec(17,-9,23));}return t;
}
TopoDS_Shape build(const Case& c){
  require(c.height>0.,"height must be positive");
  TColgp_Array1OfPnt poles(1,4);
  for(int i=0;i<4;++i){require(c.radius[i]>0.,"positive Bernstein radii required");poles(i+1)=gp_Pnt(c.radius[i],0,c.height*i/3.);}
  Handle(Geom_BezierCurve) profile=new Geom_BezierCurve(poles);
  BRepBuilderAPI_MakeWire wire;
  wire.Add(BRepBuilderAPI_MakeEdge(gp_Pnt(0,0,0),poles(1)).Edge());
  wire.Add(BRepBuilderAPI_MakeEdge(profile).Edge());
  wire.Add(BRepBuilderAPI_MakeEdge(poles(4),gp_Pnt(0,0,c.height)).Edge());
  wire.Add(BRepBuilderAPI_MakeEdge(gp_Pnt(0,0,c.height),gp_Pnt(0,0,0)).Edge());
  require(wire.IsDone(),"profile wire failed");
  BRepBuilderAPI_MakeFace face(wire.Wire(),true);require(face.IsDone(),"profile face failed");
  BRepPrimAPI_MakeRevol revol(face.Face(),gp_Ax1(gp::Origin(),gp::DZ()),2*pi,true);
  require(revol.IsDone(),"revolution failed");
  return BRepBuilderAPI_Transform(revol.Shape(),placement(c),true).Shape();
}
struct Check {double measured,error;int probes;};
Check check(const TopoDS_Shape& s,const Case& c){
  require(BRepCheck_Analyzer(s,true).IsValid(),"invalid BRep");
  TopTools_IndexedMapOfShape solids;TopExp::MapShapes(s,TopAbs_SOLID,solids);
  require(solids.Extent()==1,"not one solid");
  int planes=0,revolutions=0,other=0;
  for(TopExp_Explorer it(s,TopAbs_FACE);it.More();it.Next()){
    switch(BRepAdaptor_Surface(TopoDS::Face(it.Current())).GetType()){
      case GeomAbs_Plane:++planes;break;
      case GeomAbs_SurfaceOfRevolution:{
        ++revolutions;
        BRepAdaptor_Surface surface(TopoDS::Face(it.Current()));
        const auto basis=surface.BasisCurve();
        std::array<gp_Pnt,4> actual;
        if(basis->GetType()==GeomAbs_BezierCurve){
          auto curve=basis->Bezier();require(curve->Degree()==3 && !curve->IsRational(),"unexpected Bezier basis");
          for(int i=0;i<4;++i)actual[i]=curve->Pole(i+1);
        }else if(basis->GetType()==GeomAbs_BSplineCurve){
          auto curve=basis->BSpline();
          require(curve->Degree()==3 && curve->NbPoles()==4 && curve->NbKnots()==2 &&
                  curve->Multiplicity(1)==4 && curve->Multiplicity(2)==4 && !curve->IsRational(),"unexpected spline basis");
          for(int i=0;i<4;++i)actual[i]=curve->Pole(i+1);
        }else require(false,"unsupported basis representation");
        bool forward=true,reverse=true;
        for(int i=0;i<4;++i){
          gp_Pnt expected(c.radius[i],0,c.height*i/3.);expected.Transform(placement(c));
          forward=forward && actual[i].Distance(expected)<1e-7;
          reverse=reverse && actual[3-i].Distance(expected)<1e-7;
        }
        require(forward || reverse,"physical meridian poles changed");
        auto axis=gp_Ax1(gp::Origin(),gp::DZ());axis.Transform(placement(c));
        require(surface.AxeOfRevolution().IsCoaxial(axis,1e-10,1e-7),"axis changed");
        require(std::abs(surface.LastUParameter()-surface.FirstUParameter()-2*pi)<1e-10,"incomplete angular domain");
        break;
      }
      default:++other;
    }
  }
  require(planes==2 && revolutions==1 && other==0,"surface families not preserved");
  GProp_GProps props;double error=BRepGProp::VolumePropertiesGK(s,props,1e-11,true,true);
  require(error>=0. && std::isfinite(error),"adaptive volume failed");
  require(std::abs(props.Mass()-volume(c))<=volume(c)*1e-8,"volume mismatch");
  int probes=0;
  for(double t:{.1,.25,.5,.75,.9})for(double factor:{.5,1.01})for(double a:{.17,2.2,4.8}){
    double r=radius(c,t)*factor;
    gp_Pnt p(r*std::cos(a),r*std::sin(a),c.height*t);p.Transform(placement(c));
    BRepClass3d_SolidClassifier classifier(s,p,1e-7);
    require(classifier.State()==(factor<1?TopAbs_IN:TopAbs_OUT),"material probe mismatch");++probes;
  }
  return {props.Mass(),error,probes};
}
}
int main(int argc,char** argv){
  if(argc!=2)return 2;
  try{
    std::filesystem::path out(argv[1]);std::filesystem::create_directories(out);
    const Case cases[]={
      {"bulge",{5,9,9,5},20,false},
      {"neck",{8,3,3,8},20,false},
      {"inflection",{4,9,3,8},24,false},
      {"placed",{4,9,3,8},24,true},
      {"scaled",{6,13.5,4.5,12},36,true}
    };
    std::ofstream report(out/"report.json");report<<std::setprecision(17)<<"{\"laboratory_only\":true,\"source_recognition\":false,\"cases\":[";
    bool first=true;
    for(const auto& c:cases){
      auto shape=build(c);const auto before=check(shape,c);
      const auto path=(out/(std::string(c.name)+".step")).string();
      STEPControl_Writer writer;require(writer.Transfer(shape,STEPControl_AsIs)==IFSelect_RetDone,"STEP transfer failed");
      require(writer.Write(path.c_str())==IFSelect_RetDone,"STEP write failed");
      STEPControl_Reader reader;require(reader.ReadFile(path.c_str())==IFSelect_RetDone,"STEP read failed");
      require(reader.NbRootsForTransfer()==1 && reader.TransferRoots()==1,"STEP roots mismatch");
      const auto after=check(reader.OneShape(),c);
      if(!first)report<<',';first=false;
      report<<"{\"name\":\""<<c.name<<"\",\"planes\":2,\"revolutions\":1,\"physical_poles_verified\":true,\"expected_volume\":"<<volume(c)<<",\"before_volume\":"<<before.measured<<",\"after_volume\":"<<after.measured<<",\"integration_error_estimate\":"<<after.error<<",\"material_probes_before_and_after\":"<<before.probes+after.probes<<'}';
    }
    // Same volume and radial envelope do not certify the physical meridian.
    Case reversed=cases[2];std::swap(reversed.radius[0],reversed.radius[3]);std::swap(reversed.radius[1],reversed.radius[2]);
    require(std::abs(volume(reversed)-volume(cases[2]))<1e-8,"reversed reference volume differs");
    bool rejected=false;
    try{check(build(cases[2]),reversed);}catch(const std::runtime_error& e){rejected=std::string(e.what())=="physical meridian poles changed";}
    require(rejected,"reversed physical meridian escaped validation");
    Case axis_touch=cases[0];axis_touch.radius[0]=0.;rejected=false;
    try{build(axis_touch);}catch(const std::runtime_error& e){rejected=std::string(e.what())=="positive Bernstein radii required";}
    require(rejected,"axis-touch reference escaped domain guard");
    report<<"],\"reversed_meridian_rejected\":true,\"axis_touch_rejected\":true,\"pass\":true}\n";return 0;
  }catch(const Standard_Failure& e){std::cerr<<e.GetMessageString()<<'\n';return 1;}
   catch(const std::exception& e){std::cerr<<e.what()<<'\n';return 1;}
}
