// Continuous local-edge certificates for the canonical
// orthogonal cylinders x*x+y*y=r*r and y*y+z*z=R*R. Bounds refer to the original native curves.
// All bounds refer to the mathematical curves defined by the reimported binary
// poles/weights/knots, including their stored placements. No OCCT curve fitting
// or floating Bezier conversion is used by the certificate.
#pragma once

#include <BRep_Tool.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_BezierCurve.hxx>
#include <Geom_CylindricalSurface.hxx>
#include <Geom_RectangularTrimmedSurface.hxx>
#include <Geom_TrimmedCurve.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <Geom2d_BezierCurve.hxx>
#include <Geom2d_Line.hxx>
#include <Geom2d_TrimmedCurve.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopLoc_Location.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <exception>
#include <limits>
#include <string>
#include <utility>
#include <vector>

namespace cross_drill_lab {
struct Budget {
  double tolerance_mm = 1e-5;
  double mesh_deflection_mm = 0.;
  std::size_t max_work = 10'000'000;
  std::size_t max_leaves = 16'384;
  int max_depth = 20;
};
struct Certificate {
  bool ok = false;
  double bound3d = std::numeric_limits<double>::infinity();
  double boundPcurves = std::numeric_limits<double>::infinity();
  std::size_t work = 0, leaves = 0;
  std::size_t attempts = 0;
  double mesh_segments = 0.;
  // Diagnostics only when ok=false; these are not whole-edge certificates.
  double last_bound3d = std::numeric_limits<double>::infinity();
  double last_boundPcurves = std::numeric_limits<double>::infinity();
  std::string reason;
  int sign = 0, phase_direction = 0;
  bool phase_monotone = false;
  // Unwrapped in the native increasing curve parameter, not edge orientation.
  double phase_first = 0., phase_last = 0.;
  double phase_first_lo = 0., phase_first_hi = 0.;
  double phase_last_lo = 0., phase_last_hi = 0.;
  // Global branch winding and across-edge closure belong to the driver.
  bool coverage_proven = false;
};

namespace certificate_detail {
struct Failure { std::string reason; };
struct Counter {
  std::size_t work = 0, maximum = 0;
  void spend(std::size_t n = 1) {
    if (n > maximum || work > maximum - n) throw Failure{"work_budget"};
    work += n;
  }
};
inline thread_local Counter* arithmetic_counter = nullptr;
inline void spend() { if (arithmetic_counter) arithmetic_counter->spend(); }
struct Scope {
  Counter* previous;
  explicit Scope(Counter& c) : previous(arithmetic_counter) { arithmetic_counter = &c; }
  ~Scope() { arithmetic_counter = previous; }
};
inline double down(double x) { return std::nextafter(x, -std::numeric_limits<double>::infinity()); }
inline double up(double x) { return std::nextafter(x, std::numeric_limits<double>::infinity()); }
struct I {
  double lo = 0., hi = 0.;
  I() = default;
  explicit I(double x) : lo(x), hi(x) {
    if (!std::isfinite(x)) throw Failure{"nonfinite_interval_input"};
  }
  I(double a, double b) : lo(a), hi(b) {
    if (!std::isfinite(a) || !std::isfinite(b) || a > b) throw Failure{"invalid_interval"};
  }
};
inline I operator+(I a, I b) { spend(); return I(down(a.lo + b.lo), up(a.hi + b.hi)); }
inline I operator-(I a, I b) { spend(); return I(down(a.lo - b.hi), up(a.hi - b.lo)); }
inline I operator-(I a) { return I(-a.hi, -a.lo); }
inline I operator*(I a, I b) {
  spend();
  const std::array<double, 4> p{a.lo*b.lo, a.lo*b.hi, a.hi*b.lo, a.hi*b.hi};
  return I(down(*std::min_element(p.begin(), p.end())), up(*std::max_element(p.begin(), p.end())));
}
inline I operator/(I a, I b) {
  spend();
  if (b.lo <= 0. && b.hi >= 0.) throw Failure{"interval_division_by_zero"};
  return a * I(down(1./b.hi), up(1./b.lo));
}
inline I square(I a) {
  spend();
  const double v = std::max(a.lo*a.lo, a.hi*a.hi);
  const double l = a.lo <= 0. && a.hi >= 0. ? 0. : std::min(a.lo*a.lo, a.hi*a.hi);
  return I(l == 0. ? 0. : down(l), up(v));
}
inline I root(I a) {
  spend();
  if (a.lo < 0.) throw Failure{"negative_sqrt_interval"};
  return I(a.lo == 0. ? 0. : down(std::sqrt(a.lo)), up(std::sqrt(a.hi)));
}
inline double magnitude(I a) { return std::max(std::abs(a.lo), std::abs(a.hi)); }
inline double middle(I a) { return a.lo*.5 + a.hi*.5; }
inline I pi() {
  constexpr double p = 3.141592653589793238462643383279502884;
  return I(down(p), up(p));
}
inline I ratio(std::size_t a, std::size_t b) { return I(static_cast<double>(a))/I(static_cast<double>(b)); }
using V = std::array<I, 3>;
inline I norm(V a) {
  const I sum=square(a[0])+square(a[1])+square(a[2]);
  // Directed addition can put zero a few subnormal ulps below zero. The
  // mathematical sum of squares is nonnegative independently of rounding.
  return root(I(std::max(0.,sum.lo),sum.hi));
}
inline V sub(V a, V b) { return {a[0]-b[0], a[1]-b[1], a[2]-b[2]}; }
inline V plus(V a, V b) { return {a[0]+b[0], a[1]+b[1], a[2]+b[2]}; }
inline V scale(V a, I k) { return {a[0]*k, a[1]*k, a[2]*k}; }

// Rigorous elementary trig in small intervals: range reduction uses an
// enclosing pi and Taylor remainder, not an assumed libm sin/cos error.
inline I sin_small(I x) {
  if (magnitude(x) > .9) throw Failure{"trig_reduction_unresolved"};
  I sum(0.), power = x, factorial(1.);
  for (int n = 0; n <= 8; ++n) {
    if (n > 0) {
      power = power*square(x);
      factorial = factorial*I(2.*n)*I(2.*n+1.);
    }
    sum = n%2 == 0 ? sum + power/factorial : sum - power/factorial;
  }
  I rem_power(1.), rem_fact(1.);
  for (int i = 1; i <= 19; ++i) { rem_power = rem_power*I(magnitude(x)); rem_fact = rem_fact*I(i); }
  const double e = (rem_power/rem_fact).hi;
  return sum + I(-e, e);
}
inline std::array<I, 2> trig(I x) {
  if (magnitude(x) > 256.) throw Failure{"trig_parameter_range"};
  const int quadrant = static_cast<int>(std::nearbyint(middle(x)/(middle(pi())*.5)));
  const I reduced = x - I(quadrant)*(pi()/I(2.));
  const I s = sin_small(reduced);
  // cos(x) = sin(pi/2-x) would leave the small interval. Its own series:
  I c(0.), power(1.), fact(1.);
  for (int n = 0; n <= 9; ++n) {
    if (n > 0) { power = power*square(reduced); fact = fact*I(2.*n-1.)*I(2.*n); }
    c = n%2 == 0 ? c + power/fact : c - power/fact;
  }
  I rp(1.), rf(1.);
  for (int i = 1; i <= 20; ++i) { rp = rp*I(magnitude(reduced)); rf = rf*I(i); }
  const double e = (rp/rf).hi;
  c = c + I(-e, e);
  switch ((quadrant%4+4)%4) {
    case 0: return {s,c};
    case 1: return {c,-s};
    case 2: return {-s,-c};
    default: return {-c,s};
  }
}
inline I atan_small(I x) {
  if (magnitude(x) > .46) throw Failure{"atan_reduction_unresolved"};
  I sum(0.), power=x;
  for (int n=0; n<20; ++n) {
    if (n) power=power*square(x);
    const I term=power/I(2.*n+1.);
    sum = n%2==0 ? sum+term : sum-term;
  }
  I p(1.);
  for (int i=0; i<41; ++i) p=p*I(magnitude(x));
  const double e=(p/I(41.)).hi;
  return sum+I(-e,e);
}
inline I atan_positive(I x) {
  if (x.lo < 0. || x.hi > 1.000000000001) throw Failure{"atan_ratio_range"};
  if (x.hi <= .45) return atan_small(x);
  return pi()/I(4.) + atan_small((x-I(1.))/(x+I(1.)));
}
inline I angle(V p) {
  const double x=middle(p[0]), y=middle(p[1]);
  if (x==0. && y==0.) throw Failure{"phase_on_axis"};
  const bool x_major=std::abs(x)>=std::abs(y);
  const I quotient=x_major?I(std::abs(y))/I(std::abs(x)):I(std::abs(x))/I(std::abs(y));
  // The exact ratio of the nonnegative, ordered point coordinates lies in
  // [0,1]. Intersect this known domain: outward rounding at a zero numerator
  // otherwise introduces a harmless negative subnormal lower endpoint.
  const I positive_ratio(std::max(0.,quotient.lo),std::min(1.,quotient.hi));
  I a = x_major ? atan_positive(positive_ratio) : pi()/I(2.)-atan_positive(positive_ratio);
  if (x<0.) a=pi()-a;
  if (y<0.) a=-a;
  const I rho=root(square(I(x))+square(I(y)));
  const I error=norm({p[0]-I(x),p[1]-I(y),I(0.)});
  if (error.hi >= rho.lo*.5) throw Failure{"phase_endpoint_uncertain"};
  const double e=(error/(rho-I(error.hi))).hi;
  return a+I(-e,e);
}

using Poly = std::vector<I>;
using Net = std::array<Poly,4>;
inline I hull(const Poly& p) {
  if (p.empty()) throw Failure{"empty_polynomial"};
  double low=p[0].lo, high=p[0].hi;
  for (I x:p) { low=std::min(low,x.lo); high=std::max(high,x.hi); }
  return I(low,high);
}
inline Poly add(const Poly& a,const Poly& b,bool minus=false) {
  if (a.size()!=b.size()) throw Failure{"polynomial_degree_mismatch"};
  Poly out(a.size());
  for (std::size_t i=0;i<a.size();++i) out[i]=minus?a[i]-b[i]:a[i]+b[i];
  return out;
}
inline double binomial(int n,int k) {
  double v=1.; for(int i=1;i<=k;++i) v=v*(n-i+1)/i; return v;
}
inline Poly product(const Poly& a,const Poly& b) {
  const int n=static_cast<int>(a.size())-1,m=static_cast<int>(b.size())-1;
  Poly out(n+m+1,I(0.));
  for(int i=0;i<=n;++i) for(int j=0;j<=m;++j) {
    // Degrees <=16: these integer binomial coefficients are exactly binary64.
    const I factor=(I(binomial(n,i))*I(binomial(m,j)))/I(binomial(n+m,i+j));
    out[i+j]=out[i+j]+a[i]*b[j]*factor;
  }
  return out;
}
inline Poly scaled(Poly p,I k) { for(I& x:p) x=x*k; return p; }
inline Poly derivative(const Poly& p) {
  if(p.size()==1) return {I(0.)};
  Poly out(p.size()-1);
  for(std::size_t i=0;i<out.size();++i) out[i]=I(out.size())*(p[i+1]-p[i]);
  return out;
}
inline std::array<Poly,2> split(const Poly& p) {
  Poly tmp=p,left(p.size()),right(p.size());
  left[0]=tmp[0]; right.back()=tmp.back();
  for(std::size_t row=1;row<p.size();++row) {
    for(std::size_t i=0;i<p.size()-row;++i) tmp[i]=(tmp[i]+tmp[i+1])*I(.5);
    left[row]=tmp[0]; right[p.size()-1-row]=tmp[p.size()-1-row];
  }
  return {left,right};
}
inline std::array<Net,2> split(const Net& n) {
  std::array<Net,2> out;
  for(int j=0;j<4;++j) { auto halves=split(n[j]);out[0][j]=std::move(halves[0]);out[1][j]=std::move(halves[1]); }
  return out;
}
inline V endpoint(const Net& n,bool last) {
  const std::size_t i=last?n[0].size()-1:0;
  return {n[0][i]/n[3][i],n[1][i]/n[3][i],n[2][i]/n[3][i]};
}
// Derivatives on the normalized Bernstein interval [0,1]. Differentiating
// N = W*C gives each rational derivative in terms of lower orders.
struct Jet {
  std::array<V,5> range;
  std::array<V,2> point, tangent;
};
inline Jet jet(const Net& n) {
  std::array<Net,5> derivatives;
  derivatives[0]=n;
  Jet out;
  std::array<I,5> weights;
  for(int order=0;order<=4;++order) {
    if(order)for(int c=0;c<4;++c)derivatives[order][c]=derivative(derivatives[order-1][c]);
    weights[order]=hull(derivatives[order][3]);
    if(weights[0].lo<=0.)throw Failure{"nonpositive_weight_hull"};
    for(int c=0;c<3;++c) {
      I value=hull(derivatives[order][c]);
      for(int k=1;k<=order;++k)
        value=value-I(binomial(order,k))*weights[k]*out.range[order-k][c];
      out.range[order][c]=value/weights[0];
    }
  }
  for(int end=0;end<2;++end) {
    const auto i=end?n[0].size()-1:0, j=end?derivatives[1][0].size()-1:0;
    out.point[end]=endpoint(n,end);
    for(int c=0;c<3;++c)out.tangent[end][c]=(derivatives[1][c][j]
      -derivatives[1][3][j]*out.point[end][c])/n[3][i];
  }
  return out;
}

struct RawSpline {
  int degree=0;
  std::vector<double> knots;
  std::vector<std::array<I,4>> poles;
  // Only exterior periodic knots need interval-valued translation by a period.
  std::vector<I> knot_bounds;
  I knot(int i) const {return knot_bounds.empty()?I(knots[i]):knot_bounds[i];}
};
inline void validate(RawSpline& s) {
  if(s.degree<1 || s.degree>8 || s.poles.size()<2 || s.poles.size()>2048 || s.knots.size()!=s.poles.size()+s.degree+1 || s.knots.size()>4096)
    throw Failure{"unsupported_spline_size_or_degree"};
  double max_weight=0.,min_weight=std::numeric_limits<double>::infinity();
  for(const auto& p:s.poles) { max_weight=std::max(max_weight,p[3].hi);min_weight=std::min(min_weight,p[3].lo); }
  if(min_weight<=0. || min_weight/max_weight<std::ldexp(1.,-20)) throw Failure{"unsupported_weight_condition"};
  for(std::size_t i=0;i<s.knots.size();) {
    std::size_t j=i+1;
    while(j<s.knots.size() && s.knots[j]==s.knots[i]) ++j;
    if(!std::isfinite(s.knots[i]) || (j<s.knots.size() && s.knots[j]<s.knots[i])) throw Failure{"invalid_knots"};
    if(i>0 && j<s.knots.size() && j-i>static_cast<std::size_t>(s.degree)) throw Failure{"discontinuous_spline"};
    i=j;
  }
  // Scaling homogeneous data does not change the mathematical curve.
  const I factor(max_weight);
  for(auto& p:s.poles) for(I& x:p) x=x/factor;
}
inline std::array<I,4> homogeneous(gp_Pnt p,double weight,const gp_Trsf& tr) {
  const I w(weight);
  const V q{I(p.X())*w,I(p.Y())*w,I(p.Z())*w};
  std::array<I,4> out;
  for(int i=0;i<3;++i) {
    out[i]=I(tr.Value(i+1,4))*w;
    for(int j=0;j<3;++j) out[i]=out[i]+I(tr.Value(i+1,j+1))*q[j];
  }
  out[3]=w; return out;
}
// OCCT's periodic flat sequence extends the principal domain by
// degree+1-multiplicity(1) knots at either end. The extra poles repeat the
// first poles cyclically. Enclose the exact translations and the stored
// binary64 flat knots. Never convert or refit a periodic curve.
template<class Spline>
inline void spline_knots(RawSpline& out,const Spline& spline) {
  out.degree=spline->Degree();
  const auto& flat=spline->KnotSequence();
  if(flat.Length()>4096)throw Failure{"knot_budget"};
  for(int i=flat.Lower();i<=flat.Upper();++i)out.knots.push_back(flat(i));
  if(!spline->IsPeriodic())return;
  const int count=spline->NbPoles(),extra=out.degree+1-spline->Multiplicity(1);
  if(extra<=0 || extra>count || count+extra>2048 ||
     flat.Length()!=count+extra+out.degree+1)throw Failure{"unsupported_periodic_layout"};
  for(double k:out.knots)out.knot_bounds.emplace_back(k);
  const I period=I(spline->Knot(spline->NbKnots()))-I(spline->Knot(1));
  if(period.lo<=0.)throw Failure{"unresolved_period"};
  auto enclose=[&](int i,I exact){
    out.knot_bounds[i]=I(std::min(out.knots[i],exact.lo),std::max(out.knots[i],exact.hi));
  };
  for(int j=0;j<extra;++j) {
    enclose(j,I(out.knots[count+j])-period);
    const int last=static_cast<int>(flat.Length())-extra+j;
    enclose(last,I(out.knots[last-count])+period);
    const auto repeated=out.poles[j];
    out.poles.push_back(repeated);
  }
  // Distinct translated knots must still be strictly ordered. A repeated
  // exterior value is allowed only for the same underlying principal knot.
  for(int i=1;i<flat.Length();++i) {
    if(out.knot_bounds[i-1].hi<out.knot_bounds[i].lo)continue;
    const int tail=flat.Length()-extra;
    if(out.knots[i-1]!=out.knots[i] || i==extra || i==tail ||
       (i<extra && out.knots[count+i-1]!=out.knots[count+i]) ||
       (i>tail && out.knots[i-count-1]!=out.knots[i-count]))
      throw Failure{"unresolved_periodic_knot_spacing"};
  }
}
inline RawSpline curve3(const TopoDS_Edge& edge,double& first,double& last) {
  TopLoc_Location loc;
  Handle(Geom_Curve) curve=BRep_Tool::Curve(edge,loc,first,last);
  for(int i=0;i<4;++i) { auto t=Handle(Geom_TrimmedCurve)::DownCast(curve);if(t.IsNull())break;curve=t->BasisCurve(); }
  RawSpline out;
  const auto spline=Handle(Geom_BSplineCurve)::DownCast(curve);
  const auto bezier=Handle(Geom_BezierCurve)::DownCast(curve);
  if(!spline.IsNull()) {
    if(spline->Degree()>8 || spline->NbPoles()>2048 || spline->NbKnots()>2048) throw Failure{"unsupported_native_3d_spline"};
    for(int i=1;i<=spline->NbPoles();++i)out.poles.push_back(homogeneous(spline->Pole(i),spline->Weight(i),loc.Transformation()));
    spline_knots(out,spline);
  } else if(!bezier.IsNull()) {
    if(bezier->Degree()>8)throw Failure{"unsupported_bezier_degree"};out.degree=bezier->Degree();
    out.knots.insert(out.knots.end(),out.degree+1,0.);out.knots.insert(out.knots.end(),out.degree+1,1.);
    for(int i=1;i<=bezier->NbPoles();++i)out.poles.push_back(homogeneous(bezier->Pole(i),bezier->Weight(i),loc.Transformation()));
  } else throw Failure{"unsupported_native_3d_curve"};
  validate(out);return out;
}
inline RawSpline curve2(const TopoDS_Edge& edge,const TopoDS_Face& face,double first,double last) {
  double a,b;
  Handle(Geom2d_Curve) curve=BRep_Tool::CurveOnSurface(edge,face,a,b);
  if(curve.IsNull() || a!=first || b!=last)throw Failure{"pcurve_parameter_ranges_differ"};
  for(int i=0;i<4;++i) { auto t=Handle(Geom2d_TrimmedCurve)::DownCast(curve);if(t.IsNull())break;curve=t->BasisCurve(); }
  RawSpline out;
  const auto spline=Handle(Geom2d_BSplineCurve)::DownCast(curve);
  const auto bezier=Handle(Geom2d_BezierCurve)::DownCast(curve);
  const auto line=Handle(Geom2d_Line)::DownCast(curve);
  auto pole=[](gp_Pnt2d p,double weight) {const I w(weight);return std::array<I,4>{I(p.X())*w,I(p.Y())*w,I(0.),w};};
  if(!spline.IsNull()) {
    if(spline->Degree()>8 || spline->NbPoles()>2048 || spline->NbKnots()>2048)throw Failure{"unsupported_native_pcurve_spline"};
    for(int i=1;i<=spline->NbPoles();++i)out.poles.push_back(pole(spline->Pole(i),spline->Weight(i)));
    spline_knots(out,spline);
  } else if(!bezier.IsNull()) {
    if(bezier->Degree()>8)throw Failure{"unsupported_pcurve_bezier_degree"};out.degree=bezier->Degree();
    out.knots.insert(out.knots.end(),out.degree+1,0.);out.knots.insert(out.knots.end(),out.degree+1,1.);
    for(int i=1;i<=bezier->NbPoles();++i)out.poles.push_back(pole(bezier->Pole(i),bezier->Weight(i)));
  } else if(!line.IsNull()) {
    out.degree=1;out.knots={first,first,last,last};
    for(double t:{first,last})out.poles.push_back({I(line->Location().X())+I(t)*I(line->Direction().X()),I(line->Location().Y())+I(t)*I(line->Direction().Y()),I(0.),I(1.)});
  } else throw Failure{"unsupported_native_pcurve"};
  validate(out);return out;
}

// Cox-de Boor restricted to one original knot span, with Bernstein polynomial
// arithmetic. Unlike GeomConvert, all rounding in the original-basis conversion
// is enclosed. Common subinterval endpoints may lie inside a knot span.
inline Poly times_linear(const Poly& old,I a0,I a1) {
  const std::size_t degree=old.size();Poly out(degree+1,I(0.));
  for(std::size_t i=0;i<=degree;++i) {
    if(i<degree)out[i]=out[i]+ratio(degree-i,degree)*old[i]*a0;
    if(i>0)out[i]=out[i]+ratio(i,degree)*old[i-1]*a1;
  }
  return out;
}
inline Net bernstein(const RawSpline& s,double a,double b) {
  if(!(a<b) || a<s.knots[s.degree] || b>s.knots[s.poles.size()])throw Failure{"spline_domain_mismatch"};
  const int k=static_cast<int>(std::upper_bound(s.knots.begin(),s.knots.end(),a)-s.knots.begin())-1;
  if(k<s.degree || k>=static_cast<int>(s.poles.size()) || b>s.knots[k+1])throw Failure{"interval_crosses_original_knot"};
  std::vector<Poly> basis(s.degree+1);basis[s.degree]={I(1.)};
  for(int j=1;j<=s.degree;++j) {
    std::vector<Poly> next(s.degree+1);
    for(int i=k-j;i<=k;++i) {
      const int index=i-(k-s.degree);Poly p(j+1,I(0.));
      if(!basis[index].empty() && s.knots[i+j]!=s.knots[i]) {
        const I d=s.knot(i+j)-s.knot(i);
        p=add(p,times_linear(basis[index],(I(a)-s.knot(i))/d,(I(b)-s.knot(i))/d));
      }
      if(index+1<=s.degree && !basis[index+1].empty() && s.knots[i+j+1]!=s.knots[i+1]) {
        const I d=s.knot(i+j+1)-s.knot(i+1);
        p=add(p,times_linear(basis[index+1],(s.knot(i+j+1)-I(a))/d,(s.knot(i+j+1)-I(b))/d));
      }
      next[index]=std::move(p);
    }
    basis=std::move(next);
  }
  Net out;
  for(auto& coord:out)coord=Poly(s.degree+1,I(0.));
  for(int i=0;i<=s.degree;++i)for(int c=0;c<4;++c)out[c]=add(out[c],scaled(basis[i],s.poles[k-s.degree+i][c]));
  return out;
}
struct Cylinder {
  V origin,x,y,z;
  I radius;
};
inline Cylinder cylinder(const TopoDS_Face& face) {
  TopLoc_Location location;
  Handle(Geom_Surface) surface=BRep_Tool::Surface(face,location);
  for(int i=0;i<4;++i){auto trim=Handle(Geom_RectangularTrimmedSurface)::DownCast(surface);if(trim.IsNull())break;surface=trim->BasisSurface();}
  const auto cyl=Handle(Geom_CylindricalSurface)::DownCast(surface);
  if(cyl.IsNull())throw Failure{"ancestor_not_cylindrical"};
  const auto frame=cyl->Position();const auto tr=location.Transformation();
  auto vector=[&](const gp_Dir& d){V out;for(int i=0;i<3;++i)out[i]=I(tr.Value(i+1,1))*I(d.X())+I(tr.Value(i+1,2))*I(d.Y())+I(tr.Value(i+1,3))*I(d.Z());return out;};
  const auto h=homogeneous(frame.Location(),1.,tr);
  return {{h[0],h[1],h[2]},vector(frame.XDirection()),vector(frame.YDirection()),vector(frame.Direction()),I(cyl->Radius())};
}
// Exact orthonormal frame defined by the stored source directions. Perform
// normalization and all composed placements using outward-rounded intervals.
struct LocalFrame {
  V center;
  std::array<V,3> axes;
  I dot(V a,V b) const {return a[0]*b[0]+a[1]*b[1]+a[2]*b[2];}
  V cross(V a,V b) const {return {a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]};}
  V vector(V p) const {return {dot(p,axes[0]),dot(p,axes[1]),dot(p,axes[2])};}
  LocalFrame(const double* origin,const double* x,const double* z) {
    auto v=[](const double* p)->V{return {I(p[0]),I(p[1]),I(p[2])};};
    auto unit=[](V a){return scale(a,I(1.)/norm(a));};
    center=v(origin);axes[0]=unit(v(x));axes[1]=unit(cross(v(z),axes[0]));axes[2]=cross(axes[0],axes[1]);
  }
  void apply(RawSpline& spline) const {
    for(auto& h:spline.poles) {
      const auto p=vector(sub({h[0],h[1],h[2]},scale(center,h[3])));
      for(size_t i=0;i<3;++i) h[i]=p[i];
    }
  }
  void apply(Cylinder& c) const {
    c.origin=vector(sub(c.origin,center));c.x=vector(c.x);c.y=vector(c.y);c.z=vector(c.z);
  }
};
// For E(t) = C(t) - cylinder(u(t),v(t)), Hermite interpolation H of
// values and first derivatives gives |E-H| <= sup |E^(4)| / 384.
// The scalar remainder applies to every unit-vector projection, hence also
// in Euclidean norm. The four Bernstein controls enclose H by convexity.
inline double pcurve_bound(const Jet& space,const Jet& uv,const Cylinder& c) {
  double controls=0.;
  for(int end=0;end<2;++end) {
    const auto sc=trig(uv.point[end][0]);
    const V radial=plus(scale(c.x,sc[1]),scale(c.y,sc[0]));
    const V angular=sub(scale(c.y,sc[1]),scale(c.x,sc[0]));
    const V value=sub(space.point[end],plus(c.origin,
      plus(scale(radial,c.radius),scale(c.z,uv.point[end][1]))));
    const V slope=sub(space.tangent[end],plus(scale(angular,c.radius*uv.tangent[end][0]),
      scale(c.z,uv.tangent[end][1])));
    controls=std::max({controls,norm(value).hi,norm(plus(value,scale(slope,I(end?-1.:1.)/I(3.)))).hi});
  }
  const I u1(magnitude(uv.range[1][0])),u2(magnitude(uv.range[2][0])),
          u3(magnitude(uv.range[3][0])),u4(magnitude(uv.range[4][0]));
  // Fourth derivative of exp(i*u), after factoring exp(i*u), has real part
  // u'^4-3*u''²-4*u'*u''' and imaginary part u''''-6*u'^2*u''.
  const I angular=square(square(u1))+I(3.)*square(u2)+I(4.)*u1*u3
                  +u4+I(6.)*square(u1)*u2;
  const I m=norm(space.range[4])+c.radius*(norm(c.x)+norm(c.y))*angular
            +norm(c.z)*I(magnitude(uv.range[4][1]));
  return (I(controls)+m/I(384.)).hi;
}
struct State {
  Budget budget;
  Counter& counter;
  Certificate result;
  bool started=false;
  I phase_end;
  double r,R;
  std::array<Cylinder,2> surfaces;
};
inline bool leaf(State& s,const std::array<Net,3>& nets) {
  ++s.result.attempts;
  const auto& n=nets[0];
  const I w=hull(n[3]);if(w.lo<=0.)return false;
  const auto x2=product(n[0],n[0]),y2=product(n[1],n[1]),z2=product(n[2],n[2]),w2=product(n[3],n[3]);
  const I f1=hull(add(add(x2,y2),scaled(w2,square(I(s.r))),true))/square(w);
  const I f2=hull(add(add(y2,z2),scaled(w2,square(I(s.R))),true))/square(w);
  const I rho2=hull(add(x2,y2))/square(w);
  if(rho2.lo<=0.)return false;
  const I rho=root(rho2),z=hull(n[2])/w;
  const int sign=z.lo>0.?1:z.hi<0.?-1:0;
  if(!sign)return false;
  const double zmin=sign>0?z.lo:-z.hi;
  const I delta=root(square(I(s.R))-square(I(s.r)));
  const I radial=I(magnitude(f1))/(rho+I(s.r));
  const I axial=(I(magnitude(f1))+I(magnitude(f2)))/(I(zmin)+delta);
  const double bound=norm({radial,axial,I(0.)}).hi;
  s.result.last_bound3d=bound;
  const Poly phase=add(product(n[0],derivative(n[1])),product(n[1],derivative(n[0])),true);
  const I h=hull(phase);const int direction=h.lo>0.?1:h.hi<0.?-1:0;
  if(!direction)return false;
  const I phase_denominator=hull(add(x2,y2));
  if(phase_denominator.lo<=0.)return false;
  const I phase_speed=h/phase_denominator;
  if(magnitude(phase_speed)>=1.)return false; // <1 radian, unique phase lift.
  const Jet space=jet(n);
  const double p0=pcurve_bound(space,jet(nets[1]),s.surfaces[0]);
  const double p1=pcurve_bound(space,jet(nets[2]),s.surfaces[1]);
  s.result.last_boundPcurves=std::max(p0,p1);
  if(bound>s.budget.tolerance_mm || std::max(p0,p1)>s.budget.tolerance_mm)return false;
  if (s.budget.mesh_deflection_mm>0.) {
    V least{};
    for (size_t i=0;i<3;++i) {
      const I speed=space.range[1][i];
      least[i]=I(std::max({0.,speed.lo,-speed.hi}));
    }
    const I accel=norm(space.range[2]), speed=norm(least);
    if (speed.lo<=0.) return false;
    // D2/8n² bounds linear interpolation; D2/|D1| bounds tangent rotation.
    s.result.mesh_segments+=std::max({1.,std::ceil(root(accel/(I(8.)*I(s.budget.mesh_deflection_mm))).hi),
        std::ceil((accel/(speed*I(.35))).hi)});
  }
  I a=angle(endpoint(n,false)),b=angle(endpoint(n,true));
  if(s.started) {
    const auto periods=std::nearbyint((middle(s.phase_end)-middle(a))/(2.*middle(pi())));
    a=a+I(periods)*(pi()*I(2.));
    if(a.hi<s.phase_end.lo || a.lo>s.phase_end.hi)throw Failure{"phase_join_unresolved"};
    if(s.result.sign!=sign || s.result.phase_direction!=direction)throw Failure{"branch_or_phase_orientation_changed"};
  }
  const auto periods=std::nearbyint((middle(a)-middle(b))/(2.*middle(pi())));
  b=b+I(periods)*(pi()*I(2.));
  if(std::abs(middle(b)-middle(a))>1.1)throw Failure{"phase_unwrap_unresolved"};
  if(!s.started) {
    s.result.phase_first_lo=a.lo;s.result.phase_first_hi=a.hi;s.result.phase_first=middle(a);
    s.result.sign=sign;s.result.phase_direction=direction;s.result.bound3d=0.;s.result.boundPcurves=0.;s.started=true;
  }
  s.phase_end=b;
  s.result.phase_last_lo=b.lo;s.result.phase_last_hi=b.hi;s.result.phase_last=middle(b);
  s.result.bound3d=std::max(s.result.bound3d,bound);
  s.result.boundPcurves=std::max(s.result.boundPcurves,std::max(p0,p1));
  if(++s.result.leaves>s.budget.max_leaves)throw Failure{"leaf_budget"};
  return true;
}
inline void visit(State& s,const std::array<Net,3>& n,int depth) {
  s.counter.spend();
  if(leaf(s,n))return;
  if(depth>=s.budget.max_depth)throw Failure{"continuous_bound_not_reached"};
  std::array<Net,3> left,right;
  for(int i=0;i<3;++i){auto halves=split(n[i]);left[i]=std::move(halves[0]);right[i]=std::move(halves[1]);}
  visit(s,left,depth+1);visit(s,right,depth+1);
}
} // namespace certificate_detail

