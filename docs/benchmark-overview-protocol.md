# Twenty-five-core finite-coefficient benchmark protocol

Status: running with frozen dependency, executable and recorded run plans.
Generation and explicit-Off cost measurements are available; confirmation
integration remains in progress. No earlier historical number is relabelled as
a measurement of this campaign.

The opening report is `benchmark-overview.typ`, driven by
`benchmark-overview-results.json`. It uses A4 landscape topology blocks with
adjacent QMC and discrete-MC rows for each prescription. Settings, qualifications
and native receipts belong in the later detail section, not in the opening table.
Every displayed accuracy, uncertainty and target refers to **ε⁰**. Generation
must therefore stop at order zero, including the bubble previously generated
through ε¹. Retain every lower Laurent coefficient and the complete covariance.

## Inputs and admission

Use native HEPKit graph inputs, Symbolica algebra and the existing FastSecDec CLI.
`examples/no_deformation/benchmarks/overview/inventory.json` records exact fixture
sources, normalization and historical limitations. The primary blocks are the
massive bubble, triangle, box, massless sunrise, an above-massive-threshold kite,
and scalar and physical-numerator double boxes at the same exact rational point.
The latter two share propagator geometry, not numerator or integral identity.

The new kite uses the existing all-five-massive graph with every mass one and
exact p²=16, as selected by the native input audit. The old mixed-mass kite at
p²=3/1000 has a massless cut and a small-momentum
reference. Its reference and timings do not apply to a new point above a massive
threshold. Freeze the requested kite's complete propagator/mass list and exact
point after native input and causal-admission checks. No reference is invented
when an independent value is unavailable. The massless sunrise has no interior
F zero; retain this distinction in the detailed interpretation.

Compare fixed contour, dynamical polynomial contour and no-contour threshold
decomposition wherever each route is actually admitted. Keep sign-aware dynamics
as a separately identified controlled variant where relevant. An unsupported
generation or endpoint route gets a typed missing row. A verified GCAD partition,
open map, factor certificate or partial source is not a complete integration.
All rows must cover the entire stated integral and exact offset.

Endpoint reduction remains symbolic in every comparison. Select symbolic or
contour-only dual Jacobians using separate, recorded preparatory measurements;
freeze the selected choice before confirmation. Preserve both alternatives in
detail records. Contour-strength tuning, causal pilots and throughput calibration
must use seeds disjoint from confirmation. Tuning may not inspect confirmation
results. Native no-contour precision rescue and all mandatory checks remain on.

## Build and execution identity

Before expensive work, freeze the source commit, dirty-source inventory (normally
empty), Cargo.lock, exact dependency commits, release options, feature set,
executable SHA-256 and native library identities. The latest symGCAD adoption and
certificate-budget patch require consuming gates before this freeze. A reference
clone's HEAD is not evidence that an executable uses it.

Generate admitted artifacts with `generate --serial --workers 25`. The physical
numerator double box uses an explicitly recorded six-worker generation exception
under the shared RAM allowance, retaining 25-core affinity. This
recycles sector workers; it does not mean one worker or a sequential campaign.
QMC integration uses `integrate --serial RESIDENCE_SECONDS --workers 25` with an
explicit frozen positive residence. The native residence is a minimum sampling
time per resident sector, excluding loading/JIT and pauses. It is not a total-run
deadline. Preserve the actual command and effective settings.

The user explicitly permits ordinary `--method discrete_mc --workers 25` as the
MC scheduler exception. Do not pass `--serial` to it: the current CLI rejects that
combination. A serial-generated artifact is compatible with both consumers.
The MC exception, proposal/adaptation settings and memory cost remain explicit.

Each campaign uses at most 25 selected physical cores and a 100,000,000,000-byte
owned-process allowance. Independent campaigns may overlap; a quiet host is not
required. Record CPU affinity, hardware, sibling campaigns and observed load.
If campaigns share an allocation, their common supervisor must account for the
whole assigned cohort. Never kill an unrelated process or another campaign to
enforce one campaign's bound. Per-process address-space limits do not establish
an aggregate RSS bound.

## Work and ten-minute comparisons

Make independent calibration and confirmation actions. Calibration estimates a
common work budget that puts the comparison near ten minutes; its result is not
an accuracy observation in the Overview. Freeze this budget and each method's
attainable complete allocation before confirmation. Use the same requested total
native sector-point evaluations across methods, not the same points per sector.
Different decompositions can have different numbers of stochastic kernels.

