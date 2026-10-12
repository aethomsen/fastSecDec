// Consolidated research chapter. No production or integration claims added.
#let grad = math.op("grad")
#let Re = math.op("Re")
#let Im = math.op("Im")
#let diag = math.op("diag")
= Critical loci and future region analysis <critical-loci>

The constructions in @methods handle regular threshold sheets and admitted
endpoint charts; @support-limits records their unresolved boundaries. Here we
examine exceptional critical loci, exact scratch evidence and prospective
local resolution and region analysis. These proposals are not implemented
capabilities or new benchmarks.

== Stationary zeros, causal pinches and present diagnostics

For the weighted-gradient map, $v_i=x_i (1-x_i)partial_i F$ and
$z=x-i lambda(x)v$. At an interior stationary zero
$F(x_*)=0$, $grad F(x_*)=0$, every finite smooth strength leaves
$z(x_*)=x_*$. Increasing the safety factor or caps cannot displace it.
The Jacobian need not be one: derivatives of $v$ survive even when $v=0$.
A genuine causal pinch cannot be removed by an admitted nonsingular contour
homotopy. This is a limitation of gradient deformation already recognized in
#link("https://arxiv.org/html/1204.4152v2#S2.SS2")[Borowka, Carter and Heinrich].

A denominator zero, a trapped contour, singular kinematic dependence and a
pole in dimensional regularisation are different statements. Numerators and
the physical sheet affect the answer; a pinch may make a derivative singular
without making the integral divergent
(#link("https://arxiv.org/html/2007.04085v1#S2.SS1.SSS4")[Collins, §II.1.4]).
Here the homogeneous conditions $F=grad F=0$ with $U!=0$ concern
finite-momentum, first-type candidates. Second-type singularities involve
infinity and require additional strata. Critical equations for the
Lee--Pomeransky polynomial $G=U+F$ have a different interpretation; see
#link("https://arxiv.org/html/2311.14669")[Fevola, Mizera and Telen].
The chart gradient uses independent coordinates; faces, vanishing $U$ and
singular coordinate maps cannot be classified by the interior test alone.

*Current detection is encounter-dependent, not an exhaustive search.* After
certain terminal nonfinite or precision failures, FastSecDec's cold diagnostic
checks retained $F$ and $A=sum_i x_i (1-x_i)(partial_i F)^2$ with native certified
balls. It reports a stationary-zero context only when both enclosures are
exactly zero, not merely when they contain zero. This remains available with
optional validation off and adds no hidden per-sample checking. A successfully
evaluated, natively cancelled density is allowed; the diagnostic does not
identify a unique failing branch. In the strict interior, $A=0$ means
$grad F=0$; on a face, zero weights can hide normal derivatives.

A finite pilot can miss a lower-dimensional locus. Neither large weights nor
small strengths certify its presence. For example,
$F_0=(x-1/2)^2$ and $F_1=1+(x-1/2)^2$ have identical derivative envelopes
for constant positive $U$. At $x=1/2$, the polynomial prescription gives
$lambda=S L$ in both cases: only their displacement vanishes. A complete
preflight would instead return *certified absent*, *certified loci found*, or
*unresolved*, with evidence renewed on rebinding unless a chamber proof applies.

== Equality strata: how exact algebra and CAD can cooperate

Full CAD can represent lower-dimensional solution cells. The current symGCAD
generic-open interface does not return them as cells. This matches
#link("https://github.com/magv/gcad#api")[Vitaly's original implementation],
which accepts strict inequalities and omits the algorithm's separate boundary
list; it is not a porting regression. Retained exceptional polynomials locate
possible missing strata, but do not resolve those strata.

The native control $0<x<1$, $F=(2x-1)^2$ returns two verified open intervals,
with no critical zero-dimensional cell, although exact substitution gives
$F(1/2)=F'(1/2)=0$. Equality input is rejected. The original implementation
returns the same intervals for $F>0$; its optional merge absorbs the zero seam.
Neither a completed generic solve nor equal signs across a seam prove absence
of a critical locus. Integration can nevertheless use open cells if their
partition identity is established in a common regulated convergence domain
and continued correctly. For example,
$
 integral_(-1)^1 (x-i 0)^(-1-epsilon) dif x
 = (1-e^(i pi epsilon))/(-epsilon) arrow.r.long i pi.
$
Initially $Re epsilon<0$; the continued phases preserve the boundary
contribution without a separate point integral. Measure zero alone would not
justify discarding its singular neighbourhood.

Native Symbolica derivatives, Groebner bases and algebraic solutions reduce
critical equations. On the positive projective interior, adjoining
$h product_i x_i-1=0$ excludes coordinate boundaries. A unit ideal proves
absence; a non-unit ideal needs real-domain tests. An exact parametrization
requires original-equation substitution and the converse under nonzero guards;
symGCAD verifies the remaining inequalities. Otherwise, complete equality/section
solving or recursion on exceptional strata is needed. A native nonvanishing
certificate for $F^2+sum_i (partial_i F)^2$ also proves absence; failure to
obtain one is inconclusive.

=== Exact controls at the benchmark points

The scratch calculations use native graph polynomials, exact kinematics and
the *strict positive projective interior*, fixing the last parameter to one.
They do not classify boundary loci or entire kinematic chambers.

#block(breakable: false)[
#set text(size: 8.3pt)
#table(
  columns: (1.25fr, 1.25fr, 2.4fr), inset: 4pt,
  stroke: 0.35pt + rgb("cad5de"),
  fill: (x, y) => if y == 0 { rgb("e7eff5") } else if calc.even(y) { rgb("f5f8fa") },
  table.header([*Case*], [*Exact point*], [*Certified interior outcome*]),
  [Massive bubble], [$m^2=3, s=16$], [Absent: saturated ideal $(1)$],
  [Massive triangle], [$m^2=1, s=5$], [Absent: saturated ideal $(1)$],
  [Massive box], [$m^2=1, s=5, t=-1$], [Absent: saturated ideal $(1)$],
  [Massless / massive sunrise], [$s=1$ / $m^2=1,s=16$], [Absent: saturated ideals $(1)$],
  [Massive kite], [$m^2=1, s=16$], [Absent: strictly positive ideal member],
  [Bubble / sunrise thresholds], [$m^2=3,s=12$ / $m^2=1,s=9$], [Found at all parameters one; $U=2$ / $3$],
  [Three-loop crown], [$s=1,t=-1/5$], [Found: four-dimensional projective family],
)
]

