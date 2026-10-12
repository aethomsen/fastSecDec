// Definitions follow the opening tables so the overview remains uncluttered.
= Overview: scope and reproduction

#let overview-data = json("benchmark-overview-results.json")
#block(width: 100%, inset: 8pt, fill: rgb("edf4f7"), radius: 3pt)[
  *Final campaign scope.* The overview contains 66 measured timed sampler rows
  across eight topologies, retaining the original 44 arms and separately
  identified extensions. The user removed double-box no-contour development
  from scope. Kite no-contour remains not completed; its boundary milestone and
  substantial remaining work are documented below.
  A dash is an unavailable measurement,
  never a zero. The opening tables use the new finite-coefficient campaign only.
  Historical studies below retain their original points, settings and resources;
  they are not substituted for missing new measurements.
]

All uncertainties target $epsilon^0$ of the complete complex integral. Lower
Laurent coefficients and their full covariance are retained. The primary final
column is the relative standard error of the *magnitude* $M=abs(I_0)$.
Writing $I_0=a+i b$, first-order propagation gives
$ sigma_M^2 = (a^2 C_(R R)+2 a b C_(R I)+b^2 C_(I I))/M^2. $
The relative error is $sigma_M/M$. Zero magnitude makes this undefined;
large uncertainty relative to the magnitude receives a linearization warning.
The dagger marks joint complex SE divided by magnitude at least 0.1; this is a
warning convention, not a certified validity boundary.
This radial error measures the modulus, not accuracy in its complex phase.
The joint complex SE $sqrt(C_(R R)+C_(I I))$ remains in the detailed data and
scientific compatibility checks. Here $C$ is the covariance of the estimated
mean, not of individual sample weights. Neither SE is an independently
established error against an exact result.
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
for the original smaller scalar cases, 1024 for both D05 variants and the new
massive-sunrise/bubble extensions), three repetitions and
zero warmup; call counts and coordinate counts are retained
separately. Loading and evaluator restoration have their own receipts.
The integration elapsed clock starts after ordinary coordinator kernel loading;
serial resident loading and subsequent reloading occur inside its clock.
Process-wall and loading times are retained separately so these startup costs
are not confused with the reported sampling interval.

*Causal validation is off throughout every new table timing*: profiling,
throughput calibration, matched-work integration and the 600-second arm all
request this explicitly. No causal pilot is included. The collector rejects
contour receipts with a different policy or a nonzero causal-check count.
Mandatory solver termination, nonfinite handling and precision rescue remain
active. Earlier checked-policy measurements are separate diagnostic evidence.

Independent throughput pilots choose the work allocations and the round
average-per-sector targets listed in the data. Discrete MC samples sectors
unequally; average work is the accepted total divided by the number of stochastic
sectors, not a claim that each sector has the same allocation. The secondary
display rescales the radial relative SE by $sqrt(N_("actual")/N_("target"))$.
The primary time display rescales it by $sqrt(t_("actual")/600)$. Both corrections
are *Monte-Carlo-equivalent heuristics*, not measured QMC convergence laws.
Actual errors, counts, factors and times are retained unchanged. A topology's
time comparison is withheld if its longest/shortest accepted run exceeds 1.25.
Each accepted timed run must also lie between 480 and 750 seconds.
Each confirmation has a seed disjoint from calibration and the other arm.
The instrumented fixed-time arm stops new reservations at 600 native seconds
and drains complete in-flight tasks or batches before saving. Its exact call
count requires a clean native drain. Earlier confirmations used graceful
cancellation and retain their observed-prefix counts. Actual elapsed time,
drain and process-wall time remain separate. An externally interrupted run
does not qualify for the new timed table. The integration-RAM column uses this arm.

The secondary work display may use a clean capped estimate with an unfinished
planned allocation, provided its native full-coverage covariance comes from
committed complete batches under the same fixed design. Its strict matched-work
status remains incomplete; the actual count and square-root correction are
retained explicitly. A completed continuation is used instead of its partial
prefix, without pooling the two estimates. For the new triangle no-contour
secondary arm, QMC completed all 241,606,656 planned evaluations in 402.311
seconds; MC saved 40,550,400 at its clean 900-second deadline (904.709 seconds
including drain). That MC allocation remains incomplete. Neither secondary
time is described as a 600-second measurement.
The new sunrise no-contour secondary arms both completed their pilot-selected
26,566,656 evaluations, in 59.851 seconds for QMC and 122.960 seconds for MC.
Their correction to the same round work target is explicitly heuristic; the
separate timed confirmations still run for approximately 600 seconds.

