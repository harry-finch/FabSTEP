// EXPERIMENTAL F0 laboratory. CSG supplies a known test solid, not a source
// recognition or production trimming algorithm. No application code calls this.
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepBndLib.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <GProp_GProps.hxx>
#include <Geom2d_Curve.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_Curve.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <Standard_Failure.hxx>
#include <Standard_Version.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_ListIteratorOfListOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <gp_Ax2.hxx>
#include <gp_Vec2d.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <map>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>
#ifdef CROSS_DRILL_WITH_CERTIFICATE
#include "cross_drill_certificate.hxx"
#endif

namespace {
constexpr double r = 3., R = 6., pi = 3.1415926535897932384626433832795;
std::string quote(const std::string& value) {
  std::ostringstream s; s << '"';
  for (const char c : value) {
    if (c == '"' || c == '\\') s << '\\' << c;
    else if (c == '\n') s << "\\n";
    else if (static_cast<unsigned char>(c) < 32) s << '?';
    else s << c;
  }
  s << '"'; return s.str();
}
const char* yes(bool b) { return b ? "true" : "false"; }
std::string number(double v) {
  if (!std::isfinite(v)) return "null";
  std::ostringstream s; s << std::setprecision(17) << v; return s.str();
}
bool guard(double small, double large, double dot = 0., double offset = 0.) {
  return std::isfinite(small) && std::isfinite(large) && small > 1e-4 && large >= 2.*small
    && std::isfinite(dot) && std::abs(dot) <= 1e-12
    && std::isfinite(offset) && std::abs(offset) <= 1e-10;
}
void require(bool condition, const char* message) {
  if (!condition) throw std::runtime_error(message);
}
TopoDS_Shape cut(const TopoDS_Shape& a, const TopoDS_Shape& b) {
  BRepAlgoAPI_Cut op(a,b); require(op.IsDone(),"CSG laboratory construction failed");
  require(BRepCheck_Analyzer(op.Shape(),true).IsValid(),"CSG laboratory shape invalid");
  return op.Shape();
}
double volume(const TopoDS_Shape& shape) {
  GProp_GProps p; BRepGProp::VolumeProperties(shape,p,1e-11,true); return p.Mass();
}
double quadrature(int n) {
  // Independent numerical reference, explicitly not the curve certificate.
  const double h = (pi/2.)/n;
  auto f=[](double u) {const double c=std::cos(u),s=std::sin(u);return c*c*std::sqrt(R*R-r*r*s*s);};
  double sum=f(0.)+f(pi/2.);
  for(int i=1;i<n;++i) sum+=(i%2?4.:2.)*f(i*h);
  return 8.*r*r*h*sum/3.;
}
int support(const TopoDS_Face& face) {
  BRepAdaptor_Surface s(face,true);
  if(s.GetType()!=GeomAbs_Cylinder) return 0;
  const auto c=s.Cylinder(); const auto d=c.Axis().Direction(); const auto p=c.Location();
  if(std::abs(c.Radius()-r)<1e-9 && std::abs(std::abs(d.Z())-1.)<1e-12 && std::hypot(p.X(),p.Y())<1e-9) return 1;
  if(std::abs(c.Radius()-R)<1e-9 && std::abs(std::abs(d.X())-1.)<1e-12 && std::hypot(p.Y(),p.Z())<1e-9) return 2;
  return -1;
}
TopoDS_Edge occurrence(const TopoDS_Edge& key,const TopoDS_Face& face) {
  TopoDS_Edge result;int count=0;
  for(TopExp_Explorer it(face,TopAbs_EDGE);it.More();it.Next()) {
    const auto e=TopoDS::Edge(it.Current());if(e.IsSame(key)) {result=e;++count;}
  }
  require(count==1,"Intersection edge must occur exactly once per incident face");
  require(result.Orientation()==TopAbs_FORWARD||result.Orientation()==TopAbs_REVERSED,"Indeterminate edge orientation");
  return result;
}
struct Junction { TopoDS_Edge edge; TopoDS_Face small,large; int sign; };
#ifdef CROSS_DRILL_WITH_CERTIFICATE
struct Coverage {
  bool ok=false;
  double gap=std::numeric_limits<double>::infinity();
  double bound=std::numeric_limits<double>::infinity();
  std::string reason;
};
Coverage coverBranch(const std::vector<cross_drill_lab::Certificate>& certs,int branch,double tolerance) {
  Coverage result;
  std::vector<std::array<double,2>> intervals;
  double worst=0.,totalLo=0.,totalHi=0.;
  const double roundGuard=256.*std::numeric_limits<double>::epsilon()*pi;
  for(const auto& c:certs) {
    if(!c.ok) {result.reason="one edge lacks a continuous certificate";return result;}
    if(c.sign!=branch) continue;
    if(!c.phase_monotone) {result.reason="one edge lacks a continuous certificate";return result;}
    const bool forward=c.phase_last>c.phase_first;
    const double lo=forward?c.phase_first_hi:c.phase_last_hi;
    const double hi=forward?c.phase_last_lo:c.phase_first_lo;
    const double outerLo=forward?c.phase_first_lo:c.phase_last_lo;
    const double outerHi=forward?c.phase_last_hi:c.phase_first_hi;
    if(!(hi>lo)||hi-lo>2.*pi+roundGuard) {result.reason="unresolved edge phase extent";return result;}
    totalLo+=std::max(0.,hi-lo-roundGuard);totalHi+=outerHi-outerLo+roundGuard;
    worst=std::max(worst,c.bound3d);
    double start=lo-std::floor(lo/(2.*pi))*(2.*pi),end=start+hi-lo;
    if(end>2.*pi) {intervals.push_back({start,2.*pi});intervals.push_back({0.,end-2.*pi});}
    else intervals.push_back({start,end});
  }
  if(intervals.empty()) {result.reason="branch absent";return result;}
  std::sort(intervals.begin(),intervals.end());
  double gap=0.,end=intervals[0][1];
  for(size_t i=1;i<intervals.size();++i) {gap=std::max(gap,intervals[i][0]-end);end=std::max(end,intervals[i][1]);}
  gap=std::max(gap,intervals[0][0]+2.*pi-end)+roundGuard;
  const double a=std::sqrt(R*R-r*r),speed=std::hypot(r,r*r/(2.*a));
  result.gap=gap;
  result.bound=worst+speed*gap/2.+roundGuard;
  // Topological closure is checked separately. The intervals must describe
  // exactly one turn; a gap is charged as a genuine geometric error.
  const double phaseBudget=2.*tolerance/speed;
  result.ok=totalLo<=2.*pi+phaseBudget&&totalHi>=2.*pi-phaseBudget&&result.bound<=tolerance;
  result.reason=result.ok?"one closed degree-one branch, gaps included in bound":"phase coverage, winding or total bound unresolved";
  return result;
}
#endif
#ifdef CROSS_DRILL_WITH_CERTIFICATE
std::string certificateJson(const cross_drill_lab::Certificate& c) {
  std::ostringstream s;
  s << "{\"ok\":" << yes(c.ok) << ",\"bounds_scope\":" << quote(c.ok?"entire_edge":"accepted_leaves_only_not_a_certificate") << ",\"bound_3d_mm\":" << number(c.bound3d)
    << ",\"bound_pcurves_mm\":" << number(c.boundPcurves) << ",\"work\":" << c.work
    << ",\"branch\":" << c.sign << ",\"phase_first\":" << number(c.phase_first)
    << ",\"phase_last\":" << number(c.phase_last) << ",\"phase_monotone\":" << yes(c.phase_monotone)
    << ",\"leaves\":" << c.leaves << ",\"attempts\":" << c.attempts
    << ",\"last_bound_3d_mm\":" << number(c.last_bound3d) << ",\"last_bound_pcurves_mm\":" << number(c.last_boundPcurves)
    << ",\"reason\":" << quote(c.reason) << '}'; return s.str();
}
#endif
}