The triangle has two massless external legs; the box has massless external
legs and $u=-4$. All five kite masses equal one. For the kite, an independently
checked combination of its five homogeneous derivatives equals
$1+x_0+x_1+x_2+x_3+16x_0 x_3>0$. Its complex critical variety need not be empty.
These exact exclusions are small algebra diagnostics, not integral-generation
timings. The threshold controls show that critical points are not confined to
three loops. No double-box critical-locus classification was performed.

=== The crown: retain tangent and normal directions separately

The three-loop, eight-propagator massless crown polynomial comes from
#link("https://arxiv.org/html/2407.13738#S3.SS3")[Gardi, Herzog, Jones and Ma,
§3.3], also treated by
#link("https://arxiv.org/html/2506.24073v1#S4.SS1.SSS6")[Olsson and Stone].
With zero-based labels, set $x_1=1$ and use
$
 a=x_0, quad b=x_3, quad c=x_5, quad d=x_7,
 quad x_2=a b+u, quad x_4=a c+v, quad x_6=a d+w.
$
Native substitution and matrix operations verify
$
 F=(s+t)d u v-t c u w-s b v w, quad det H_perp=2s t(s+t)b c d.
$
For $s t(s+t)!=0$ and positive $b,c,d$, the normal equations force
$u=v=w=0$. All eight original derivatives and $F$ then vanish, proving
conditional completeness of this positive interior family. It has four free
positive projective parameters and
$U|_(u=v=w=0)=(a+1)^3(b c+b d+c d+b c d)>0$.
GCAD verifies these reduced inequalities as one positive-$U$ cell; it does
not discover the locus by itself. The full projective Hessian has rank three,
so an isolated-point treatment would be inappropriate.

The scratch process took 0.0432 s and 12.7 MiB, using warm native libraries;
reduced GCAD solve/verification took 7.36/2.31 ms. No crown integrand was
generated or integrated. Exact revisions, commands and independent review
are in `docs/reviews/gcad-critical-loci.md` and
`docs/gcad-critical-loci-results.json`; these observations do not time a
general critical-locus solver.

== From a certified pinch to local subtraction

Local power counting determines whether a pinch produces a divergent value,
a threshold branch or a regulator pole. For an isolated definite quadratic
point with regular nonzero numerator and $U$, the model is
$F=tau+abs(y)^2+"higher orders"$. For $Re tau>0$, $Re beta>m/2$,
$
 integral_(RR^m) frac(dif^m y,(tau+abs(y)^2)^beta)
 =pi^(m/2) frac(Gamma(beta-m/2),Gamma(beta)) tau^(m/2-beta).
