# Accepted complex-weight diagnostics

The native integration sessions now collect absolute complex weights and observed
sector-local maxima for every Laurent order. The complete common component layout
is authoritative; real and imaginary values are paired at the same sampled point.
The original mean, full covariance, random streams, proposal adaptation, and
stopping criteria are unchanged.

For each order the absolute-integral estimate is the sum of sector absolute-weight
means plus the magnitude of the **combined exact coefficient once**. Its ratio to
the magnitude of the accepted total complex mean is reported without clamping.
The maximum ratio uses the largest actually observed stochastic sector-local
weight. Both retain continuous importance correction and coordinate-transform
weights. Discrete Havana removes only inverse sector-selection probability from
the maximum; its absolute-integral mean retains the full proposal correction and
uses all global draws. Thus the maximum is proposal dependent, not a supremum.
The grouping of exact terms and stochastic sectors also affects these diagnostics.

Zero denominators yield unavailable ratios. An exact-only problem has no observed
stochastic maximum. Numeric-range failures invalidate diagnostics rather than
altering an integral or inventing zero samples.

## Native reuse and bounded state

The implementation reuses native QMC plans/work intervals, Numerica's Havana
samples and immutable `DiscreteGrid::bins[index].pdf`, and Numerica `DoubleFloat`
arithmetic. In the adopted Symbolica/Numerica `ae0c82b` source,
`DiscreteGrid::sample` multiplies the continuous child weight by inverse bin
probability; `add_training_sample` itself multiplies the resulting weight by the
bin's `pdf` when undoing only the discrete selection. The diagnostic follows that
same operation after the complete weighted complex vector has been evaluated.

The scalar `StatisticsAccumulator` API was checked before introducing the small
mean-only state: `add_sample`, `merge_samples_no_reset`, `update_iter(false)`, and
`get_live_estimate`. These APIs necessarily store raw sums and square sums. A
focused native Rust probe confirmed ordinary `[1,2,3,4]` gives mean 2.5, but four
`1e308` values and two merged `1e308` singletons produce a nonfinite mean despite
a representable result. Centering fixes the constant case, but centered
`[1e308,0,0,0]` still overflows although its mean is `2.5e307`. The missing operation
is a mergeable, normalized mean-only accumulator without raw-sum/square overflow.
The narrow state therefore uses Numerica double-float centered mean updates and
normalized merge weights, checked counts, and per-order maxima. It supplies no
alternative variance estimator and retains no sampled coordinates or values.
The probe source, build command, native-source hash, and output are preserved in
`target/no-deformation-weight-diagnostics/native-accumulator-*` for this session.

## Admission, continuation, and saved output

Worker summaries are committed with the existing task/epoch/reservation checks.
QMC uses exactly the complete shifts selected by its estimator, including common
shifts for democratic integration. Serial QMC/Havana retain the previous complete
allocation's diagnostics until the replacement allocation becomes authoritative.
Ordinary and discrete Havana use whole accepted batches. Pilot work, failed
prefixes, retries that were not accepted, and duplicate/stale returns do not enter
these statistics.

Optional checkpoint sidecars preserve historical JSON serialization when absent;
the existing integrity/digest checks remain in force. If any selected accepted
work lacks the old pointwise information, the whole selected epoch reports
`LegacyUnavailable`, even after newer work arrives. A genuine prechange checkpoint
with accepted work and an outstanding reservation is retained as a regression
fixture; no statistic is reconstructed from sector means.

`IntegrationObservation` carries the same structured diagnostic used for streamed
and saved presentation. Native validation checks the full component/order layout,
selected sample count, finite fields, exact and total magnitudes, and derived sums
and ratios. Operational evaluator counts and historical operational maxima remain
separate from these accepted-production statistics.

## Complex-norm uncertainty

`VectorEstimate::complex_norm_uncertainty` propagates the full real–imaginary
covariance onto the radial direction of the complex mean using Numerica
DoubleFloat, with covariance scaling before summation so a representable standard
error is not lost when an unscaled trace or projection overflows. Detected
exponent loss or near cancellation takes an observation-only fallback: native
`Rational::try_from(f64)` evaluates the quadratic numerator and squared norm
exactly, then native `to_multi_prec_float(128).sqrt()` computes the final square
roots. Ordinary observations stay on the DoubleFloat path. It reports
radial standard error and relative error separately from
`sqrt(trace(C))/|mean|`. Zero norm, nonfinite projection, and negative projected
variance remain unavailable; negative variance is never replaced by its absolute
value. A joint-relative error of at least 0.1 triggers a conservative linearization
warning heuristic, not a mathematical validity boundary. This observational API
does not change the native solver's accuracy target.

Focused native probes retain two counterexamples: global scaling can erase the
active `1e-308` entry of `diag(1e-308,1e308)` for mean `(1,0)`, while fixed
Float128 arithmetic can lose the positive residual of covariance
`[[1e-308,-5e307],[-5e307,1e308]]` for mean `(1,1)`. The latter is a numerical
validation stress case, not a positive-semidefinite covariance claim. Exact
native arithmetic recovers radial errors `1e-154` and approximately
`7.0710678e-155`, respectively. Negative exact projected variance is unavailable;
true exact zero is retained. A nonzero error below the serialized f64 range is
also unavailable. These controls and the native conversion/source receipts live
under `target/no-deformation-weight-diagnostics/native-*projection*`.

## Validation

The full native integration prefix passes all 43 tests, including eleven focused
weight groups: complex cancellation, asymmetric
discrete probabilities, continuous importance weighting, common QMC shifts,
reordered/checkpointed work, stale serial leases and previous epochs, genuine
legacy checkpoint integrity, exact-only/zero-denominator cases, pointwise complex
maxima, radial covariance propagation, failed-prefix exclusion, and checked
counter/interval overflow. The last covariance-scaling amendment adds a finite
`1e308` off-diagonal case whose unscaled trace and projection overflow, together
with zero covariance, mixed exponent ranges, and exact cancellation controls.
The final exact-fallback consuming gate passes all eleven focused groups; saved
results pass twelve tests, and CLI results/timed-stop controls pass five tests.
The final joined workspace/all-target strict Clippy check passes without warnings
in 25.06 seconds; formatting also passes. Exact commands and source pins remain in
the milestone acceptance receipts.

A bounded debug control evaluated 32,768 native points and two complex orders in
16.49 ms without the diagnostic and 33.83 ms with it; the native partial bytes
were identical. This measures overhead for an illustrative cheap callback, not a
physical integrand benchmark. New physical confirmations must include the enabled
diagnostic cost.

## Independent presentation and binding review

The saved-result field defaults to absent for historical documents. Validation
delegates to the native diagnostic validator with the manifest-checked component
layout, exact vector, accepted total, and checked sum of selected-sector point
counts. Derived ratios are checked; stored sample statistics cannot be recovered
or independently re-estimated from a result document. The regression controls
alter accepted counts, maximum ratio, and total magnitude and require rejection.

CLI report and storage paths copy the same `IntegrationObservation` payload.
Dashboard norm uncertainty comes from its accepted total covariance; live previews
retain their explicitly labeled joint uncertainty and do not fabricate radial
error. Rendering operates on a cached snapshot without sampling or changing it.
The Python binding's existing `IntegrationObservation.to_json` serializes the
whole native owner, so the new optional field is forwarded automatically. No new
Python numeric primitive or sampler is needed; a dedicated Python convenience
getter and notebook display are outside this slice. This binding conclusion is a
source/API audit, not a new Python execution claim.
