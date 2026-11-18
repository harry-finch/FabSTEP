// Open-shell laboratory for reconciled sectors, alone or joined by tangent tubes.
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Curve2d.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <Geom2d_BSplineCurve.hxx>
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
#include <vector>

namespace {
constexpr double eps=5e-8, pi=3.14159265358979323846;
void check(bool c, const char* reason) { if(!c) throw std::runtime_error(reason); }

struct Sector {
    gp_Pnt center; gp_Dir axis,start;
    double R,r,u,v0,v1;
    gp_Dir radial(double angle) const {return start.Rotated(gp_Ax1(center,axis),angle);}
    gp_Pnt point(double az,double mer) const {
        return center.Translated(gp_Vec(radial(az))*(R+r*std::cos(mer))+gp_Vec(axis)*(r*std::sin(mer)));
    }
    double area() const {return r*u*(R*(v1-v0)+r*(std::sin(v1)-std::sin(v0)));}
};
Sector read(std::istream& input) {
    std::array<double,14> x;
    for(auto& n:x)check(bool(input>>n)&&std::isfinite(n),"invalid measured value");
    Sector s{{x[0],x[1],x[2]},{x[3],x[4],x[5]},{x[6],x[7],x[8]},x[9],x[10],x[11],x[12],x[13]};
    check(s.R>s.r && s.r>0 && s.u>0 && s.u<pi && std::abs(s.v1-s.v0-pi/2.)<eps &&
          s.axis.IsNormal(s.start,1e-10),"invalid measured torus chart");
    return s;
}
struct Tube {int sector, end; gp_Vec extrusion;};
using Sectors=std::vector<Sector>;
using Tubes=std::vector<Tube>;
TopoDS_Shape build(const Sectors& sectors,const Tubes& tubes) {
    BRepBuilderAPI_Sewing sew(eps);
    for(const auto& s:sectors) {
        const gp_Torus torus(gp_Ax3(s.center,s.axis,s.start),s.R,s.r);
        sew.Add(BRepBuilderAPI_MakeFace(torus,0,s.u,s.v0,s.v1).Face());
    }
    for(const auto& t:tubes) {
        const auto& s=sectors.at(t.sector);
        const gp_Dir radial=s.radial(t.end*s.u);
        check(t.extrusion.Magnitude()>eps && t.extrusion.Crossed(gp_Vec(s.axis.Crossed(radial))).Magnitude()<eps,
              "tube leaves tangent plane");
        const gp_Circ circle(gp_Ax2(s.center.Translated(gp_Vec(radial)*s.R),radial.Crossed(s.axis),radial),s.r);
        sew.Add(BRepPrimAPI_MakePrism(BRepBuilderAPI_MakeEdge(circle,s.v0,s.v1).Edge(),t.extrusion).Shape());
    }
    sew.Perform();return sew.SewedShape();
}
TopoDS_Shape roundtrip(const TopoDS_Shape& shape, const std::filesystem::path& output) {
    STEPControl_Writer writer;
    check(writer.Transfer(shape,STEPControl_ShellBasedSurfaceModel)==IFSelect_RetDone &&
          writer.Write(output.c_str())==IFSelect_RetDone && std::filesystem::file_size(output)<1024*1024,
          "shell STEP write");
    STEPControl_Reader reader;
    check(reader.ReadFile(output.c_str())==IFSelect_RetDone && reader.TransferRoots()>0,"shell STEP read");
    return reader.OneShape();
}
int number(const TopoDS_Shape& shape,TopAbs_ShapeEnum type) {
    TopTools_IndexedMapOfShape items; TopExp::MapShapes(shape,type,items); return items.Extent();
}
struct Arc {
    gp_Pnt a,mid,b,center;
    gp_Dir normal;
    double radius,span;
};
Arc boundary(const Sector& s,int side) {
    if(side<2) {
        const double u=side*s.u;
        return {s.point(u,s.v0),s.point(u,(s.v0+s.v1)/2),s.point(u,s.v1),
                s.center.Translated(gp_Vec(s.radial(u))*s.R),s.radial(u).Crossed(s.axis),s.r,s.v1-s.v0};
    }
    const double v=side==2?s.v0:s.v1;
    return {s.point(0,v),s.point(s.u/2,v),s.point(s.u,v),s.center.Translated(gp_Vec(s.axis)*(s.r*std::sin(v))),
            s.axis,s.R+s.r*std::cos(v),s.u};
}
struct Limits {double pointError=0.,areaError=0.; int loops=0; std::vector<double> faceAreaErrors;};
int match(const Sector& s,const BRepAdaptor_Curve& edge,Limits& limits) {
    check(edge.GetType()==GeomAbs_Circle,"noncircular 3D edge");
    const auto circle=edge.Circle();
    for(int side=0;side<4;++side) {
        const auto ref=boundary(s,side);
        const double lo=edge.FirstParameter(),hi=edge.LastParameter();
        if(circle.Location().Distance(ref.center)>eps || std::abs(circle.Radius()-ref.radius)>eps ||
           !circle.Axis().Direction().IsParallel(ref.normal,eps/ref.radius) ||
           std::abs(hi-lo-ref.span)*ref.radius>eps) continue;
        auto a=edge.Value(lo), b=edge.Value(hi);
        if(a.Distance(ref.b)<eps) std::swap(a,b);
        const double error=std::max({a.Distance(ref.a),b.Distance(ref.b),edge.Value((hi+lo)/2).Distance(ref.mid)});
        if(error>eps) continue;
        limits.pointError=std::max(limits.pointError,error);
        return side;
    }
    throw std::runtime_error("physical edge outside expected domain");
}
int sectorIndex(const TopoDS_Face& face,const Sectors& sectors) {
    const BRepAdaptor_Surface support(face);
    check(support.GetType()==GeomAbs_Torus,"non-torus face");
    const auto torus=support.Torus();
    for(size_t i=0;i<sectors.size();++i) {
        const auto& s=sectors[i];
        if(torus.Location().Distance(s.center)<eps && std::abs(torus.MajorRadius()-s.R)<eps &&
           std::abs(torus.MinorRadius()-s.r)<eps && torus.Axis().Direction().IsParallel(s.axis,eps/(s.R+s.r)))return i;
    }
    throw std::runtime_error("displaced torus domain");
}
TopAbs_Orientation use(const TopoDS_Face& face,const TopoDS_Edge& edge) {
    for(TopExp_Explorer it(face,TopAbs_EDGE);it.More();it.Next())
        if(it.Current().IsSame(edge))return it.Current().Orientation();
    throw std::runtime_error("missing coedge");
}
int chartBorder(const TopoDS_Face& face,const TopoDS_Edge& edge,bool constantU,double uSpan,double vSpan) {
    const BRepAdaptor_Surface surf(face);
    const BRepAdaptor_Curve2d trim(edge,face);
    // The STEP writer represents some second-face pcurves of shared
    // circles by a two-pole, degree-one B-spline. Such a single
    // nonrational span is exactly the line between its two endpoints.
    if(trim.GetType()!=GeomAbs_Line) {
        check(trim.GetType()==GeomAbs_BSplineCurve && trim.Degree()==1 && trim.NbPoles()==2 &&
              trim.NbKnots()==2 && !trim.IsRational(),"nonlinear chart trim");
        const auto line=trim.BSpline();
        check(!line->IsPeriodic() && line->Multiplicity(1)==2 && line->Multiplicity(2)==2 &&
              trim.FirstParameter()>=line->Knot(1)-1e-12 && trim.LastParameter()<=line->Knot(2)+1e-12 &&
              trim.LastParameter()>trim.FirstParameter(),"unbounded linear spline trim");
    }
    const auto a=trim.Value(trim.FirstParameter()), b=trim.Value(trim.LastParameter());
    const double uScale=(surf.GetType()==GeomAbs_Torus) ? surf.Torus().MajorRadius()+surf.Torus().MinorRadius() : surf.Cylinder().Radius();
    const double vScale=(surf.GetType()==GeomAbs_Torus) ? surf.Torus().MinorRadius() : 1.;
    if(constantU) {
        check(std::abs(b.X()-a.X())*uScale<eps &&
              std::abs(std::abs(b.Y()-a.Y())-vSpan)*vScale<eps,"constant U trim");
        if(std::abs(a.X()-surf.FirstUParameter())*uScale<eps) return 0;
        if(std::abs(a.X()-surf.LastUParameter())*uScale<eps) return 1;
    } else {
        check(std::abs(b.Y()-a.Y())*vScale<eps &&
              std::abs(std::abs(b.X()-a.X())-uSpan)*uScale<eps,"constant V trim");
        if(std::abs(a.Y()-surf.FirstVParameter())*vScale<eps) return 2;
        if(std::abs(a.Y()-surf.LastVParameter())*vScale<eps) return 3;
    }
    throw std::runtime_error("wrong rectangular border");
}
// Test the four boundaries independently from face traversal order.
int tubeIndex(const TopoDS_Face& face,const Sectors& sectors,const Tubes& tubes,Limits& limits) {
    const BRepAdaptor_Surface surf(face);
    check(surf.GetType()==GeomAbs_Cylinder && number(face,TopAbs_WIRE)==1 && number(face,TopAbs_EDGE)==4,
          "tube topology");
    const auto cylinder=surf.Cylinder();
    for(size_t index=0;index<tubes.size();++index) {
        const auto& t=tubes[index];
        const auto& sector=sectors[t.sector];
        const auto section=boundary(sector,t.end);
        if(std::abs(cylinder.Radius()-section.radius)>eps ||
           !cylinder.Axis().Direction().IsParallel(gp_Dir(t.extrusion),eps/t.extrusion.Magnitude()) ||
           gp_Lin(cylinder.Axis()).Distance(section.center)>eps) continue;
        std::array<int,4> sides{}, chart{};
        for(TopExp_Explorer edges(face,TopAbs_EDGE);edges.More();edges.Next()) {
            const BRepAdaptor_Curve curve(TopoDS::Edge(edges.Current()));
            const double lo=curve.FirstParameter(), hi=curve.LastParameter();
            auto a=curve.Value(lo), b=curve.Value(hi), mid=curve.Value((lo+hi)/2);
            for(int side=0;side<4;++side) {
                const bool circular=side<2;
                if(curve.GetType()!=(circular?GeomAbs_Circle:GeomAbs_Line)) continue;
                gp_Pnt first,last,middle;
                if(circular) {
                    const auto offset=t.extrusion*double(side);
                    if(curve.Circle().Location().Distance(section.center.Translated(offset))>eps ||
                       !curve.Circle().Axis().Direction().IsParallel(section.normal,eps/section.radius) ||
                       std::abs(curve.Circle().Radius()-section.radius)>eps ||
                       std::abs(hi-lo-section.span)*section.radius>eps) continue;
                    first=section.a.Translated(offset);last=section.b.Translated(offset);
                    middle=section.mid.Translated(offset);
                } else {
                    first=side==2?section.a:section.b;last=first.Translated(t.extrusion);
                    middle=first.Translated(t.extrusion*.5);
                }
                const double endError=std::min(std::max(a.Distance(first),b.Distance(last)),
                                               std::max(a.Distance(last),b.Distance(first)));
                const double error=std::max(endError,mid.Distance(middle));
                if(error<=eps) {
                    ++sides[side];
                    ++chart[chartBorder(face,TopoDS::Edge(edges.Current()),!circular,section.span,t.extrusion.Magnitude())];
                    limits.pointError=std::max(limits.pointError,error);
                    break;
                }
            }
        }
        if(sides!=std::array<int,4>{1,1,1,1} || chart!=sides) continue;
        GProp_GProps prop;BRepGProp::SurfaceProperties(face,prop,1e-12);
        const double error=std::abs(prop.Mass()-section.radius*section.span*t.extrusion.Magnitude());
        limits.faceAreaErrors.at(sectors.size()+index)=error;
        limits.areaError=std::max(limits.areaError,error);
        return static_cast<int>(sectors.size()+index);
    }
    throw std::runtime_error("tube outside measured boundaries");
}

int boundaryLoops(const std::vector<TopoDS_Edge>& boundary,bool belt) {
    TopTools_IndexedMapOfShape vertices;
    std::vector<std::vector<int>> adjacent(2*boundary.size()+1);
    for(const auto& edge:boundary) {
        TopoDS_Vertex first,last;TopExp::Vertices(edge,first,last);
        check(!first.IsNull() && !last.IsNull() && !first.IsSame(last),"open boundary endpoints");
        const int a=vertices.Add(first),b=vertices.Add(last);
        adjacent[a].push_back(b);adjacent[b].push_back(a);
    }
    int loops=0;
    std::vector<bool> seen(adjacent.size(),false);
    for(int i=1;i<=vertices.Extent();++i) {
        check(adjacent[i].size()==2,"branched boundary wire");
        if(seen[i])continue;
        ++loops;
        size_t count=0;
        std::vector<int> walk{i};
        while(!walk.empty()) {
            const int v=walk.back();walk.pop_back();if(seen[v])continue;
            seen[v]=true;++count;
            for(int n:adjacent[v]) if(!seen[n])walk.push_back(n);
        }
        if(belt)check(count==12,"incomplete free contour");
    }
    return loops;
}

Limits verify(const TopoDS_Shape& shape,const Sectors& sectors,const Tubes& tubes) {
    Limits limits;
    const bool belt=!tubes.empty();
    const int faceCount=static_cast<int>(sectors.size()+tubes.size());
    limits.faceAreaErrors.resize(faceCount);
    std::vector<int> known(faceCount,0);
    TopTools_IndexedMapOfShape faceMap;
    std::vector<int> ids(faceCount+1,-1);
    check(number(shape,TopAbs_FACE)==faceCount && number(shape,TopAbs_SOLID)==0,"unexpected solid/face count");
    for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
        const auto face=TopoDS::Face(it.Current());
        const bool torus=BRepAdaptor_Surface(face).GetType()==GeomAbs_Torus;
        const int id=torus?sectorIndex(face,sectors):tubeIndex(face,sectors,tubes,limits);
        ids[faceMap.Add(face)]=id;
        check(++known[id]==1,"repeated domain");
        if(!torus)continue;
        const auto& s=sectors[id];
        const BRepAdaptor_Surface surf(face);
        check(number(face,TopAbs_WIRE)==1 && number(face,TopAbs_EDGE)==4 &&
              std::abs(surf.LastUParameter()-surf.FirstUParameter()-s.u)*(s.R+s.r)<eps &&
              std::abs(surf.LastVParameter()-surf.FirstVParameter()-s.v1+s.v0)*s.r<eps,"chart extents");
        std::array<int,4> physical{},chart{};
        for(TopExp_Explorer edges(face,TopAbs_EDGE);edges.More();edges.Next()) {
            auto edge=TopoDS::Edge(edges.Current());
            const BRepAdaptor_Curve c(edge);
            const int side=match(s,c,limits);
            ++physical[side];
            ++chart[chartBorder(face,edge,side<2,s.u,s.v1-s.v0)];
        }
        check(physical==std::array<int,4>{1,1,1,1} && chart==physical,"incomplete rectangle");
        GProp_GProps prop; BRepGProp::SurfaceProperties(face,prop,1e-12);
        limits.faceAreaErrors[id]=std::abs(prop.Mass()-s.area());
        limits.areaError=std::max(limits.areaError,limits.faceAreaErrors[id]);
    }
    check(number(shape,TopAbs_SHELL)==1 && number(shape,TopAbs_EDGE)==(belt?36:10) &&
          BRepCheck_Analyzer(shape,true).IsValid() && limits.areaError<1.e-7,"sewn shell");
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,adj);
    std::vector<TopoDS_Edge> boundaryEdges;
    std::vector<std::vector<int>> graph(faceCount);
    int direct=0, tubeJoins=0;
    for(int i=1;i<=adj.Extent();++i) {
        const auto& neighbors=adj.FindFromIndex(i);
        const auto edge=TopoDS::Edge(adj.FindKey(i));
        if(neighbors.Extent()==1) {boundaryEdges.push_back(edge);continue;}
        check(neighbors.Extent()==2,"ambiguous join");
        const auto a=TopoDS::Face(neighbors.First()),b=TopoDS::Face(neighbors.Last());
        const int x=ids.at(faceMap.FindIndex(a)), y=ids.at(faceMap.FindIndex(b));
        check(x>=0 && y>=0 && x!=y && use(a,edge)!=use(b,edge),"oriented adjacency");
        const BRepAdaptor_Curve curve(edge);
        check(curve.GetType()==GeomAbs_Circle,"join is not a circle");
        const int sectorCount=static_cast<int>(sectors.size());
        if(x<sectorCount) check(match(sectors[x],curve,limits)<2,"expected shared meridian");
        if(y<sectorCount) check(match(sectors[y],curve,limits)<2,"expected shared meridian");
        if(x<sectorCount && y<sectorCount) ++direct;
        else {check(x<sectorCount || y<sectorCount,"tube-to-tube junction");++tubeJoins;}
        graph[x].push_back(y);graph[y].push_back(x);
    }
    check(direct==2 && tubeJoins==(belt?10:0) && boundaryEdges.size()==(belt?24:8),"junction counts");
    for(int i=0;i<faceCount;++i) {
        check(graph[i].size()==(belt || i==0 ? 2 : 1),"unexpected join per face");
        if(graph[i].size()==2) check(graph[i][0]!=graph[i][1],"repeated join");
    }
    std::vector<int> pending{0};std::vector<bool> visited(faceCount,false);
    while(!pending.empty()) {
        const int node=pending.back();pending.pop_back();if(visited[node])continue;
        visited[node]=true;
        for(int n:graph[node])if(!visited[n])pending.push_back(n);
    }
    check(std::all_of(visited.begin(),visited.end(),[](bool v){return v;}),"disconnected face graph");
    limits.loops=boundaryLoops(boundaryEdges,belt);
    check(limits.loops==(belt?2:1),"unexpected boundary cycles");
    return limits;
}
} // namespace
int main(int argc,char** argv) {
    try {
        check(argc==2,"artifact directory");
        int sectorCount=0, tubeCount=0;
        check(bool(std::cin>>sectorCount>>tubeCount) &&
              ((sectorCount==3 && tubeCount==0) || (sectorCount==7 && tubeCount==5)),"fixture dimensions");
        const bool belt=tubeCount!=0;
        Sectors sectors;Tubes tubes;
        for(int i=0;i<sectorCount;++i)sectors.push_back(read(std::cin));
        for(int i=0;i<tubeCount;++i) {
            int sector,end;
            double dx,dy,dz;
            check(bool(std::cin>>sector>>end>>dx>>dy>>dz) && sector>=0 && sector<sectorCount &&
                  (end==0 || end==1) && std::isfinite(dx) && std::isfinite(dy) && std::isfinite(dz),"tube vector");
            tubes.push_back({sector,end,gp_Vec(dx,dy,dz)});
        }
        std::cin>>std::ws;check(std::cin.eof(),"unexpected parameters");
        const std::filesystem::path dir=argv[1];
        const auto source=build(sectors,tubes);
        const auto before=verify(source,sectors,tubes);
        const auto after=verify(roundtrip(source,dir/"shared-tori.step"),sectors,tubes);
        auto shifted=sectors;shifted[1].center.Translate(gp_Vec(shifted[1].axis)*.05);
        const auto wrong=roundtrip(build(shifted,tubes),dir/"displaced-shared-tori.step");
        bool rejected=false;
        try {verify(wrong,sectors,tubes);}
        catch(const std::runtime_error& err) {const std::string reason=err.what();
            rejected=reason=="displaced torus domain" || (belt && reason=="tube outside measured boundaries");}
        check(rejected,"displaced neighbor accepted or unexpected rejection");
        std::ofstream report(dir/(belt?"belt-report.json":"shared-tori-report.json"));
        report<<std::setprecision(16)<<"{\"solids\":0,\"shells\":1,\"tori\":"<<sectorCount<<",\"cylinders\":"<<tubeCount
              <<",\"shared_arcs\":"<<(belt?12:2)<<",\"free_edges\":"<<(belt?24:8)<<",\"boundary_loops\":"<<after.loops<<","
              <<"\"max_face_area_error_mm2\":"<<std::max(before.areaError,after.areaError)
              <<",\"max_arc_sample_error_mm\":"<<std::max(before.pointError,after.pointError)
              <<",\"after_step_face_area_errors_mm2\":[";
        for(size_t i=0;i<after.faceAreaErrors.size();++i) report<<(i?",":"")<<after.faceAreaErrors[i];
        report<<"],\"displaced_step_rejected\":true}\n";
        return report.good()?0:1;
    } catch(const std::exception& e){std::cerr<<e.what()<<'\n';return 1;}
      catch(const Standard_Failure& e){std::cerr<<e.GetMessageString()<<'\n';return 1;}
}
