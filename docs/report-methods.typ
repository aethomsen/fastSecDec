#let note(body) = block(width: 100%, inset: 8pt, fill: rgb("edf4f7"), radius: 3pt, body)
#let diag = math.op("diag")
#let tr = math.op("tr")
#let grad = math.op("grad")
#let Re = math.op("Re")
#let Im = math.op("Im")

= Threshold-regularisation methods <methods>

The benchmark compares two ways to represent a causal integral: deform its
parameter contour, or partition the real domain and continue the resulting
endpoint integrals analytically. Both preserve the original numerator,
normalization and complete Laurent vector. The second route additionally
requires endpoint certificates; a successful sign decomposition alone is
insufficient. This chapter defines the constructions behind the overview.

== Common geometry and fixed strength
After sector mapping, separate the real endpoint monomials from the smooth
density. Let $F(x)$ denote the designated residual causal polynomial and let
$U(x)$ denote each retained positive residual factor. The definition of $F$
includes its physical scale: rescaling $F$ also rescales this deformation.
For a sampled sector with real masses, real kinematics and $x in [0,1]^n$
($n>=1$), define
$
  w_i = x_i (1-x_i), quad v_i = w_i partial_i F,
  quad z_i(x) = x_i - i lambda(x) v_i(x).
$
Exact zero-dimensional contributions bypass sampling. The coordinate faces
remain fixed in their normal direction. With
$A = sum_i w_i (partial_i F)^2 >= 0$, the ray obeys
$ Im F(x-i t v) = -t A + O(t^3). $
Thus a sufficiently small positive strength moves the causal denominator to
the lower half-plane. The scalar $lambda$ is a strength, not the norm of the
displacement; no undocumented normalization by $F$ or its gradient is used.
Since cube coordinates are dimensionless, $[lambda]=[F]^(-1)$; the same units
apply to $L$. The displacement limit $R$ is in parameter-space units.

*Fixed mode* takes a finite runtime value $lambda(x)=lambda_0>0$. Its Jacobian is
$
 J_(i j) = delta_(i j)-i lambda_0 partial_j v_i,
 quad partial_j v_i = delta_(i j)(1-2x_i)partial_i F
   + w_i partial_j partial_i F.
$
The complex, oriented determinant $det J$ multiplies the density. A zero
determinant alone is not a pole crossing. Neither its absolute value nor a
frozen-gradient approximation gives the same contour integral.

For endpoint exponents $a_i+b_i epsilon$, the real endpoint monomials are
retained and the smooth density receives
$
 det J product_i rho_i^(a_i+b_i epsilon), quad
 rho_i = z_i/x_i = 1-i lambda(x)(1-x_i)partial_i F.
$
The last expression defines $rho_i$ even at $x_i=0$. Deform the numerator and
residual factors as well. Apply this pullback *before* Taylor/IBP subtraction
and Laurent construction. Boundary terms restrict the same full-sector map;
they do not construct a new, lower-dimensional deformation.

== Branches and checking policy
Continue each factor separately. Endpoint ratios have real part one; positive
residuals must keep the appropriate branch, and the causal logarithm has the
lower-lip value $log(F-i 0)=log(abs(F))-i pi$ when $F<0$. Combining these continued
logarithms into the principal logarithm of their product can change the sheet.
A stationary point with $F != 0$ is allowed. A surviving singularity at $F=0$
and $A=0$ requires an unresolved-deformation diagnostic, not a fabricated zero;
a natively cancelled denominator or finite smooth density remains finite.

Optional validation has three policies: *always* checks pilot and production,
*pilot* checks separate pilot work and removes optional production checks, and
*off*, now the default, removes both. Solver termination, precision rescue and
nonfinite handling remain active. The overview's sampling and convergence
timings use *off*; separate validation probes are excluded.
Fixed-mode failures identify the factor and point; they never silently shrink
the requested strength. Finite pilots are safeguards, not global certificates.
The gradient-map convention and subtraction ordering follow the established
sector-decomposition approach; see #link("https://arxiv.org/abs/1703.09692")[pySecDec].

#pagebreak()
== Dynamic strength: a smooth sufficient causal radius
Set $lambda(x)=S r(x)$ with $0<S<1$. The radius bounds the *whole homotopy*
$x-i t v(x)$ for $0<t<=S r(x)$. It is not the distance to the nearest complex
root: a positive real ray root need not exist, and a root-modulus prescription
can vanish at the very physical pole one must avoid.

In the auxiliary ray expansion, hold $v(x)$ fixed. For odd $k>=3$, define
$
 T_k = D^k F(x)[v^(k-2), dot, dot], quad
 B_k = frac(sum_(i,j) w_i w_j (T_k)_(i j)^2, (k!)^2).
