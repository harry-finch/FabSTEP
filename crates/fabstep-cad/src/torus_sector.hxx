// A solid sector of an annulus with a rounded upper outer edge. The five
// meridian corners and their two radial endpoints define the entire boundary.
namespace torus_sector {
constexpr double pi = 3.14159265358979323846;
constexpr double precision = 5e-8;
using Data = TorusSectorSolidData;

void require(bool condition, Status status) {
  if (!condition)
    throw CadFailure{status, "Le secteur torique ne correspond pas aux limites et aux supports attendus."};
}

struct Geometry {
  Data d;
  gp_Pnt origin;
  gp_Dir z, x, y;
  double outer, shoulder;
  explicit Geometry(const Data& data)
    : d(data), origin(point_at(d.origin, 0)), z(direction(d.axis)), x(direction(d.start)),
      y(z.Crossed(x)), outer(d.major_radius + d.minor_radius), shoulder(d.height - d.minor_radius) {
    const double scale = std::max({1., d.height, outer});
    const gp_Vec zv(d.axis[0],d.axis[1],d.axis[2]), xv(d.start[0],d.start[1],d.start[2]);
    require(std::isfinite(scale) && std::abs(zv.Magnitude()-1.)*scale < precision &&
            std::abs(xv.Magnitude()-1.)*scale < precision && std::abs(z.Dot(x))*scale < precision &&
            d.sweep > 0 && d.sweep < pi && d.inner_radius > precision &&
            d.major_radius > d.inner_radius + precision && d.minor_radius > precision &&
            d.major_radius > d.minor_radius + precision && shoulder > precision, invalid_model);
    x = y.Crossed(z);
  }
  gp_Dir radial(double a) const {
    return gp_Dir(gp_Vec(x)*std::cos(a) + gp_Vec(y)*std::sin(a));
  }
  gp_Pnt point(double r, double a, double h) const {
    return origin.Translated(gp_Vec(radial(a))*r + gp_Vec(z)*h);
  }
  // Order around the cross section starting at the bottom inner corner.
  std::array<double,5> radii() const {return {d.inner_radius,outer,outer,d.major_radius,d.inner_radius};}
  std::array<double,5> heights() const {return {0,0,shoulder,d.height,d.height};}
  double volume() const {
    const double R=d.major_radius, r=d.minor_radius, i=d.inner_radius;
    return d.sweep/2 * (shoulder*(outer-i)*(outer+i) + r*(R-i)*(R+i) + R*pi*r*r/2 + 2*r*r*r/3);
  }
};

bool partial_torus(const BRepAdaptor_Surface& s) {
  return s.GetType()==GeomAbs_Torus && s.LastUParameter()-s.FirstUParameter()<2*pi-1e-9;
}

// Infer an expected frame when loading a standalone STEP for the document
// writer. The center of the top circular edge gives the upward direction;
// the centers of the two meridians specify the oriented azimuth interval.
Data infer(const TopoDS_Shape& solid, Status status) {
  TopoDS_Face wall;
  std::vector<gp_Cylinder> cylinders;
  std::vector<gp_Pln> planes;
  for(TopExp_Explorer it(solid,TopAbs_FACE);it.More();it.Next()) {
    auto f=TopoDS::Face(it.Current());
    const BRepAdaptor_Surface s(f);
    if (s.GetType()==GeomAbs_Torus && wall.IsNull()) wall=f;
    else if (s.GetType()==GeomAbs_Cylinder && cylinders.size()<2) cylinders.push_back(s.Cylinder());
    else if (s.GetType()==GeomAbs_Plane && planes.size()<4) planes.push_back(s.Plane());
    else require(false,status);
  }
  require(!wall.IsNull() && planes.size()==4 && cylinders.size()==2,status);
  const auto torus=BRepAdaptor_Surface(wall).Torus();
  std::vector<gp_Pnt> ends;
  std::optional<gp_Pnt> top;
  for(TopExp_Explorer it(wall, TopAbs_EDGE);it.More();it.Next()) {
    const BRepAdaptor_Curve c(TopoDS::Edge(it.Current()));
    require(c.GetType()==GeomAbs_Circle,status);
    const auto circle=c.Circle();
    if(std::abs(circle.Radius()-torus.MinorRadius())<precision) {
      require(ends.size()<2,status);
      ends.push_back(circle.Location());
    } else if(std::abs(circle.Radius()-torus.MajorRadius())<precision) {
      require(!top.has_value(),status);
      top=circle.Location();
    }
  }
  require(ends.size()==2 && top.has_value(),status);
  auto z=torus.Axis().Direction();
  if (gp_Vec(torus.Location(),*top).Dot(gp_Vec(z))<0.) z.Reverse();
  const double height=std::accumulate(planes.begin(),planes.end(),0.,[&](double h,const gp_Pln& p) {
    return p.Axis().Direction().IsParallel(z,1e-7) ? std::max(h,p.Distance(*top)) : h;
  });
  gp_Dir x(gp_Vec(torus.Location(),ends[0])), end(gp_Vec(torus.Location(),ends[1]));
  const double sweep=x.AngleWithRef(end,z);
  if(sweep<0.) std::swap(x,end);
  const gp_Pnt base=top->Translated(-gp_Vec(z)*height);
  return {{base.X(),base.Y(),base.Z()},{z.X(),z.Y(),z.Z()},{x.X(),x.Y(),x.Z()},
          height,std::min(cylinders[0].Radius(),cylinders[1].Radius()),torus.MajorRadius(),torus.MinorRadius(),std::abs(sweep)};
}

struct Border {
  gp_Pnt a, mid, b, center;
  double radius, span;
  gp_Dir normal;
};
std::array<Border,15> borders(const Geometry& g) {
  std::array<Border,15> result{};
  const auto radii=g.radii(), h=g.heights();
  for(int i=0;i<5;++i) {
    const int j=(i+1)%5;
    for(int end=0;end<2;++end) {
      const double u=end*g.d.sweep;
      auto& edge=result[5*end+i];
      edge.a=g.point(radii[i],u,h[i]); edge.b=g.point(radii[j],u,h[j]);
      edge.mid=gp_Pnt((edge.a.XYZ()+edge.b.XYZ())/2.);
      if(i==2) {
        const double sub=g.d.minor_radius/std::sqrt(2.);
        edge.mid=g.point(g.d.major_radius+sub,u,g.shoulder+sub);
        edge.center=g.point(g.d.major_radius,u,g.shoulder);
        edge.normal=g.radial(u).Crossed(g.z);
        edge.radius=g.d.minor_radius; edge.span=pi/2;
      }
    }
    result[10+i]={g.point(radii[i],0,h[i]),g.point(radii[i],g.d.sweep/2,h[i]),
                  g.point(radii[i],g.d.sweep,h[i]),g.point(0,0,h[i]),radii[i],g.d.sweep,g.z};
  }
  return result;
}

int match_border(const BRepAdaptor_Curve& c,const std::array<Border,15>& expected,Status status) {
  const double first=c.FirstParameter(), last=c.LastParameter();
  const auto a=c.Value(first), b=c.Value(last), m=c.Value((first+last)/2);
  int match=-1;
  for(int i=0;i<15;++i) {
    const auto& e=expected[i];
    if (!((a.Distance(e.a)<precision && b.Distance(e.b)<precision) ||
          (a.Distance(e.b)<precision && b.Distance(e.a)<precision)) || m.Distance(e.mid)>=precision) continue;
    if (e.radius) {
      if(c.GetType()!=GeomAbs_Circle) continue;
      const auto circle=c.Circle();
      if(circle.Location().Distance(e.center)>=precision || std::abs(circle.Radius()-e.radius)>=precision ||
         !circle.Axis().Direction().IsParallel(e.normal,precision/std::max(1.,e.radius)) ||
         std::abs(last-first-e.span)*std::max(1.,e.radius)>=precision) continue;
    } else if(c.GetType()!=GeomAbs_Line) continue;
    require(match==-1,status);
    match=i;
  }
  require(match>=0,status);
  return match;
}

// Match pcurves as well as the physical edges. Planar circles stay circles;
// on cylindrical/toroidal charts each of these boundaries is an isoparameter.
void check_trim(const TopoDS_Edge& edge,const TopoDS_Face& face,const Border& expected,Status status) {
  const BRepAdaptor_Surface surface(face);
  const BRepAdaptor_Curve curve(edge);
  BRepAdaptor_Curve2d trim(edge,face);
  const bool plane=surface.GetType()==GeomAbs_Plane;
  require(plane || surface.GetType()==GeomAbs_Cylinder || surface.GetType()==GeomAbs_Torus,status);
  require(trim.GetType()==(plane && expected.radius ? GeomAbs_Circle : GeomAbs_Line),status);
  if(!plane) {
    const auto delta=trim.Value(trim.LastParameter()).XY()-trim.Value(trim.FirstParameter()).XY();
    const double R=surface.GetType()==GeomAbs_Torus ? surface.Torus().MajorRadius()+surface.Torus().MinorRadius() :
                                                      surface.Cylinder().Radius();
    const bool vertical=(surface.GetType()==GeomAbs_Torus &&
                std::abs(expected.radius-surface.Torus().MinorRadius())<precision) || expected.radius==0.;
    const double u=vertical ? 0. : expected.span;
    const double v=vertical ? (expected.radius ? expected.span : expected.a.Distance(expected.b)) : 0.;
    const double v_scale=surface.GetType()==GeomAbs_Torus ? surface.Torus().MinorRadius() : 1.;
    require(std::abs(std::abs(delta.X())-u)*R<precision &&
            std::abs(std::abs(delta.Y())-v)*v_scale<precision,status);
  } else if(expected.radius) {
    const auto circle=trim.Circle();
    const auto center=circle.Location();
    require(surface.Value(center.X(),center.Y()).Distance(expected.center)<precision &&
            std::abs(circle.Radius()-expected.radius)<precision,status);
    require(std::abs(trim.LastParameter()-trim.FirstParameter()-expected.span)*expected.radius<precision,status);
  }
  for(double f : {0.,.25,.5,.75,1.}) {
    auto uv=trim.Value(trim.FirstParameter()+(trim.LastParameter()-trim.FirstParameter())*f);
    require(surface.Value(uv.X(),uv.Y()).Distance(curve.Value(curve.FirstParameter()+
             (curve.LastParameter()-curve.FirstParameter())*f))<precision,status);
  }
}

struct Proof {Data domain; std::array<TopoDS_Face,7> faces;};
Proof verify(const TopoDS_Shape& shape,const Data* expected,Status status) {
  Proof proof{infer(shape,status), {}};
  const Geometry actual(proof.domain);
  const Geometry g(expected ? *expected : proof.domain);
  if (expected) {
    // Independently recover the physical frame on both STEP transfers. A
    // direction tolerance alone would permit excessive tip displacement on
    // a tall solid or a large radius. Azimuth displacement scales with the
    // radius, independent of the height of the solid.
    const double scale=std::max({1.,g.outer,g.d.height});
    require(actual.origin.Distance(g.origin)<precision &&
            gp_Vec(actual.z).Subtracted(gp_Vec(g.z)).Magnitude()*scale<precision &&
            gp_Vec(actual.x).Subtracted(gp_Vec(g.x)).Magnitude()*g.outer<precision &&
            std::abs(actual.d.sweep-g.d.sweep)*g.outer<precision &&
            std::abs(actual.d.height-g.d.height)<precision &&
            std::abs(actual.d.inner_radius-g.d.inner_radius)<precision &&
            std::abs(actual.d.major_radius-g.d.major_radius)<precision &&
            std::abs(actual.d.minor_radius-g.d.minor_radius)<precision,status);
  }
  const auto edges=borders(g);
  std::array<TopoDS_Edge,15> first_use{};
  std::array<unsigned,15> uses{};
  const std::array<std::set<int>,7> layout{{
    {0,5,10,11}, {3,8,13,14}, {0,1,2,3,4}, {5,6,7,8,9},
    {4,9,14,10}, {1,6,11,12}, {2,7,12,13}
  }};
  const double R=g.d.major_radius, r=g.d.minor_radius;
  const double location_scale=std::max({1.,g.outer,g.d.height});
  for(TopExp_Explorer it(shape,TopAbs_FACE);it.More();it.Next()) {
    auto f=TopoDS::Face(it.Current());
    TopExp_Explorer wires(f,TopAbs_WIRE);
    require(wires.More(),status);
    wires.Next(); require(!wires.More(),status);
    std::set<int> boundary;
    for(TopExp_Explorer edge(f,TopAbs_EDGE);edge.More();edge.Next()) {
      const auto e=TopoDS::Edge(edge.Current());
      const int id=match_border(BRepAdaptor_Curve(e),edges,status);
      require(boundary.insert(id).second && uses[id]<2,status);
      require(e.Orientation()==TopAbs_FORWARD || e.Orientation()==TopAbs_REVERSED,status);
      if(uses[id]++ == 0) first_use[id]=e;
      else require(e.IsSame(first_use[id]) && e.Orientation()!=first_use[id].Orientation(),status);
      check_trim(e,f,edges[id],status);
    }
    auto pos=std::find(layout.begin(),layout.end(),boundary);
    require(pos!=layout.end(),status);
    const int id=static_cast<int>(pos-layout.begin());
    require(proof.faces[id].IsNull(),status);
    proof.faces[id]=f;
    const BRepAdaptor_Surface s(f);
    if(id<4) {
      require(s.GetType()==GeomAbs_Plane,status);
      const auto n=id<2 ? g.z : g.radial(id==2 ? 0:g.d.sweep).Crossed(g.z);
      require(s.Plane().Axis().Direction().IsParallel(n,precision/location_scale) &&
              s.Plane().Distance(g.point(0,0,id==1 ? g.d.height:0))<precision,status);
    } else if(id<6) {
      require(s.GetType()==GeomAbs_Cylinder,status);
      const auto c=s.Cylinder();
      require(std::abs(c.Radius()-(id==4 ? g.d.inner_radius:g.outer))<precision &&
              gp_Lin(c.Axis()).Distance(g.origin)<precision &&
              c.Axis().Direction().IsParallel(g.z,precision/location_scale),status);
    } else {
      require(s.GetType()==GeomAbs_Torus,status);
      const auto t=s.Torus();
      require(std::abs(t.MajorRadius()-R)<precision && std::abs(t.MinorRadius()-r)<precision &&
              t.Location().Distance(g.point(0,0,g.shoulder))<precision &&
              t.Axis().Direction().IsParallel(g.z,precision/location_scale),status);
    }
  }
  for(const auto& face:proof.faces) require(!face.IsNull(),status);
  for(unsigned count:uses) require(count==2,status);
  return proof;
}

TopoDS_Solid build(const Data& d,const size_t* tokens,FaceLabels* labels) {
  const Geometry g(d);
  const auto edges=borders(g);
  BRepBuilderAPI_MakeWire wire;
  for(int i=0;i<5;++i) {
    const auto& e=edges[i];
    if(i==2) {
      const gp_Circ c(gp_Ax2(e.center,e.normal,g.x),d.minor_radius);
      wire.Add(BRepBuilderAPI_MakeEdge(c,0,pi/2).Edge());
    } else wire.Add(BRepBuilderAPI_MakeEdge(e.a,e.b).Edge());
  }
  require(wire.IsDone(),invalid_model);
  BRepPrimAPI_MakeRevol sweep(BRepBuilderAPI_MakeFace(wire.Wire(),true).Face(),gp_Ax1(g.origin,g.z),d.sweep);
  const auto solid=TopoDS::Solid(single_solid(sweep.Shape(),invalid_model));
  if(labels) {
    const auto proof=verify(solid,&d,invalid_model);
    for(size_t i=0;i<proof.faces.size();++i) labels->emplace_back(proof.faces[i],tokens[i]);
  }
  return solid;
}
} // namespace torus_sector
