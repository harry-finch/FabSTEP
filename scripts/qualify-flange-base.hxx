// Measured underside of Flansch. This laboratory only writes open shells.
struct Bore {
    gp_Pnt center;
    double radius, farStation, farRadius;
};
struct Base {
    double station, sourceArea;
    bool cones;
    std::array<Bore,5> holes;
};

Base readBase(std::istream& input) {
    Base base{};
    int cones;
    check(bool(input>>base.station>>cones>>base.sourceArea) && std::isfinite(base.station) &&
          (cones==0 || cones==1),"invalid base station");
    base.cones=cones!=0;
    check(std::isfinite(base.sourceArea) && (!base.cones || base.sourceArea>0.),"missing source area");
    for(auto& hole:base.holes) {
        double y,z;
        check(bool(input>>y>>z>>hole.radius) && std::isfinite(y) && std::isfinite(z) &&
              std::isfinite(hole.radius) && hole.radius>eps,"invalid base circle");
        hole.center=gp_Pnt(base.station,y,z);
        if(base.cones)
            check(bool(input>>hole.farStation>>hole.farRadius) && std::isfinite(hole.farStation) &&
                  std::isfinite(hole.farRadius) && hole.farStation>base.station+eps &&
                  hole.farRadius>eps && std::abs(hole.farRadius-hole.radius)>eps,"invalid exit circle");
    }

    return base;
}

TopoDS_Wire undersideWire(const TopoDS_Shape& belt,double station) {
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(belt,TopAbs_EDGE,TopAbs_FACE,adj);
    TopTools_ListOfShape low;
    for(int i=1;i<=adj.Extent();++i) {
        if(adj(i).Extent()!=1)continue;
        const auto edge=TopoDS::Edge(adj.FindKey(i));
        const BRepAdaptor_Curve c(edge);
        const double a=c.FirstParameter(), b=c.LastParameter();
        if(std::abs(c.Value(a).X()-station)>eps || std::abs(c.Value(b).X()-station)>eps)continue;
        check(c.GetType()==GeomAbs_Line || (c.GetType()==GeomAbs_Circle &&
              c.Circle().Axis().Direction().IsParallel(gp::DX(),eps/c.Circle().Radius())),
              "belt outline leaves the base plane");
        low.Append(edge);
    }
    check(low.Extent()==12,"underside contour incomplete");
    BRepBuilderAPI_MakeWire wire;wire.Add(low);
    check(wire.IsDone() && wire.Wire().Closed() && number(wire.Wire(),TopAbs_EDGE)==12,"underside wire");
    return wire.Wire();
}

double planarArea(const TopoDS_Wire& outer,const Base& base) {
    // Green's integral: oriented chord area plus the circular segment for arcs.
    double integral=0.;
    int count=0;
    for(BRepTools_WireExplorer it(outer);it.More();it.Next()) {
        const auto edge=it.Current();
        const BRepAdaptor_Curve c(edge);
        const double lo=c.FirstParameter(),hi=c.LastParameter();
        auto a=c.Value(lo), b=c.Value(hi);
        const bool reversed=edge.Orientation()==TopAbs_REVERSED;
        if(reversed)std::swap(a,b);
        integral+=a.Y()*b.Z()-a.Z()*b.Y();
        if(c.GetType()==GeomAbs_Circle) {
            const auto circle=c.Circle();
            const double angle=(hi-lo)*circle.Axis().Direction().Dot(gp::DX())*(reversed?-1.:1.);
            integral+=circle.Radius()*circle.Radius()*(angle-std::sin(angle));
        }
        ++count;
    }
    check(count==12,"wire traversal incomplete");
    double area=std::abs(integral)*.5;
    for(const auto& hole:base.holes)area-=pi*hole.radius*hole.radius;
    check(area>0.,"invalid net base area");
    return area;
}

