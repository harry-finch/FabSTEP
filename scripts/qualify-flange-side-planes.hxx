// Each selected rectangle is one complete measured source face including all its loops.
struct Rectangle {
    std::array<gp_Pnt,4> corners;
    gp_Dir outward;
    double area;
};

Rectangle sideRectangle(const Flanks& flanks,const Sectors& sectors,const Tube& band) {
    const auto& flank=flanks[band.sector];
    const auto& lower=sectors[flank.lower];
    const auto& upper=flank.upper;
    const auto a=lower.point(band.end*lower.u,lower.v1),d=upper.point(band.end*upper.u,upper.v0);
    const gp_Vec rise(a,d);
    check(rise.Magnitude()>eps && rise.IsNormal(band.extrusion,eps/band.extrusion.Magnitude()),"oblique side window");
    return {{{a,a.Translated(band.extrusion),d.Translated(band.extrusion),d}},
            lower.radial(band.end*lower.u),rise.Crossed(band.extrusion).Magnitude()};
}

int rectangleEdge(const BRepAdaptor_Curve& edge,const Rectangle& rect) {
    if(edge.GetType()!=GeomAbs_Line)return -1;
    const auto a=edge.Value(edge.FirstParameter()),b=edge.Value(edge.LastParameter());
    for(int i=0;i<4;++i) {
        const auto c=rect.corners[i],d=rect.corners[(i+1)%4];
        if((a.Distance(c)<eps && b.Distance(d)<eps) || (a.Distance(d)<eps && b.Distance(c)<eps))return i;
    }
    return -1;
}

TopoDS_Wire rectangleWire(const TopTools_IndexedDataMapOfShapeListOfShape& adj,const Rectangle& rect) {
    TopTools_ListOfShape sides;
    std::array<int,4> counts{};
    for(int i=1;i<=adj.Extent();++i) {
        if(adj(i).Extent()!=1)continue;
        const auto edge=TopoDS::Edge(adj.FindKey(i));
        const int side=rectangleEdge(BRepAdaptor_Curve(edge),rect);
        if(side>=0){++counts[side];sides.Append(edge);}
    }
    check(counts==std::array<int,4>{1,1,1,1},"incomplete plane window");
    BRepBuilderAPI_MakeWire wire;wire.Add(sides);
    check(wire.IsDone() && wire.Wire().Closed(),"open rectangle");
    return wire.Wire();
}

TopoDS_Shape closeRectangles(const TopoDS_Shape& shell,const std::vector<Rectangle>& rects) {
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shell,TopAbs_EDGE,TopAbs_FACE,adj);
    BRepBuilderAPI_Sewing sew(eps);sew.Add(shell);
    for(const auto& rect:rects) {
        sew.Add(BRepBuilderAPI_MakeFace(gp_Pln(rect.corners[0],rect.outward),rectangleWire(adj,rect),true).Face());
    }
    sew.Perform();return sew.SewedShape();
}

Limits verifySides(const TopoDS_Shape& shape,const Base& base,const Sectors& sectors,const Tubes& tubes,
                   const Flanks& flanks,const Sectors& upper,const Tubes& upperTubes,double area,
                   const std::vector<Rectangle>& rects) {
    check(number(shape,TopAbs_FACE)==33 && number(shape,TopAbs_SHELL)==1 && number(shape,TopAbs_SOLID)==0 &&
          BRepCheck_Analyzer(shape,true).IsValid(),"invalid side plane shell");
    TopoDS_Shell sub;BRep_Builder builder;builder.MakeShell(sub);
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,adj);
    std::array<int,2> uses{};
    double areaError=0.;
    for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
        const auto face=TopoDS::Face(it.Current());
        const BRepAdaptor_Surface surf(face);
        if(surf.GetType()!=GeomAbs_Plane || surf.Plane().Axis().Direction().IsParallel(gp::DX(),eps)) {
            builder.Add(sub,face);continue;
        }
        int index=-1;
        for(size_t i=0;i<rects.size();++i) {
            const auto& r=rects[i];const auto plane=surf.Plane();
            auto normal=plane.Axis().Direction();if(face.Orientation()==TopAbs_REVERSED)normal.Reverse();
            if(normal.IsEqual(r.outward,eps/100.) && plane.Distance(r.corners[0])<eps)index=i;
        }
        check(index>=0,"unexpected side plane");
        ++uses[index];
        check(number(face,TopAbs_WIRE)==1 && number(face,TopAbs_EDGE)==4,"side trim topology");
        std::array<int,4> sides{};
        for(TopExp_Explorer eit(face,TopAbs_EDGE);eit.More();eit.Next()) {
            const auto edge=TopoDS::Edge(eit.Current());
            const auto c=BRepAdaptor_Curve(edge);
            const int side=rectangleEdge(c,rects[index]);
            check(side>=0,"side edge outside measured rectangle");++sides[side];
            const BRepAdaptor_Curve2d trim(edge,face);
            check(trim.GetType()==GeomAbs_Line,"nonlinear side trim");
            for(double t:{0.,1.}) {
                const auto uv=trim.Value(trim.FirstParameter()+t*(trim.LastParameter()-trim.FirstParameter()));
                const auto xyz=c.Value(c.FirstParameter()+t*(c.LastParameter()-c.FirstParameter()));
                check(surf.Value(uv.X(),uv.Y()).Distance(xyz)<eps,"misplaced side trim");
            }
            const auto& faces=adj.FindFromKey(edge);
            check(faces.Extent()==2,"unshared side");
            const auto other=TopoDS::Face(faces.First().IsSame(face)?faces.Last():faces.First());
            const BRepAdaptor_Surface support(other);
            check(support.GetType()==GeomAbs_Cylinder && use(face,edge)!=use(other,edge),"side neighbor");
            const bool vertical=support.Cylinder().Axis().Direction().IsParallel(gp::DX(),eps);
            check(vertical==(side%2==1),"wrong tangency at side edge");
        }
        check(sides==std::array<int,4>{1,1,1,1},"side rectangle coverage");
        GProp_GProps props;BRepGProp::SurfaceProperties(face,props,1e-12);
        areaError=std::max(areaError,std::abs(props.Mass()-rects[index].area));
    }
    check(uses==std::array<int,2>{1,1} && areaError<1.e-7,"side identities or area");
    auto limits=verifyUpperTubes(sub,base,sectors,tubes,flanks,upper,upperTubes,area);
    limits.areaError=std::max(limits.areaError,areaError);
    std::vector<TopoDS_Edge> perimeter;
    int holes=0;
    for(int i=1;i<=adj.Extent();++i)if(adj(i).Extent()==1) {
        const auto face=TopoDS::Face(adj(i).First());
        if(BRepAdaptor_Surface(face).GetType()==GeomAbs_Cone)++holes;
        else perimeter.push_back(TopoDS::Edge(adj.FindKey(i)));
    }
    check(holes==5 && perimeter.size()==24 && boundaryLoops(perimeter,false)==2,"side boundary cycles");
    return limits;
}

