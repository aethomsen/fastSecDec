= Campaign design and reading guide <campaign>

The overview contains *66 measured sampler rows across eight topologies*,
plus two unavailable massive-kite no-contour slots. It compares complete
integrals through $I_0$, retaining lower Laurent orders and covariance.
A dash is missing evidence, never zero. No-contour double-box integration
was removed from scope; its partial preparation appears in @support-limits.

== What the columns measure

*Generation* is wall time from launching generation to durable artifact
publication, excluding the software build. *Artifacts* counts the manifest
and referenced indexed data once. *Sectors* counts executable stochastic
kernels; geometry cells, endpoint charts and exact contributions are distinct
objects. This distinction is essential when reading partial GCAD results.

*Generation RAM* and *integration RAM* are sampled aggregate resident memory
of the owned process tree, in decimal GB. Shared mappings can be counted more
than once and short peaks can be missed. These measurements are neither
unique memory nor an exact high-water mark.

*f64 max / avg* is the maximum and arithmetic mean of per-sector mean raw
kernel costs, not the slowest individual sample. *Σ f64 / checked* sums those
means for one sweep over all sectors; the checked cost retains ordinary
stability handling and precision rescue. It is not a parallel wall-time
prediction. A sparse profile need not reproduce a long integration's rescue
frequency; profiling settings are recorded below.

*Sector calls* counts logical sector-integrand evaluations during the timed
invocation, including reported pilot and discarded work. Replaying a point
at higher precision is a backend call, not an additional sampling point.
Unprefixed totals require complete native deadline/drain accounting; $gt.eq$
marks an older observed prefix. Accepted statistical work is recorded
separately and exactly.

The cancellation diagnostic is
$
 C_("abs") =
 frac(sum_s integral abs(w_s (u)) dif mu_s (u)+abs(I_("exact")), abs(I_0)),
$
where $w_s$ is the final complex finite-coefficient weight, including its
continuous importance and coordinate factors. The displayed column *C*
uses committed production estimates of these absolute-weight integrals.
It is not the sum of absolute values of the sector integrals. The combined
exact offset enters once. Discrete MC accounts for sector-selection
probabilities in this integral estimate. *max / |I|* instead uses the largest
observed sector-local complex weight: it removes the discrete selection
factor while keeping continuous importance corrections. This
proposal-dependent sample maximum is not a supremum or a tail bound.

== Uncertainty and display normalization

For $I_0=a+i b$ and $M=abs(I_0)$, the final column reports the relative
standard error of the complex *magnitude*. With covariance $Sigma$ of the
estimated real and imaginary means,
$
 sigma_M^2 = frac(a^2 Sigma_(R R)+2a b Sigma_(R I)+b^2 Sigma_(I I), M^2),
 quad "relative SE"=sigma_M/M.
$
It is undefined at $M=0$. A dagger warns that the joint complex standard
error $sqrt(tr Sigma)/M$ is at least $0.1$, where first-order norm propagation
may be unreliable. This is a warning convention, not a certified boundary.
Radial uncertainty does not measure phase accuracy. Full complex covariance
and joint SE remain the basis of compatibility checks; discrete-MC covariance
is the native global-batch covariance, not a sum of sector marginals.

The two relative-error columns apply the requested *MC-equivalent display
heuristic*: multiply the measured relative SE by $sqrt(W/W_("target"))$
for accepted work, or by $sqrt(t/600)$ for native elapsed seconds. These are
not measured QMC convergence laws and never alter estimates or covariance.
Average work is total accepted calls divided by stochastic sector count,
not equal allocation; targets appear in topology headers. Large corrections
require caution. The last, near-ten-minute column is the primary metric.

#pagebreak()
== Execution, clocks and numerical policy

Each accepted timed run lasts 480–750 native seconds and comparisons within a
topology require a longest/shortest ratio at most $1.25$. Instrumented runs
stop new reservations at 600 seconds, drain complete in-flight work and save
the native result. A budget stop is not an accuracy stop. External kills do
not qualify. Earlier graceful-cancellation observations retain their prefix
qualification rather than being relabelled exact totals.

The integration clock starts after ordinary coordinator loading. Serial
resident loading and later reloads occur inside that clock. Process wall,
loading, drain and publication receipts are kept separately; their clocks
must not be added as though they were disjoint. Integration RAM comes from
the timed arm. Independent throughput calibrations select work allocations;
confirmation streams are disjoint from calibration and other arms.
Different designs, lattice epochs, seeds and covariance matrices are never
pooled to construct the displayed observations.

The secondary work display may retain a clean capped MC estimate whose
planned allocation is incomplete, provided committed production batches give
full coverage and available covariance under one fixed design. It uses the
actual count and remains explicitly incomplete requested work. A completed
continuation replaces its overlapping prefix. Some newer extensions use the
timed run itself for the secondary display; their receipts identify that
choice. No such correction creates a matched-work experiment.

Generation uses serial sector residency with 25 workers, except for the
physical-numerator double box's six-worker memory allowance. Integration
uses 25 workers. QMC has serial residency with a one-second minimum stay;
MC uses the existing *ordinary discrete-MC scheduler*, the explicit
exception. The owned campaign cohort has a 100 GB memory allowance. Runs
may overlap; no quiet-machine condition is imposed. Preparation time limits
are not integration deadlines or completed-generation measurements.