The new box no-contour QMC secondary arm completed 161,243,136 planned
evaluations in 319.459 seconds. MC saved 19,148,800 at its clean
900-second deadline (908.258 seconds including drain); that planned
allocation remains incomplete. Its actual-work display correction is only the
same square-root heuristic. The two independent timed arms remain near 600 seconds.

The timed-call column counts observed logical sector-integrand evaluations,
including reported pilot and discarded work. Precision-rescue backend calls
are retained separately and do not become new sampling points. A leading
$gt.eq$ identifies an older lower bound: process cancellation may lose the
unpublished tail since a worker's last status message. Unprefixed counts require
complete native deadline/drain accounting. Accepted production work
is recorded separately and exactly. New weight-diagnostic runs require their
own complete accounting; no total is inferred from elapsed time or throughput.

The cancellation and maximum-weight columns use fresh native
accepted-production accumulators. A dash means that this arm has no accepted
instrumented measurement yet; old receipts cannot reconstruct these statistics.
Accumulator and observation costs remain inside the new native elapsed clock.
The cancellation numerator is the sum of absolute sector integrals plus the magnitude
of the combined exact offset, divided by $abs(I_0)$. The maximum uses actual
complex magnitudes, with continuous importance correction retained and discrete
sector-selection correction removed for a conditional sector weight. It depends
on the proposal and sampled work; it is neither a supremum nor a tail bound.

== Execution and topology definitions
Generation uses serial sector residency with 25 workers for the scalar suite.
The physical-numerator double box uses six generation workers to control its
larger per-sector memory; this is the explicit generation exception. Integration
uses 25 workers for both samplers. QMC uses serial residency with a one-second
minimum stay. MC denotes the existing *ordinary discrete-MC scheduler*, the
user-authorized exception to serial integration. The shared owned campaign
cohort observes a 100 GB allowance. Campaigns may overlap; no quiet-machine assumption is made.
The caller now permits up to five hours for a double-box generation attempt;
this ceiling is a preparation allowance, not a change to the 600-second
integration comparison or a relabelling of any completed measurement.

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
symGCAD revision `644a560e69a064f811fa93aff33a247ea7fcdfca`. The original 38
instrumented timed arms use executable `cf9eb450`; the subsequently admitted
triangle no-contour arms use `35c92289`, with their distinct source/build receipt.
The subsequently admitted sunrise no-contour arms use `3cf81e4b`.
The subsequently admitted box no-contour arms use `d94dd90d`.
The triangle executable retains normal stability checks and precision rescue in checked
costs and integration, even when the raw f64 evaluation is much cheaper.
Its timed QMC run rescued 115,307,844 of 340,410,368 logical calls (33.9%);
the MC run rescued all 61,915,136 calls, with zero failures and maximum precision
256 bits. Their weighted-check / additional-replay counts were 21 / 12 and
144 / 0. The deterministic profiling coordinates triggered no rescue. These
are different sampling workloads; the archive has no per-trigger rescue reason
histogram. The checked integration clock retains this overhead.
The new sunrise timed QMC run rescued 103,781,710 of 306,384,896 calls (33.9%);
MC rescued all 106,426,368 calls. Both reached 256 bits with zero failures.
Their weighted-check / additional-replay counts were 8 / 6 and 48 / 0.

The new box timed QMC run rescued 98,390,993 of 305,291,264 calls
(32.2%); MC rescued all 29,368,320 calls. Both
returned zero failures; maximum precision was 384 and 256 bits, respectively.
The checked clocks retain this overhead, independently of the raw f64 profile.

== Current admission limits
The bubble, triangle, massless sunrise, massive sunrise and massive box have
complete no-contour integration rows. The
triangle uses its unchanged native HEPKit graph input, four freshly verified
source cells, and 18 endpoint charts; its quoted generation time includes the
complete publication step. Fresh-process loading and separate QMC/MC
confirmations use that actual artifact. The earlier unsupported attempt is
retained as historical evidence rather than overwritten.
The sunrise uses its unchanged native graph, one verified source cell and six
endpoint charts. Its separately seeded confirmations agree with the retained
analytic identity under the reported full-covariance diagnostics.

