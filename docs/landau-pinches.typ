// Documentation-only inclusion fragment. No new algorithm or benchmark claim.
// Source/formula audit: SOURCES.json in the accompanying private handoff.
#let grad = math.op("grad")
#let Re = math.op("Re")
#let Im = math.op("Im")
= Landau pinches: what deformation can and cannot remove

A zero of a causal denominator, a trapped contour, a singular dependence on
kinematics, and a pole in dimensional regularisation are different statements.
The Landau conditions locate candidate singular configurations; the physical
sheet and numerator cancellations must also be respected. A pinch alone does
not determine whether the integrated answer diverges or only a derivative is
singular. This distinction is explicit in
#link("https://arxiv.org/html/2007.04085v1#S2.SS1.SSS4")[Collins, §II.1.4].
Here $i 0$ denotes the causal boundary value and $epsilon$ the dimensional
regulator; they are not the same limiting parameter.

== Why a strength change cannot unpinch the present map

For the real chart used by the contour methods,
$ w_i=x_i (1-x_i), quad v_i=w_i partial_i F, quad z_i=x_i-i lambda(x)v_i. $
At an interior stationary zero $F(x_*)=0$, $grad F(x_*)=0$, the direction
vanishes exactly. Every finite smooth choice of strength leaves $z(x_*)=x_*$.
Changing the safety factor $S$, strength cap $L$ or displacement scale $R$
cannot move this point. The usual local causal argument,
$ Im F(x-i lambda v)=-lambda sum_i w_i (partial_i F)^2+O(lambda^3), $
also has a vanishing leading term there. This is the same gradient-deformation
mechanism described by
#link("https://arxiv.org/html/1204.4152v2#S2.SS2")[Borowka, Carter and Heinrich, §2.2].

The full Jacobian still matters:
$ J_(i j)=delta_(i j)-i lambda partial_j v_i-i v_i partial_j lambda. $
Although $v(x_*)=0$, its derivative need not vanish; the determinant is not
generally one. Freezing $grad lambda$, dropping the determinant or making
$lambda$ singular at the point would not preserve the admitted construction.
A genuine causal pinch cannot be removed by an allowed nonsingular contour
homotopy. A different representation can instead expose its regulated local
contribution.

These statements concern an interior first-type configuration in a chart where
$U$ is nonzero; the gradient is with respect to the independent chart
coordinates. Zeros of face weights, singular coordinate maps, $U=0$, and
intersections of several factors require separate analysis. Failure of this
particular gradient direction, or a high-variance sample, is not by itself a
complete Landau classification.

== Local power counting does not imply a universal epsilon pole

An elementary local calculation illustrates the distinction. Suppose the
critical point is isolated, the numerator and $U$ are regular and nonzero,
and the quadratic form is positive definite in $m$ real variables. After a
linear change of variables, the leading model is
$ F= tau+abs(y)^2+"higher orders", quad (F-i 0)^(-beta(epsilon)). $
Here $tau$ measures distance from the threshold. For a scalar graph with
$ell$ loops and total propagator power $N_nu$, the standard parametric
exponent is $beta=N_nu-ell D/2$, $D=4-2epsilon$; see
#link("https://arxiv.org/html/1204.4152v2#S2.SS1")[the parametric representation, eq. (2)].
The following model identity follows directly by a Schwinger parameter and a
Gaussian integral:
$
 integral_(RR^m) frac(dif^m y, (tau+abs(y)^2)^beta)
 = pi^(m/2) frac(Gamma(beta-m/2), Gamma(beta)) tau^(m/2-beta),
 quad Re tau>0, quad Re beta>m/2.
$
With a local cutoff, analytic continuation of this expression describes the
leading nonanalytic term, up to analytic terms and a smooth nonzero factor.
Consequently the exponent $alpha=m/2-beta(0)$ determines a possible threshold
power: positive noninteger $alpha$ gives a finite branch term, while negative
$alpha$ can give a power divergence. At nonnegative integer $alpha=k$, the
generic nonanalytic term is $tau^k log(tau-i 0)$, including a logarithm for
$k=0$, provided its coefficient survives; for example $1/Gamma(beta)$ can
vanish. Apparent poles in the model's Gamma factor can cancel against the
analytic part of the cutoff integral; they are not automatically poles of the
complete regulated amplitude.