int main(int argc,char** argv) {
  const std::string directory=argc>1?argv[1]:"artifacts/cross-drill-lab";
  std::ofstream report(directory+"/report.json");
  try {
    bool elementaryPassed=false;std::string elementaryJson="null";
#ifdef CROSS_DRILL_WITH_CERTIFICATE
    const auto elementary=cross_drill_lab::selfTest();elementaryPassed=elementary.ok;
    std::ostringstream elementaryStream;
    elementaryStream << "{\"ok\":" << yes(elementary.ok) << ",\"work\":" << elementary.work << ",\"reason\":" << quote(elementary.reason) << '}';
    elementaryJson=elementaryStream.str();std::cout << "elementary controls " << elementaryJson << '\n';
#endif
    require(guard(r,R),"Nominal precondition failed");
    const bool guards = !guard(0.,R) && !guard(r,5.99) && !guard(r,R,0.01) && !guard(r,R,0.,0.01)
      && !guard(std::numeric_limits<double>::quiet_NaN(),R);
    require(guards,"Adversarial precondition accepted");
    auto shape=cut(cut(BRepPrimAPI_MakeBox(gp_Pnt(-10.,-10.,-10.),20.,20.,20.).Shape(),
      BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(0.,0.,-11.),gp_Dir(0.,0.,1.)),r,22.).Shape()),
      BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(-11.,0.,0.),gp_Dir(1.,0.,0.)),R,22.).Shape());
    const double beforeVolume=volume(shape);
    STEPControl_Writer writer;
    require(writer.Transfer(shape,STEPControl_ManifoldSolidBrep)==IFSelect_RetDone,"STEP transfer failed");
    const std::string stepPath=directory+"/experimental-cross-drill.step";
    require(writer.Write(stepPath.c_str())==IFSelect_RetDone,"STEP write failed");
    STEPControl_Reader reader;
    require(reader.ReadFile(stepPath.c_str())==IFSelect_RetDone,"STEP read failed");
    require(reader.TransferRoots()==1,"STEP root count differs"); shape=reader.OneShape();
    require(BRepCheck_Analyzer(shape,true).IsValid(),"Reimported shape invalid");
    TopTools_IndexedMapOfShape faces,solids; TopExp::MapShapes(shape,TopAbs_FACE,faces);TopExp::MapShapes(shape,TopAbs_SOLID,solids);
    int planes=0,cylinders=0,other=0;std::map<int,int> supportCounts;
    for(int i=1;i<=faces.Extent();++i) {
      const auto face=TopoDS::Face(faces(i)); BRepAdaptor_Surface s(face,true);
      if(s.GetType()==GeomAbs_Plane) ++planes;
      else if(s.GetType()==GeomAbs_Cylinder) {++cylinders;++supportCounts[support(face)];}
      else ++other;
    }
    require(solids.Extent()==1 && faces.Extent()==9 && planes==6 && cylinders==3 && other==0
      && supportCounts[1]==2 && supportCounts[2]==1,"Unexpected native topology/supports");
    Bnd_Box bounds; BRepBndLib::Add(shape,bounds,false);double x0,y0,z0,x1,y1,z1;bounds.Get(x0,y0,z0,x1,y1,z1);
    for(double v:{x0,y0,z0}) require(std::abs(v+10.)<1e-5,"Reimported minimum bound differs");
    for(double v:{x1,y1,z1}) require(std::abs(v-10.)<1e-5,"Reimported maximum bound differs");
    GProp_GProps nonAdaptive; BRepGProp::VolumeProperties(shape,nonAdaptive,true);
    const double afterVolume=volume(shape),intersection=quadrature(8192);
    const double expectedVolume=8000.-20.*pi*(r*r+R*R)+intersection;
    require(std::abs(intersection-quadrature(4096))<1e-8,"Numerical volume reference unconverged");
    std::cout << std::setprecision(17) << "volume before=" << beforeVolume << " after=" << afterVolume << " reference=" << expectedVolume << "\n";
    require(std::abs(afterVolume-beforeVolume)<1e-5 && std::abs(afterVolume-expectedVolume)<1e-4,"Native volume mismatch");
    int probeCount=0;
    auto probe=[&](const gp_Pnt& p,TopAbs_State expected) {
      BRepClass3d_SolidClassifier classifier(shape,p,1e-7);++probeCount;
      require(classifier.State()==expected,"Material/void probe mismatch");
    };
    probe(gp_Pnt(9.,9.,9.),TopAbs_IN);probe(gp_Pnt(0.,0.,0.),TopAbs_OUT);probe(gp_Pnt(0.,0.,9.),TopAbs_OUT);probe(gp_Pnt(9.,0.,0.),TopAbs_OUT);
    // Four resolved material/void sectors at four locations of each branch.
    for(int sign:{-1,1}) for(int i=0;i<4;++i) {
      const double u=(i+0.5)*pi/2.,x=r*std::cos(u),y=r*std::sin(u),z=sign*std::sqrt(R*R-y*y);
      const gp_Vec a(x/r,y/r,0.),b(0.,y/R,z/R);const double c=a.Dot(b),denom=1.-c*c;
      const gp_Vec dualA=(a-b*c)/denom,dualB=(b-a*c)/denom;
      for(int sa:{-1,1}) for(int sb:{-1,1}) {
        const gp_Pnt p=gp_Pnt(x,y,z).Translated((dualA*sa+dualB*sb)*0.02);
        const bool material=std::hypot(p.X(),p.Y())>r && std::hypot(p.Y(),p.Z())>R;
        require(material==(sa>0&&sb>0),"Ideal probe sector not resolved");
        probe(p,material?TopAbs_IN:TopAbs_OUT);
      }
    }
    TopTools_IndexedDataMapOfShapeListOfShape incidence;TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,incidence);
    std::vector<Junction> junctions;std::ostringstream edgeJson;bool first=true;