// For centered, orthogonal axes the two branches remain separated by
// 2*sqrt(R²-r²). The local edge bounds below use that actual separation.
inline bool resolvedRadii(double small, double large, double uncertainty = 0.) {
  return std::isfinite(small) && std::isfinite(large) && small > 0. &&
         large - 1.5 * small > 2.5 * uncertainty;
}

inline Certificate certifyEdge(const TopoDS_Edge& edge,const TopoDS_Face& face1,
                               const TopoDS_Face& face2,double r,double R,
                               const Budget& budget=Budget{},const certificate_detail::LocalFrame* frame=nullptr) {
  using namespace certificate_detail;
  Counter counter{0,budget.max_work};Scope scope(counter);
  Certificate result;
  State state{budget,counter,{},false,I(0.),r,R,{}};
  try {
    static_assert(std::numeric_limits<double>::is_iec559,"binary64 interval arithmetic required");
    if(!resolvedRadii(r,R)||!std::isfinite(budget.tolerance_mm)||budget.tolerance_mm<=0.||budget.max_depth<0||budget.max_depth>30||budget.max_leaves==0||budget.max_leaves>65'536)
      throw Failure{"invalid_laboratory_contract"};
    double first,last;
    RawSpline space=curve3(edge,first,last);
    if(frame)frame->apply(space);
    if(!std::isfinite(first)||!std::isfinite(last)||!(first<last))throw Failure{"invalid_edge_parameter_domain"};
    const std::array<RawSpline,3> curves{space,curve2(edge,face1,first,last),curve2(edge,face2,first,last)};
    std::vector<double> cuts{first,last};
    for(const auto& c:curves)for(double k:c.knots)if(k>first&&k<last)cuts.push_back(k);
    std::sort(cuts.begin(),cuts.end());cuts.erase(std::unique(cuts.begin(),cuts.end()),cuts.end());
    if(cuts.size()>1024)throw Failure{"common_span_budget"};
    state.surfaces={cylinder(face1),cylinder(face2)};
    if(frame)for(auto& c:state.surfaces)frame->apply(c);
    for(std::size_t i=0;i+1<cuts.size();++i) {
      std::array<Net,3> n;
      for(int c=0;c<3;++c)n[c]=bernstein(curves[c],cuts[i],cuts[i+1]);
      visit(state,n,0);
    }
    result=state.result;
    result.ok=state.started;result.phase_monotone=state.started;
    result.reason=state.started?"continuous_local_edge_bounds_proven_global_winding_not_certified":"empty_certificate";
  } catch(const Failure& failure) {result=state.result;result.ok=false;result.reason=failure.reason;}
    catch(const Standard_Failure&) {result=state.result;result.ok=false;result.reason="native_geometry_exception";}
    catch(const std::exception&) {result=state.result;result.ok=false;result.reason="certificate_exception";}
  result.work=counter.work;
  return result;
}

// For this centered pair of through holes the common volume is
// 8 r² R times the integral of cos²(u) sqrt(1-q sin²(u)), u in [0,pi/2].
// Integrate the binomial series with q=(r/R)². The moments of
// cos²(u)sin^(2k)(u) have ratio (2k+1)/(2k+4), and moment zero is pi/4.
// All terms after the constant are negative and their ratios are at most q.
inline certificate_detail::I idealVolume(double lx,double ly,double lz,double small,double large) {
  using namespace certificate_detail;
  if(!(lx>2.*small && ly>2.*large && lz>2.*large && resolvedRadii(small,large)))throw Failure{"unsupported_volume_domain"};
  const I q=square(I(small)/I(large));
  I term=pi()/I(4.),sum(0.);
  // Keep the historical enclosure when q <= 1/4; for q < 4/9,
  // 48 terms keep the geometric tail below binary64 roundoff.
  const int terms = large >= 2.*small ? 20 : 48;
  for(int k=0;k<terms;++k) {
    sum=sum+term;
    term=term*q*(I(k)-I(.5))/I(k+1)*ratio(2*k+1,2*k+4);
  }
  sum=sum+I(-(I(magnitude(term))/(I(1.)-q)).hi,0.);
  return I(lx)*I(ly)*I(lz)-pi()*(I(lz)*square(I(small))+I(lx)*square(I(large)))
         +I(8.)*square(I(small))*I(large)*sum;
}

inline certificate_detail::I idealVolume(double side,double small,double large) {
 return idealVolume(side,side,side,small,large);
}

struct SelfTestResult { bool ok=false; std::string reason; std::size_t work=0; };
// Independent exact low-degree examples exercise arithmetic, original-knot
// extraction, and a genuinely accepted continuous 3D+pcurve+phase bound. They
// do not turn rejected real native edges into successful adversarial tests.
inline SelfTestResult selfTest() {
  using namespace certificate_detail;
  Counter counter{0,1'000'000};Scope scope(counter);
  SelfTestResult out;
  auto contains=[](I interval,double exact) {return interval.lo<=exact && interval.hi>=exact;};
  auto require=[](bool condition,const char* why) {if(!condition)throw Failure{why};};
  try {
    require(contains((I(1.)/I(3.))*I(3.),1.),"selftest_interval_division");
    require(contains(root(I(4.)),2.),"selftest_interval_sqrt");
    require(contains(norm({I(0.),I(0.),I(0.)}),0.),"selftest_zero_norm");
    require(contains(trig(pi()/I(6.))[0],.5),"selftest_sin_taylor");
    require(contains(trig(pi()/I(3.))[1],.5),"selftest_cos_taylor");
    const I quarter=pi()/I(4.),diagonal=angle({I(1.),I(1.),I(0.)});
    require(diagonal.lo<=quarter.lo && diagonal.hi>=quarter.hi && diagonal.hi-diagonal.lo<1e-12,"selftest_phase_interval");
    RawSpline spline;
    spline.degree=2;spline.knots={0.,0.,0.,.5,1.,1.,1.};
    for(const auto& p:std::array<std::array<double,2>,4>{{{0.,0.},{1.,1.},{2.,-1.},{3.,0.}}})
      spline.poles.push_back({I(p[0]),I(p[1]),I(0.),I(1.)});
    validate(spline);
    const Net first=bernstein(spline,0.,.5),second=bernstein(spline,.5,1.);
    const std::array<double,3> expected_x{0.,1.,1.5},expected_y{0.,1.,0.};
    for(int i=0;i<3;++i) {
      require(contains(first[0][i]/first[3][i],expected_x[i]),"selftest_cox_bernstein_x");
      require(contains(first[1][i]/first[3][i],expected_y[i]),"selftest_cox_bernstein_y");
    }
    const V mid1=endpoint(split(first)[0],true),mid2=endpoint(split(second)[0],true);
    require(contains(mid1[0],.875)&&contains(mid1[1],.5),"selftest_first_span_midpoint");
    require(contains(mid2[0],2.125)&&contains(mid2[1],-.5),"selftest_second_span_midpoint");
    // Independent quadratic uniform periodic example. The basis on the
    // first span [0,1] has controls (P1+P2)/2, P2, (P2+P3)/2.
    TColgp_Array1OfPnt cyclicPoles(1,4);
    TColStd_Array1OfReal cyclicKnots(1,5);
    TColStd_Array1OfInteger cyclicMult(1,5);
    for(int i=1;i<=4;++i) cyclicPoles.SetValue(i,gp_Pnt(i*i,0.,0.));
    for(int i=1;i<=5;++i){cyclicKnots.SetValue(i,i-1);cyclicMult.SetValue(i,1);}
    Handle(Geom_BSplineCurve) periodic=new Geom_BSplineCurve(cyclicPoles,cyclicKnots,cyclicMult,2,true);
    RawSpline cycle;
    for(int i=1;i<=4;++i)cycle.poles.push_back({I(i*i),I(0.),I(0.),I(1.)});
    spline_knots(cycle,periodic);
    validate(cycle);
    const Net start=bernstein(cycle,0.,1.),finish=bernstein(cycle,3.,4.);
    require(periodic->Value(0.).X()==2.5 && periodic->Value(4.).X()==2.5 &&
            periodic->Value(.5).X()==4.25 && periodic->Value(3.5).X()==3.25,
            "selftest_periodic_occt_control_order");
    for(const auto& test:std::array<std::pair<const Net*,std::array<double,3>>,2>{{
      {&start,{2.5,4.,6.5}},{&finish,{8.5,1.,2.5}}}}) {
      for(std::size_t i=0;i<3;++i)
        require(contains((*test.first)[0][i]/(*test.first)[3][i],test.second[i]),"selftest_periodic_bernstein_controls");
    }
    // Nonuniform cyclic spline with different seam/interior multiplicities.
    // Compare OCCT evaluation, including the seam and every span midpoint,
    // with the outward-rounded nets obtained from the native flat sequence.
    TColgp_Array1OfPnt variablePoles(1,6);
    TColStd_Array1OfReal variableKnots(1,5);
    TColStd_Array1OfInteger variableMult(1,5);
    const std::array<double,5> parameters{0.,.125,.375,.75,1.};
    const std::array<int,5> multiplicities{2,1,2,1,2};
    RawSpline variable;
    for(int i=1;i<=6;++i) {
      const gp_Pnt p(i*i,i*(7-i),i%3);
      variablePoles.SetValue(i,p);
      variable.poles.push_back({I(p.X()),I(p.Y()),I(p.Z()),I(1.)});
    }
    for(int i=1;i<=5;++i){
      variableKnots.SetValue(i,parameters[i-1]);variableMult.SetValue(i,multiplicities[i-1]);
    }
    Handle(Geom_BSplineCurve) nonuniform=new Geom_BSplineCurve(variablePoles,variableKnots,variableMult,3,true);
    spline_knots(variable,nonuniform);validate(variable);
    for(std::size_t i=0;i+1<parameters.size();++i) {
      const double a=parameters[i],b=parameters[i+1];
      const Net net=bernstein(variable,a,b);
      const auto halves=split(net);
      const std::array<V,3> values{endpoint(net,false),endpoint(halves[0],true),endpoint(net,true)};
      for(int j=0;j<3;++j) {
        const gp_Pnt native=nonuniform->Value(a+j*(b-a)*.5);
        for(int c=0;c<3;++c)
          require(contains(values[j][c],native.Coord(c+1)),"selftest_nonuniform_native_periodic_phase");
      }
    }
    // C(t)=(3,e*t,6), e=2^-14, is within O(e²) of both exact
    // cylinders. Its pcurves are the independent linear angle approximations.
    const double e=std::ldexp(1.,-14);
    std::array<Net,3> nets;
    nets[0]={Poly{I(3.),I(3.)},Poly{I(0.),I(e)},Poly{I(6.),I(6.)},Poly{I(1.),I(1.)}};
    nets[1]={Poly{I(0.),I(e)/I(3.)},Poly{I(6.),I(6.)},Poly{I(0.),I(0.)},Poly{I(1.),I(1.)}};
    nets[2]={Poly{I(0.),I(e)/I(6.)},Poly{I(3.),I(3.)},Poly{I(0.),I(0.)},Poly{I(1.),I(1.)}};
    const V zero{I(0.),I(0.),I(0.)},x{I(1.),I(0.),I(0.)},y{I(0.),I(1.),I(0.)},z{I(0.),I(0.),I(1.)};
    const std::array<Cylinder,2> surfaces{Cylinder{zero,x,y,z,I(3.)},Cylinder{zero,z,y,x,I(6.)}};
    Budget budget;budget.tolerance_mm=1e-7;
    State positive{budget,counter,{},false,I(0.),3.,6.,surfaces};
    require(leaf(positive,nets),"selftest_positive_local_certificate");
    require(positive.result.sign==1&&positive.result.phase_direction==1&&positive.result.bound3d<1e-7&&positive.result.boundPcurves<1e-7,"selftest_positive_quantities");
    // Perturb only the middle quartic pole. Its endpoint values and
    // first derivatives remain unchanged. The interior error at t=1/2
    // is 6/16 times 0.01 in the axial direction.
    Net interior;
    for(int c=0;c<4;++c)for(int i=0;i<=4;++i)
      interior[c].push_back(nets[1][c][0]+I(i)/I(4.)*(nets[1][c][1]-nets[1][c][0]));
    interior[1][2]=interior[1][2]+I(.01);
    require(pcurve_bound(jet(nets[0]),jet(interior),surfaces[0])>=.00375,"selftest_hermite_interior_corruption");
    auto wrong=nets;
    for(I& v:wrong[1][1])v=v+I(.01);
    State adversary{budget,counter,{},false,I(0.),3.,6.,surfaces};
    require(!leaf(adversary,wrong),"selftest_pcurve_corruption_not_rejected");
    wrong=nets;
    for(I& v:wrong[0][0])v=v+I(.01);
    State displaced{budget,counter,{},false,I(0.),3.,6.,surfaces};
    require(!leaf(displaced,wrong),"selftest_curve_corruption_not_rejected");
    out.ok=true;out.reason="interval_periodic_bernstein_hermite_and_positive_continuous_local_certificate_passed";
  } catch(const Failure& error) {out.reason=error.reason;}
    catch(const std::exception&) {out.reason="selftest_exception";}
  out.work=counter.work;return out;
}
} // namespace cross_drill_lab
