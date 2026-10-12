// Documentation-only inclusion fragment. No new algorithm or benchmark claim.
// Source/formula audit: SOURCES.json in the accompanying private handoff.
#let grad = math.op("grad")
#let Re = math.op("Re")
#let Im = math.op("Im")
#let diag = math.op("diag")
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

== Detection: local evidence is not a domain-wide search

*The current dynamic prescription does not automatically locate all stationary
zeros.* It retains $F$ and $A=sum_i w_i (partial_i F)^2$. After certain terminal
nonfinite or precision failures, a cold diagnostic evaluates this retained
context with native certified balls. It reports a stationary-zero context only
when both enclosures are exactly zero, rather than merely containing zero.
This diagnostic also operates with optional causal validation off; it is not a
hidden per-sample validation cost. It does not identify a unique failing density
branch. A successfully evaluated density, including an exact numerator
cancellation, does not trigger a rejection simply because its unreduced $F$
has a stationary zero.

In the strict real interior, $w_i>0$, so $A=0$ is equivalent to $grad F=0$.
On a face, vanishing weights hide normal derivatives: the same assertion then
has only its weighted/tangential meaning. A finite pilot or production sample
can miss an isolated point or a lower-dimensional locus. Small residuals, large
weights and repeated precision rescue are useful candidates for investigation,
not proof that a stationary zero exists or that all such zeros were found.

Nor is a collapsing strength a reliable warning. With constant positive $U$,
consider $F_0=(x-1/2)^2$ and $F_1=1+(x-1/2)^2$. Their derivative envelopes and
dynamic strengths are identical, although only the first has a stationary
zero. At $x=1/2$, the polynomial construction gives $r=L$ and $lambda=S L$:
the displacement vanishes because $v=0$, not because $lambda$ vanishes.
Sign-aware smoothing may alter the limiting strength, but still
does not distinguish the two cases. Successful radius solving therefore
cannot certify absence of a pinch.

A proposed automatic preflight would instead solve the *critical system*
$ F(x;k)=0, quad partial_i F(x;k)=0, quad 0<x_i<1 $
in the independent projective/chart coordinates at the requested kinematics
$k$, retaining nonvanishing $U$ and all original factor/domain conditions.
Native derivatives and algebraic elimination could locate candidates; certified
real-root isolation or subdivision must then establish existence and coverage.
For a nonsingular Hessian, certified solution of $grad F=0$ still needs a
separate exact check of $F=0$ at that algebraic point. A ball containing zero
is not that equality check. Singular solutions and critical manifolds require
additional algebraic/stratified treatment; ordinary Newton iteration is not
a complete solver. Face strata require their own restricted equations.

Algebraic approaches to Landau loci are developed in
#link("https://arxiv.org/html/2109.08036")[Mizera and Telen, Landau Discriminants]
and #link("https://arxiv.org/html/2311.14669")[Fevola, Mizera and Telen, Landau
Singularities Revisited]. Their qualifications about boundary/scaling strata
matter: the displayed interior system is a useful restricted problem, not a
complete classification of every graph singularity. The proposed preflight
would have to distinguish *certified absent*, *certified loci found*, and
*unresolved*. Evidence must be renewed on kinematic rebinding unless a parameter
chamber proof applies. This solver orchestration is not currently implemented.

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

== Targeted local blow-ups while retaining the contour

*A hybrid route can avoid constructing a global sign decomposition of $F$.*
The proposed target is the usually smaller set $F=grad F=0$, not every regular
point on the threshold hypersurface $F=0$. Isolate and certify neighborhoods
of the critical loci, resolve only those neighborhoods, and retain the contour
on the complement. The published pinch-dissection examples of
#link("https://arxiv.org/html/2407.13738")[Gardi, Herzog, Jones and Ma]
provide a precedent for exposing a pinch by coordinate changes and domain
splitting before sector decomposition. They do not establish a general fast
algorithm for every topology. The construction below is our local derivation,
not an implemented feature or a performance result attributed to that paper.