The box now uses six verified source cells and 54 endpoint charts, with the
original physical graph, point and OneLOop normalization. Its explicit native
order changes the CAD presentation only. Full publication, fresh-process
loading and separate 600-second QMC/MC confirmations are measured.

The original frozen benchmark CLI reported “Unsupported” for box and kite
before invoking GCAD. Those original refusal receipts remain historical. The
kite still lacks a complete endpoint artifact and no-contour numerical row.
A later native solve and fresh verification produced 9,840 cells. Selected
moving-boundary admission now passes, but full interval densities, closed-face
continuation and whole-atlas coverage remain unfinished.
These are implementation gaps, not rejected physical integrals or evidence of
GCAD infeasibility. Geometry-only work is not counted as complete generation.
The existing native
HEPKit CLI graph-parametrization route binds the prepared source and input
fingerprint; it is distinct from the stricter pre-parametrization GraphPoint
owner API. Surviving inexact geometric coefficients are refused, not rounded.
The user stopped double-box no-contour development after the documented local
admission result. Those two methods are omitted from the opening overview;
the subsequent boundary discussion records their atlas, chart counts and
implementation limits. GCAD-only timings are not complete generation
measurements. The studies below retain their original scope and measurements.

#set text(size: 8pt)
*Campaign completion update.* The massive sunrise now has measurements for all
four intended prescriptions and both samplers. The scalar
and physical double boxes now both include measured sign-aware results.
The bubble now also contains its separate sign-aware confirmation. Previously
measured entries retain their original values and run identities.

For massive-kite cell 0, all ten replacement boundary charts pass 40 strict
native 128/256-bit checks. This is a selected fourth boundary over three
integration directions, not a complete four-dimensional interval density.
The remaining 9,839 cells and Laurent continuation make completion substantial
work; it is reported as *Not completed* under the final user scope. The
double-box scalar diagnostic admitted 307 charts of one original cell, with
zero compiled kernels or integral estimates. Neither preparation result is
entered as full-integral generation or integration time.

*Scalar double-box sign-aware extension.* Fresh serial generation with 25 workers
took 34.15 seconds and 2.03 GB sampled peak RSS, producing 18 kernels and
88.60 MB of artifacts. Symbolic endpoint reduction and symbolic contour
Jacobians use Horner10 and SymJIT O2. The recipe was absent from the previous
campaign, rather than rejected by the implementation. The sign-aware bound
penalizes higher-order terms that oppose the causal direction, allowing helpful
terms without imposing the same absolute-magnitude penalty. It does not imply
better variance for every topology.

At the predeclared $S=0.8$, $L=R=1$, independent QMC and ordinary discrete-MC
confirmations ran for 600.159 and 601.285 native seconds. Their 600-second
normalized radial relative errors are 0.197 and 0.586, both carrying the native
large-uncertainty warning. These runs show no improvement over the existing
polynomial-dynamic measurements; they do not establish an optimal tuning or a
high-precision integral value. Sixteen marginal comparisons with the currently
reported fixed and polynomial arms pass the stated compatibility diagnostic,
with a maximum difference of 1.349 combined standard errors. Full covariance
and distinct artifact identities are retained. The secondary target remains
four million accepted evaluations per sector, with the explicit square-root
display correction. Compact results and exact invocation settings are in
`docs/scalar-double-box-sign-aware-overview.json`.

*Physical double-box sign-aware extension.* Independent full-source generation
took 1536.10 seconds and 36.67 GB peak aggregate RSS with six resident generation
workers. It produced 30 kernels and 99.71 MB of artifacts, retaining the physical
numerator and full four-component Laurent vector. The raw f64 maximum/average
sector costs are 9.108/7.724 microseconds; their full-sweep sum is 231.72
microseconds, or 236.72 microseconds with stability handling in this profile.

At the same predeclared $S=0.8$, $L=R=1$, QMC made 331.70 million logical calls
in 600.135 seconds and ordinary discrete MC made 43.59 million in 603.712
seconds. Their normalized radial relative errors are 0.708 and 0.680, with
large-uncertainty warnings. All 32 marginal comparisons with the fixed and
polynomial confirmations pass the stated compatibility diagnostic; the largest
is 2.643 combined SE. This is a noisy baseline, not an accuracy certificate.
The physical compact record is `docs/physical-double-box-sign-aware-overview.json`.