For example, $integral_RR dif y/(tau+y^2)=pi/sqrt(tau)$ for $tau>0$ has a
threshold power singularity without any dimensional regulator. Conversely,
numerator zeros can soften or cancel a singular term. A nonvanishing angular
average of a leading numerator of degree $sigma$ shifts the local scaling to
$(m+sigma)/2-beta$. An indefinite or rank-deficient Hessian, a critical
manifold or a boundary pinch changes the analysis; replacing $m$ by the
Hessian rank without treating the remaining directions is unjustified.

A possible dimensional pole can be seen directly at $tau=0$. The radial
power is $r^(m-1-2beta(epsilon))$. In the marginal case
$beta(epsilon)=m/2+b epsilon$, $b=beta'(0)!=0$, a constant smooth term gives,
apart from its angular coefficient,
$ integral_0^(r_0) r^(-1-2b epsilon) dif r
   = frac(r_0^(-2b epsilon), -2b epsilon), quad Re(b epsilon)<0. $
Continuation therefore exposes a possible simple $epsilon$ pole. Its residue
must still be combined with all sectors, numerators and prefactors. This
explicit marginal mechanism is distinct from a general threshold power or
logarithm at nonzero $tau$.

== A possible resolution and subtraction route

Conceptually, one could identify the singular locus, split cells so that it
lies on chart boundaries, and apply checked blowups or ramified maps until
the complete density has the form
$ product_j t_j^(a_j+b_j epsilon+c_j eta) h(t;epsilon,eta). $
The regular remainder must be controlled on every closed face. This requires
all numerator, $U$, $F$, transported denominator and Jacobian factors, exact
coverage of the original domain, and the original causal phase. It is not
enough to factor $F$ alone. Standard sector-decomposition termination results
already assume that the input polynomials have no interior zeros; they do not
automatically resolve a physical interior pinch
(#link("https://arxiv.org/html/0709.4092v2#S3")[Bogner and Weinzierl, §3]).

For one resolved variable, let $rho=a+b epsilon+c eta$ and
$T_(K-1)h(t)=sum_(k=0)^(K-1) h^((k))(0)t^k/k!$. The exact subtraction identity is
$
 integral_0^1 t^rho h(t) dif t
 = sum_(k=0)^(K-1) frac(h^((k))(0), k! (rho+k+1))
   + integral_0^1 t^rho (h(t)-T_(K-1)h(t)) dif t.
$
It is first justified in a common convergence domain and then continued
meromorphically; choose $K$ so the remainder is locally integrable in the
target regulator domain. The denominators display possible regulator poles.
A pole contributes only if its coefficient survives the complete sector and
prefactor sum. If dimensional regularisation does not change a divergent
direction ($b=0$), an additional analytic regulator $eta$, with its branch and
removal prescription fixed, may be needed. Multiple-regulator subtraction is
an established technique, illustrated by
#link("https://arxiv.org/html/1703.09692v3#S4.SS8")[pySecDec, §4.8].
It does not guarantee that every proposed resolution has a common starting
domain. The limits $tau arrow.r 0$, $epsilon arrow.r 0$ and $eta arrow.r 0$
must not be silently interchanged.

== Present implementation boundary

FastSecDec builds the weighted gradient and all map derivatives with native
Symbolica operations. Its retained causal-context diagnostic can certify a
stationary zero when reporting an unresolved deformation. It does not infer a
unique failing density branch or a complete Landau classification, and a
natively cancelled finite density remains allowed. Existing subtraction acts
on admitted resolved charts; there is no automatic general Landau-pinch
resolver or automatic extraction of new pinch-induced epsilon poles.

The splitting, resolution and auxiliary-regulator procedure above is a
theoretical direction, not an implementation or a benchmark result in this
report. At exact exceptional kinematics, the required result may be a
kinematic limiting value or a regulated distribution rather than a finite
ordinary integral. An unresolved proof or numerical failure cannot be
replaced by zero.
