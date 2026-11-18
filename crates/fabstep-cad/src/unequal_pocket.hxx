// Bounded orthogonal pocket of ADR-0043. All trims are shared topological
// edges; only the two UV representations of the exact 3D quartic interpolate.
namespace unequal_pocket {
using Data = UnequalPocketData;
constexpr double tol = 1.e-7, pi = 3.14159265358979323846;
void require(bool condition, Status status, const char* detail) {
  if (!condition) throw CadFailure{status, detail};
}

struct Frame {
  const Data& d;
  gp_Pnt origin;
  std::array<gp_Dir,3> axes;
  double r, R, q, end;
  explicit Frame(const Data& data, Status status)
      : d(data), origin(point_at(data.origin,0)),
        axes{direction(d.axes[0]),direction(d.axes[1]),direction(d.axes[2])},
        r(d.radii[0]), R(d.radii[1]), q(1-r/R), end(1/(1+std::sqrt(1+q))) {
    double extent = 1.;
    for (int i=0; i<3; ++i) {
      require(std::isfinite(d.origin[i]) && std::isfinite(d.outer_min[i]) &&
              std::isfinite(d.outer_max[i]) && d.outer_min[i]<-tol, status, "Limites de poche invalides.");
      extent=std::max(extent,d.outer_max[i]-d.outer_min[i]);
    }
    for (int i=0; i<3; ++i) {
      const gp_Vec v(d.axes[i][0], d.axes[i][1], d.axes[i][2]);
      require(std::abs(v.Magnitude()-1.)*extent<tol &&
              std::abs(axes[i].Dot(axes[(i+1)%3]))*extent<tol, status, "Repère de poche non orthonormé.");
    }
    require(r>tol && R-r>tol && d.pocket_spans[0]-R>tol && d.pocket_spans[1]-r>tol &&
            d.outer_max[0]-d.pocket_spans[0]>tol && d.outer_max[1]-d.pocket_spans[1]>tol &&
            d.outer_max[2]-R>tol, status,"Rayons ou terminaisons de poche invalides.");
  }
  gp_Pnt point(double x, double y, double z) const {
    return origin.Translated(gp_Vec(axes[0])*x + gp_Vec(axes[1])*y + gp_Vec(axes[2])*z);
  }
  double inset(double radius, double h) const {
    return (radius-h)*(radius-h)/(radius+std::sqrt(h*(2*radius-h)));
  }
  gp_Ax3 cylinder(int wall) const {
    const gp_Dir bottom=axes[2].Reversed(), side=axes[1-wall].Reversed();
    return gp_Ax3(wall ? point(R,0,R) : point(0,r,r),bottom.Crossed(side),bottom);
  }
};

// Suffixes name physical vertices, independent of enumeration by OCCT. The
// vertical prism has four corners at its bottom (0..3) and rim (4..7).
enum Vertex { b00,b10,b11,b01,t00,t10,t11,t01, p00,p10,p11,p01,
              joint_floor,joint_side,large_upper,small_floor,small_side,
              large_floor,large_side,far_floor,vertex_count };
using Loop = std::vector<int>;
const std::array<Loop,13> outlines{{
  {b00,b01,t01,t00}, {b10,b11,t11,t10}, {b00,b10,t10,t00}, {b01,b11,t11,t01},
  {b00,b10,b11,b01}, {t00,t10,t11,t01},
  {large_upper,large_side,p01,p00}, {small_side,small_floor,far_floor,p11,p10},
  {joint_side,small_side,p10,p00,large_upper}, {large_side,large_floor,far_floor,p11,p01},
  {joint_floor,small_floor,far_floor,large_floor},
  {joint_floor,small_floor,small_side,joint_side},
  {joint_floor,large_floor,large_side,large_upper,joint_side}
}};
const Loop opening{p00,p10,p11,p01};
const std::array<int,11> plane_axes{0,0,1,1,2,2,0,0,1,1,2};
const std::array<int,11> normal_sign{-1,1,-1,1,-1,1,1,-1,1,-1,1};
using Key = std::pair<int,int>;
Key key(int a,int b) { return std::minmax(a,b); }
std::array<gp_Pnt,vertex_count> vertices(const Frame& f) {
  const double *lo=f.d.outer_min,*hi=f.d.outer_max,L=f.d.pocket_spans[0],W=f.d.pocket_spans[1];
  const double x[]={lo[0],hi[0],hi[0],lo[0]}, y[]={lo[1],lo[1],hi[1],hi[1]};
  std::array<gp_Pnt,vertex_count> points;
  for (int i=0;i<4;++i) {
    points[i]=f.point(x[i],y[i],lo[2]);
    points[i+4]=f.point(x[i],y[i],hi[2]);
    points[i+8]=f.point((i==1 || i==2) ? L : 0.,i>=2 ? W : 0.,hi[2]);
  }
  points[joint_floor]=f.point(f.R,f.r,0);
  points[joint_side]=f.point(f.inset(f.R,f.r),0,f.r);
  points[large_upper]=f.point(0,0,f.R);
  points[small_floor]=f.point(L,f.r,0); points[small_side]=f.point(L,0,f.r);
  points[large_floor]=f.point(f.R,W,0); points[large_side]=f.point(0,W,f.R);
  points[far_floor]=f.point(L,W,0);
  return points;
}

Handle(Geom_BSplineCurve) quartic(const Frame& f) {
  const double q=f.q,r=f.r,R=f.R,g=std::sqrt(r*R);
  const double D[]={1.,0.,4-2*q,0.,q*q}, X[]={0,4*g,0,4*g*q,0}, Y[]={0,4*r,0,-4*r*q,0};
  constexpr int choose[5][5]={{1},{1,1},{1,2,1},{1,3,3,1},{1,4,6,4,1}};
  TColgp_Array1OfPnt poles(1,5);
  TColStd_Array1OfReal weights(1,5);
  for (int i=0;i<=4;++i) {
    double w=0.,x=0.,y=0.,z=0.;
    for (int k=0;k<=i;++k) {
      const double scale=std::pow(f.end,k)*choose[i][k]/choose[4][k];
      w+=scale*D[k]; x+=scale*X[k]; y+=scale*Y[k];
      if (k==2) z+=scale*8*r;
    }
    poles.SetValue(i+1,f.point(R-x/w,r-y/w,z/w));
    weights.SetValue(i+1,w);
  }
  TColStd_Array1OfReal knots(1,2); knots(1)=0.; knots(2)=1.;
  TColStd_Array1OfInteger mults(1,2); mults.Init(5);
  return new Geom_BSplineCurve(poles,weights,knots,mults,4);
}

// In coordinates (angle from floor, distance along the cylinder axis), D is
// positive and t<=1/2. Differentiating D*f=P four times bounds the sum of
// spatial coordinate errors by 80712*R/(384*(2*n)^4). Derivation and
// independently executable exact-rational bound: qualify-unequal-pocket-traces.py.
int trace_spans(double radius, Status status) {
  int n=1;
  while (80712*radius/(384*std::pow(2.*n,4)) > 1.e-8) {
    require(n<1024,status,"Budget de tracé de la jonction dépassé.");
    n*=2;
  }
  return n;
}
Handle(Geom2d_BSplineCurve) trace(const Frame& f, int wall, Status status) {
  const int n=trace_spans(f.R,status);
  const double sign=f.cylinder(wall).Direction().Dot(f.axes[wall]);
  const double a=wall ? 2*std::sqrt(f.r/f.R) : 2, b=wall ? f.q : -f.q;
  const double origin=wall ? f.r : f.R, factor=wall ? 4*f.r : 4*std::sqrt(f.R*f.r);
  TColgp_Array1OfPnt2d poles(1,3*n+1);
  TColStd_Array1OfReal knots(1,n+1);
  TColStd_Array1OfInteger mults(1,n+1);
  for (int i=0;i<=n;++i) {
    const double t=f.end*i/n, tt=t*t, D=1+(4-2*f.q)*tt+f.q*f.q*tt*tt;
    const double dd=2*(4-2*f.q)*t+4*f.q*f.q*t*tt;
    const double p=factor*t*(1-b*tt), dp=factor*(1-3*b*tt);
    gp_Pnt2d uv(2*std::atan2(a*t,1+b*tt),sign*(origin-p/D));
    gp_Vec2d handle(2*a*(1-b*tt)/D,-sign*(dp*D-p*dd)/(D*D));
    handle.Multiply(f.end/(3*n));
    if (i) poles(3*i)=uv.Translated(-handle);
    poles(3*i+1)=uv;
    if (i<n) poles(3*i+2)=uv.Translated(handle);
    knots(i+1)=double(i)/n; mults(i+1)=i==0 || i==n ? 4 : 3;
  }
  return new Geom2d_BSplineCurve(poles,knots,mults,3);
}

struct Arc { int a,b,wall; double axial,first,last; };
std::array<Arc,3> arcs(const Frame& f) {
  return {{{small_floor,small_side,0,f.d.pocket_spans[0],0.,pi/2},
           {large_floor,large_side,1,f.d.pocket_spans[1],0.,pi/2},
           {joint_side,large_upper,1,0.,std::acos(f.q),pi/2}}};
}
gp_Circ circle(const Frame& f,const Arc& arc) {
  gp_Ax2 frame=f.cylinder(arc.wall).Ax2();
  frame.Translate(gp_Vec(f.axes[arc.wall])*arc.axial);
  return gp_Circ(frame,f.d.radii[arc.wall]);
}

TopoDS_Solid build(const Data& data,const size_t* planes,const size_t* walls,FaceLabels* labels) {
  const Frame f(data,invalid_model);
  const auto points=vertices(f);
  BRep_Builder builder;
  std::array<TopoDS_Vertex,vertex_count> nodes;
  for (int i=0;i<vertex_count;++i) builder.MakeVertex(nodes[i],points[i],tol);
  std::map<Key,TopoDS_Edge> edges;
  auto insert=[&](int a,int b,const TopoDS_Edge& e) {
    edges.emplace(key(a,b),a<b ? e : TopoDS::Edge(e.Reversed()));
  };
  for (const auto& arc : arcs(f)) {
    BRepBuilderAPI_MakeEdge edge(new Geom_Circle(circle(f,arc)),nodes[arc.a],nodes[arc.b],arc.first,arc.last);
    require(edge.IsDone(),invalid_model,"Arc de butée invalide.");
    insert(arc.a,arc.b,edge.Edge());
  }
  BRepBuilderAPI_MakeEdge seam(quartic(f),nodes[joint_floor],nodes[joint_side],0.,1.);
  require(seam.IsDone(),invalid_model,"Extrémités de jonction invalides.");
  insert(joint_floor,joint_side,seam.Edge());
  auto wire=[&](const Loop& ids) {
    BRepBuilderAPI_MakeWire make;
    for (size_t j=0;j<ids.size();++j) {
      const int a=ids[j],b=ids[(j+1)%ids.size()];
      auto it=edges.find(key(a,b));
      if (it==edges.end()) {
        BRepBuilderAPI_MakeEdge line(nodes[a],nodes[b]);
        require(line.IsDone(),invalid_model,"Génératrice ou bord de poche invalide.");
        insert(a,b,line.Edge()); it=edges.find(key(a,b));
      }
      make.Add(a<b ? it->second : TopoDS::Edge(it->second.Reversed()));
    }
    require(make.IsDone() && BRep_Tool::IsClosed(make.Wire()),invalid_model,"Contour de poche ouvert.");
    return make.Wire();
  };
  BRepBuilderAPI_Sewing sewing(tol);
  for (int i=0;i<13;++i) {
    const auto outline=wire(outlines[i]);
    TopoDS_Face face;
    if (i<11) {
      const gp_Pln plane(points[outlines[i].front()],f.axes[plane_axes[i]]);
      BRepBuilderAPI_MakeFace make(plane,outline,true);
      if (i==5) {
        // Both outer and inner wires are normalized in the same plane chart.
        BRepBuilderAPI_MakeFace hole(plane,wire(opening),true);
        make.Add(TopoDS::Wire(BRepTools::OuterWire(hole.Face()).Reversed()));
      }
      require(make.IsDone(),invalid_model,"Plan de poche invalide.");
      face=make.Face();
    } else {
      const int wall=i-11;
      const auto frame=f.cylinder(wall);
      Handle(Geom_CylindricalSurface) support=new Geom_CylindricalSurface(frame,data.radii[wall]);
      const double sign=frame.Direction().Dot(f.axes[wall]);
      for (const auto& arc : arcs(f)) if (arc.wall==wall) {
        const auto& edge=edges.at(key(arc.a,arc.b));
        builder.UpdateEdge(edge,new Geom2d_Line(gp_Pnt2d(0,sign*arc.axial),gp_Dir2d(1,0)),support,TopLoc_Location(),0.);
        builder.Range(edge,support,TopLoc_Location(),arc.first,arc.last);
      }
      const std::array<Key,2> rails=wall ? std::array<Key,2>{key(joint_floor,large_floor),key(large_upper,large_side)}
                                          : std::array<Key,2>{key(joint_floor,small_floor),key(joint_side,small_side)};
      for (int k=0;k<2;++k) {
        const auto& edge=edges.at(rails[k]);
        const BRepAdaptor_Curve line(edge);
        const double first=line.FirstParameter(),last=line.LastParameter();
        const double v=gp_Vec(frame.Location(),line.Value(first)).Dot(gp_Vec(frame.Direction()));
        const double rate=std::copysign(1.,line.Line().Direction().Dot(frame.Direction()));
        builder.UpdateEdge(edge,new Geom2d_Line(gp_Pnt2d(k*pi/2,v-rate*first),gp_Dir2d(0,rate)),support,TopLoc_Location(),0.);
        builder.Range(edge,support,TopLoc_Location(),first,last);
      }
      builder.UpdateEdge(seam.Edge(),trace(f,wall,invalid_model),support,TopLoc_Location(),0.);
      builder.Range(seam.Edge(),support,TopLoc_Location(),0.,1.);
      BRepBuilderAPI_MakeFace make(support,outline,true);
      require(make.IsDone(),invalid_model,"Paroi de poche invalide.");
      face=make.Face();
    }
    sewing.Add(face);
    if (labels) labels->emplace_back(face,i<11 ? planes[i] : walls[i-11]);
  }
  auto solid=solid_from_sewing(sewing);
  if (labels) track_sewn_faces(solid,sewing,*labels);
  return solid;
}

struct Inspection { Data data; TopoDS_Edge seam; };
gp_Dir plane_normal(const TopoDS_Face& face) {
  auto n=BRepAdaptor_Surface(face).Plane().Axis().Direction();
  return face.Orientation()==TopAbs_REVERSED ? n.Reversed() : n;
}
// The adaptor can inspect these counts without copying or evaluating the
// source poles. BRepAdaptor_Curve::BSpline() may allocate a transformed copy.
bool is_quartic(const BRepAdaptor_Curve& curve) {
  return curve.GetType()==GeomAbs_BSplineCurve && curve.Degree()==4 && curve.NbPoles()==5 &&
         curve.NbKnots()==2 && curve.IsRational() && !curve.IsPeriodic();
}
// Both 2D splines must still represent the certified Hermite interpolant.
// The STEP reader may rotate the angular origin, reverse an axis or reverse
// the edge. Express every expected control in that chart before comparing.
void verify_seam(const Frame& frame,int wall,const TopoDS_Face& face,
                 const TopoDS_Edge& edge,bool reversed,Status status) {
  const BRepAdaptor_Curve curve(edge);
  const double lo=curve.FirstParameter(),hi=curve.LastParameter();
  require(is_quartic(curve) && BRep_Tool::Tolerance(edge)<=tol &&
          BRep_Tool::SameRange(edge) && BRep_Tool::SameParameter(edge),
          status,"Arête commune de la poche invalide.");
  const auto actual=curve.BSpline();
  require(actual->Multiplicity(1)==5 && actual->Multiplicity(2)==5 &&
          std::abs(lo)<1.e-12 && std::abs(hi-1.)<1.e-12 &&
          std::abs(lo-actual->FirstParameter())<1.e-12 && std::abs(hi-actual->LastParameter())<1.e-12,
          status,"La jonction doit être une quartique rationnelle entière.");
  const auto correct=quartic(frame);
  if (reversed) correct->Reverse();
  for (int j=1;j<=5;++j) {
    // Only ratios of homogeneous weights matter. Scale the error by the
    // radius so the same threshold applies in millimetres.
    require(actual->Pole(j).Distance(correct->Pole(j))<tol &&
            std::abs(actual->Weight(j)/actual->Weight(1)-correct->Weight(j)/correct->Weight(1))
              *std::max(1.,frame.R)<1.e-8,status,"Pôles ou poids de la quartique incorrects.");
  }
  double low,high;
  const auto pc=Handle(Geom2d_BSplineCurve)::DownCast(BRep_Tool::CurveOnSurface(edge,face,low,high));
  const int n=trace_spans(frame.R,status);
  require(!pc.IsNull() && pc->Degree()==3 && !pc->IsRational() && !pc->IsPeriodic() &&
          pc->NbKnots()==n+1 && pc->NbPoles()==3*n+1 &&
          std::abs(lo-low)<1.e-12 && std::abs(hi-high)<1.e-12 && BRep_Tool::Tolerance(face)<=tol,
          status,"Tracé paramétrique de poche absent ou hors budget.");
  const auto chart=trace(frame,wall,status);
  if (reversed) chart->Reverse();
  const auto source=frame.cylinder(wall), target=BRepAdaptor_Surface(face).Cylinder().Position();
  double u0=std::atan2(source.XDirection().Dot(target.YDirection()),source.XDirection().Dot(target.XDirection()));
  const auto tangent=gp_Vec(target.YDirection())*std::cos(u0)-gp_Vec(target.XDirection())*std::sin(u0);
  const double su=std::copysign(1.,tangent.Dot(gp_Vec(source.YDirection())));
  const double sv=std::copysign(1.,source.Direction().Dot(target.Direction()));
  const double v0=gp_Vec(target.Location(),source.Location()).Dot(gp_Vec(target.Direction()));
  u0+=2*pi*std::round((pc->Pole(1).X()-u0-su*chart->Pole(1).X())/(2*pi));
  for(int j=1;j<=pc->NbKnots();++j) {
    require(pc->Multiplicity(j)==chart->Multiplicity(j) &&
            std::abs(pc->Knot(j)-chart->Knot(j))*std::max(1.,frame.R)<1.e-12,
            status,"Nœuds de la trace de jonction invalides.");
  }
  for(int j=1;j<=pc->NbPoles();++j) {
    const auto src=chart->Pole(j), dst=pc->Pole(j);
    require(std::hypot((dst.X()-u0-su*src.X())*frame.d.radii[wall],dst.Y()-v0-sv*src.Y())<1.e-9,
            status,"Pôles de la trace de jonction invalides.");
  }
  BRepLib_CheckCurveOnSurface check_uv(edge,face);
  check_uv.Perform();
  require(check_uv.IsDone() && check_uv.MaxDistance()<1.e-8,status,"Tracé de la quartique hors du cylindre.");
}

Inspection verify(const TopoDS_Shape& solid,const Data* expected,Status status) {
  const auto check=[&](bool ok,const char* detail) {require(ok,status,detail);};
  std::array<TopoDS_Face,2> walls;
  std::vector<TopoDS_Face> planes;
  size_t nw=0;
  for (TopExp_Explorer it(solid,TopAbs_FACE);it.More();it.Next()) {
    const auto face=TopoDS::Face(it.Current());
    const auto type=BRepAdaptor_Surface(face).GetType();
    if (type==GeomAbs_Cylinder && nw<2) walls[nw++]=face;
    else if (type==GeomAbs_Plane && planes.size()<11) planes.push_back(face);
    else check(false,"Familles de la poche à deux rayons invalides.");
  }
  check(nw==2 && planes.size()==11,"Faces de la poche à deux rayons manquantes.");
  const auto radius=[](const TopoDS_Face& face){return BRepAdaptor_Surface(face).Cylinder().Radius();};
  if (radius(walls[0])>radius(walls[1])) std::swap(walls[0],walls[1]);
  std::array<gp_Dir,3> axes;
  Data inferred{};
  TopoDS_Edge seam;
  for (int w=0;w<2;++w) {
    const auto& face=walls[w];
    const BRepAdaptor_Surface s(face);
    inferred.radii[w]=s.Cylinder().Radius();
    check(std::abs(s.LastUParameter()-s.FirstUParameter()-pi/2)<1.e-8,"Ouverture du cylindre de poche invalide.");
    TopoDS_Edge quarter,junction;
    for (TopExp_Explorer it(face,TopAbs_EDGE);it.More();it.Next()) {
      const auto edge=TopoDS::Edge(it.Current());
      const BRepAdaptor_Curve c(edge);
      if (c.GetType()==GeomAbs_BSplineCurve) {
        check(junction.IsNull() && is_quartic(c),"Jonction quartique absente ou multiple."); junction=edge;
      } else if (c.GetType()==GeomAbs_Circle && std::abs(c.LastParameter()-c.FirstParameter()-pi/2)<1.e-8) {
        // A very short difference to pi/2 can occur for extreme radius ratios.
        // Both candidate arcs are checked in full against the domain below.
        if (quarter.IsNull() || BRepAdaptor_Curve(quarter).LastParameter()-BRepAdaptor_Curve(quarter).FirstParameter()
                                  < c.LastParameter()-c.FirstParameter()) quarter=edge;
      }
    }
    check(!quarter.IsNull() && !junction.IsNull() && (!w || seam.IsSame(junction)),"Butée ou jonction partagée manquante.");
    seam=junction;
    const BRepAdaptor_Curve c(seam);
    const gp_Pnt middle=c.Value((c.FirstParameter()+c.LastParameter())/2);
    axes[w]=s.Cylinder().Axis().Direction();
    if (gp_Vec(middle,BRepAdaptor_Curve(quarter).Circle().Location()).Dot(gp_Vec(axes[w]))<0) axes[w].Reverse();
  }
  check(std::abs(axes[0].Dot(axes[1]))<1.e-8,"Axes des deux butées non orthogonaux.");
  axes[2]=axes[0].Crossed(axes[1]);
  const gp_Pnt ref=BRepAdaptor_Surface(planes.front()).Plane().Location();
  using Supports=std::array<std::vector<std::pair<double,TopoDS_Face>>,3>;
  Supports supports;
  for (auto face : planes) {
    const auto plane=BRepAdaptor_Surface(face).Plane();
    const auto axis=std::find_if(axes.begin(),axes.end(),[&](const gp_Dir& v){
      return plane.Axis().Direction().IsParallel(v,1.e-8);
    });
    check(axis!=axes.end(),"Plan de poche oblique.");
    supports[axis-axes.begin()].emplace_back(gp_Vec(ref,plane.Location()).Dot(gp_Vec(*axis)),face);
  }
  for (int i=0;i<3;++i) {
    auto& group=supports[i];
    check(group.size()==size_t(i==2 ? 3 : 4),"Supports plans de poche incomplets.");
    std::sort(group.begin(),group.end(),[](const auto& a,const auto& b){return a.first<b.first;});
  }
  if (plane_normal(supports[2][1].second).Dot(axes[2])<0) {
    axes[2].Reverse();
    for (auto& row:supports[2]) row.first=-row.first;
    std::reverse(supports[2].begin(),supports[2].end());
  }
  gp_Pnt origin=ref;
  for (int i=0;i<3;++i) {
    const auto& group=supports[i];
    const double zero=group[1].first;
    origin.Translate(gp_Vec(axes[i])*zero);
    inferred.outer_min[i]=group.front().first-zero;
    inferred.outer_max[i]=group.back().first-zero;
    if (i<2) inferred.pocket_spans[i]=group[2].first-zero;
    for (int j=0;j<3;++j) inferred.axes[i][j]=axes[i].Coord(j+1);
  }
  for (int i=0;i<3;++i) inferred.origin[i]=origin.Coord(i+1);
  const Frame frame(expected ? *expected : inferred,status);
  const auto points=vertices(frame);
  const std::array<TopoDS_Face,13> faces={
    supports[0][0].second,supports[0][3].second,supports[1][0].second,supports[1][3].second,
    supports[2][0].second,supports[2][2].second,supports[0][1].second,supports[0][2].second,
    supports[1][1].second,supports[1][2].second,supports[2][1].second,walls[0],walls[1]};
  const auto vertex_id=[&](const gp_Pnt& p) {
    int id=-1;
    for (int i=0;i<vertex_count;++i) if (points[i].Distance(p)<tol) {
      check(id==-1,"Sommets analytiques de poche ambigus."); id=i;
    }
    check(id!=-1,"Sommet hors du domaine de poche."); return id;
  };
  const auto circles=arcs(frame);
  std::map<Key,TopoDS_Edge> shared;
  for (int i=0;i<13;++i) {
    const BRepAdaptor_Surface surface(faces[i]);
    if (i<11) {
      const auto n=plane_normal(faces[i]);
      check(n.Dot(frame.axes[plane_axes[i]])*normal_sign[i]>1.-1.e-12 &&
            surface.Plane().Distance(points[outlines[i][0]])<tol,"Orientation ou position du plan de poche incorrecte.");
    } else {
      const int w=i-11;
      const auto actual=surface.Cylinder();
      const auto target=frame.cylinder(w);
      check(std::abs(actual.Radius()-frame.d.radii[w])<tol &&
            actual.Axis().Direction().IsParallel(target.Direction(),1.e-8) &&
            gp_Lin(actual.Axis()).Distance(target.Location())<tol,"Cylindre de poche déplacé.");
      const double u=(surface.FirstUParameter()+surface.LastUParameter())/2;
      const double v=(surface.FirstVParameter()+surface.LastVParameter())/2;
      gp_Pnt p; gp_Vec du,dv; surface.D1(u,v,p,du,dv);
      auto normal=gp_Dir(du.Crossed(dv));
      if (faces[i].Orientation()==TopAbs_REVERSED) normal.Reverse();
      check(gp_Vec(actual.Location(),p).Dot(gp_Vec(normal))<0.,"Paroi de poche convexe.");
    }
    std::set<Key> remaining;
    const auto expect_loop=[&](const Loop& loop) {
      for(size_t k=0;k<loop.size();++k) remaining.insert(key(loop[k],loop[(k+1)%loop.size()]));
    };
    expect_loop(outlines[i]);
    if (i==5) expect_loop(opening);
    int loops=0;
    for (TopExp_Explorer it(faces[i],TopAbs_WIRE);it.More();it.Next()) ++loops;
    check(loops==(i==5 ? 2 : 1),"Contours auxiliaires de poche inattendus.");
    for (TopExp_Explorer it(faces[i],TopAbs_EDGE);it.More();it.Next()) {
      const auto edge=TopoDS::Edge(it.Current());
      check(!BRep_Tool::Degenerated(edge),"Arête de poche dégénérée.");
      const BRepAdaptor_Curve curve(edge);
      const double lo=curve.FirstParameter(),hi=curve.LastParameter();
      const int a=vertex_id(curve.Value(lo)), b=vertex_id(curve.Value(hi));
      const Key id=key(a,b);
      check(remaining.erase(id)==1,"Contour de face de poche incorrect.");
      auto inserted=shared.emplace(id,edge);
      check(inserted.second || inserted.first->second.IsSame(edge),"Arêtes de poche non partagées.");
      if (id==key(joint_floor,joint_side)) {
        check(edge.IsSame(seam) && i>=11,"La jonction n’est pas commune aux deux parois.");
        verify_seam(frame,i-11,faces[i],edge,a!=joint_floor,status);
      } else {
        const auto arc=std::find_if(circles.begin(),circles.end(),[&](const Arc& x){return key(x.a,x.b)==id;});
        if(arc==circles.end()) check(curve.GetType()==GeomAbs_Line,"Bord de poche non rectiligne.");
        else {
          check(curve.GetType()==GeomAbs_Circle,"Butée de poche non circulaire.");
          const auto c=circle(frame,*arc);
          const double mid=(arc->first+arc->last)/2;
          const auto midpt=c.Location().Translated((gp_Vec(c.XAxis().Direction())*std::cos(mid)+
                            gp_Vec(c.YAxis().Direction())*std::sin(mid))*c.Radius());
          check(c.Location().Distance(curve.Circle().Location())<tol && std::abs(c.Radius()-curve.Circle().Radius())<tol &&
                curve.Value((lo+hi)/2).Distance(midpt)<tol && std::abs(hi-lo-arc->last+arc->first)*c.Radius()<tol,
                "Ouverture ou position de la butée circulaire incorrecte.");
        }
      }
    }
    check(remaining.empty(),"Arêtes manquantes sur la poche.");
  }
  return {inferred,seam};
}

void volume(const Data& d,double actual,Status status) {
  const Frame f(d,status);
  // At each height both fillets retain a strip. Their common rectangle is
  // counted twice. Positive decreasing inset products give rigorous upper
  // and lower Riemann sums for that overlap, including at the floor cusp.
  constexpr int slices=4096;
  const double dh=f.r/slices;
  double interior=0.;
  for(int i=1;i<slices;++i) interior+=f.inset(f.R,i*dh)*f.inset(f.r,i*dh);
  const double L=d.pocket_spans[0],W=d.pocket_spans[1],*lo=d.outer_min,*hi=d.outer_max;
  const double blank=(hi[0]-lo[0])*(hi[1]-lo[1])*(hi[2]-lo[2]);
  const double high=blank-L*W*hi[2]+(1-pi/4)*(L*f.r*f.r+W*f.R*f.R)-dh*interior;
  const double low=high-dh*f.r*f.R, rounding=blank*1.e-10;
  require(std::isfinite(high) && actual>=low-rounding && actual<=high+rounding,status,
          "Le volume de la poche sort de l’intervalle des deux congés.");
}
double mesh_segments(const Data& d,double deflection,double angle) {
  // On 0 <= u <= 1, t=end*u. Differentiating the three rational coordinates
  // twice bounds their t-second derivatives by (203.75,203.75,191)*R.
  // end<=1/2 gives |C''(u)|<90*R. On the small wall |C'| >= r*end*4/D
  // with end>0.4 and D<2.1, so r/2 is a lower bound on speed.
  const double bound=90*d.radii[1];
  return std::ceil(std::max(std::sqrt(bound/(8*deflection)),bound/(0.5*d.radii[0]*angle)));
}
} // namespace unequal_pocket