TopoDS_Shape buildBase(const TopoDS_Shape& belt,const Base& base) {
    const auto outer=undersideWire(belt,base.station);
    const gp_Pln support(gp_Pnt(base.station,0,0),gp::DX());
    BRepBuilderAPI_MakeFace plane(support,outer,true);
    BRepBuilderAPI_Sewing sewing(eps);
    for(const auto& hole:base.holes) {
        const gp_Ax3 frame(hole.center,gp::DX(),gp::DY());
        auto edge=BRepBuilderAPI_MakeEdge(gp_Circ(frame.Ax2(),hole.radius)).Edge();
        if(base.cones) {
            const double height=hole.farStation-base.station, dr=hole.farRadius-hole.radius;
            const gp_Cone cone(frame,std::atan2(dr,height),hole.radius);
            auto wall=BRepBuilderAPI_MakeFace(cone,0.,2*pi,0.,std::hypot(height,dr)).Face();
            int found=0;
            for(TopExp_Explorer it(wall,TopAbs_EDGE);it.More();it.Next()) {
                const auto candidate=TopoDS::Edge(it.Current());
                const BRepAdaptor_Curve c(candidate);
                if(c.GetType()==GeomAbs_Circle && c.Circle().Location().Distance(hole.center)<eps) {
                    edge=TopoDS::Edge(candidate.Oriented(TopAbs_FORWARD));++found;
                }
            }
            check(found==1,"missing cone inlet");
            sewing.Add(wall);
        }
        const BRepAdaptor_Curve circle(edge);
        if(circle.Circle().Axis().Direction().Dot(gp::DX())>0.)edge.Reverse();
        plane.Add(BRepBuilderAPI_MakeWire(edge).Wire());
    }
    check(plane.IsDone() && BRepCheck_Analyzer(plane.Face(),true).IsValid(),"perforated plane");
    sewing.Add(plane.Face());sewing.Add(belt);sewing.Perform();
    auto result=sewing.SewedShape();
    for(TopExp_Explorer it(result,TopAbs_FACE);it.More();it.Next()) {
        const auto face=TopoDS::Face(it.Current());
        const BRepAdaptor_Surface surface(face);
        if(surface.GetType()==GeomAbs_Plane) {
            auto normal=surface.Plane().Axis().Direction();
            if(face.Orientation()==TopAbs_REVERSED)normal.Reverse();
            if(normal.X()>0)result.Reverse();
            return result;
        }
    }
    throw std::runtime_error("base missing after sewing");
}

bool atBore(const BRepAdaptor_Curve& c,const gp_Pnt& center,double radius) {
    return c.GetType()==GeomAbs_Circle && c.Circle().Location().Distance(center)<eps &&
        std::abs(c.Circle().Radius()-radius)<eps && c.Circle().Axis().Direction().IsParallel(gp::DX(),eps/radius) &&
        std::abs(c.LastParameter()-c.FirstParameter()-2*pi)*radius<eps;
}

int checkBore(const TopoDS_Face& face,const Base& base,Limits& limits) {
    const BRepAdaptor_Surface surf(face);
    const auto cone=surf.Cone();
    for(size_t i=0;i<base.holes.size();++i) {
        const auto& hole=base.holes[i];
        const gp_Pnt far(hole.farStation,hole.center.Y(),hole.center.Z());
        const double dr=hole.farRadius-hole.radius,h=hole.farStation-base.station;
        const double slant=std::hypot(dr,h);
        if(!cone.Axis().Direction().IsParallel(gp::DX(),eps/slant) ||
           gp_Lin(cone.Axis()).Distance(hole.center)>eps ||
           std::abs(std::abs(cone.SemiAngle())-std::abs(std::atan2(dr,h)))*slant>eps)continue;
        check(number(face,TopAbs_WIRE)==1 && number(face,TopAbs_EDGE)==3 &&
              std::abs(surf.LastUParameter()-surf.FirstUParameter()-2*pi)*std::max(hole.farRadius,hole.radius)<eps &&
              std::abs(surf.LastVParameter()-surf.FirstVParameter()-slant)<eps,"cone chart extents");
        gp_Pnt sample;
        gp_Vec du,dv;
        surf.D1((surf.FirstUParameter()+surf.LastUParameter())*.5,
                (surf.FirstVParameter()+surf.LastVParameter())*.5,sample,du,dv);
        gp_Vec normal=du.Crossed(dv);
        if(face.Orientation()==TopAbs_REVERSED)normal.Reverse();
        gp_Vec radial(hole.center,sample);radial.SetX(0.);
        check(normal.Dot(radial)<0.,"cone normal outside bore");
        int nearEdges=0,farEdges=0,seams=0;
        std::array<int,4> sides{};
        for(TopExp_Explorer it(face,TopAbs_EDGE);it.More();it.Next()) {
            const auto edge=TopoDS::Edge(it.Current());
            const BRepAdaptor_Curve c(edge);
            ++sides[chartBorder(face,edge,BRep_Tool::IsClosed(edge,face),2*pi,slant)];
            if(atBore(c,hole.center,hole.radius))++nearEdges;
            else if(atBore(c,far,hole.farRadius))++farEdges;
            else if(BRep_Tool::IsClosed(edge,face) && c.GetType()==GeomAbs_Line)++seams;
        }
        check(sides==std::array<int,4>{1,1,1,1} && nearEdges==1 && farEdges==1 && seams==2,"incomplete cone boundary");
        GProp_GProps prop;BRepGProp::SurfaceProperties(face,prop,1.e-12);
        limits.areaError=std::max(limits.areaError,std::abs(prop.Mass()-pi*(hole.radius+hole.farRadius)*slant));
        return int(i);
    }
    throw std::runtime_error("unmatched bore support");
}