$
For every positive residual $U$, write the real even-ray expansion
$ Re U(x-i t v) = U(x)+sum_(k>=2, "even") d_k(x)t^k,
  quad C_k = d_k^2/U(x)^2. $
Native polynomial positivity admission supplies a strictly positive residual
$U$ on the closed cube; an arbitrary positive floor is not substituted.
Let $m_F$ and $m_U$ count the structurally retained odd and even orders.
With positive cap $L$ and displacement scale $R$, solve
$
 H(x,r) = r^2/L^2 + r^2 norm(v)^2/R^2
  + m_F sum_(k>=3, "odd") B_k r^(2k-2)
  + sum_U m_U sum_(k>=2, "even") C_k r^(2k) = 1.
$
All coefficients are nonnegative. The cap term makes the positive root unique
and simple, with $r<=L$ and $r H_r>=2$ at the root. Both constraints are smooth;
there is no hard minimum whose derivative would jump.

*Why the bound is causal.* Put $W=diag(w_i)$ and $q=sqrt(W) grad F$, so
$A=q^T q$ and $v=sqrt(W)q$. The higher odd ray term satisfies
$
 abs(D^k F[v^k]/k!) <= A sqrt(B_k), quad
 sum_k sqrt(B_k)t^(k-1) <= sqrt(m_F sum_k B_k t^(2k-2)).
$
Consequently higher imaginary terms cannot cancel the leading $-t A$ along
the admitted homotopy when $A>0$. The even-factor bound similarly keeps
$Re U(z)>0$. The proof does not divide by $A$, so small gradients do not create
an artificial singular bound. At $A=0$, the stationary-point rules still apply.

== Full local-strength derivatives
Dynamic strength changes the Jacobian:
$ J_(i j) = delta_(i j)-i(lambda partial_j v_i + v_i partial_j lambda). $
Implicit differentiation of $H(x,r(x))=1$ gives
$ partial_j r = -(partial_j H)/(partial_r H), quad partial_j lambda=S partial_j r. $
Here $partial_j H$ holds $r$ fixed but differentiates all physical coefficient
dependence, including $v$ and the smooth constraints. Symbolic endpoint
reduction differentiates the complete deformed density, including this identity
and the determinant. Solver iteration choices are never differentiated. Every face retains the full-sector dimension,
structural counts and cap parameters. Omitting the rank-one term
$-i v (grad lambda)^T$ changes the integral.

#note[The polynomial construction is an independently checked baseline.
Its exact-arithmetic proof is distinct from numerical validation of a rounded
implementation. Disabling optional checks removes their runtime cost, not
solver failures or nonfinite-value diagnostics.]

#pagebreak()
== Sign-aware refinement of the dynamic bound
The baseline bounds beneficial as well as harmful higher terms. A smooth,
sign-sensitive alternative bounds terms that can oppose causality. Define the
real symmetric matrix
$
 R_k = frac((-1)^((k+1)/2) L^(k-1), k!) sqrt(W) T_k sqrt(W).
$
It need not be built with explicit square roots of $W$. Weighted trace and
squared-norm identities provide the native polynomial quantities
$
 a_k = tr(R_k)/n, quad
 b_k^2 = frac(n-1,n)(tr(R_k^2)-tr(R_k)^2/n).
$
For a symmetric matrix its largest eigenvalue is at most $a_k+sqrt(b_k^2)$.
Smooth both the spectral bound and its positive part with a fixed
dimensionless $delta=10^(-3)$:
$
 t_k=a_k+sqrt(b_k^2+delta^2), quad
 p_delta(t)=frac(t+sqrt(t^2+delta^2),2), quad mu_k=p_delta(t_k).
$
For negative $t$, evaluate the exactly equivalent expression
$ p_delta(t)=delta^2/(2(sqrt(t^2+delta^2)-t)) $
to avoid cancellation. For each even $U$ coefficient, use
$ nu_(U,k) = p_delta(-d_k L^k/U). $
With $u=r/L$, form
$ E_F(u)=sum_(k>=3, "odd") mu_k u^(k-1), quad
  E_U(u)=sum_(k>=2, "even") nu_(U,k) u^k. $
Then solve the unique positive root
$
 u^2 + frac(L^2 u^2 norm(v)^2,R^2)+E_F(u)^2+sum_U E_U(u)^2=1,
 quad lambda=S L u.
$
The nonnegative envelope coefficients retain monotonicity and smoothness.
Their derivatives, including smoothing, enter the same implicit-jet machinery.
A larger permitted displacement is a diagnostic; it does not establish lower
variance or lower time to precision. The baseline and sign-aware construction
must be compared with their full evaluation costs.

