// Perforated wall: every polygonal hole is retained as an open interface.
using LoopUV=std::vector<gp_Pnt2d>;
struct PiercedSide {
    Rectangle rect;
    gp_Ax3 frame;
    std::vector<LoopUV> holes;
    size_t edges=0;
    double area=0.;
    gp_Pnt point(const gp_Pnt2d& p) const {
        return frame.Location().Translated(gp_Vec(frame.XDirection())*p.X()+gp_Vec(frame.YDirection())*p.Y());
    }
    gp_Pnt2d project(const gp_Pnt& p) const {
        const auto d=gp_Vec(frame.Location(),p);
        return {d.Dot(gp_Vec(frame.XDirection())),d.Dot(gp_Vec(frame.YDirection()))};
    }
};

double signedArea(const LoopUV& loop) {
    double sum=0.;
    for(size_t i=0;i<loop.size();++i) {
        const auto& a=loop[i];const auto& b=loop[(i+1)%loop.size()];
        sum+=a.X()*b.Y()-a.Y()*b.X();
    }
    return .5*sum;
}

PiercedSide readPierced(const Flanks& flanks,const Sectors& sectors,const Tubes& bands) {
    int sector,end;
    std::array<double,9> v;
    check(bool(std::cin>>sector>>end),"missing pierced face");
    for(auto& x:v)check(bool(std::cin>>x) && std::isfinite(x),"invalid pierced frame");
    const auto t=std::find_if(bands.begin(),bands.end(),[&](const Tube& band){
        return flanks[band.sector].lower==sector && band.end==end;
    });
    check(t!=bands.end(),"unknown pierced window");
    const gp_Pnt origin(v[0],v[1],v[2]);const gp_Dir along(v[3],v[4],v[5]);
    const auto rect=sideRectangle(flanks,sectors,*t);
    const double length=t->extrusion.Magnitude(),height=rect.corners[0].Distance(rect.corners[3]);
    check(rect.corners[0].Distance(origin)<eps && along.IsEqual(gp_Dir(t->extrusion),eps/length) &&
          along.IsNormal(gp::DX(),eps/length) && std::abs(v[6]-length)<eps && std::abs(v[7]-height)<eps,
          "misplaced pierced frame");
    PiercedSide side{rect,gp_Ax3(origin,along.Crossed(gp::DX()),along),{},0,v[8]};
    double area=rect.area;
    for(size_t j=0;j<10;++j) {
        int count;
        check(bool(std::cin>>count) && count>=3 && count<=4096-int(side.edges),"bounded hole contour");
        LoopUV loop;
        for(int i=0;i<count;++i) {
            double u,vv;check(bool(std::cin>>u>>vv) && u>0. && u<length && vv>0. && vv<height,
                              "hole outside measured wall");
            loop.emplace_back(u,vv);
        }
        const double a=signedArea(loop);
        check(a<0.,"incorrect hole orientation");
        area+=a;side.edges+=count;side.holes.push_back(std::move(loop));
    }
    std::cin>>std::ws;check(area>0. && std::abs(area-side.area)<1.e-7,"net pierced area");
    return side;
}

TopoDS_Shape buildPierced(const TopoDS_Shape& lower,const PiercedSide& side) {
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(lower,TopAbs_EDGE,TopAbs_FACE,adj);
    BRepBuilderAPI_MakeFace plane(gp_Pln(side.frame),rectangleWire(adj,side.rect),true);
    for(const auto& hole:side.holes) {
        BRepBuilderAPI_MakePolygon polygon;
        for(const auto& point:hole)polygon.Add(side.point(point));
        polygon.Close();
        check(polygon.IsDone() && number(polygon.Wire(),TopAbs_EDGE)==int(hole.size()),"invalid hole polygon");
        plane.Add(polygon.Wire());
    }
    check(plane.IsDone() && BRepCheck_Analyzer(plane.Face(),true).IsValid(),"invalid pierced face");
    BRepBuilderAPI_Sewing sewing(eps);sewing.Add(lower);sewing.Add(plane.Face());sewing.Perform();
    return sewing.SewedShape();
}

bool samePolygon(const LoopUV& observed,const LoopUV& reference) {
    if(observed.size()!=reference.size())return false;
    const int n=int(observed.size());
    for(int first=0;first<n;++first) {
        if(observed[0].Distance(reference[first])>eps)continue;
        for(int sign:{1,-1}) {
            bool identical=true;
            for(int i=1;i<n;++i)if(observed[i].Distance(reference[(first+sign*i+n)%n])>eps) {
                identical=false;break;
            }
            if(identical)return true;
        }
    }
    return false;
}