#ifdef CROSS_DRILL_WITH_CERTIFICATE
    std::vector<cross_drill_lab::Certificate> certificates,strictCertificates;
    size_t remainingWork=10000000,strictRemainingWork=10000000;
#endif
    for(int i=1;i<=incidence.Extent();++i) {
      TopTools_IndexedMapOfShape owners;
      for(TopTools_ListIteratorOfListOfShape it(incidence.FindFromIndex(i));it.More();it.Next()) owners.Add(it.Value());
      if(owners.Extent()!=2) continue;
      auto f1=TopoDS::Face(owners(1)),f2=TopoDS::Face(owners(2));
      if(support(f1)==2 && support(f2)==1) std::swap(f1,f2);
      if(support(f1)!=1 || support(f2)!=2) continue;
      const auto key=TopoDS::Edge(incidence.FindKey(i));
      const auto edge=occurrence(key,f1),otherUse=occurrence(key,f2);
      require(edge.Orientation()!=otherUse.Orientation(),"Intersection uses are not opposed");
      double lo,hi;TopLoc_Location location;
      auto curve=BRep_Tool::Curve(edge,location,lo,hi);require(!curve.IsNull()&&std::isfinite(lo)&&std::isfinite(hi)&&hi>lo,"Intersection curve missing");
      const auto p=curve->Value((lo+hi)*0.5).Transformed(location.Transformation());const int sign=p.Z()>0.?1:-1;
      double a0,a1,b0,b1;auto pc1=BRep_Tool::CurveOnSurface(edge,f1,a0,a1);auto pc2=BRep_Tool::CurveOnSurface(edge,f2,b0,b1);
      require(!pc1.IsNull()&&!pc2.IsNull(),"Intersection pcurve missing");
      junctions.push_back({edge,f1,f2,sign});
      const auto bs=Handle(Geom_BSplineCurve)::DownCast(curve);
      require(!bs.IsNull() && bs->NbPoles()<=2048 && bs->NbKnots()<=256 && bs->Degree()<=8,"3D spline quota exceeded");
      const auto bs1=Handle(Geom2d_BSplineCurve)::DownCast(pc1),bs2=Handle(Geom2d_BSplineCurve)::DownCast(pc2);
      require(!bs1.IsNull()&&!bs2.IsNull()&&bs1->NbPoles()<=2048&&bs2->NbPoles()<=2048&&bs1->Degree()<=8&&bs2->Degree()<=8,"Pcurve quota exceeded");
      std::cout << "edge " << i << " degree/poles/knots=" << bs->Degree() << '/' << bs->NbPoles() << '/' << bs->NbKnots()
        << " rational=" << bs->IsRational() << " periodic=" << bs->IsPeriodic() << " pcurves=" << bs1->Degree() << '/' << bs1->NbPoles() << '/' << bs1->IsRational()
        << ',' << bs2->Degree() << '/' << bs2->NbPoles() << '/' << bs2->IsRational() << '\n';
      if(!first) edgeJson << ',';first=false;
      edgeJson << "{\"id\":" << i << ",\"branch\":" << sign << ",\"curve_type\":" << quote(curve->DynamicType()->Name())
        << ",\"degree\":" << bs->Degree() << ",\"poles\":" << bs->NbPoles() << ",\"knots\":" << bs->NbKnots()
        << ",\"periodic\":" << yes(bs->IsPeriodic()) << ",\"rational\":" << yes(bs->IsRational())
        << ",\"pcurve_types\":[" << quote(pc1->DynamicType()->Name()) << ',' << quote(pc2->DynamicType()->Name())
        << "],\"range\":[" << number(lo) << ',' << number(hi) << ']';
#ifdef CROSS_DRILL_WITH_CERTIFICATE
      cross_drill_lab::Budget budget;budget.tolerance_mm=1e-5;budget.max_work=remainingWork;
      cross_drill_lab::Certificate cert;
      if(remainingWork) cert=cross_drill_lab::certifyEdge(edge,f1,f2,r,R,budget);
      else cert.reason="global_work_budget";
      remainingWork-=std::min(remainingWork,cert.work);
      budget.tolerance_mm=1e-7;budget.max_work=strictRemainingWork;
      cross_drill_lab::Certificate strict;
      if(strictRemainingWork) strict=cross_drill_lab::certifyEdge(edge,f1,f2,r,R,budget);
      else strict.reason="global_work_budget";
      strictRemainingWork-=std::min(strictRemainingWork,strict.work);
      certificates.push_back(cert);strictCertificates.push_back(strict);
      edgeJson << ",\"certificate\":" << certificateJson(cert) << ",\"strict_certificate\":" << certificateJson(strict);
      if(bs->IsPeriodic()) {
        // Independent bounded capability probe: do not confuse the global
        // document stop with support for this native representation.
        budget.tolerance_mm=1e-5;budget.max_work=10000;
        const auto representation=cross_drill_lab::certifyEdge(edge,f1,f2,r,R,budget);
        edgeJson << ",\"independent_representation_probe\":" << certificateJson(representation);
      }
#endif
      edgeJson << '}';
    }
    require(!junctions.empty()&&junctions.size()<=32,"Unexpected intersection edge count");
    std::map<int,int> branchEdges;
    for(int sign:{-1,1}) {
      TopTools_IndexedMapOfShape vertices;std::map<int,int> degree,incoming,outgoing;std::map<int,std::vector<int>> neighbors;
      for(const auto& j:junctions) if(j.sign==sign) {
        ++branchEdges[sign]; TopoDS_Vertex v1,v2;TopExp::Vertices(j.edge,v1,v2,true);
        require(!v1.IsNull()&&!v2.IsNull(),"Intersection edge lacks vertices");
        const int a=vertices.Add(v1),b=vertices.Add(v2);++degree[a];++degree[b];++outgoing[a];++incoming[b];neighbors[a].push_back(b);neighbors[b].push_back(a);
      }
      require(branchEdges[sign]>0,"One analytic branch missing");
      for(const auto& d:degree) require(d.second==2&&incoming[d.first]==1&&outgoing[d.first]==1,"Intersection branch is not one coherently oriented cycle");
      std::vector<int> pending{1};std::map<int,bool> visited;
      while(!pending.empty()) {const int v=pending.back();pending.pop_back();if(visited[v]) continue;visited[v]=true;for(int next:neighbors[v]) pending.push_back(next);}
      require(visited.size()==static_cast<size_t>(vertices.Extent()),"Intersection branch has multiple components");
    }
    bool certified=false,strictCertified=false;
    std::string certificationJson="null",adversarialJson="null";