#include "qualify-flange-pierced-side.hxx"

int qualifySidePlanes(const std::filesystem::path& dir,const Sectors& sectors,const Tubes& tubes,
                      const Base& base,const Flanks& flanks,const Sectors& upper,const Tubes& upperTubes,
                      const TopoDS_Shape& lower,double area) {
    std::vector<Rectangle> rects;
    std::vector<bool> chosen(upperTubes.size());
    for(int i=0;i<2;++i) {
        int sector,end;
        double sourceArea;
        check(bool(std::cin>>sector>>end>>sourceArea) && std::isfinite(sourceArea),"invalid side measurement");
        const auto t=std::find_if(upperTubes.begin(),upperTubes.end(),[&](const Tube& band){
            return flanks[band.sector].lower==sector && band.end==end;
        });
        check(t!=upperTubes.end() && !chosen[t-upperTubes.begin()],"duplicate or unknown measured side");
        chosen[t-upperTubes.begin()]=true;
        auto rect=sideRectangle(flanks,sectors,*t);
        check(std::abs(rect.area-sourceArea)<1.e-7,"source rectangle area differs");
        rects.push_back(rect);
    }
    std::cin>>std::ws;
    const auto shape=closeRectangles(lower,rects);
    if(!std::cin.eof())return qualifyPiercedSide(dir,sectors,tubes,base,flanks,upper,upperTubes,shape,area,rects);
    const auto before=verifySides(shape,base,sectors,tubes,flanks,upper,upperTubes,area,rects);
    const auto after=verifySides(roundtrip(shape,dir/"side-planes.step"),base,sectors,tubes,flanks,upper,upperTubes,area,rects);
    // A different four-edge opening also admits a valid native patch.
    // The reference must reject that placement even if the BRep sews.
    auto altered=rects;
    bool alternate=false;
    for(size_t i=0;i<upperTubes.size();++i)if(!chosen[i]) {
        const auto& t=upperTubes[i];
        const auto far=boundary(upper[t.sector],t.end).center.Translated(t.extrusion);
        if(std::any_of(upper.begin(),upper.end(),[&](const Sector& s){
            return boundary(s,0).center.Distance(far)<eps || boundary(s,1).center.Distance(far)<eps;
        })) {altered[0]=sideRectangle(flanks,sectors,t);alternate=true;break;}
    }
    check(alternate,"missing alternate native window");
    const auto wrong=roundtrip(closeRectangles(lower,altered),dir/"wrong-window.step");
    bool rejected=false;
    try {verifySides(wrong,base,sectors,tubes,flanks,upper,upperTubes,area,rects);}
    catch(const std::runtime_error& e){rejected=std::string(e.what())=="unexpected side plane";}
    check(rejected,"wrong side window accepted or unexpected rejection");
    std::ofstream out(dir/"side-planes-report.json");
    out<<std::setprecision(16)<<"{\"solids\":0,\"shells\":1,\"faces\":33,\"complete_side_planes\":2,"
       <<"\"shared_side_edges\":8,\"free_edges\":29,\"free_loops\":7,\"net_base_area_mm2\":"<<area
       <<",\"max_face_area_error_mm2\":"<<std::max(before.areaError,after.areaError)
       <<",\"max_arc_sample_error_mm\":"<<std::max(before.pointError,after.pointError)
       <<",\"wrong_window_rejected\":true}\n";
    return out.good()?0:1;
}
