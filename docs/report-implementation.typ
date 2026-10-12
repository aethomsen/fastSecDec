= Implementation and performance <implementation>

The methods in @methods share the same native source, symbolic subtraction
and saved evaluator infrastructure. This chapter explains the engineering
choices behind their different preparation and sampling costs. The focused
measurements below isolate particular changes; they have their own settings
and do not replace the complete-generation or ten-minute rows in the overview.

== Preserve structure through generation and restoration

HEPKit and Linnet own graphs, models, routing and kinematics. Symbolica supplies
factored expressions, differentiation, function maps, native duals, evaluator
optimization and serialization. Numerica supplies arithmetic, matrices, RNG
primitives and Havana. FastSecDec coordinates these native objects in Rust;
the HEPKit bridge wraps them without a second algebra implementation.

Factored smooth densities and shared function bodies avoid repeatedly inserting
large Jacobians into numerator terms. For one physical D05 source sector with
236 smooth terms, native structured construction and a shared determinant
reduced the mapped record from 683.8 to 365.6 MB and sampled peak RSS from
2.178 to 1.288 GB. Mapping took 10.93 versus 7.95 seconds; a separate native
debug compilation fell from 169.49 to 58.16 seconds. Saved full-vector parity
passed at the admitted points, while saved program size increased by 6.6%.
These are representation diagnostics for one sector, not full-generation
speedups or repeated timing distributions.

*Optimization settings.* New generation defaults to ten Horner iterations and
SymJIT O2. In the studied Symbolica revision, Horner zero takes the direct
linearization route and skips subsequent Horner/common-instruction passes;
it does not disable SymJIT O2. Earlier programs are never relabelled with the
new default. Rebinding runtime strengths and caps restores the optimized
program rather than repeating symbolic construction or Horner optimization.

For no-contour maps, the same structure-preserving principle keeps root
sections and coordinate maps separate from the density they feed. Symbolic
chain rules retain map and Jacobian derivatives. Finite endpoint series supply
local coefficients, not approximations to bulk root functions. The unresolved
whole-atlas problem in @support-limits is therefore distinct from whether a
single high-degree root can be evaluated efficiently.

== Symbolic subtraction with two Jacobian implementations

Both supported comparison arms keep endpoint reduction and subtraction
symbolic. The symbolic Jacobian is differentiated and optimized with the
complete density. The contour-only dual construction substitutes surviving
first derivatives of the deformation image at final compilation using
Symbolica's degree-one dual machinery. Higher endpoint derivatives have
already been constructed symbolically. It does not use numerical-dual
endpoint jets to reduce power singularities.

The full determinant and position-dependent strength are differentiated
before this final representation choice. Coordinate seeds remain one on
subtraction faces; parameter seeds remain zero. Exact-only boundary terms
retain symbolic cancellation. Tests cover fixed and both dynamic prescriptions,
higher endpoint poles, saved-program restoration and full-vector parity.
Conditional bodies requiring hoisted image inputs remain unsupported by this
opt-in compilation route. No change in the order of deformation and
subtraction is assumed.

The following retained source-sector study is the matched evidence for this
choice. Each pair uses the same 8,192 points, symbolic endpoints, SymJIT O2,
*Horner zero* and common-pair cap 1000. Fixed uses $lambda=10^(-6)$;
polynomial dynamics uses $S=.8$, $L=10^(-6)$ and $R=1$.

#table(columns: (1.3fr, 1fr, 1fr, .8fr), inset: 5pt,
  stroke: .35pt + rgb("c7d4df"),
  table.header([*Contour / Jacobian*], [*Time / peak RSS*],
    [*Exact / JIT IR*], [*Mean sample*]),
  [Fixed / symbolic], [559.9 s / 12.01 GB], [108.3 / 156.2 kB], [14.64 µs],
  [Fixed / dual], [566.9 s / 12.01 GB], [136.8 / 202.8 kB], [19.20 µs],
  [Dynamic / symbolic], [1511.6 s / 28.44 GB], [294.0 / 524.6 kB], [37.71 µs],
  [Dynamic / dual], [794.9 s / 16.73 GB], [372.0 / 520.0 kB], [29.99 µs],
)