#ifdef CROSS_DRILL_WITH_CERTIFICATE
    const auto plus=coverBranch(certificates,1,1e-5),minus=coverBranch(certificates,-1,1e-5);
    const auto strictPlus=coverBranch(strictCertificates,1,1e-7),strictMinus=coverBranch(strictCertificates,-1,1e-7);
    size_t totalWork=0;for(const auto& c:certificates) totalWork+=c.work;
    auto coherentOrientation=[&](const std::vector<cross_drill_lab::Certificate>& values) {
      bool coherent=true;std::map<int,int> directions;
      for(size_t i=0;i<values.size();++i) {
        const auto& c=values[i];const auto& j=junctions[i];
        if(!c.ok||!c.phase_monotone||c.phase_direction==0) {coherent=false;continue;}
        const int direction=c.phase_direction*(j.edge.Orientation()==TopAbs_FORWARD?1:-1);
        if(directions.count(j.sign)&&directions[j.sign]!=direction) coherent=false;
        directions[j.sign]=direction;
      }
      return coherent;
    };
    const bool orientedPhase=coherentOrientation(certificates),strictOrientedPhase=coherentOrientation(strictCertificates);
    certified=plus.ok&&minus.ok&&orientedPhase&&elementaryPassed&&totalWork<=10000000;
    strictCertified=strictPlus.ok&&strictMinus.ok&&strictOrientedPhase&&elementaryPassed;
    std::ostringstream coverage;
    coverage << "{\"nominal_tolerance_mm\":1e-5,\"strict_tolerance_mm\":1e-7,\"total_work\":" << totalWork
      << ",\"oriented_phase_consistent\":" << yes(orientedPhase) << ",\"plus_gap_rad\":" << number(plus.gap) << ",\"minus_gap_rad\":" << number(minus.gap)
      << ",\"plus_bound_mm\":" << number(plus.bound) << ",\"minus_bound_mm\":" << number(minus.bound)
      << ",\"plus_reason\":" << quote(plus.reason) << ",\"minus_reason\":" << quote(minus.reason) << '}';
    certificationJson=coverage.str();
    const auto& j=junctions.front();cross_drill_lab::Budget budget;
    budget.tolerance_mm=1e-5;budget.max_work=1;
    const auto exhausted=cross_drill_lab::certifyEdge(j.edge,j.small,j.large,r,R,budget);
    budget=cross_drill_lab::Budget{};budget.tolerance_mm=1e-5;
    double begin,finish;TopLoc_Location loc;auto original=BRep_Tool::Curve(j.edge,loc,begin,finish);
    auto changed=Handle(Geom_BSplineCurve)::DownCast(original->Copy());
    auto pole=changed->Pole(1);pole.SetZ(pole.Z()+0.01);changed->SetPole(1,pole);
    auto badCurve=TopoDS::Edge(j.edge.EmptyCopied());BRep_Builder builder;builder.UpdateEdge(badCurve,changed,1e-7);
    const auto moved=cross_drill_lab::certifyEdge(badCurve,j.small,j.large,r,R,budget);
    auto badPcurve=TopoDS::Edge(j.edge.EmptyCopied());double p0,p1;
    auto pc=BRep_Tool::CurveOnSurface(j.edge,j.small,p0,p1);auto shifted=Handle(Geom2d_Curve)::DownCast(pc->Copy());
    shifted->Translate(gp_Vec2d(0.,0.01));builder.UpdateEdge(badPcurve,shifted,j.small,1e-7);
    const auto shiftedCert=cross_drill_lab::certifyEdge(badPcurve,j.small,j.large,r,R,budget);
    bool zeroWeightRejected=false;
    try {auto zero=Handle(Geom_BSplineCurve)::DownCast(original->Copy());zero->SetWeight(1,0.);} catch(const Standard_Failure&) {zeroWeightRejected=true;}
    // Isolated phase-aggregation control: two proved half turns cover once;
    // duplicating those valid inputs must fail rather than relying on a failed
    // native positive case to reject its corrupted copies.
    std::vector<cross_drill_lab::Certificate> known(2);
    for(size_t i=0;i<known.size();++i) {
      auto& c=known[i];c.ok=true;c.sign=1;c.phase_monotone=true;c.bound3d=0.;c.boundPcurves=0.;
      c.phase_first=c.phase_first_lo=c.phase_first_hi=i*pi;
      c.phase_last=c.phase_last_lo=c.phase_last_hi=(i+1)*pi;
    }
    const bool phasePositive=coverBranch(known,1,1e-5).ok;
    auto duplicate=known;duplicate.insert(duplicate.end(),known.begin(),known.end());
    const bool multipleTurnRejected=phasePositive&&!coverBranch(duplicate,1,1e-5).ok;
    const bool rejected=!exhausted.ok&&!moved.ok&&!shiftedCert.ok&&zeroWeightRejected&&multipleTurnRejected;
    const bool positiveControl=certificates.front().ok;
    const bool adversaries=rejected&&positiveControl;
    certified=certified&&adversaries;strictCertified=strictCertified&&adversaries;
    std::ostringstream adversarial;
    adversarial << "{\"all_passed\":" << yes(adversaries) << ",\"native_positive_control\":" << yes(positiveControl)
      << ",\"rejections_observed\":" << yes(rejected) << ",\"phase_positive_control\":" << yes(phasePositive)
      << ",\"exhausted_budget\":" << certificateJson(exhausted) << ",\"moved_3d_pole\":" << certificateJson(moved)
      << ",\"shifted_pcurve\":" << certificateJson(shiftedCert) << ",\"zero_weight_rejected_by_occt\":" << yes(zeroWeightRejected)
      << ",\"duplicate_winding_rejected\":" << yes(multipleTurnRejected) << '}';adversarialJson=adversarial.str();