void writeMainContour(const std::filesystem::path& path,const std::vector<TopoDS_Edge>& edges) {
    check(edges.size()==20 && boundaryLoops(edges,false)==1,"incomplete main boundary");
    std::ofstream out(path);
    out<<std::setprecision(17)<<"{\"edges\":[";
    auto xyz=[&](const auto& p){out<<"["<<p.X()<<","<<p.Y()<<","<<p.Z()<<"]";};
    for(size_t i=0;i<edges.size();++i) {
        const BRepAdaptor_Curve curve(edges[i]);
        const double lo=curve.FirstParameter(),hi=curve.LastParameter();
        const auto a=curve.Value(lo),b=curve.Value(hi);
        if(i)out<<",";
        out<<"{\"start_mm\":";xyz(a);
        out<<",\"end_mm\":";xyz(b);
        if(curve.GetType()==GeomAbs_Circle) {
            const auto c=curve.Circle();
            const double span=hi-lo;
            check(span>0. && span<2*pi && a.Rotated(c.Axis(),span).Distance(b)<eps &&
                  a.Rotated(c.Axis(),span/2.).Distance(curve.Value((hi+lo)/2.))<eps,"invalid measured circle sweep");
            out<<",\"kind\":\"circle\",\"center_mm\":";xyz(c.Location());
            out<<",\"axis\":";xyz(c.Axis().Direction());
            out<<",\"radius_mm\":"<<c.Radius()<<",\"sweep_rad\":"<<span;
        } else {
            check(curve.GetType()==GeomAbs_Line,"unknown main boundary edge");
            out<<",\"kind\":\"line\"";
        }
        out<<"}";
    }
    out<<"]}\n";out.close();check(out.good(),"writing main contour failed");
}

Limits verifyPierced(const TopoDS_Shape& shape,const Base& base,const Sectors& sectors,const Tubes& tubes,
                     const Flanks& flanks,const Sectors& upper,const Tubes& upperTubes,double area,
                     const std::vector<Rectangle>& rects,const PiercedSide& side,
                     const std::filesystem::path& contourPath={}) {
    check(number(shape,TopAbs_SOLID)==0 && number(shape,TopAbs_SHELL)==1 && number(shape,TopAbs_FACE)==34 &&
          BRepCheck_Analyzer(shape,true).IsValid(),"invalid pierced shell");
    TopoDS_Face wall;
    TopoDS_Shell sub;BRep_Builder builder;builder.MakeShell(sub);
    for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
        const auto face=TopoDS::Face(it.Current());
        const BRepAdaptor_Surface surf(face);
        if(surf.GetType()==GeomAbs_Plane && surf.Plane().Distance(side.frame.Location())<eps &&
           surf.Plane().Axis().Direction().IsParallel(side.rect.outward,eps/100.)) {
            check(wall.IsNull(),"duplicate pierced wall");wall=face;
        } else builder.Add(sub,face);
    }
    check(!wall.IsNull() && number(wall,TopAbs_WIRE)==11 && number(wall,TopAbs_EDGE)==int(4+side.edges),
          "incomplete pierced perimeter");
    auto limits=verifySides(sub,base,sectors,tubes,flanks,upper,upperTubes,area,rects);
    const BRepAdaptor_Surface plane(wall);
    auto normal=plane.Plane().Axis().Direction();
    if(wall.Orientation()==TopAbs_REVERSED)normal.Reverse();
    check(normal.IsEqual(side.rect.outward,eps/100.),"wrong pierced normal");
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,adj);
    const auto outer=BRepTools::OuterWire(wall);
    std::vector<int> matches(side.holes.size());
    std::array<int,4> sides{};
    double outerArea=0.,holeSign=0.;
    for(TopExp_Explorer it(wall,TopAbs_WIRE);it.More();it.Next()) {
        const auto wire=TopoDS::Wire(it.Current());
        const bool exterior=wire.IsSame(outer);
        LoopUV loop;
        for(BRepTools_WireExplorer wit(wire);wit.More();wit.Next()) {
            const auto edge=wit.Current();
            const auto position=BRep_Tool::Pnt(wit.CurrentVertex());
            const auto uv=side.project(position);
            check(position.Distance(side.point(uv))<eps,"pierced vertex outside plane");
            loop.push_back(uv);
            const BRepAdaptor_Curve c(edge);const BRepAdaptor_Curve2d pcurve(edge,wall);
            check(c.GetType()==GeomAbs_Line && pcurve.GetType()==GeomAbs_Line,"nonstraight pierced edge");
            for(double t:{0.,1.}) {
                const auto p=pcurve.Value(pcurve.FirstParameter()+t*(pcurve.LastParameter()-pcurve.FirstParameter()));
                check(plane.Value(p.X(),p.Y()).Distance(c.Value(c.FirstParameter()+t*(c.LastParameter()-c.FirstParameter())))<eps,
                      "misplaced pierced trim");
            }
            const auto& faces=adj.FindFromKey(edge);
            if(exterior) {
                const int k=rectangleEdge(c,side.rect);
                check(k>=0 && faces.Extent()==2,"incomplete shared pierced rectangle");++sides[k];
                const auto other=TopoDS::Face(faces.First().IsSame(wall)?faces.Last():faces.First());
                const BRepAdaptor_Surface support(other);
                check(support.GetType()==GeomAbs_Cylinder && use(wall,edge)!=use(other,edge) &&
                      support.Cylinder().Axis().Direction().IsParallel(gp::DX(),eps)==(k%2==1),
                      "wrong pierced adjacency");
            } else check(faces.Extent()==1,"unexpected neighbor inside decorated contour");
        }
        check(int(loop.size())==number(wire,TopAbs_EDGE) && wire.Closed(),"incomplete polygon loop");
        const double polygonArea=signedArea(loop);
        if(exterior)outerArea=polygonArea;
        else {
            check(polygonArea!=0. && (holeSign==0. || polygonArea*holeSign>0.),"inconsistent inner wire orientations");
            holeSign=polygonArea;
            int found=0;
            for(size_t j=0;j<side.holes.size();++j)if(samePolygon(loop,side.holes[j])){++matches[j];++found;}
            check(found==1,"wrong recessed contour");
        }
    }
    GProp_GProps prop;BRepGProp::SurfaceProperties(wall,prop,1e-12);
    limits.areaError=std::max(limits.areaError,std::abs(prop.Mass()-side.area));
    check(outerArea*holeSign<0. && std::abs(std::abs(outerArea)-side.rect.area)<1.e-7 &&
          sides==std::array<int,4>{1,1,1,1} && std::all_of(matches.begin(),matches.end(),[](int i){return i==1;}) &&
          limits.areaError<1.e-7,"pierced side area or holes");
    std::vector<TopoDS_Edge> free,main;int bores=0;
    for(int i=1;i<=adj.Extent();++i)if(adj(i).Extent()==1) {
        const auto face=TopoDS::Face(adj(i).First());
        if(BRepAdaptor_Surface(face).GetType()==GeomAbs_Cone)++bores;
        else {
            const auto edge=TopoDS::Edge(adj.FindKey(i));
            free.push_back(edge);
            if(!face.IsSame(wall))main.push_back(edge);
        }
    }
    check(bores==5 && free.size()==20+side.edges && boundaryLoops(free,false)==11,"pierced boundary graph");
    if(!contourPath.empty())writeMainContour(contourPath,main);
    return limits;
}

