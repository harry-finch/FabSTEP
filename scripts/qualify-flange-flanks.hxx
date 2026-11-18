// Attach each measured short wall and its upper fillet to the same base.
struct Flank {
    int lower;
    double radius;
    Sector upper;
};
using Flanks=std::vector<Flank>;

Flanks readFlanks(std::istream& in,const Sectors& sectors) {
    Flanks flanks;
    std::vector<bool> seen(sectors.size());
    for(int i=0;i<4;++i) {
        int lower;
        double radius;
        check(bool(in>>lower>>radius) && lower>=0 && size_t(lower)<sectors.size() && !seen[lower] &&
              std::isfinite(radius) && radius>0.,"invalid short flank");
        seen[lower]=true;
        const auto top=read(in);
        const auto& bottom=sectors[lower];
        const auto near=boundary(bottom,3), far=boundary(top,2);
        const gp_Vec rise(near.center,far.center);
        check(near.normal.IsEqual(gp::DX(),eps/radius) && far.normal.IsEqual(gp::DX(),eps/radius) &&
              std::abs(near.radius-radius)<eps && std::abs(far.radius-radius)<eps &&
              std::abs(near.span-far.span)*radius<eps && rise.X()>eps && std::hypot(rise.Y(),rise.Z())<eps &&
              near.a.Translated(rise).Distance(far.a)<eps && near.b.Translated(rise).Distance(far.b)<eps,
              "noncoaxial short wall boundaries");
        flanks.push_back({lower,radius,top});
    }
    in>>std::ws;
    return flanks;
}

TopoDS_Shape buildFlanks(const TopoDS_Shape& base,const Sectors& sectors,const Flanks& flanks) {
    BRepBuilderAPI_Sewing sew(eps);sew.Add(base);
    for(const auto& f:flanks) {
        const auto& bottom=sectors[f.lower];
        const auto low=boundary(bottom,3);
        const auto& high=f.upper;
        const gp_Ax3 frame(low.center,low.normal,bottom.start);
        const auto height=boundary(high,2).center.X()-low.center.X();
        sew.Add(BRepBuilderAPI_MakeFace(gp_Cylinder(frame,f.radius),0.,bottom.u,0.,height).Face());
        const gp_Torus torus(gp_Ax3(high.center,high.axis,high.start),high.R,high.r);
        sew.Add(BRepBuilderAPI_MakeFace(torus,0.,high.u,high.v0,high.v1).Face());
    }
    sew.Perform();return sew.SewedShape();
}

int checkShortWall(const TopoDS_Face& face,const Sectors& sectors,const Flanks& flanks,Limits& limits) {
    const BRepAdaptor_Surface surface(face);
    const auto cylinder=surface.Cylinder();
    for(size_t i=0;i<flanks.size();++i) {
        const auto& f=flanks[i];
        const auto low=boundary(sectors[f.lower],3), high=boundary(f.upper,2);
        const auto rise=gp_Vec(low.center,high.center);
        const double height=rise.X();
        if(!cylinder.Axis().Direction().IsParallel(gp::DX(),eps/height) ||
           std::abs(cylinder.Radius()-f.radius)>eps ||
           gp_Lin(cylinder.Axis()).Distance(low.center)>eps)continue;
        check(number(face,TopAbs_EDGE)==4 && number(face,TopAbs_WIRE)==1,"short wall topology");
        std::array<int,4> sides{},chart{};
        for(TopExp_Explorer it(face,TopAbs_EDGE);it.More();it.Next()) {
            const auto edge=TopoDS::Edge(it.Current());
            const BRepAdaptor_Curve c(edge);
            const double lo=c.FirstParameter(),hi=c.LastParameter();
            if(c.GetType()==GeomAbs_Circle) {
                Limits measured;
                if(c.Circle().Location().Distance(low.center)<eps) {
                    check(match(sectors[f.lower],c,measured)==3,"short wall base arc");++sides[0];
                } else {
                    check(match(f.upper,c,measured)==2,"short wall upper arc");++sides[1];
                }
                limits.pointError=std::max(limits.pointError,measured.pointError);
            } else {
                check(c.GetType()==GeomAbs_Line,"nonlinear generator");
                auto a=c.Value(lo),b=c.Value(hi);
                if(a.X()>b.X())std::swap(a,b);
                if(a.Distance(low.a)<eps && b.Distance(high.a)<eps)++sides[2];
                else {check(a.Distance(low.b)<eps && b.Distance(high.b)<eps,"misplaced generator");++sides[3];}
            }
            ++chart[chartBorder(face,edge,c.GetType()==GeomAbs_Line,low.span,height)];
        }
        check(sides==std::array<int,4>{1,1,1,1} && chart==sides,"incomplete short wall chart");
        GProp_GProps prop;BRepGProp::SurfaceProperties(face,prop,1e-12);
        limits.areaError=std::max(limits.areaError,std::abs(prop.Mass()-f.radius*low.span*height));
        return int(i);
    }
    throw std::runtime_error("unmatched short wall");
}