Count accepted work from native committed sector contributions. Also retain
attempted evaluations, refinements, discarded incomplete allocations, pilot work,
precision rescues and per-sector lattice/batch counts. QMC work must end at valid
complete shifted-lattice boundaries; MC at native complete production batches.
Report actual work and any allocation-granularity mismatch. Do not interpolate
an estimate or its covariance to pretend counts match. A native scheduling gap
that prevents a prescribed total count must be resolved or labelled before
launching the matched-work arm.

The fixed-time arm requests **600 seconds** and reports actual elapsed time and
the last native complete estimate. The serial residence flag and ordinary
`production_seconds` must not be relabelled as a verified global 600-second
deadline. A source-audited caller timer/cancellation path must preserve committed
statistics and save the native outcome. Keep initialization, adaptation,
artifact loading, drain and result publication clocks separately; freeze exactly
which clock starts the 600-second allocation before execution. An external kill
without a valid native estimate is censored, not an integration result.

Use one confirmation seed per sampler and arm, disjoint from pilot and tuning
identities. Work- and time-arm seeds are separate. Never pool different designs,
epochs or covariance matrices
in the collector. Shared seed labels across methods do not supply cross-method
covariance; variance/time ratios are descriptive, with no calibrated difference
significance claim.

## Overview metrics

| Key | Display | Definition |
| --- | --- | --- |
| a | Generation s | Complete process wall time from generation launch through durable artifact publication, with native stage ledger alongside in detail. Build time excluded. |
| b | Generation GB | Sampled aggregate owned generation/coordinator/worker RSS peak, decimal GB. |
| c | Saved MB | Manifest plus all published indexed artifact data, counted once; no temporary build/staging files. |
| d | Sectors | Stochastic executable sectors required for the complete integral. Preserve original charts, symmetry groups and exact contributions separately. |
| e | f64 max / avg µs | Maximum and arithmetic mean of per-sector mean **raw f64 kernel** costs under the frozen evaluator harness. Not a maximum individual point. |
| f | Integration GB | Sampled aggregate owned integration RSS peak. This implements the stated interpretation of the duplicated RAM request. |
| g | Sum f64 / checked µs | Sum of raw f64 per-sector mean costs for one full sweep; separately, measured wall cost for the same complete sweep with normal stability/rescue enabled. Do not substitute summed worker spans for wall time. |
| h | Average-work relative error | Delta-method standard error of the complex ε⁰ norm, divided by that norm, rescaled to the displayed average evaluations per sector. |
| i | Average-work SE | Absolute joint ε⁰ standard error rescaled to that same average-work target. |
| j | 600 s relative error | Delta-method standard error of the complex ε⁰ norm, divided by that norm, rescaled to 600 native seconds. This is the primary comparison. |
| k | Sector calls | Reported logical callbacks during the fixed-time invocation, in millions, including observed pilot/discarded work. Prefix-only counters are labelled ≥, never exact totals. |

For finite-component indices `(r,i)`, native joint SE is
`sqrt(C[r,r] + C[i,i])` with the covariance **of the estimated mean**. It is not
raw integrand standard deviation or a confidence radius. A zero finite mean
gives undefined relative SE; keep absolute SE and do not replace the denominator
with one. An independent-reference error, when available, is a distinct detailed
metric `abs(I0-reference0)` (and relative error when its denominator is nonzero).
Neither relative SE nor agreement with a contour arm is an independent accuracy
certificate. Runtime and RAM may differ between work and time arms; retain both,
and use the fixed-time integration peak for column f.

The relative-error columns use the **radial** delta-method uncertainty, distinct
from joint SE. For finite mean `I=x+iy`, `R=hypot(x,y)` and native covariance
entries `Cxx,Cxy,Cyy`, its variance is
`(x² Cxx + 2xy Cxy + y² Cyy)/R²`; divide its square root by `R` for relative
uncertainty. Retain the covariance and raw joint SE unchanged. At `R=0` the
relative metric is unavailable. Flag the displayed value with † when joint
`SE/R >= 0.1`: first-order norm propagation may be unreliable. It is not a
confidence interval or an accuracy certificate.