An explicit favorable case is an isolated interior stationary zero $x_*$ with
real nonsingular Hessian $H$, positive face-weight matrix
$W_*=diag(w_i (x_*))$, nonzero $U$ and otherwise smooth density factors.
Keep the original smooth causal map with finite $lambda_*=lambda(x_*)>0$.
Writing $x=x_*+y$ gives
$ z(x_*+y)=x_*+M y+O(abs(y)^2), quad M=I-i lambda_* W_* H, $
$ F(z(x_*+y))=1/2 y^T M^T H M y+O(abs(y)^3), $
and hence
$ Im F(z(x_*+y))=-lambda_* y^T H W_* H y+O(abs(y)^3). $
Because $H$ is nonsingular and $W_*$ is positive definite, $H W_* H$ is
positive definite even when $H$ is indefinite. The leading imaginary quadratic
form is therefore strictly negative in every nonzero real direction. Moreover,
$W_* H$ is similar to a real symmetric matrix, so $M$ is invertible. This
argument uses the ordinary transpose, not the Hermitian transpose, as required
by the analytic polynomial. Strength derivatives remain in the full Jacobian;
they first affect higher terms here because $v(x_*)=0$.
The same local argument applies to an admitted positive fixed strength.
Dynamic causal bounds help establish map admissibility; they do not perform
the critical-locus search.

Choose a certified small cube $abs(y_i)<=delta$ around the exact critical
point. Split its $2^d$ orthants into $d$ regions according to the largest
$abs(y_i)$. In the region with largest coordinate $p$, use
$ y_p=delta s_p r, quad y_j=delta s_j r t_j quad (j!=p), $
$ 0<=r<=1, quad 0<=t_j<=1, quad s_j in {-1,1}. $
Its positive real volume factor is $delta^d r^(d-1)$, with chart orientation
accounted for. Boundaries shared by charts must be assigned consistently.
Exact stationarity extracts
$ F(z(x_*+delta r a(t)))=r^2 Q(r,t). $
At $r=0$, $Q$ is its smooth derivative-defined extension, never a numerical
$0/0$ quotient. Native symbolic derivatives supply its endpoint coefficients.
The strict quadratic estimate and a certified higher-order remainder bound
give $Im Q<-c<0$ on these closed angular charts for sufficiently small $delta$.
The radial endpoint carries the pinch; the complex smooth factor has no local
angular zero, including for a nonsingular saddle Hessian. A purely real blow-up
has this simple unit property for a definite Hessian, but an indefinite one
can leave a real angular zero cone. Retaining the contour is what removes that
additional obstacle in the hybrid argument.

For a factor $(F-i 0)^(-beta(epsilon))$, the radial measure becomes
$ r^(d-1-2 beta(epsilon)) h(r,t;epsilon). $
Native symbolic subtraction can then expose any regulator pole as above,
provided the complete regulated family and boundary derivatives are admitted.
The original complex $det J_z$ multiplies the radial volume factor exactly
once. Numerators, positive factors and all derivatives of $lambda$ remain in
$h$. Compact composed maps and finite symbolic endpoint coefficients would
avoid expanding the entire substituted integrand.

For this particular isolated nondegenerate case, the explicit construction
starts with *at most $d 2^d$ local radial charts per point*: 8 in two dimensions,
24 in three, or 384 in six. This excludes complement charts, further factor
resolution and Laurent kernel splitting; it is not a bound for degenerate
pinches or the final sector count. No number of such points has been established
for the double-box benchmark. Its 644,699 sign cells do not count Landau points
and do not predict the cost of a critical-locus search.

There are essential qualifications. Partition the pullback of the *same global
contour*, or prove compatible homotopies across artificial interfaces; resetting
the contour face weights independently in each new cube could freeze regular
threshold zeros on new seams. Establish the partition identity in a common
regulated convergence domain before continuing to the physical regulators.
An algebraic critical center must remain exact/certified: approximate recentring
leaves constant or linear terms and invalidates the exact $r^2$ extraction.
Degenerate Hessians, nonisolated or intersecting loci, collisions with existing
faces and vanishing $U$ need further stratification or resolution. Near a
threshold, a nonzero unfolding parameter also changes the radial form and may
need its own parameter-dependent treatment. Proving that all relevant loci have
been covered can still be expensive. Thus this hybrid plausibly avoids the
specific global-GCAD cell explosion in favorable cases; it has no universal
small-complexity or improved-variance guarantee.

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