Limits verifyBase(const TopoDS_Shape& shape,const Base& base,const Sectors& sectors,const Tubes& tubes,double expectedArea) {
    check(number(shape,TopAbs_SOLID)==0 && number(shape,TopAbs_SHELL)==1 &&
          number(shape,TopAbs_FACE)==(base.cones?18:13) && BRepCheck_Analyzer(shape,true).IsValid(),"invalid base shell");
    BRep_Builder builder;
    TopoDS_Shell belt;builder.MakeShell(belt);
    TopoDS_Face underside;
    std::vector<TopoDS_Face> bores;
    for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
        const auto face=TopoDS::Face(it.Current());
        switch(BRepAdaptor_Surface(face).GetType()) {
        case GeomAbs_Torus: case GeomAbs_Cylinder: builder.Add(belt,face);break;
        case GeomAbs_Cone:bores.push_back(face);break;
        case GeomAbs_Plane:check(underside.IsNull(),"duplicate plane");underside=face;break;
        default:throw std::runtime_error("unexpected base support");
        }
    }
    auto limits=verify(belt,sectors,tubes);
    check(bores.size()==(base.cones?5:0) && !underside.IsNull() &&
          number(underside,TopAbs_WIRE)==6 && number(underside,TopAbs_EDGE)==17,"base domain count");
    const auto support=BRepAdaptor_Surface(underside).Plane();
    auto normal=support.Axis().Direction();
    if(underside.Orientation()==TopAbs_REVERSED)normal.Reverse();
    check(normal.IsEqual(-gp::DX(),eps/100.) && std::abs(support.Location().X()-base.station)<eps,"base normal/station");
    std::array<int,5> boreUses{}, inletUses{}, exitUses{};
    for(const auto& bore:bores)++boreUses[checkBore(bore,base,limits)];
    GProp_GProps prop;BRepGProp::SurfaceProperties(underside,prop,1.e-12);
    limits.areaError=std::max(limits.areaError,std::abs(prop.Mass()-expectedArea));
    check(limits.areaError<1.e-7,"base surface area");
    // In the plane, both representations must be lines or circles of equal
    // length with the same physical position and direction of traversal.
    const BRepAdaptor_Surface plane(underside);
    for(TopExp_Explorer it(underside,TopAbs_EDGE);it.More();it.Next()) {
        const auto edge=TopoDS::Edge(it.Current());
        const BRepAdaptor_Curve c(edge);
        const BRepAdaptor_Curve2d uv(edge,underside);
        check(uv.GetType()==c.GetType() && (c.GetType()==GeomAbs_Line || c.GetType()==GeomAbs_Circle),
              "nonanalytic base trim");
        const double extent=c.LastParameter()-c.FirstParameter();
        const double uvExtent=uv.LastParameter()-uv.FirstParameter();
        const double scale=c.GetType()==GeomAbs_Circle?c.Circle().Radius():1.;
        check(std::abs(extent-uvExtent)*scale<eps,"inconsistent base trim range");
        if(c.GetType()==GeomAbs_Circle) {
            const auto ref=uv.Circle();
            check(plane.Value(ref.Location().X(),ref.Location().Y()).Distance(c.Circle().Location())<eps &&
                  std::abs(ref.Radius()-scale)<eps,"inconsistent base trim circle");
        }
        for(const double t:{0.,.25,.5,1.}) {
            const auto p=uv.Value(uv.FirstParameter()+t*uvExtent);
            check(plane.Value(p.X(),p.Y()).Distance(c.Value(c.FirstParameter()+t*extent))<eps,"misplaced base trim");
        }
    }
    const auto exterior=BRepTools::OuterWire(underside);
    TopTools_IndexedMapOfShape outerEdges;TopExp::MapShapes(exterior,TopAbs_EDGE,outerEdges);
    check(outerEdges.Extent()==12,"plane outer wire count");
    TopTools_IndexedDataMapOfShapeListOfShape adj;TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,adj);
    int perimeter=0,seams=0,sharedInlets=0;
    std::vector<TopoDS_Edge> freeBelt;
    for(int i=1;i<=adj.Extent();++i) {
        const auto edge=TopoDS::Edge(adj.FindKey(i));
        const auto& faces=adj(i);
        check(faces.Extent()==1 || faces.Extent()==2,"ambiguous base interface");
        const auto a=TopoDS::Face(faces.First()),b=TopoDS::Face(faces.Last());
        const bool plane=a.IsSame(underside)||b.IsSame(underside);
        const BRepAdaptor_Curve c(edge);
        if(faces.Extent()==2 && !a.IsSame(b))check(use(a,edge)!=use(b,edge),"base coedge orientation");
        if(outerEdges.Contains(edge)) {
            const auto& other=a.IsSame(underside)?b:a;
            auto kind=BRepAdaptor_Surface(other).GetType();
            check(plane && faces.Extent()==2 && (kind==GeomAbs_Cylinder || kind==GeomAbs_Torus),"base/belt adjacency");
            ++perimeter;
        } else if(plane) {
            const auto& other=a.IsSame(underside)?b:a;
            check(base.cones ? faces.Extent()==2 && BRepAdaptor_Surface(other).GetType()==GeomAbs_Cone :
                  faces.Extent()==1,"hole adjacency");
            bool found=false;
            for(size_t j=0;j<base.holes.size();++j)if(atBore(c,base.holes[j].center,base.holes[j].radius)){
                ++inletUses[j];found=true;
                if(base.cones){check(checkBore(other,base,limits)==int(j),"incorrect bore neighbor");++sharedInlets;}
            }
            check(found,"unmatched inlet circle");
        } else if(BRepAdaptor_Surface(a).GetType()==GeomAbs_Cone) {
            if(a.IsSame(b) && faces.Extent()==2) {check(BRep_Tool::IsClosed(edge,a),"unshared cone seam");++seams;}
            else {
                check(faces.Extent()==1,"unexpected exit join");
                const int j=checkBore(a,base,limits);
                const auto& hole=base.holes[j];
                check(atBore(c,gp_Pnt(hole.farStation,hole.center.Y(),hole.center.Z()),hole.farRadius),"cone exit circle");
                ++exitUses[j];
            }
        } else if(faces.Extent()==1)freeBelt.push_back(edge);
    }
    check(perimeter==12 && sharedInlets==(base.cones?5:0) && seams==(base.cones?5:0) &&
          freeBelt.size()==12 && boundaryLoops(freeBelt,true)==1,"base incidences");
    for(size_t i=0;i<5;++i)check(inletUses[i]==1 && exitUses[i]==int(base.cones) &&
                                 boreUses[i]==int(base.cones),"five distinct bores");
    return limits;
}

