// Rebuild: nix-shell -p typst --run 'typst compile docs/contour-deformation.typ docs/contour-deformation.pdf'
// Measurements are accompanied by native full-covariance summaries and audit records.
#set document(title: "Threshold regularisation in FastSecDec", author: "FastSecDec collaboration")
#set page(paper: "a4", margin: (x: 19mm, top: 18mm, bottom: 19mm),
  footer: context [#text(size: 8pt, fill: rgb("64748b"))[FastSecDec · threshold benchmarks] #h(1fr) #counter(page).display()])
#set text(font: "Libertinus Serif", size: 10.2pt)
#set par(justify: true, leading: 0.54em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): set text(size: 17pt, fill: rgb("173e59"))
#show heading.where(level: 2): set text(size: 11.5pt, fill: rgb("173e59"))
#show table: set par(justify: false)
#set math.equation(numbering: none)
#let note(body) = block(width: 100%, inset: 8pt, fill: rgb("edf4f7"), radius: 3pt, body)
#let pending = text(fill: rgb("9b4a16"))[Pending measurement]
#let diag = math.op("diag")
#let tr = math.op("tr")
#let grad = math.op("grad")
#let Re = math.op("Re")
#let Im = math.op("Im")

#include "benchmark-overview.typ"
#pagebreak()
#include "benchmark-overview-details.typ"
#pagebreak()
#include "benchmark-compatibility.typ"
#pagebreak()

#text(size: 24pt, weight: "bold", fill: rgb("173e59"))[Threshold regularisation]

#text(size: 13pt)[FastSecDec · contour deformation and threshold decomposition]
#v(3mm)
#text(size: 9pt, fill: rgb("64748b"))[Benchmark checkpoint · 11 October 2026]

= Deformation styles and formulae
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
nonfinite handling remain active. The new overview's sampling and convergence
timings explicitly use *off*; separate validation probes are excluded.
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
The baseline bounds beneficial as well as harmful higher terms. A smoother,
tighter alternative bounds only terms that can oppose causality. Define the
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
= Physical $g g -> h h$: the 1000 GeV double box
The input is the existing individual *D05* s-channel double-box contribution,
with incoming $++$ helicities, $sqrt(s)=1000$ GeV, $cos(theta)=4/5$,
$m_H=125$ GeV and $m_t=y_m=172.5$ GeV. HEPKit supplies the native graph,
contracted numerator, wavefunctions and kinematics. The colour contraction is
unnormalized $delta_(a b)$; the measure is
$product_l d^D k_l/(i pi^(D/2))$, $D=4-2epsilon$, with multiplier one.
This is not a complete diagram sum or a spin/colour averaged matrix element.

#note[Both prescriptions use symbolic endpoint IBP. Fixed mode uses the symbolic
Jacobian; dynamic mode uses contour-only dual evaluation. Both complete
artifacts contain 30 sampled sectors and 30 exact records.]

== Complete generation
#table(columns: (1fr, .65fr, .85fr, .9fr, 1fr, 1.1fr), inset: 4pt, stroke: 0.4pt + rgb("cbd5e1"),
  table.header([*Contour*], [*Workers*], [*Minutes*], [*Peak GB*], [*Saved MB*], [*Exact / JIT MB*]),
  [Fixed], [6], [43.57], [54.09], [70.41], [3.20 / 4.66],
  [Dynamic], [2 → 4], [106.81], [60.94], [51.34], [11.33 / 15.98],
)
Both use serial generation, SymJIT O2, Horner zero and common-pair cap 1000.
Dynamic time includes the initial two-worker attempt and its four-worker
continuation; the 26.1-second restart gap is excluded. Initial native series
widths are one and two, respectively. These are complete-campaign observations,
with different worker counts; the matched Jacobian comparison is in section 4.
GB and MB are decimal; RSS includes the owned parent and child processes.

*No-contour status:* the double-box threshold-decomposition comparison is not
available yet. The new no-contour profiles below cover complete small inputs.
The extended exact-point GCAD trial in section 3 now completes with independent
verification; the physical density and endpoint-resolution bridge remains open.