The timer includes generation, compilation, saving and pointwise checks;
IR sizes exclude the complete owner. Both fixed arms rescue 27 points in
double-double, both dynamic arms 28. Fresh-process restoration and complete
Laurent-vector parity pass. This sector favors symbolic J for fixed mode and
dual J for dynamic mode; dual J is not a universal optimization. Full evidence
and the clarified mathematical ordering are retained in
`docs/contour-deformation-results.json` and
`docs/reviews/contour-symbolic-endpoints.md`.

#pagebreak()
== Selective residency and native evaluator loading

Universal indexed artifacts allow a generation worker to persist one completed
sector and exit. Serial integration loads only resident records; eviction
releases the process before its replacement starts. The memory bound includes
compact coordinator/source state and the active-sector peaks, including
decoding, JIT and precision scratch. Whole-process measurements are necessary:
dropping a Rust object alone does not prove that its pages returned to the OS.
Ordinary discrete MC is the explicit residency exception in this campaign.

Records save Symbolica's optimized exact IR and native primary JIT payload.
The latter restores the lowered application and callback descriptors; the
owner still emits machine code. It is not a persisted machine-code cache.
Exact IR remains available for eager, portable and precision-rescue paths.
Immutable templates and buffers can be shared while mutable stacks and
callback environments stay worker-local.

Two preserved records gave optimized cache-miss/cache-hit load times of
0.1804/0.1437 seconds (fixed) and 0.3865/0.3025 seconds (polynomial), with a
13.34% record-size increase. These historical loader probes do not measure
current full-integral convergence. A separate 70.41 MB artifact ownership
probe constructed 50 whole-owner clones: post-clone RSS fell from 6.68 to
1.79 GB and clone time from 3.32 to 0.78 seconds, while uncached loading stayed
similar at 3.81 versus 3.71 seconds. It excludes binding, pilots and sampling.

Native owner corrections addressed callback-environment ownership
(#link("https://github.com/symbolica-dev/symbolica/pull/62")[Symbolica PR 62])
and remapping operands before common-subexpression lookup
(#link("https://github.com/symbolica-dev/symbolica/pull/63")[PR 63]).
Their public consumer revision was `74225696`. These explain the measured
implementation changes; old workspace test counts are not a current build
certificate.

== Deformation clearance and Jacobian amplification

The tuned double-box results in @benchmark-findings show why caps are runtime
parameters. A larger causal displacement is not necessarily a smoother
integrand: map distortion and derivatives of the strength also affect weights.
A supplementary mixed-mass kite study illustrates the mechanism. Its point
$p^2=3/1000$ with three massive and two massless lines is distinct from the
five-massive $p^2=16$ overview case.

At one recorded large-weight point in the polynomial construction:

#table(columns: (.5fr, .8fr, .8fr, .9fr, 1.3fr), inset: 5pt,
  stroke: .35pt + rgb("c7d4df"),
  table.header([*$L$*], [*$lambda(x)$*], [*$abs(F(z))$*],
    [*$abs(det J)$*], [*Weighted real output*]),
  [.1], [.0800], [1.087], [1.040], [.1722],
  [1], [.7989], [1.088], [5.322], [−.7322],
  [10], [6.728], [1.201], [820.8], [−108.94],
)

The causal factor moves farther from zero, but the determinant amplifies the
density. At $L=10$ the full determinant is smaller than the frozen-strength
diagnostic (821 versus 1194), so this observation does not justify dropping
the strength derivative. Two independently confirmed seeds at tuned $L=1$,
$R=.25$, $S=.8$ reduced variance by 3.318 times relative to tuned fixed
$lambda=.4$; the polynomial evaluator's extra cost left mean variance-times-time
at 1.044 times fixed. There was no demonstrated sampling-speed gain.
`docs/contour-cap-scalar-results.json` retains the identities, replayed points,
timings and full covariance. A determinant-distortion penalty is a possible
future ansatz, not an implemented result.

== Statistical ownership and focused reproduction

The caller owns scheduling and stream reservations. Pilots, production
replicas and retries retain separate work identities; duplicate or stale
returns cannot enter the estimate. Accepted complete QMC replicas or MC
batches update the full vector covariance. Cancellation drains accepted work
without turning an unfinished allocation or a missing sector into zero.
Contour evaluation consumes no production RNG stream.

For quick generation studies, `generation.source_sectors = [2, 5]` selects
original geometry-chart IDs before symmetry and subtraction. Artifacts retain
the original total and local-to-original mapping and remain explicitly partial.
Omit the option for a complete integral. The reproduction inventory in
@campaign associates each complete benchmark with its input, saved artifact,
frozen executable and independent stream identities.
