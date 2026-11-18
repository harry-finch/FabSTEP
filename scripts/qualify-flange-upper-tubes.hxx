// Close the upper perimeter wherever a full tangent band was independently measured.
Tubes readUpperTubes(std::istream& input,const Flanks& flanks) {
    Tubes tubes;
    for(int i=0;i<5;++i) {
        int lower,end;
        double dx,dy,dz;
        check(bool(input>>lower>>end>>dx>>dy>>dz) && (end==0 || end==1) &&
              std::isfinite(dx) && std::isfinite(dy) && std::isfinite(dz),"invalid upper band");
        const auto flank=std::find_if(flanks.begin(),flanks.end(),[&](const Flank& f){return f.lower==lower;});
        check(flank!=flanks.end(),"upper band has unknown support");
        tubes.push_back({int(flank-flanks.begin()),end,gp_Vec(dx,dy,dz)});
    }
    input>>std::ws;
    return tubes;
}

TopoDS_Shape buildUpperTubes(const TopoDS_Shape& lower,const Sectors& upper,const Tubes& tubes) {
    BRepBuilderAPI_Sewing sew(eps);sew.Add(lower);
    for(const auto& t:tubes)sew.Add(buildTube(upper,t));
    sew.Perform();return sew.SewedShape();
}

Limits verifyUpperTubes(const TopoDS_Shape& shape,const Base& base,const Sectors& sectors,const Tubes& tubes,
                        const Flanks& flanks,const Sectors& upper,const Tubes& topTubes,double area) {
    check(number(shape,TopAbs_FACE)==31 && number(shape,TopAbs_SHELL)==1 && number(shape,TopAbs_SOLID)==0 &&
          BRepCheck_Analyzer(shape,true).IsValid(),"unexpected upper shell");
    TopoDS_Shell lower;BRep_Builder builder;builder.MakeShell(lower);
    TopTools_IndexedMapOfShape bands;
    std::array<size_t,6> index{};
    Limits limits;
    limits.faceAreaErrors.resize(upper.size()+topTubes.size());
    for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
        const auto face=TopoDS::Face(it.Current());
        const BRepAdaptor_Surface surf(face);
        const bool topCylinder=surf.GetType()==GeomAbs_Cylinder && std::any_of(topTubes.begin(),topTubes.end(),
            [&](const Tube& t){
                const auto arc=boundary(upper[t.sector],t.end);
                const auto axis=surf.Cylinder().Axis();
                return axis.Direction().IsParallel(gp_Dir(t.extrusion),eps/t.extrusion.Magnitude()) &&
                    gp_Lin(axis).Distance(arc.center)<eps;
            });
        if(topCylinder)index.at(bands.Add(face))=tubeIndex(face,upper,topTubes,limits)-upper.size();
        else builder.Add(lower,face);
    }
    check(bands.Extent()==5,"incomplete upper bands");
    const auto sub=verifyFlanks(lower,base,sectors,tubes,flanks,area);
    limits.areaError=std::max(limits.areaError,sub.areaError);
    limits.pointError=std::max(limits.pointError,sub.pointError);
    TopTools_IndexedDataMapOfShapeListOfShape adj;
    TopExp::MapShapesAndAncestors(shape,TopAbs_EDGE,TopAbs_FACE,adj);
    std::array<int,5> proximal{},distal{},freeArc{},lines{};
    std::array<std::array<int,2>,4> meridians{};
    std::vector<TopoDS_Edge> perimeter;
    int holes=0;
    for(int i=1;i<=adj.Extent();++i) {
        const auto& faces=adj(i);
        check(faces.Extent()==1 || faces.Extent()==2,"ambiguous upper interface");
        const auto edge=TopoDS::Edge(adj.FindKey(i));
        const auto a=TopoDS::Face(faces.First()),b=TopoDS::Face(faces.Last());
        const auto c=BRepAdaptor_Curve(edge);
        if(faces.Extent()==1) {
            if(BRepAdaptor_Surface(a).GetType()==GeomAbs_Cone)++holes;
            else perimeter.push_back(edge);
        }
        if(!bands.Contains(a) && !bands.Contains(b))continue;
        const bool first=bands.Contains(a);
        const auto band=first?a:b,other=first?b:a;
        const size_t j=index.at(bands.FindIndex(band));
        const auto& t=topTubes[j];
        if(faces.Extent()==2) {
            check(!bands.Contains(other) && use(band,edge)!=use(other,edge),"upper join orientation");
            const int k=sectorIndex(other,upper),side=match(upper[k],c,limits);
            check(side<2,"joined wrong upper rim");
            ++meridians[k][side];
            if(k==t.sector && side==t.end)++proximal[j];else ++distal[j];
        } else if(c.GetType()==GeomAbs_Circle) {
            const gp_Pnt far=boundary(upper[t.sector],t.end).center.Translated(t.extrusion);
            check(c.Circle().Location().Distance(far)<eps,"unexpected free upper meridian");++freeArc[j];
        } else {check(c.GetType()==GeomAbs_Line,"unexpected upper boundary curve");++lines[j];}
    }
    for(const auto& ends:meridians)check(ends==std::array<int,2>{1,1},"incomplete upper sector joints");
    int joinedEnds=0;
    for(size_t j=0;j<5;++j) {
        check(proximal[j]==1 && distal[j]+freeArc[j]==1 && lines[j]==2,"unexpected upper band domain");
        joinedEnds+=distal[j];
    }
    check(joinedEnds==3 && holes==5 && perimeter.size()==32 && boundaryLoops(perimeter,false)==4 &&
          limits.areaError<1.e-7,"upper tube graph or area");
    return limits;
}

