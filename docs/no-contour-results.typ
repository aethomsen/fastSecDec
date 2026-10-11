= No-contour threshold decomposition: first profiles
== Scope and generation cost
These are *complete small-input integrations* using the current
`threshold_decomposition` option, measured on 11 October 2026. They exercise
verified GCAD cells, regular algebraic boundary maps, causal phases, symbolic
endpoint subtraction and saved native evaluators. General singular-boundary
resolution remains under development. No no-contour double-box timing or
convergence result is available in this checkpoint.

The graph bubble has $s=16$, $m_1^2=m_2^2=3$, $mu^2=1$, $D=4-2epsilon$ and
native OneLOop-compatible $r_Gamma$ normalization. Both arms compute through
$epsilon^1$. The fixed-contour comparator uses the *same* graph, kinematics,
normalization and orders, with $lambda=.1$ admitted by a 16-point causal pilot;
optional production contour checks are disabled. Both arms retain tracked
numerical stability and precision rescue.

The two algebraic controls integrate $(P-i 0)^(-1-epsilon)$ over the unit square,
through $epsilon^0$. Degree seven uses $P=y^7+y-x$, with two cells and two
stochastic kernels. The sextic uses $P=y^6-y+(1+x)/16$, with three cells and
four stochastic kernels. Root boundaries retain exact defining polynomials
and selectors; radical formulae are unnecessary. These are regular-branch
controls, not tests of unresolved root collisions.

#set text(size: 8.7pt)
#include "no-contour-generation-table.typ"
#set text(size: 10.2pt)
All arms use release mode, eight physical cores of an AMD EPYC 9754, symbolic
Taylor subtraction, SymJIT O2, Horner ten, CPE iteration cap 1000 and
common-pair distance 1000. Native
seconds are the original generation ledger total. † CLI wall and sampled
aggregate RSS come from a separate identical profiling repeat. It includes
loading the input, generation and publication; native totals have narrower
scope. Saved size is the manifest plus indexed data file. Exact/JIT sizes
sum the inspected stochastic evaluator programs, excluding metadata and machine
code. Memory and size units are decimal. Each generation is measured once
with each observer; these are observations, not timing distributions.

*Native stage ledger, milliseconds.* These fields are retained as emitted;
parallel worker spans and coordinator time do not form an exclusive partition.
The current ledger does not isolate GCAD solving, independent verification,
endpoint certification or root evaluation into separate timers.
#set text(size: 8.7pt)
#include "no-contour-stages-table.typ"
#set text(size: 10.2pt)

== Evaluator cost and precision rescue
#set text(size: 8.7pt)
#include "no-contour-cost-table.typ"
#set text(size: 10.2pt)
The diagnostic uses 256 interior points and three repetitions per kernel.
† Its average is the unweighted mean of kernel means; its maximum is the
largest repetition mean, not the slowest individual point. QMC columns use
both final-budget runs: range of mean sector costs, and the largest sector
mean across either run. These operational costs include amortized preparation
and precision rescue; they are not integration wall time. All sectors receive
equal work here, so the pooled and unweighted means coincide.

The high-degree cases exercise numerical root evaluation and substantial
arbitrary-precision replay. Their checked sampling costs are currently around
one millisecond per sector point. The ledger does not identify how much is
root solving versus precision replay. Certified branch geometry and tracked
whole-vector evaluation do not amount to an interval certificate for every
sample or the integrated result. No failed or cutoff-zero evaluations occur.

#pagebreak()
== Integration time, memory and final-coefficient SD
Democratic QMC uses Kuo33002, Korobov3 and eight independent shifts. Each row
below uses 4096 points *per shift per stochastic kernel* and eight workers.
A/B are independent seed families, ending in 3701/3702 at this budget. Every
run completes its allocation. Accepted work counts actual sector-point
evaluations: six bubble threshold kernels cost six times the work of the
one-kernel contour arm at the same per-kernel lattice size.

#set text(size: 8.7pt)
#include "no-contour-integration-table.typ"
#set text(size: 10.2pt)
† All timing and RSS columns in this table come from identical profiling
repeats, with bit-identical full estimates and covariance to the original
scientific runs. Native integration time includes driver initialization,
worker setup and checkpointing. Artifact loading is separate. The causal
pilot occurs after loading and before that integration clock. CLI time spans
process launch through observed exit, including loading, pilot, result saving
and reporting. These clocks must not be added or described as pure sampling.