$
With a local cutoff, continuation determines the leading nonanalytic term up
to analytic terms and smooth factors. Negative powers can diverge; positive
noninteger powers can remain finite; integer limits can produce logarithms
if their coefficients survive. Numerator zeros change the power. Apparent
Gamma poles can cancel with the cutoff's analytic part. At $tau=0$ with
$beta(epsilon)=m/2+b epsilon$, the marginal radial model gives
$
 integral_0^(r_0) r^(-1-2b epsilon) dif r
 =r_0^(-2b epsilon)/(-2b epsilon), quad Re(b epsilon)<0.
$
This exposes a *possible* dimensional pole, whose residue must survive the
complete sum. It does not establish a universal epsilon pole at every pinch.
Degenerate or indefinite forms and critical manifolds require their own
normal/tangent analysis, not substitution of Hessian rank for $m$.

=== A targeted hybrid can avoid the full sign atlas

One prospective route resolves only certified critical neighbourhoods and
retains the contour elsewhere. It need not partition every regular point of
$F=0$. Pinch-dissection precedents exist in
#link("https://arxiv.org/html/2407.13738")[Gardi et al.]; the following
local argument is a separate, unimplemented construction.

At an isolated stationary point with nonsingular real Hessian $H$,
$W_*=diag(x_i (1-x_i))>0$, $U!=0$ and finite $lambda_*>0$, write
$x=x_*+y$. The same smooth global contour gives
$
 z=x_*+M y+O(abs(y)^2), quad M=I-i lambda_*W_*H,
$
$
 Im F(z)=-lambda_* y^T H W_* H y+O(abs(y)^3).
$
The leading imaginary form is strictly negative for nonzero real $y$, even
for an indefinite $H$, and $M$ is invertible. The analytic expansion uses
ordinary transpose, not conjugate transpose. Local-strength derivatives
remain in the full Jacobian and affect higher terms.

Split a certified cube $abs(y_i)<=delta$ into orthants and largest-coordinate
wedges. In each, use $y_p=delta s_p r$, $y_j=delta s_j r t_j$ for $j!=p$,
with $0<=r,t_j<=1$ and $s_j in {-1,1}$. The real volume factor is
$delta^d r^(d-1)$ and exact stationarity extracts
$F(z(x_*+delta r a(t)))=r^2 Q(r,t)$.
The value at $r=0$ is the smooth derivative-defined extension, never numerical
$0/0$. A certified higher-order remainder yields $Im Q<-c<0$ on the closed
angular charts for a sufficiently small cube. The radial density then has
the form $r^(d-1-2beta(epsilon))h(r,t;epsilon)$ and admits symbolic subtraction.

This favorable construction starts with at most $d 2^d$ local charts per
isolated point; it excludes complement charts and later kernel splitting.
For the crown, a transverse analogue would retain four tangent parameters
and start locally with $3 times 2^3=24$ normal charts, subject to uniform
bounds. This is neither a global sector forecast nor the paper's different
24-ordering dissection. The double-box's 644,699 sign cells count neither
critical points nor local pinch charts.

Preserving one global contour across artificial interfaces is essential:
resetting face weights on each new cube can freeze regular threshold zeros on
the seams. Exact centres, full numerators/factors/Jacobians, coverage and all
closed-face derivatives must be retained. Degeneracies, tangent-boundary
collisions and $U=0$ require further resolution. Subtraction identities must
first hold in a common regulated family; additional analytic regulators can
be necessary (#link("https://arxiv.org/html/1703.09692v3#S4.SS8")[pySecDec,
§4.8]). No numerical failure or unresolved chart becomes a zero contribution.

#pagebreak()
== Prospects for complete expansion by regions

The natural objective is a *complete, possibly redundant asymptotic expansion
to a requested order*, with coverage and remainder control. A minimal list of
nonzero regions is a harder separate problem. Exact critical-locus analysis
could improve construction, but it does not supply this theorem by itself.

=== Position relative to the literature

#link("https://arxiv.org/abs/1011.4863")[Pak--Smirnov geometry] associates
scalings with Newton-polytope faces. The
#link("https://arxiv.org/abs/1206.0546")[asy2 method] already exposes potential
and Glauber regions in its examples;
#link("https://arxiv.org/abs/2108.10807")[pySecDec's regions implementation]
discusses cancellations in Minkowski kinematics. A hidden region can become a
facet region after a change of variables: “interior” refers here to parameter
space, not an invariant location in the original exponent polytope.