#include "qualify-flange-flanks.hxx"

int qualifyBase(const std::filesystem::path& dir,const Sectors& sectors,const Tubes& tubes,const Base& base) {
    const auto belt=build(sectors,tubes);
    const double area=planarArea(undersideWire(belt,base.station),base);
    if(base.cones)check(std::abs(area-base.sourceArea)<1.e-7,"source plane area differs");
    const auto source=buildBase(belt,base);
    std::cin>>std::ws;
    if(!std::cin.eof())return qualifyFlanks(dir,sectors,tubes,base,source,area);
    const auto before=verifyBase(source,base,sectors,tubes,area);
    const auto imported=roundtrip(source,dir/"flange-base.step");
    const auto after=verifyBase(imported,base,sectors,tubes,area);
    auto shifted=base;shifted.holes.front().center.Translate(gp_Vec(0.,.05,0.));
    const auto wrong=roundtrip(buildBase(belt,shifted),dir/"displaced-bore.step");
    bool rejected=false;
    try {verifyBase(wrong,base,sectors,tubes,area);}
    catch(const std::runtime_error& e){const std::string why=e.what();
        rejected=why=="unmatched bore support" || why=="unmatched inlet circle";}
    check(rejected,"displaced bore accepted or unexpected rejection");
    std::ofstream out(dir/"base-report.json");
    out<<std::setprecision(16)<<"{\"solids\":0,\"shells\":1,\"faces\":"<<(base.cones?18:13)
       <<",\"cones\":"<<(base.cones?5:0)<<",\"belt_base_shared_edges\":12,\"shared_inlets\":"<<(base.cones?5:0)
       <<",\"free_edges\":17,\"free_loops\":6,\"net_base_area_mm2\":"<<area
       <<",\"max_face_area_error_mm2\":"<<std::max(before.areaError,after.areaError)
       <<",\"displaced_bore_rejected\":true}\n";
    return out.good()?0:1;
}