== Five minutes on 50 physical cores
#table(columns: (1.05fr, .9fr, 1.15fr, 1fr, 1fr), inset: 5pt, stroke: 0.4pt + rgb("cbd5e1"),
  table.header([*Method*], [*Contour*], [*Finite coefficient*], [*Joint variance*], [*Relative SE*]),
  [QMC], [Fixed], [$68.35+26.24 i$], [2.46049], [2.142%],
  [QMC], [Dynamic], [$69.45+24.11 i$], [7.8327], [3.807%],
  [Discrete MC], [Fixed], [$69.07+24.93 i$], [0.256372], [0.690%],
  [Discrete MC], [Dynamic], [$68.45+24.65 i$], [0.500102], [0.972%],
)
For the finite complex estimate $hat I_0$, the joint estimator variance is
$V=tr Sigma_0$ and relative joint standard error is $sqrt(V)/abs(hat I_0)$.
This is not pointwise integrand variance. The reproducibility JSON retains
both diagonal covariance entries and the off-diagonal entry, all Laurent
components, exact offsets, work counts and precision-rescue diagnostics.

#table(columns: (1.05fr, .9fr, .85fr, .9fr, 1.3fr), inset: 4pt, stroke: 0.4pt + rgb("cbd5e1"),
  table.header([*Method*], [*Contour*], [*Seconds*], [*Peak GB*], [*Sample µs avg / max*]),
  [QMC], [Fixed], [302.32], [1.94], [3.73 / 4.02],
  [QMC], [Dynamic], [302.37], [2.02], [16.07 / 28.97],
  [Discrete MC], [Fixed], [303.53], [4.13], [23.99 / 43.54],
  [Discrete MC], [Dynamic], [302.27], [11.31], [71.79 / 164.48],
)
Sample costs are the average and maximum of per-sector mean integrand costs,
including precision rescue and amortized setup/adaptation work; they are not
maximum single-point latency. A separate fixed all-sector probe measured
10.44 µs average and 14.06 µs maximum sector mean on a smaller fixed point set.

Fixed uses $lambda=10^(-6)$; dynamic uses $S=.8$, $L=10^(-6)$, $R=1$,
selected with separate tuning seeds. Dynamic/fixed variance ratios are 3.18
(QMC) and 1.95 (MC): fixed performs better in these runs. Each row is one
independent production run; these ratios have no repeated-run confidence interval.
The 300-second native integration budget includes worker setup and Havana
adaptation. Loading, binding and causal Pilot16 precede that clock; optional
production causal checks are disabled. Complete accepted work is drained after
cancellation, explaining the modest overshoot. All rows retain full covariance
and independently reserved production streams; incomplete work is not added.
No independent numerical reference exists for this point. These frozen
campaign executables precede the loader-cache update in section 4.

#pagebreak()
#include "contour-scalar-results.typ"

#pagebreak()
#include "contour-cap-results.typ"

#pagebreak()
#include "contour-cap-double-box.typ"

#pagebreak()
#include "no-contour-results.typ"

#pagebreak()
#include "no-contour-gghh-feasibility.typ"

#pagebreak()
#include "no-contour-gghh-verified.typ"

#pagebreak()
#include "no-contour-gghh-density.typ"

#pagebreak()
= Implementation and performance
== Preserve algebraic structure and reuse native facilities
FastSecDec owns the sector/integration interfaces in Rust. Symbolica supplies
factored atoms, differentiation, function maps, evaluator optimization, duals,
and serialization; Numerica supplies arithmetic, matrices, RNG primitives and
Havana. HEPKit and Linnet retain graph, model and kinematic ownership.
HEPKit's Python bridge wraps native objects rather than duplicating algorithms.

The smooth density is deformed before subtraction. Native function definitions
can share repeated Jacobian and envelope bodies while preserving their
derivatives and explicit radius dependencies. The saved program restores the
optimized evaluator; binding a strength or cap must not rerun Horner/CPE.
SymJIT O2 remains the native backend default; portable exact evaluator IR is
retained separately for eager and high-precision execution.

