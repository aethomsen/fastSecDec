// Definitions follow the opening tables so the overview remains uncluttered.
= Overview: scope and reproduction

#let overview-data = json("benchmark-overview-results.json")
#block(width: 100%, inset: 8pt, fill: rgb("edf4f7"), radius: 3pt)[
  *Campaign in progress.* A dash is an unavailable or pending measurement,
  never a zero. The opening tables use the new finite-coefficient campaign only.
  Historical studies below retain their original points, settings and resources;
  they are not substituted for missing new measurements.
]

All uncertainties target $epsilon^0$ of the complete complex integral. Lower
Laurent coefficients and their full covariance are retained. The reported joint
standard error is $sqrt(C_(R R)+C_(I I))$, where $C$ is the covariance of the
estimated mean, not that of individual sample weights. Relative SE divides
this quantity by $abs(I_0)$ and is undefined at a zero mean. It is an estimated
precision, not an independently established error against an exact result.
The discrete-MC total uses its native full-batch covariance; summing sector
marginal variances would lose its cross-sector correlations.

The bubble QMC estimates reach the scale of floating-point rounding. Their
very small reported replica SE does not include common numerical bias or imply
accuracy beyond f64. Its independent finite reference is
$2-3 log(3)/2+i pi/2$; reference discrepancies are kept separate from sampling SE.

== Clocks, work and memory
*Generation* is launch-to-durable-publication wall time, excluding the software
build. Artifact size counts the manifest and its indexed data exactly once.
Sector count means executable stochastic kernels; preparation charts and exact
contributions have separate native records. Generation and integration RAM are
sampled aggregate resident memory of each owned process tree, in decimal GB.
Shared mappings can be counted more than once and sampling can miss peaks;
these numbers are not a measurement of unique physical pages.

*f64 max / avg* means the maximum and arithmetic mean of per-sector mean kernel
costs, not the slowest individual point. *Σ f64 / checked* sums those means for
one complete sweep and separately sums the per-sector measured wall costs with
normal stability checks and precision rescue. It is not a 25-core wall-time
prediction. The native profiling harness uses deterministic coordinates (4096
for the smaller scalar cases, 1024 for the scalar D05), three repetitions and
zero warmup; call counts and coordinate counts are retained
separately. Loading and evaluator restoration have their own receipts.

*Causal validation is off throughout every new table timing*: profiling,
throughput calibration, matched-work integration and the 600-second arm all
request this explicitly. No causal pilot is included. The collector rejects
contour receipts with a different policy or a nonzero causal-check count.
Mandatory solver termination, nonfinite handling and precision rescue remain
active. Earlier checked-policy measurements are separate diagnostic evidence.

Independent throughput pilots choose a common total sector-point budget per
topology for the matched-work arm. Complete QMC lattices and MC batches define
the attainable granularity. Native accepted counts, rather than requested
counts or extrapolation, determine comparability. Each confirmation has a seed
disjoint from calibration and the other arm. The fixed-time arm requests
600 seconds on the native elapsed clock, then uses graceful cancellation to
save the last committed estimate; actual elapsed time, drain and cancelled
allocations are recorded separately. An external kill without a native result
does not supply an error estimate. The integration-RAM column uses this arm.

== Execution and topology definitions
Generation uses serial sector residency with 25 workers for the scalar suite.
The physical-numerator double box uses six generation workers to control its
larger per-sector memory; this is the explicit generation exception. Integration
uses 25 workers for both samplers. QMC uses serial residency with a one-second
minimum stay. MC denotes the existing *ordinary discrete-MC scheduler*, the
user-authorized exception to serial integration. The shared owned campaign
cohort observes a 100 GB allowance. Campaigns may overlap; no quiet-machine assumption is made.

The input cards in `examples/no_deformation/benchmarks/overview/` specify the
native HEPKit graphs, exact masses/invariants, normalization, symbolic endpoint
subtraction and order zero. The massive kite has all five masses equal to one
and $p^2=16$; the historical mixed-mass small-momentum kite is a different case.
The massless sunrise has no interior threshold zero and is retained as a
non-crossing control. The scalar and physical-numerator D05 double boxes share
$s=10^6$, $t=-97077$, $u=-871673$, $m_H=125$, $m_t=345/2$. The scalar numerator
is one; the physical amplitude retains its original numerator and prefactors.

The smaller scalar cases use fixed $lambda=0.1$. The D05 comparison uses
$lambda=10^(-6)$; its trial value 0.1 failed a separate causal admission check
and contributes no accepted benchmark result. Dynamic runs use $S=0.8$, $L=1$
and $R=1$, with polynomial and sign-aware constructions labelled separately.
The D05 fixed and polynomial settings passed separate finite pilots; those
checks supply sampled safeguards, not a global certificate, and their clocks
are excluded from every overview timing.

Symbolic and contour-only dual Jacobians are compared in preparation while
endpoint reduction remains symbolic. The selected candidate minimizes the
measured full-sweep cost before any confirmation result is available. Full
commands, artifact identities, evaluator sizes, component layout and binary
digests are retained in `docs/benchmark-overview-results.json`; the protocol and
input inventory describe reproduction. These measured binaries use the pinned
symGCAD revision `644a560e69a064f811fa93aff33a247ea7fcdfca`.

== Current admission limits
The bubble has a complete no-contour integration route. Native CLI preparation
currently refuses the triangle, box, sunrise and kite because their affine
projective domains require a complete multidimensional cell-to-endpoint atlas
beyond the admitted native-cube route. Verified double-box geometry and open
maps likewise do not yet provide the complete endpoint continuation needed for
a full no-contour integral. Those rows remain unavailable; their generation
time is not replaced by GCAD-only time. The detailed preparation studies below
report this partial progress separately.

#set text(size: 8pt)
#table(
  columns: (1.1fr, 1fr, 3fr), inset: 4pt,
  stroke: .35pt + rgb("c7d4df"),
  table.header([*Topology*], [*Prescription*], [*Measurement status*]),
  ..overview-data.topologies.map(t => t.methods.map(m =>
    (t.label, m.label, m.status_detail)).flatten()).flatten(),
)