== Sampling, diagnostics and interpretation
Korobov remains an outer real map: $y=T(q)$ followed by $z=z(y)$. Its total
Jacobian is $det J_z(y) product_i T_i'(q_i)$, with each factor applied once.
No inverse Korobov derivative is introduced at an endpoint. Contour evaluation
is deterministic and consumes no production random streams.

QMC uncertainties come from complete independently randomized lattices;
democratic QMC retains covariance between sectors sharing a shift. Discrete
Havana MC retains its frozen production proposal and uses complete batches.
Both accumulate the full Laurent vector and its covariance. A partial
allocation, a missing sector or an unvisited zero is not exact evidence.

The smooth sufficient-radius bounds above are FastSecDec's parameter-space
construction. They are not attributed to pySecDec or to the LTD algorithm.
Related work includes #link("https://arxiv.org/abs/2112.09145")[Winterhalder et al.,
_Targeting Multi-Loop Integrals with Neural Networks_] on optimized contour
choices, and #link("https://arxiv.org/abs/1912.09291")[Capatti et al.,
_Numerical Loop-Tree Duality: contour deformation and subtraction_] on causal
contour construction and physical multiloop tests. Their timings are not
directly comparable FastSecDec speed measurements.

#pagebreak()
== Real-domain threshold decomposition

The no-contour route uses the verified generic cylindrical algebraic
decomposition of the causal geometry, following the developments in
#link("https://arxiv.org/abs/2506.24073")[the 2025 no-contour construction]
and #link("https://arxiv.org/abs/2603.05444")[its 2026 GCAD extension].
FastSecDec calls symGCAD through its native Rust interface and independently
replays the returned proof. The physical density is transported to each
admitted cell; no denominator sign is silently removed.

For a triangular cell $a_i(x_(<i))<x_i<b_i(x_(<i))$, the finite-interval map is
$
 x_i=a_i(x_(<i))+(b_i(x_(<i))-a_i(x_(<i)))t_i,
 quad 0<t_i<1.
$
Its Jacobian is the product of the positive widths. Projective gauge changes,
ray compactifications and endpoint charts supply their own Jacobians, each
exactly once. On a negative causal cell the continuation is
$
 (F-i 0)^(q(epsilon))=abs(F)^(q(epsilon)) e^(-i pi q(epsilon)).
$
The regulator-dependent phase must survive Laurent extraction. A sign-definite
denominator does not make a complex numerator or amplitude positive.

The resulting boundaries can be algebraic. The native representation stores
each exact defining polynomial and selected real branch, rather than demanding
a radical formula. On a certified regular branch $P(x,r(x))=0$,
$ partial_j r=-(partial_j P)/(partial_r P). $
Symbolica supplies symbolic derivatives and composed evaluator programs.
Prepared native root evaluation supplies the selected section, and the map
feeds the original numerator and smooth factors without expanding the full
substitution. Colliding roots or $partial_r P=0$ require resolved coordinates;
the regular implicit derivative cannot be extended through them by fiat.

=== Endpoint admission is a separate requirement
Integration is admitted only after the transported density has a supported
endpoint form and its required closed-face behavior has been checked. Symbolic
Taylor or IBP subtraction then produces integrable Laurent coefficients.
The symbolic endpoint policy is independent of the contour-Jacobian choice.
Generic open cells omit exceptional sets, but these cannot simply be discarded
from a distributional causal integral: the regulated partition and its analytic
continuation must retain their effect.

Current complete no-contour graph examples include the bubble, triangle, box,
massless sunrise and massive sunrise in the overview. These establish useful
sufficient classes, not the general algebraic resolver described in the
research plan. Narrow single-chamber parameter templates also exist; they do
not establish uniform support for arbitrary parametric families. The native
CLI and HEPKit interfaces use the same admission boundary. Threshold support
in WASM is outside this campaign.

Regular degree-six and degree-seven two-variable controls demonstrate that
high degree alone is not the barrier: their causal polynomials are
$y^6-y+(1+x)/16$ and $y^7+y-x$. Retained imaginary-part identities and cancelled
pole coefficients check complete integrations, while higher-degree section
tests check branch evaluation. Neither proves general collision resolution.
Their supplementary measurements remain in `docs/no-contour-results.json`;
they are not substituted into the eight-topology overview.

The difficult whole-atlas cases and the distinction between verified cells,
admitted charts and executable sectors are developed in @support-limits.
The separate critical-locus discussion in @critical-loci explains why an
open-cell proof is also insufficient for certifying the absence of a pinch.