*Horner defaults.* New generation uses ten Horner iterations in the native,
CLI and Python settings. The earlier double-box and scalar tables retain their
explicit historical zero setting. In the pinned Symbolica implementation,
zero takes the direct linearization path and skips the subsequent Horner and
common-instruction optimization passes; it does not disable SymJIT O2.
Contour-only dual composition performs additional native common-expression
optimization separately. No historical program is relabelled as Horner ten.

An actual D05 source sector contains 236 smooth terms. The initial representation
copied its large Jacobian into each term. Native structured construction plus
a shared determinant body reduced that sector's mapped record from 683.8 MB
to 365.6 MB and sampled peak RSS from 2.178 GB to 1.288 GB. Mapping wall time
was 10.93 s versus 7.95 s on a shared host. In the native debug graph, compilation
fell from 169.49 s to 58.16 s. Saved full-vector parity passes at two admitted
points; both representations reject the same excessive strength. Saved program
size increases by 6.6%. These are one-sector measurements, not full-generation
timings or a convergence result.

== Symbolic versus dual determinant
The symbolic route constructs a native determinant and optimizes it together
with the density. It offers common-expression elimination across outputs but
may make generation expensive when a large body is repeatedly substituted.
The first optional dual route also applied numerical-dual subtraction to the
whole density. It coupled the two choices, contrary to the clarified requirement.
Its historical measurements include high-order endpoint jets and cannot establish
the cost or limits of the corrected contour-only construction.

*Independent contour-only construction.* Endpoint reduction and subtraction
remain symbolic in both variants. Native differentiation retains every bulk,
boundary, determinant and local-strength derivative. At final compilation,
native exact-subtree replacement supplies surviving first-image derivatives
from a degree-one image-only Dualizer; higher endpoint derivatives are already
symbolic. Coordinate seeds remain one on subtraction faces, and parameter seeds
remain zero. Native pruning removes unused inputs before composition. Exact-only
boundary contributions retain symbolic evaluation and cancellation.

Five focused compiler controls pass, including third-order endpoint poles with
fixed and both dynamic prescriptions, saved programs, unused callbacks and
private-input collisions. Public Symbolic IBP controls also pass with both
implicit-radius prescriptions, complete complex Laurent vectors and restored
programs. Conditional bodies requiring hoisted image inputs are explicitly
unsupported by this opt-in route. The matched source-sector comparison below
includes generation, RSS, program sizes and sampling cost. Full-integral
generation and all four physical integration runs have completed.
No mathematical reordering, alternative CAS or AD implementation is introduced.
An optional initial native Series width avoids a known discarded attempt; the
strict absolute Laurent-remainder coverage check and checked retries remain
unchanged. It defaults to one. Twenty-three focused controls and independent
native-reuse reviews pass. The measurements above distinguish generation savings
from the cost of sampling the resulting evaluator.

== Matched Jacobian measurements on one source sector
#table(columns: (1.3fr, 1fr, 1fr, .85fr), inset: 4pt, stroke: 0.4pt + rgb("cbd5e1"),
  table.header([*Contour / Jacobian*], [*Time / peak RSS*], [*Exact / JIT IR*], [*Mean sample*]),
  [Fixed / symbolic], [559.9 s / 12.01 GB], [108.3 / 156.2 kB], [14.64 µs],
  [Fixed / dual], [566.9 s / 12.01 GB], [136.8 / 202.8 kB], [19.20 µs],
  [Dynamic / symbolic], [1511.6 s / 28.44 GB], [294.0 / 524.6 kB], [37.71 µs],
  [Dynamic / dual], [794.9 s / 16.73 GB], [372.0 / 520.0 kB], [29.99 µs],
)
Times include generation, compilation, saving and pointwise checks in separate
processes; sizes describe evaluator IR, not the complete saved owner. Settings
are SymJIT O2, Horner zero and common-pair cap 1000. Each matched pair uses
8,192 identical points: fixed $lambda=10^(-6)$ and dynamic $S=.8$, $L=10^(-6)$,
$R=1$. Both fixed arms rescue 27 points in double-double; both dynamic arms
rescue 28. Fresh-process restoration and complete-vector parity pass.
This sector favors symbolic J for fixed mode and dual J for dynamic mode.