The faster observer targets 5 ms samples of parent plus descendant RSS,
usually observing 5--7 ms intervals. The sextic A repeat also records a
4.24-second observation gap; its longer elapsed time is retained without
adjustment. Shared pages may be counted in more than one process; sequential
reads and missed short peaks prevent interpretation as unique memory or an
exact simultaneous high-water mark. The earlier coarse observer missed brief
compiler peaks; its measurements remain in the JSON, not in these tables.

*Last requested coefficient and its uncertainty.* SD means the native
standard deviation of the *estimated mean* (standard error), derived from
complete independent QMC shifts. It is not the standard deviation of raw
integrand values. Joint SD is $sqrt(tr Sigma_k)$ for the real/imaginary
covariance block of coefficient $I_k$, not a confidence radius.
#set text(size: 7.8pt)
#include "no-contour-uncertainty-table.typ"
#set text(size: 10.2pt)
Displayed means are rounded for readability; the JSON preserves every native
digit, all Laurent coefficients and full covariance matrices. All 24 original
runs report full-integral scope, available uncertainty and the requested
accuracy stop ($10^(-12)$ absolute, $10^(-8)$ relative on the last coefficient).
This numerical stop is not an independent proof of accuracy.

*Independent controls.* The bubble pole is one and its finite coefficient is
$2-3 log(3)/2+i pi/2$. Both arms agree with this reference; their
$epsilon^1$ comparison has no independent integrated analytic oracle.
The polynomial poles cancel exactly in the retained vectors. The degree-seven
finite imaginary part is $pi r=2.5024178911883874$, where $r^7+r=1$;
the sextic reference is $3.8521160461762443$ from the endpoint root-gap
identity. No independent finite-real reference is supplied for these two
polynomial controls. Reference residuals and statistical uncertainties remain
distinct quantities.

#pagebreak()
== Convergence versus accepted work and native elapsed time
#image("no-contour-convergence.svg", width: 100%)
Each line is one seed family; markers show independent lattice sizes
1024, 2048 and 4096. Lighter dashed lines are the second family. Curves use
the *original* native integration-driver times, not the profiling repeats.
Estimates and covariance are never pooled across seeds or lattice sizes.
The work axis counts all accepted sector points. Shared seeds between the
bubble arms do not supply cross-arm covariance: a calibrated significance
for their difference is not inferred.

The fixed-contour bubble is cheaper and has smaller reported uncertainty at
these settings. Its largest-budget SD is already near floating-point roundoff;
that value must not be treated as a validated error bound or extrapolated
scaling law. The no-contour bubble reaches joint SD about $4 times 10^(-13)$,
with a larger sector count and substantial precision rescue.

The higher-degree examples have low reported uncertainty but expensive checked
sampling. Their largest-budget joint SD is about $2.4$--$2.7 times 10^(-10)$
(degree seven) and $1.2$--$1.4 times 10^(-10)$ (sextic). The improvement slows
between the final two budgets in these observations. Three small budgets and
two seeds do not establish an asymptotic rate or calibrated uncertainty coverage.
No fixed/dynamic contour comparison is measured for these polynomial controls.

== Reproduction and outstanding work
The frozen scientific source is `20bb812e31bc09a8a28a122d914a1de75e92cae7`
on `no_deformation`; the report adds no sampling implementation changes.
The optimized executable SHA-256 begins `7f41b135fdde78d8`.
Complete settings, source and artifact identities, per-sector costs,
precision diagnostics, native vectors and covariance are recorded in
`docs/no-contour-results.json`. Reproducible cards and commands are in
`examples/no_deformation/benchmarks/`; the independent measurement review
is `docs/reviews/no-deformation-benchmark-audit.md`.

This checkpoint measures ordinary generation/integration. It does not repeat
the separate normal/serial correctness matrix. Native GCAD preparation and
compiler workers are caller-owned. Generic singular endpoints, broader
parameter families and the physical ggHH comparison remain delivery work.
The double-box feasibility study will test parametric geometry first, followed
if necessary by explicitly rationalized physical invariants and masses;
verified GCAD completion and a complete regularized artifact are separate gates.