#include "qualify-flange-long.hxx"

int qualifyPiercedSide(const std::filesystem::path& dir,const Sectors& sectors,const Tubes& tubes,
                       const Base& base,const Flanks& flanks,const Sectors& upper,const Tubes& upperTubes,
                       const TopoDS_Shape& lower,double area,const std::vector<Rectangle>& rects) {
    const auto side=readPierced(flanks,sectors,upperTubes);
    const auto shape=buildPierced(lower,side);
    if(!std::cin.eof()) {
        const auto walls=readLongWalls(sectors);
        auto verifyLower=[&](const TopoDS_Shape& oldFaces){
            return verifyPierced(oldFaces,base,sectors,tubes,flanks,upper,upperTubes,area,rects,side);
        };
        return qualifyLong(dir,shape,sectors,walls,verifyLower);
    }
    const auto before=verifyPierced(shape,base,sectors,tubes,flanks,upper,upperTubes,area,rects,side);
    constexpr size_t maxBytes=16*1024*1024;
    const auto after=verifyPierced(roundtrip(shape,dir/"pierced-side.step",maxBytes),base,sectors,tubes,flanks,
                                   upper,upperTubes,area,rects,side,dir/"main-contour.json");
    auto missing=side;missing.holes.pop_back();
    const auto wrong=roundtrip(buildPierced(lower,missing),dir/"missing-contour.step",maxBytes);
    bool rejected=false;
    try{verifyPierced(wrong,base,sectors,tubes,flanks,upper,upperTubes,area,rects,side);}
    catch(const std::runtime_error& e){rejected=std::string(e.what())=="incomplete pierced perimeter";}
    check(rejected,"filled contour accepted or unexpected rejection");
    std::ofstream out(dir/"pierced-side-report.json");
    out<<std::setprecision(16)<<"{\"solids\":0,\"shells\":1,\"faces\":34,\"pierced_loops\":10,"
       <<"\"pierced_polygon_edges\":"<<side.edges<<",\"shared_pierced_edges\":4,\"free_edges\":"<<25+side.edges
       <<",\"free_loops\":16,\"net_pierced_area_mm2\":"<<side.area
       <<",\"max_face_area_error_mm2\":"<<std::max(before.areaError,after.areaError)
       <<",\"missing_contour_rejected\":true}\n";
    return out.good()?0:1;
}