#include "double-box-dynamic-tuning.typ"

*Bubble sign-aware completion.* A fresh 0.865-second generation produces one
kernel. Independent QMC/MC confirmations take 600.043/602.843 seconds and give
normalized radial relative errors $9.85 times 10^(-17)$/$1.00 times 10^(-4)$.
The first value is at floating-point roundoff and is not a numerical accuracy
claim. Eight full-vector marginal checks against the retained independent
reference pass, with a maximum 1.203 combined SE.
See `docs/bubble-sign-aware-overview.json` for exact settings and native results.

*Massive-sunrise extension (12 October).* The new opening block is a distinct
complete source with all three masses one and $s=16$, native measure one and
$D=4-2epsilon$. Release `6647950` generated all 62 charts in 6.70 seconds,
including the fresh solve and automatic endpoint admission. Horner10, CPE1000
and SymJIT O2 were used. QMC has one-second minimum serial residence; MC uses
the authorized ordinary scheduler. Both use 25 physical cores, Korobov3,
4096-point initial designs, eight initial replicas, fixed-size continuation,
validated algebraic precision rescue and optional causal checks off.

Independent confirmation seeds 202610125021 and 202610125022 ran for 604.97 and
602.67 seconds including native task drain. The preceding 30-second calibrations
are not pooled. The secondary work display scales to 20 million accepted calls
in total (20 million / 62 per sector); it is a square-root heuristic from these
same timed runs, not an additional matched-work experiment. The final column
uses native radial norm uncertainty and the small square-root correction to
600 seconds. Complete covariance, accepted counts, total calls, diagnostic
timings and settings are in `docs/no-contour-massive-sunrise-overview.json`.
Fresh matching ten-minute fixed, polynomial-dynamic and sign-aware contour
arms now appear alongside it; earlier short controls are not substituted.
The portable reproduction card is
`examples/no_deformation/benchmarks/controls/massive_sunrise_no_contour.toml`;
commands and measurement scope are in `docs/benchmark-overview-protocol.md`.

The six contour confirmations use the identical frozen executable, full
Laurent orders $-2,-1,0$, symbolic endpoint reduction and symbolic Jacobians.
Their generation times are 1.61, 1.44 and 1.44 seconds, with one compiled kernel
per recipe. Fixed $lambda=0.1$ gives relative norm SE of $5.27 times 10^(-13)$
(QMC) and $3.78 times 10^(-5)$ (MC) after the small 600-second correction.
Polynomial dynamics gives $4.29 times 10^(-11)$ and $2.16 times 10^(-4)$;
sign-aware dynamics gives $2.15 times 10^(-10)$ and $2.20 times 10^(-4)$.
All dynamic settings remain the predeclared $S=0.8$, $L=R=1$.

The secondary average-work target is the same $20 times 10^6/62$ evaluations
per sector for every method, so a one-kernel contour arm is rescaled to that
total rather than to 20 million. This large work rescaling remains a heuristic;
the near-ten-minute column is the primary comparison. Native covariance,
diagnostics, settings, exact generation metrics and all six fresh results are
in `docs/massive-sunrise-contour-overview.json`.

Thirty-six marginal comparisons with the independently generated pySecDec
Laurent reference pass, with maximum discrepancy 2.029 combined SE. The
existing native Numerica matrix diagnostic additionally compares all eight
campaign arms over all three complex Laurent blocks: 84 comparisons, largest
defined descriptive Mahalanobis distance 2.611. Five pole comparisons are
rounding-sensitive and two covariance blocks are not positive definite; their
diagnostics are retained without regularization or inflated errors. Estimated
QMC replica SE does not bound common numerical bias.

#table(
  columns: (1.1fr, 1fr, 3fr), inset: 4pt,
  stroke: .35pt + rgb("c7d4df"),
  table.header([*Topology*], [*Prescription*], [*Measurement status*]),
  ..overview-data.topologies.map(t => t.methods.map(m =>
    (t.label, m.label, m.status_detail)).flatten()).flatten(),
)