#link("https://arxiv.org/html/2609.29442#S2.SS4")[Gardi's Hidden Region Finder
(HRF), September 2026] is the closest comparator. It connects cancellation
loci, dissection and asymptotic scaling; §2.4.3 proposes a scaling-first
face/positive-Landau search as a theoretical alternative to its negative
audits. Its study treats massless, unit-numerator examples and leaves massive
and numerator-dependent extensions open.
#link("https://arxiv.org/html/2503.16601v2#S5")[SOFIA, by Correia, Giroux and
Mizera], instead studies candidate kinematic singularities and alphabets via
Baikov representations and Fubini reduction. It is not a facet-only regions
tool. Certified local exponents and physical-sheet analysis could complement
its outputs.

=== What a completeness claim would require

The limiting family carries information absent from its critical set:
$F_2=(x-a)^2+rho$ and $F_4=(x-a)^4+rho$ share the same limiting stationary
zero but have widths $rho^(1/2)$ and $rho^(1/4)$. Neither has a real stationary
zero at positive $rho$. A solve at one finite kinematic point is insufficient.
A viable certificate contract would require:

1. An exact limiting family, physical chamber, causal sheet and target order,
   including projective faces, infinity and relevant $U=0$ intersections.
2. Certified real solutions of the relevant *leading* systems. Scaling can
   promote cancellations absent from the stationary set of the unscaled $F$.
3. Local charts retaining multiplicities, tangent parameters, branches and
   Jacobians; resolution of nested or colliding strata and the regular
   complement; ordinary boundary regions alongside hidden ones.
4. A partition or explicit overlap prescription, and bounds on the integrated
   remainder in a common convergence domain before analytic continuation.
   Factors $rho^(a+b epsilon)$ must survive until the Laurent expansion.

Equality-aware CAD and variety sub-CAD can avoid irrelevant ambient cells
under their respective hypotheses
(#link("https://arxiv.org/abs/1903.08999")[England, Bradford and Davenport] and
#link("https://arxiv.org/abs/1401.0647")[Wilson et al.]). Degenerate equations
and vanishing projection polynomials need a complete fallback. The current
symGCAD open-cell interface is not that equality solver.
#link("https://arxiv.org/abs/1111.2589")[Jantzen's regional identities] require
covering/convergence assumptions and overlap terms. Regulator-pole
cancellation cannot certify completeness: an omitted contribution can be
finite. Leading-order vanishing does not justify dropping all higher orders;
exact coefficient cancellation may require integral identities.

=== Where symGCAD and the native implementation could help

#block(breakable: false)[
#set text(size: 8.3pt)
#table(
  columns: (1.15fr, 1.7fr, 1.85fr), inset: 4pt,
  stroke: 0.35pt + rgb("cad5de"),
  fill: (x,y) => if y == 0 { rgb("e7eff5") } else if calc.even(y) { rgb("f5f8fa") },
  table.header([*Opportunity*], [*Actual foundation*], [*Prospective gain / missing work*]),
  [Target critical geometry],
  [Scalar exclusions; crown's exact normal reduction.],
  [Avoid a global sign atlas; general component coverage still needed.],
  [Reject inadmissible candidates],
  [Exact positivity/nonvanishing certificates and kite identity.],
  [Reduce expensive real-domain work; failed certificates remain unresolved.],
  [Reduce and reuse algebra],
  [Linear/quadratic reductions, positive-root covers, bounded ordering, exact caches.],
  [Share guarded geometry across faces/families; preserve exceptional strata.],
  [Treat the full density],
  [Native numerators, symbolic derivatives, composed root maps and subtraction.],
  [Massive algebraic branches and numerator-dependent orders; general family resolution and remainder proofs missing.],
)
]

The inventory is pinned to
#link("https://github.com/alphal00p/symGCAD/blob/36bfd9ebedf617a884518458d51cf1c2adcd30bd/reference/FINAL_CHECKPOINT.md")[symGCAD
`36bfd9e`] checkpoint; some options remain experimental. Its
#link("https://github.com/alphal00p/symGCAD/blob/36bfd9ebedf617a884518458d51cf1c2adcd30bd/README.md#results-and-validation")[matched
study] reports a 6.82-fold geometric-mean improvement in generic-CAD algebra
phases over the original implementation. This is not an end-to-end regions
speedup, and unresolved cases remain. The most concrete extension beyond
HRF's stated scope is massive families with algebraic branches and realistic
numerators, not an already demonstrated faster or complete region finder.

A first comparative study should reproduce Euclidean regions, massive
threshold controls, the crown and HRF's moving/boundary examples. Compare
exact scalings and coefficient expansions before speed; record discovery,
resolution, verification, chart counts and memory, including negative cases
and incomplete searches. The acceptance criterion is certified coverage with
a controlled remainder. Literature and reuse evidence are retained in
`docs/reviews/region-analysis-prospects.md`; pinch assumptions and source
audits are in `docs/reviews/landau-pinches.md`.
