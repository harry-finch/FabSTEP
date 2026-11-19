#pragma once

#include "cross_drill_certificate.hxx"
#include <stdexcept>

namespace cross_drill_lab {

// For an orthogonal through-bore, integrate the chord of the outer cylinder
// over the bore disk. Its projected coordinate X has density 2 sqrt(1-X²)/π.
// The removed volume is 2πr²R E[sqrt(1-(a+bX)²)], a=|offset|/R, b=r/R.
inline certificate_detail::I shaft_volume(double height, double shaft_radius,
                                          double bore_radius, double offset) {
  using namespace certificate_detail;
  constexpr int terms = 64;
  try {
    const I R(shaft_radius), r(bore_radius), h(height);
    if (!(height > 0. && shaft_radius > 0. && bore_radius > 0.))
      throw Failure{"invalid_shaft_dimensions"};
    const I a=I(std::abs(offset))/R, b=r/R, q=square(a+b);
    if (q.hi >= .65) throw Failure{"unsupported_shaft_offset"};

    std::array<I, 2*terms+1> powers;
    powers[0]=I(1.);
    for (int n=1; n<=2*terms; ++n) powers[n]=powers[n-1]*a;

    // Odd moments vanish. For the disk, E[X^(2j)]/E[X^(2j-2)] = (2j-1)/(2j+2).
    std::array<I,terms+1> disk;
    disk[0]=I(1.);
    const I b2=square(b);
    for (int j=1; j<=terms; ++j)
      disk[j]=disk[j-1]*b2*ratio(2*j-1,2*j+2);

    I sum(1.), coefficient(.5), q_power(1.);
    for (int k=1; k<=terms; ++k) {
      const int n=2*k;
      I choose(1.), moment=powers[n];
      for (int j=1; j<=k; ++j) {
        const int m=2*j;
        choose=choose*ratio((n-m+2)*(n-m+1),m*(m-1));
        moment=moment+choose*powers[n-m]*disk[j];
      }
      sum=sum-coefficient*moment;
      coefficient=coefficient*(I(k)-I(.5))/I(k+1);
      q_power=q_power*q;
    }
    // All omitted binomial terms have the same sign, coefficients <= 1,
    // and (a+bX)^2 <= q for all points of the disk.
    const I remainder=q_power*q/(I(1.)-q);
    sum=sum-I(0.,remainder.hi);
    return pi()*square(R)*h-I(2.)*pi()*square(r)*R*sum;
  } catch (const Failure& e) {
    throw std::runtime_error("Volume de l’arbre percé non certifié : "+e.reason);
  }
}
} // namespace cross_drill_lab