*Optional causal validation is off in every overview timing*, including
profiling and throughput calibration. The collector rejects a causal pilot
or nonzero production causal-check count. Mandatory root termination,
nonfinite handling, stability checks and precision rescue remain active.
Separate finite causal pilots support admission of the retained double-box
settings, but are neither timed here nor global certificates. New generation
uses Horner ten and SymJIT O2. Historical Horner-zero studies elsewhere in
the repository are not substituted for this campaign.

== Topologies and runtime settings

#table(columns: (1.1fr, 2.55fr), inset: 5pt, stroke: .35pt + rgb("c7d4df"),
 table.header([*Input*], [*Definition*]),
 [Bubble], [$B_0(16;3,3)$, $mu^2=1$; arguments after the semicolon are squared masses.],
 [Triangle], [$C_0(0,0,5;1,1,1)$, $mu^2=1$.],
 [Box], [$D_0(0,0,0,0,5,-1;1,1,1,1)$, $mu^2=1$.],
 [Massless sunrise], [$p^2=1$, three massless lines; a branch/endpoint control without an interior threshold zero.],
 [Massive sunrise], [$p^2=16$, three masses equal to one, native measure one.],
 [Massive kite], [$p^2=16$, all five masses equal to one. The earlier mixed-mass small-momentum kite is a different input.],
 [D05 double boxes], [$s=10^6$, $t=-97077$, $u=-871673$, $m_H=125$, $m_t=345/2$. Scalar numerator one; physical input uses the original contracted numerator.],
)

All inputs use $D=4-2epsilon$. Native cards retain exact masses, invariants,
normalization, graphs and symbolic endpoint subtraction. The physical D05
case is an individual s-channel contribution with incoming $++$ helicities,
unnormalized external color contraction $delta_(a b)$ and loop measure
$product_l dif^D k_l/(i pi^(D/2))$, with multiplier one. It is not a complete
diagram sum or a spin/color-averaged matrix element. HEPKit supplies the graph,
contracted numerator, wavefunctions and kinematics.

Fixed strength is $lambda=0.1$ for the smaller scalar cases and $10^(-6)$ for
D05. The D05 trial $0.1$ failed a separate causal admission check and is not
a benchmark result. Dynamic baselines use $S=0.8$, $L=R=1$. Tuned D05 rows use
$S=0.8$, $R=10^6$, with $L=1.25 times 10^(-5)$ for scalar QMC and
$1.25 times 10^(-6)$ for scalar MC and both physical samplers. The settings
are frozen before confirmation; their interpretation follows in the results
chapter. Binding these runtime caps does not regenerate an artifact.

#pagebreak()
== Reproduction and evidence boundaries

The current measurement source is `docs/benchmark-overview-results.json`.
It preserves invocation arguments, binary and artifact identities, complete
Laurent vectors, covariance, work ledgers, diagnostics and data-source
associations. The exact collection contract and commands are in
`docs/benchmark-overview-protocol.md`; native input cards are in
`examples/no_deformation/benchmarks/overview/` and the explicitly associated
control directories. The generation/profile receipts distinguish symbolic
and contour-only dual Jacobians while keeping endpoint subtraction symbolic.
Candidate selection precedes confirmation and uses measured full-sweep cost.

The native profiler uses deterministic coordinates, three repetitions and
zero warmup: 4096 coordinates for the original smaller scalars, 1024 for the
double boxes and later bubble/massive-sunrise measurements. Actual kernel
calls and coordinate counts are recorded separately.

The baseline campaign pins symGCAD
`644a560e69a064f811fa93aff33a247ea7fcdfca`. Original instrumented timings use
executable identity `cf9eb450`; the admitted triangle, massless-sunrise and box
routes use `35c92289`, `3cf81e4b` and `d94dd90d`, respectively. Massive-sunrise
and final double-box tuning receipts identify frozen source `6647950`.
These are separate recorded builds, not a claim that all rows came from one
binary. Later algorithmic discussion must not retroactively attribute changes
to those measurements.

#table(columns: (1.15fr, 2.4fr), inset: 5pt, stroke: .35pt + rgb("c7d4df"),
 table.header([*Evidence*], [*Compact record in `docs/`*]),
 [Main comparison], [`benchmark-overview-results.json`; `benchmark-overview-protocol.md`],
 [Independent references], [`benchmark-compatibility-references.json`; `benchmark-compatibility-results.json`],
 [Massive sunrise], [`no-contour-massive-sunrise-overview.json`; `massive-sunrise-contour-overview.json`],
 [Sign-aware confirmations], [`bubble-sign-aware-overview.json`; `scalar-double-box-sign-aware-overview.json`; `physical-double-box-sign-aware-overview.json`],
 [Runtime-cap tuning], [`double-box-dynamic-tuning-overview.json`; `double-box-dynamic-work-overview.json`],
 [Scope and acceptance], [`reviews/benchmark-final-scope.md`; `reviews/no-contour-final-scope.md`],
)

Historical five-minute/50-core runs, Horner-zero studies, early refusal
receipts and one-worker correctness probes remain available in their compact
source records. They are not additional rows of the current comparison and
are not used to fill missing measurements. Older reports of unsupported
triangle/box routes are superseded by complete artifacts and fresh
confirmations; the massive-kite interval/whole-atlas gap remains real.

The campaign is complete under the final narrowed scope. This does not mean
that the general algebraic resolver is complete, that fixed deformation is
globally certified by finite pilots, or that a small stochastic SE certifies
floating-point accuracy. The benchmark findings distinguish these limits
from measured efficiency and reference agreement.