#include "qualify-flange-side-planes.hxx"

int qualifyUpperTubes(const std::filesystem::path& dir,const Sectors& sectors,const Tubes& tubes,
                      const Base& base,const Flanks& flanks,const TopoDS_Shape& lower,double area) {
    Sectors upper;for(const auto& f:flanks)upper.push_back(f.upper);
    const auto upperTubes=readUpperTubes(std::cin,flanks);
    const auto shape=buildUpperTubes(lower,upper,upperTubes);
    if(!std::cin.eof())return qualifySidePlanes(dir,sectors,tubes,base,flanks,upper,upperTubes,shape,area);
    const auto before=verifyUpperTubes(shape,base,sectors,tubes,flanks,upper,upperTubes,area);
    const auto after=verifyUpperTubes(roundtrip(shape,dir/"upper-tubes.step"),base,sectors,tubes,flanks,upper,upperTubes,area);
    // Shorten a band with a free distal end. Count, support and incidence
    // remain identical; the measured rectangular domain must still reject it.
    auto wrong=upperTubes;
    auto free=std::find_if(wrong.begin(),wrong.end(),[&](const Tube& t){
        const auto far=boundary(upper[t.sector],t.end).center.Translated(t.extrusion);
        return std::none_of(upper.begin(),upper.end(),[&](const Sector& s){
            return boundary(s,0).center.Distance(far)<eps || boundary(s,1).center.Distance(far)<eps;
        });
    });
    check(free!=wrong.end(),"no free distal end for control");
    free->extrusion*=1.-.05/free->extrusion.Magnitude();
    const auto shifted=roundtrip(buildUpperTubes(lower,upper,wrong),dir/"shortened-upper.step");
    bool rejected=false;
    try {verifyUpperTubes(shifted,base,sectors,tubes,flanks,upper,upperTubes,area);}
    catch(const std::runtime_error& e){const std::string why=e.what();
        rejected=why=="tube outside measured boundaries" || why=="constant U trim";}
    check(rejected,"shortened upper band accepted or unexpected rejection");
    std::ofstream out(dir/"upper-tubes-report.json");
    out<<std::setprecision(16)<<"{\"solids\":0,\"shells\":1,\"faces\":31,\"upper_tubes\":5,"
       <<"\"upper_shared_meridians\":8,\"free_upper_meridians\":2,\"free_edges\":37,\"free_loops\":9,"
       <<"\"net_base_area_mm2\":"<<area<<",\"max_face_area_error_mm2\":"<<std::max(before.areaError,after.areaError)
       <<",\"max_arc_sample_error_mm\":"<<std::max(before.pointError,after.pointError)
       <<",\"shortened_upper_rejected\":true}\n";
    return out.good()?0:1;
}