Displayed errors use the requested **MC-equivalent heuristic**: multiply by
`sqrt(t/600)` for actual native time `t`, or
`sqrt(W/(K*N_average))` for accepted work `W`, native stochastic-sector count
`K` and displayed average target `N_average`. This does not alter any native
estimate, covariance, seed or allocation, and is **not a QMC convergence law**.
The average-work targets are pilot-derived round display values: bubble 30M,
triangle 80M, box 70M, sunrise 60M, kite 40M, scalar double box 4M and physical
double box 6M evaluations per sector. The physical target uses the completed
independent fixed and dynamical pilots.
Unequal actual sector allocations remain intact. Record each correction factor;
large work rescalings remain heuristic. Within each topology, accepted timed
runs must each lie between 480 and 750 native seconds and satisfy
`max(t)/min(t) <= 1.25`; otherwise suppress the normalized timed comparison and
retain the raw observations. A common large overrun cannot pass solely because
all arms take equally long.

Logical call counts come from native operational `evaluations`, separately from
accepted statistical work. Serial cancellation can retain only the last
published worker prefix; an unfinished matrix batch can also contain backend
calls beyond its classified logical callbacks. Existing counts therefore carry
an observed-prefix qualification. Keep f64/double-float/arbitrary backend counts
separate: a precision rescue is not a new sampling point.

Cancellation and peak-weight ratios require a future instrumented measurement;
the existing receipts do not contain the committed absolute-weight sums.
The cancellation numerator is the sector sum of production averages of the
absolute final ε⁰ complex weight (all coordinate, periodization and continuous
importance factors once), plus the magnitude of the combined exact offset once.
Divide by `|I|` and retain the exact offset separately in the native receipt.
In discrete MC the absolute-integral estimator includes inverse sector-selection
probability and averages over all global batch points. The observed maximum
uses sector-local weights, removing that discrete selection factor while
retaining continuous importance factors. It is a proposal-dependent sample
maximum, not a supremum. Existing pilot-inclusive operational maxima do not
stand in for committed-production statistics.

Raw-f64 and checked costs use the same explicit `--contour-validation off`
native diagnostic with `benchmark --warmup 0`, on deterministic coordinates.
The collector requires zero production `diagnostic_checks.checked_arguments`
and no causal pilot in this measurement. Raw-f64 e/g use the native f64-tier
counters; checked wall includes stability checks and mandatory precision rescue.
Retain raw-call counts and checked-coordinate counts separately. Essential native
callback/root work remains part of evaluation. No physical integrand or parameter
is changed by disabling optional causal validation.

Earlier `always` and `pilot` diagnostics remain separate scientific evidence,
and their timings never enter the Overview. Under `always`, dynamic evaluation
places optional repeated causal validation inside the native f64-tier timer,
whereas fixed validation occurs outside it; those counters are not comparable
raw-f64 measurements. Throughput calibration and work/time production also pass
explicit `--contour-validation off`. Any separately requested causal preparation
or validation has its own clock outside the reported integration allocation.
Preserve the superseded attempts and their policy rather than relabelling them.

## Observation, failures and publication

Record child PID/start-time identities, requested/observed sampling cadence,
launch/exit monotonic clocks and per-process high-water RSS where available.
Sampled aggregate RSS sums shared mappings and is not unique resident memory;
asynchronous reads and missed peaks also prevent calling it an exact high-water
mark or strict lower bound. Preserve any substantial observer gap. Short jobs
may get a separately labelled identical profiling repeat, with original timing
and repeat receipt both retained and native scientific equality checked.

Keep failure/refusal/cancellation/timeout/partial outcomes in the manifest.
Unavailable uncertainty, failed stability or incomplete source coverage never
becomes a zero result or a successful row. Retain settings, full native vectors,
covariance, exact offsets, complete work ledgers, source/artifact/point hashes and
all attempts. The renderer accepts only reviewed measurements with ε⁰ identity;
historical receipts are stored under a separate historical section.

The existing graceful stop is one SIGINT sent to the coordinator PID only.
The CLI sets its cancellation flag, reaps its workers, releases uncommitted
reservations and saves its native checkpoint/result. Preserve the native
Cancelled reason and the actual completeness/uncertainty separately. A second
signal forces exit and is reserved for a separately recorded bounded drain
failure. Verify this path with a short control before the timed campaign.

The requested stopping checkpoint is the scalar suite and both exact-point
double-box rows, with the updated Overview and detailed PDF. Any unresolved
no-contour double-box entry must state its actual blocker. Completing this
benchmark checkpoint does not complete the general endpoint resolver.