== Residency, callback ownership and statistical work
Universal indexed artifacts let serial generation publish one completed
sector and release its worker process. Selective loading retains optimized
native programs. Ordinary discrete MC uses caller-owned parallel execution;
the 50-core campaign must account for all resident evaluator contexts.
Aggregate parent-and-child RSS measures actual residency, including JIT scratch.

Independent probes exposed a native JIT-clone callback issue: cloned evaluator
tables were not used by machine code, which retained the original callback
environment and serialized its mutable workspace. The narrow owner correction,
#link("https://github.com/symbolica-dev/symbolica/pull/62")[Symbolica PR 62], is
retained in public consumer revision `74225696`. The same consumer adds
#link("https://github.com/symbolica-dev/symbolica/pull/63")[Symbolica PR 63]:
common-subexpression lookup remaps operands before hashing them, avoiding one
optimizer pass per level of a duplicate dependency chain. Native branch
ancestry and callback argument order remain unchanged. The preceding published
workspace gate passed 935 tests (34 explicit ignores, including a child invoked
separately), and its portable suite passed 83 tests. The source-selection
follow-up passes 426 core tests (20 existing ignores), plus focused artifact,
CLI process, portable selection and saved-result controls. Strict workspace
Clippy, formatting and thin-binding checks pass. These are native/portable-host
gates, not installed Python-wheel or browser/WASM execution evidence.

The physical QMC run also exposed avoidable coordinator work: matching common
shifts used linear searches within already-sorted native replica records.
Binary search preserves the selected rows and arithmetic order. Cancellation
now drains dispatched results without repeating expensive observational
reductions and checkpoint writes for every return; the final native checkpoint
and report still include accepted returns. Deterministic cancellation and
reordered, gapped-shift covariance/restoration controls cover these changes.

== Persist the native primary evaluator
The old artifact saved exact evaluator IR but discarded the compiled primary.
The new record also saves Symbolica's native JIT payload. Compatible loads
restore its lowered application and callback descriptors; the owner still emits
machine code. Exact IR remains available for precision rescue and portable
execution. Existing mathematical identities and caller-owned scheduling remain
unchanged. Immutable exact templates and artifact buffers are shared across
workers; mutable stacks and callback environments remain independent.

On two preserved source-sector records, matched optimized cache-miss/cache-hit
loads measured 0.1804/0.1437 s (fixed) and 0.3865/0.3025 s (polynomial).
The optional cache adds 13.34% to those records. These are loader measurements
on historical endpoint constructions, not new physical integration results.
Changing dynamic validation factories also reuses the native primary instead
of rebuilding it from exact IR. Optional byte-integrity checks remain optional;
format, compatibility and native decoding checks are retained.

A second probe restores the same complete 70.41 MB fixed artifact in the old
and new optimized implementations, then constructs 50 whole-owner worker
clones. Post-clone RSS falls from 6.68 to 1.79 GB, and clone time from 3.32 to
0.78 s. Loading the original uncached artifact remains similar, 3.81 versus
3.71 s. This measures immutable ownership improvements without binding, pilot
or sampling work; it is separate from the cache-hit comparison above.

Stream identities are coordinator-owned. Pilots, production replicas and
retries keep separate identities; rejected, duplicate or stale work cannot
enter the estimate. Optional causal checks are truly absent in unchecked
production, while solver/nonfinite failures remain visible. Accepted common
QMC shifts and complete MC batches update the full covariance, preserving
sector cancellations even at a time-budget stop.

#v(2mm)
#text(size: 9pt)[*Reproduction and source:* #link("https://github.com/alphal00p/fastSecDec/tree/contour_deformation")[FastSecDec, contour_deformation].
Native input: `examples/contour/gghh_double_box_1000/`. Mathematical audit:
`docs/reviews/contour-foundation.md`. Physical campaign provenance and complete covariance are in
`docs/contour-deformation-results.json`. The scalar protocol, complete vectors,
covariance, work identities and measurements are in `docs/contour-scalar-results.json`
and `docs/reviews/contour-scalar-benchmark-audit.md`. The maintained Rust driver
and reproduction commands are in `examples/contour/scalar_benchmarks/`.]
