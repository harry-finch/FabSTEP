// Fixed, independent reference solids. No mesh recognition or production calls.
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepBndLib.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepLib.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <GProp_GProps.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Solid.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <set>
#include <limits>
#include <stdexcept>
#include <string>

namespace {
constexpr double L=20.,W=16.,H=10.,r=2.,pi=3.14159265358979323846;
constexpr double tolerance=1e-7;

void require(bool ok,const char* why){if(!ok)throw std::runtime_error(why);}
bool close(double a,double b,double eps=tolerance){return std::isfinite(a)&&std::abs(a-b)<=eps;}
int count(const TopoDS_Shape& shape,TopAbs_ShapeEnum type){
  TopTools_IndexedMapOfShape items;TopExp::MapShapes(shape,type,items);return items.Extent();
}

TopoDS_Shape corner(){
  auto box=BRepPrimAPI_MakeBox(L,W,H).Shape();
  BRepFilletAPI_MakeFillet maker(box);
  TopTools_IndexedMapOfShape edges; TopExp::MapShapes(box,TopAbs_EDGE,edges);
  int selected=0;
  for(int i=1;i<=edges.Extent();++i){
    TopoDS_Vertex a,b;TopExp::Vertices(TopoDS::Edge(edges(i)),a,b);
    if(BRep_Tool::Pnt(a).Distance(gp::Origin())<tolerance || BRep_Tool::Pnt(b).Distance(gp::Origin())<tolerance){
      maker.Add(r,TopoDS::Edge(edges(i)));++selected;
    }
  }
  require(selected==3,"corner must have three incident box edges");
  maker.Build();require(maker.IsDone(),"native three-edge fillet failed");return maker.Shape();
}

TopoDS_Face polygon(std::initializer_list<gp_Pnt> points){
  BRepBuilderAPI_MakePolygon wire;for(const auto& point:points)wire.Add(point);
  wire.Close();return BRepBuilderAPI_MakeFace(wire.Wire(),true).Face();
}

// Horizontal caps through the actual endpoints. The physical branch solves
// x+y+sqrt(2(1+b²)xy)=r(z); a circular same-z stack is a different surface.
TopoDS_Face cap(double z,double radius,double slope){
  const double b2=slope*slope,center=radius/(1.-b2);
  gp_Elips ellipse(gp_Ax2(gp_Pnt(center,center,z),gp::DZ(),gp_Dir(-1.,-1.,0.)),
                   radius*std::sqrt(1.+b2)/(1.-b2),radius/std::sqrt(1.-b2));
  const double theta=std::asin(std::sqrt((1.-b2)/2.));
  const gp_Pnt x(radius,0.,z),y(0.,radius,z),c(L,0.,z),d(L,W,z),e(0.,W,z);
  BRepBuilderAPI_MakeWire wire;
  wire.Add(BRepBuilderAPI_MakeEdge(ellipse,-theta,theta).Edge());
  for(const auto& ends:{std::pair{x,c},std::pair{c,d},std::pair{d,e},std::pair{e,y}})
    wire.Add(BRepBuilderAPI_MakeEdge(ends.first,ends.second).Edge());
  return BRepBuilderAPI_MakeFace(wire.Wire(),true).Face();
}

bool admitted(double a,double b){
  const double end=a+H*b;
  return std::isfinite(a)&&std::isfinite(b)&&std::abs(b)>0.&&std::abs(b)<1.&&
         std::min(a,end)>0.&&std::max(a,end)<std::min(L,W);
}
TopoDS_Shape envelope(double a,double b){
  require(admitted(a,b),"linear radius fixture outside analytic domain");
  const double end=a+b*H, q=std::sqrt(1.+2.*b*b);
  const gp_Vec axis=gp_Vec(b,b,1.).Normalized()*std::copysign(1.,b);
  const gp_Vec ex(-1.,0.,0.),ey(0.,-1.,0.);
  const gp_Vec radial=ex+ey-axis*(ex+ey).Dot(axis);
  const double alpha=std::asin(std::abs(b)/q), half=std::acos(-b*b/(1.+b*b))/2.;
  gp_Cone cone(gp_Ax3(gp_Pnt(0.,0.,-a/b),gp_Dir(axis),gp_Dir(radial)),alpha,0.);
  // Finite supporting patch and slab enclose both fixed fixtures with margin.
  const auto support=BRepBuilderAPI_MakeFace(cone,-half,half,.1,200.).Face();
  const auto slab=BRepPrimAPI_MakeBox(gp_Pnt(-50.,-50.,0.),100.,100.,H).Shape();
  BRepAlgoAPI_Common clipped(support,slab);
  require(clipped.IsDone()&&count(clipped.Shape(),TopAbs_FACE)==1,"cone/slab must yield one face");
  TopExp_Explorer face(clipped.Shape(),TopAbs_FACE);
  BRepBuilderAPI_Sewing sew(tolerance);
  sew.Add(face.Current());sew.Add(cap(0.,a,b));sew.Add(cap(H,end,b));
  sew.Add(polygon({{a,0,0},{L,0,0},{L,0,H},{end,0,H}}));
  sew.Add(polygon({{0,a,0},{0,W,0},{0,W,H},{0,end,H}}));
  sew.Add(polygon({{L,0,0},{L,W,0},{L,W,H},{L,0,H}}));
  sew.Add(polygon({{0,W,0},{L,W,0},{L,W,H},{0,W,H}}));
  sew.Perform();require(sew.NbFreeEdges()==0&&sew.NbMultipleEdges()==0,"sewing left open or multiple edges");
  const auto sewn=sew.SewedShape();
  require(sewn.ShapeType()==TopAbs_SHELL,"sewing did not return one shell");
  auto solid=BRepBuilderAPI_MakeSolid(TopoDS::Shell(sewn)).Solid();
  require(BRepLib::OrientClosedSolid(solid),"failed orienting sewn solid");return solid;
}

double idealVolume(double a,double b){
  // Integrating area by u=sqrt(x/y) gives K=integral_0^inf u/(u²+ku+1)² du.
  const double k=std::sqrt(2.*(1.+b*b)),h=std::sqrt(1.-k*k/4.);
  const double K=(2.-k*std::acos(k/2.)/h)/(4.-k*k),end=a+H*b;
  return L*W*H-K*H*(a*a+a*end+end*end)/3.;
}
double mass(const TopoDS_Shape& shape){
  GProp_GProps p;BRepGProp::VolumeProperties(shape,p,1e-11,true);return p.Mass();
}

TopoDS_Shape roundTrip(const TopoDS_Shape& shape,const std::string& path){
  require(BRepCheck_Analyzer(shape,true).IsValid(),"invalid generated reference");
  STEPControl_Writer writer;require(writer.Transfer(shape,STEPControl_ManifoldSolidBrep)==IFSelect_RetDone,"STEP transfer failed");
  require(writer.Write(path.c_str())==IFSelect_RetDone,"STEP write failed");
  require(std::filesystem::file_size(path)<1024*1024,"fixture STEP exceeds size cap");
  STEPControl_Reader reader;require(reader.ReadFile(path.c_str())==IFSelect_RetDone&&reader.TransferRoots()==1,"STEP read failed");
  const auto result=reader.OneShape();
  require(count(result,TopAbs_SOLID)==1&&count(result,TopAbs_SHELL)==1&&BRepCheck_Analyzer(result,true).IsValid(),"invalid reimport");
  require(count(result,TopAbs_FACE)<=10&&count(result,TopAbs_EDGE)<=32,"fixture topology exceeds size caps");
  Bnd_Box bounds;BRepBndLib::AddOptimal(result,bounds,false,false);double x,y,z,X,Y,Z;bounds.Get(x,y,z,X,Y,Z);
  require(close(x,0.)&&close(y,0.)&&close(z,0.)&&close(X,L)&&close(Y,W)&&close(Z,H),"unexpected reimport bounds");
  return result;
}

int axisIndex(const gp_Dir& direction){
  for(int i=1;i<=3;++i)if(close(std::abs(direction.Coord(i)),1.,1e-12))return i;
  throw std::runtime_error("axis is not aligned with a box edge");
}
gp_Pnt coordinate(gp_Pnt p,int axis,double value){
  if(axis==1)p.SetX(value);
  else if(axis==2)p.SetY(value);
  else p.SetZ(value);
  return p;
}
bool matches(const gp_Pnt& first,const gp_Pnt& last,const gp_Pnt& a,const gp_Pnt& b){
  return (first.Distance(a)<tolerance&&last.Distance(b)<tolerance) ||
         (first.Distance(b)<tolerance&&last.Distance(a)<tolerance);
}
TopAbs_Orientation faceUse(const TopoDS_Face& face,const TopoDS_Edge& key){
  int uses=0;TopAbs_Orientation direction=TopAbs_EXTERNAL;
  for(TopExp_Explorer it(face,TopAbs_EDGE);it.More();it.Next())if(it.Current().IsSame(key)){
    ++uses;direction=it.Current().Orientation();
  }
  require(uses==1&&(direction==TopAbs_FORWARD||direction==TopAbs_REVERSED),"ambiguous edge use");return direction;
}
struct Result{
  int planes=0,cylinders=0,spheres=0,cones=0,circleJoins=0,circleCaps=0,ellipseCaps=0,lineJoins=0,probes=0;
  double native=0.,ideal=0.;
};
void probe(const TopoDS_Shape& shape,const gp_Pnt& point,bool inside,Result& r){
  BRepClass3d_SolidClassifier classifier(shape,point,tolerance);
  require(classifier.State()==(inside?TopAbs_IN:TopAbs_OUT),"material probe mismatch");++r.probes;
}
Result qualify(const TopoDS_Shape& shape,bool d,double a,double b){
  Result result;
  const gp_Pnt center(r,r,r);
  std::set<int> cylinderAxes,cornerArcs,capAxes;
  std::set<std::pair<int,int>> contacts;
  std::set<bool> ellipseLevels;
  const std::array<double,3> lengths{L,W,H};
  for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()){
    BRepAdaptor_Surface s(TopoDS::Face(it.Current()));
    switch(s.GetType()){
      case GeomAbs_Plane: ++result.planes;break;
      case GeomAbs_Cylinder:{
        const auto c=s.Cylinder();
        require(d&&close(c.Radius(),r)&&gp_Lin(c.Axis()).Distance(center)<tolerance,"wrong cylinder");
        require(cylinderAxes.insert(axisIndex(c.Axis().Direction())).second,"repeated cylinder axis");
        ++result.cylinders;break;
      }
      case GeomAbs_Sphere: require(d&&close(s.Sphere().Radius(),r)&&s.Sphere().Location().Distance(center)<tolerance,"wrong sphere");++result.spheres;break;
      case GeomAbs_Cone:{
        const auto c=s.Cone();
        require(!d&&c.Apex().Distance(gp_Pnt(0.,0.,-a/b))<tolerance&&
          std::abs(c.Axis().Direction().Dot(gp_Dir(b,b,1.)))>1.-1e-12&&
          close(std::abs(std::sin(c.SemiAngle())),std::abs(b)/std::sqrt(1.+2.*b*b)),"wrong tilted cone");
        ++result.cones;break;
      }
      default:require(false,"unexpected surface support");
    }
  }
  require(result.planes==6&&result.cylinders==(d?3:0)&&result.spheres==(d?1:0)&&result.cones==(d?0:1),"wrong surface census");
  TopTools_IndexedDataMapOfShapeListOfShape owners;
  TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,owners);
  for(int i=1;i<=owners.Extent();++i){
    const auto edge=TopoDS::Edge(owners.FindKey(i));
    if(BRep_Tool::Degenerated(edge))continue;
    const auto& faces=owners.FindFromIndex(i);
    require(faces.Extent()==2,"edge incidence differs from two");
    const auto first=TopoDS::Face(faces.First()),last=TopoDS::Face(faces.Last());
    if(first.IsSame(last))continue; // Periodic seam on one support.
    require(faceUse(first,edge)!=faceUse(last,edge),"shared edge uses not opposed");
    const BRepAdaptor_Surface f(first),g(last);BRepAdaptor_Curve curve(edge);
    const gp_Pnt start=curve.Value(curve.FirstParameter()),end=curve.Value(curve.LastParameter());
    const auto F=f.GetType(),G=g.GetType();
    auto pair=[&](GeomAbs_SurfaceType x,GeomAbs_SurfaceType y){return (F==x&&G==y)||(F==y&&G==x);};
    if(F==GeomAbs_Plane&&G==GeomAbs_Plane){require(curve.GetType()==GeomAbs_Line,"nonlinear plane edge");continue;}
    if(pair(GeomAbs_Sphere,GeomAbs_Cylinder)){
      require(curve.GetType()==GeomAbs_Circle&&curve.Circle().Location().Distance(center)<tolerance&&
        close(curve.Circle().Radius(),r)&&close(curve.LastParameter()-curve.FirstParameter(),pi/2.),"incorrect sphere seam");
      const auto cylinder=(F==GeomAbs_Cylinder?f:g).Cylinder();
      const int axis=axisIndex(cylinder.Axis().Direction());
      const int j=axis%3+1,k=j%3+1;
      const gp_Pnt p=coordinate(center,j,0.),q=coordinate(center,k,0.);
      require(cornerArcs.insert(axis).second&&matches(start,end,p,q)&&
        axis==axisIndex(curve.Circle().Axis().Direction()),"sphere/cylinder tangent arc not on expected great circle");
      ++result.circleJoins;continue;
    }
    if(pair(GeomAbs_Plane,GeomAbs_Cylinder)&&curve.GetType()==GeomAbs_Circle){
      require(close(curve.Circle().Radius(),r)&&close(curve.LastParameter()-curve.FirstParameter(),pi/2.),"incorrect cylinder cap");
      const auto c=curve.Circle();const int axis=axisIndex(c.Axis().Direction());
      const gp_Pnt capCenter=coordinate(center,axis,lengths[axis-1]);
      require(c.Location().Distance(capCenter)<tolerance&&capAxes.insert(axis).second,"wrong remote rounded end");
      ++result.circleCaps;continue;
    }
    if(pair(GeomAbs_Plane,GeomAbs_Cone)&&curve.GetType()==GeomAbs_Ellipse){
      const auto ellipse=curve.Ellipse(); const auto c=ellipse.Location();const double z=c.Z(),rz=a+b*z;
      const gp_Pnt expected(rz/(1.-b*b),rz/(1.-b*b),z);
      const double half=std::asin(std::sqrt((1.-b*b)/2.));
      require((close(z,0.)||close(z,H))&&c.Distance(expected)<tolerance&&
        close(ellipse.MajorRadius(),rz*std::sqrt(1.+b*b)/(1.-b*b))&&close(ellipse.MinorRadius(),rz/std::sqrt(1.-b*b))&&
        close(curve.LastParameter()-curve.FirstParameter(),2.*half),"incorrect end ellipse");
      require(axisIndex(ellipse.Axis().Direction())==3&&matches(start,end,{rz,0.,z},{0.,rz,z})&&
        ellipseLevels.insert(close(z,H)).second,"incorrect ellipse plane or endpoints");
      ++result.ellipseCaps;continue;
    }
    require((pair(GeomAbs_Plane,GeomAbs_Cylinder)||pair(GeomAbs_Plane,GeomAbs_Cone))&&curve.GetType()==GeomAbs_Line,"unexpected blend boundary");
    const auto plane=(F==GeomAbs_Plane?f:g).Plane();
    const int normal=axisIndex(plane.Axis().Direction());
    require(plane.Distance(gp::Origin())<tolerance,"wrong contact plane");
    if(d){
      const auto cyl=(F==GeomAbs_Cylinder?f:g).Cylinder();
      const int axis=axisIndex(cyl.Axis().Direction());
      const gp_Pnt p=coordinate(center,normal,0.),q=coordinate(p,axis,lengths[axis-1]);
      require(normal!=axis&&contacts.emplace(axis,normal).second&&matches(start,end,p,q)&&
        close(plane.Distance(cyl.Location()),r),"wrong cylinder/plane tangent generator");
    }else{
      require(normal==1||normal==2,"unexpected cone contact plane");
      const gp_Pnt p=coordinate({0.,0.,0.},3-normal,a),q=coordinate({0.,0.,H},3-normal,a+b*H);
      require(contacts.emplace(3,normal).second&&matches(start,end,p,q),"wrong cone/plane tangent generator");
    }
    ++result.lineJoins;
  }
  require(result.circleJoins==(d?3:0)&&result.circleCaps==(d?3:0)&&result.ellipseCaps==(d?0:2)&&result.lineJoins==(d?6:2),"wrong trim census");
  if(d){
    result.ideal=L*W*H-r*r*(1.-pi/4.)*(L+W+H-3.*r)-r*r*r*(1.-pi/6.);
    for(double t:{.35,.85}){
      const double x=r*(1.-t);
      probe(shape,{x,x,x},3.*t*t<1.,result);
      probe(shape,{L*.6,x,x},2.*t*t<1.,result);
      probe(shape,{x,W*.6,x},2.*t*t<1.,result);
      probe(shape,{x,x,H*.6},2.*t*t<1.,result);
    }
  }else{
    result.ideal=idealVolume(a,b);
    for(double z:{1.,H/2.,H-1.}){
      const double rz=a+b*z,threshold=rz/(2.+std::sqrt(2.*(1.+b*b)));
      for(double delta:{-.005,.005})probe(shape,{threshold+delta,threshold+delta,z},delta>0.,result);
      // A quarter-circle stack skips the inclination of the moving spheres.
      const double stacked=rz/(2.+std::sqrt(2.));
      probe(shape,{(threshold+stacked)*.5,(threshold+stacked)*.5,z},true,result);
    }
  }
  result.native=mass(shape);
  require(close(result.native,result.ideal,1e-5),"independent analytic volume mismatch");return result;
}