Limits verifyFlanks(const TopoDS_Shape& shape,const Base& base,const Sectors& sectors,const Tubes& tubes,
                    const Flanks& flanks,double area) {
    check(number(shape,TopAbs_FACE)==26 && number(shape,TopAbs_SHELL)==1 && number(shape,TopAbs_SOLID)==0 &&
          BRepCheck_Analyzer(shape,true).IsValid(),"unexpected combined shell");
    Sectors upper;
    for(const auto& f:flanks)upper.push_back(f.upper);
    TopTools_IndexedMapOfShape highFaces,wallFaces;
    std::array<int,5> highIndex{},wallIndex{};
    Limits measured;
    TopoDS_Shell lower;BRep_Builder builder;builder.MakeShell(lower);
    for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
        const auto face=TopoDS::Face(it.Current());
        const BRepAdaptor_Surface surf(face);
        if(surf.GetType()==GeomAbs_Torus && std::any_of(upper.begin(),upper.end(),[&](const Sector& s){
            return s.center.Distance(surf.Torus().Location())<eps; })) {
            const int index=sectorIndex(face,upper);
            highIndex.at(highFaces.Add(face))=index;
            const double error=verifySector(face,upper[index],measured);
            measured.areaError=std::max(measured.areaError,error);
        } else if(surf.GetType()==GeomAbs_Cylinder && surf.Cylinder().Axis().Direction().IsParallel(gp::DX(),1.e-7)) {
            wallIndex.at(wallFaces.Add(face))=checkShortWall(face,sectors,flanks,measured);
        } else builder.Add(lower,face);
    }
    check(highFaces.Extent()==4 && wallFaces.Extent()==4,"missing short flank");
    const auto bottom=verifyBase(lower,base,sectors,tubes,area);
    measured.areaError=std::max(measured.areaError,bottom.areaError);
    measured.pointError=std::max(measured.pointError,bottom.pointError);
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,adj);
    std::array<int,4> lowJoins{},highJoins{};
    std::vector<TopoDS_Edge> perimeter;
    int closedRings=0;
    for(int i=1;i<=adj.Extent();++i) {
        const auto edge=TopoDS::Edge(adj.FindKey(i));
        const auto& faces=adj(i);
        check(faces.Extent()==1 || faces.Extent()==2,"combined incidence count");
        const auto a=TopoDS::Face(faces.First()),b=TopoDS::Face(faces.Last());
        if(faces.Extent()==1) {
            if(BRepAdaptor_Surface(a).GetType()==GeomAbs_Cone)++closedRings;
            else perimeter.push_back(edge);
            continue;
        }
        if(!a.IsSame(b))check(use(a,edge)!=use(b,edge),"combined coedge orientation");
        if(!wallFaces.Contains(a) && !wallFaces.Contains(b)) {
            check(!highFaces.Contains(a) && !highFaces.Contains(b),"unexpected upper join");continue;
        }
        const auto wall=wallFaces.Contains(a)?a:b, other=wallFaces.Contains(a)?b:a;
        const int j=wallIndex[wallFaces.FindIndex(wall)];
        const BRepAdaptor_Curve curve(edge);
        if(highFaces.Contains(other)) {
            check(highIndex[highFaces.FindIndex(other)]==j && match(upper[j],curve,measured)==2,"wrong upper join");
            ++highJoins[j];
        } else {
            const int k=flanks[j].lower;
            check(sectorIndex(other,sectors)==k && match(sectors[k],curve,measured)==3,"wrong lower join");
            ++lowJoins[j];
        }
    }
    check(lowJoins==std::array<int,4>{1,1,1,1} && highJoins==lowJoins && measured.areaError<1.e-7,
          "incorrect short flank joins or face area");
    check(perimeter.size()==28 && boundaryLoops(perimeter,false)==1 && closedRings==5,"remaining combined boundary");
    return measured;
}

#include "qualify-flange-upper-tubes.hxx"

int qualifyFlanks(const std::filesystem::path& dir,const Sectors& sectors,const Tubes& tubes,
                  const Base& base,const TopoDS_Shape& source,double area) {
    check(base.cones,"flanks require the five measured bores");
    const auto flanks=readFlanks(std::cin,sectors);
    const auto combined=buildFlanks(source,sectors,flanks);
    if(!std::cin.eof())return qualifyUpperTubes(dir,sectors,tubes,base,flanks,combined,area);
    const auto before=verifyFlanks(combined,base,sectors,tubes,flanks,area);
    const auto after=verifyFlanks(roundtrip(combined,dir/"base-flanks.step"),base,sectors,tubes,flanks,area);
    auto shifted=flanks;shifted.front().upper.center.Translate(gp_Vec(.05,0.,0.));
    const auto wrong=roundtrip(buildFlanks(source,sectors,shifted),dir/"shifted-upper.step");
    bool rejected=false;
    try {verifyFlanks(wrong,base,sectors,tubes,flanks,area);}
    catch(const std::runtime_error& e){const std::string why=e.what();
        rejected=why=="misplaced generator" || why=="physical edge outside expected domain";}
    check(rejected,"shifted upper accepted or unexpected rejection");
    std::ofstream out(dir/"base-flanks-report.json");
    out<<std::setprecision(16)<<"{\"solids\":0,\"shells\":1,\"faces\":26,\"short_flanks\":4,"
       <<"\"shared_flank_arcs\":8,\"free_edges\":33,\"free_loops\":6,\"net_base_area_mm2\":"<<area
       <<",\"max_face_area_error_mm2\":"<<std::max(before.areaError,after.areaError)
       <<",\"max_arc_sample_error_mm\":"<<std::max(before.pointError,after.pointError)
       <<",\"shifted_upper_rejected\":true}\n";
    return out.good()?0:1;
}