#endif
    report << "{\"status\":\"";
#ifdef CROSS_DRILL_WITH_CERTIFICATE
    report << (certified?"CURVES_CERTIFIED_AT_1E-5":"CERTIFICATE_UNRESOLVED");
#else
    report << "INSPECT_ONLY";
#endif
    report << "\",\"experimental\":true,\"production_routed\":false,\"occt_version\":" << quote(OCC_VERSION_COMPLETE)
      << ",\"method\":\"laboratory CSG; not a source reconstruction proof\",\"solids\":" << solids.Extent()
      << ",\"planes\":" << planes << ",\"cylindrical_faces\":" << cylinders << ",\"faces\":" << faces.Extent()
      << ",\"volume_before_mm3\":" << number(beforeVolume) << ",\"volume_after_mm3\":" << number(afterVolume)
      << ",\"nonadaptive_volume_mm3\":" << number(nonAdaptive.Mass()) << ",\"numerical_reference_volume_mm3\":" << number(expectedVolume) << ",\"reference_volume_certified\":false"
      << ",\"bounds_mm\":[[" << number(x0) << ',' << number(y0) << ',' << number(z0) << "],[" << number(x1) << ',' << number(y1) << ',' << number(z1) << "]]"
      << ",\"precondition_adversaries_passed\":" << yes(guards) << ",\"probe_count\":" << probeCount
      << ",\"elementary_controls\":" << elementaryJson << ",\"intersection_edges\":[" << edgeJson.str() << "],\"branch_coverage\":" << certificationJson << ",\"adversaries\":" << adversarialJson
      << ",\"certificate_scope\":\"reimported intersection curves, pcurves and branch coverage only\",\"source_reconstruction_proven\":false,\"curves_certified\":" << yes(certified)
      << ",\"strict_curves_certified\":" << yes(strictCertified) << "}\n";
#ifdef CROSS_DRILL_WITH_CERTIFICATE
    return certified?0:2;
#else
    return 0;
#endif
  } catch(const Standard_Failure& e) {
    report << "{\"status\":\"FAILED\",\"certified\":false,\"reason\":" << quote(e.GetMessageString()?e.GetMessageString():"OCCT failure") << "}\n";
  } catch(const std::exception& e) {
    report << "{\"status\":\"FAILED\",\"certified\":false,\"reason\":" << quote(e.what()) << "}\n";
  }
  return 1;
}