void reportCase(std::ostream& out,const char* name,const Result& v){
  out << "{\"case\":\"" << name << "\",\"solids\":1,\"planes\":" << v.planes << ",\"cylinders\":" << v.cylinders
      << ",\"spheres\":" << v.spheres << ",\"cones\":" << v.cones << ",\"sphere_cylinder_arcs\":" << v.circleJoins
      << ",\"cylinder_cap_arcs\":" << v.circleCaps << ",\"elliptic_cap_arcs\":" << v.ellipseCaps
      << ",\"contact_generators\":" << v.lineJoins << ",\"material_probes\":" << v.probes
      << ",\"volume_reimport_mm3\":" << v.native << ",\"analytic_volume_evaluated_mm3\":" << v.ideal << '}';
}
}
int main(int argc,char** argv){
  if(argc!=2)return 1;
  const std::string dir=argv[1];std::ofstream report(dir+"/report.json");
  try{
    require(!admitted(0.,.2)&&!admitted(2.,0.)&&!admitted(1.,-.2)&&!admitted(2.,1.)&&!admitted(2.,1.1)
      &&!admitted(15.,.2)&&!admitted(2.,std::numeric_limits<double>::quiet_NaN()),"invalid law admitted");
    auto D=qualify(roundTrip(corner(),dir+"/trihedral.step"),true,0.,0.);
    auto G=qualify(roundTrip(envelope(3.,.2),dir+"/linear-increasing.step"),false,3.,.2);
    auto M=qualify(roundTrip(envelope(5.,-.2),dir+"/linear-decreasing.step"),false,5.,-.2);
    require(close(G.native,M.native),"mirrored law volume mismatch");
    report<<std::setprecision(17)<<"{\"status\":\"PASS\",\"occt_version\":\"7.9.3\",\"production_routed\":false,\"source_recognition_proven\":false,"
      "\"volume_reference\":\"analytic formula evaluated in binary64\",\"volume_interval_certified\":false,\"domain_rejections\":7,\"cases\":[";
    reportCase(report,"trihedral",D);report<<',';reportCase(report,"increasing",G);report<<',';reportCase(report,"decreasing",M);report<<"]}\n";
    return 0;
  }catch(const Standard_Failure& e){std::cerr<<"OCCT failure "<<e.GetMessageString()<<'\n';}
   catch(const std::exception& e){std::cerr<<e.what()<<'\n';}
  report<<"{\"status\":\"FAILED\",\"production_routed\":false}\n";return 1;
}
