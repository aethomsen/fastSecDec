# Double-box dynamic-strength tuning review

The independent tuning slice retained the original contour artifacts and used
separate calibration and confirmation seeds. The coordinator checked all 79
pins in the immutable first timed-result handoff, all eight complete native
results and their full Laurent covariance. Caller settings match the frozen
calibration policy except for declared independent streams and work/time limits.
No samples are pooled, duplicated or silently omitted after numerical failure.

The tuned strength/displacement settings change both the shape and permitted
size of the contour. Native controls retain the strength-gradient contribution
to its Jacobian and verify the small-strength behavior. They are selected-point
diagnostics, not a global certificate for an arbitrary fixed contour. The
highest scalar sign-aware calibration cap failed the existing root-convergence
check and was excluded without relaxing that check.

The coordinator independently used the existing `benchmark_compatibility`
example and Numerica's exact-rational matrix determinant/solve on saved full
covariances. Twelve original baseline estimates, eight new timed confirmations
and three complete scalar QMC work controls give 168 complex-block comparisons.
The maximum descriptive Mahalanobis distances are 3.169 for scalar finite,
3.805 for the physical pole and 2.548 for physical finite coefficients. No
non-positive covariance or rounding-sensitivity flags occur in this set. These
are estimated-covariance diagnostics, not calibrated p-values or an analytic
reference for the physical diagram.

The complete scalar QMC controls have exactly 71,958,528 accepted calls each;
their polynomial/fixed covariance-trace ratio is 0.193. Scalar MC controls are
cap-incomplete and remain labeled as such. Physical work controls were pending
at this immutable timed-result cutoff. The secondary table value therefore
uses an explicitly labeled square-root heuristic where no complete work control
is available; the final ten-minute error column is primary.

Generation receipts are explicitly reused. Runtime profiles remain specific to
the selected sampler settings. The original baseline rows are unchanged. The
coordinator compiled the Typst report and visually checked both new double-box
overview pages for alignment, readable grouping and the complete metric set.
Compact native results, settings, source identities, commands and this native
joint audit are in `docs/double-box-dynamic-tuning-overview.json`.

The no-contour double-box work was subsequently removed from scope by the user.
Neither these contour results nor a partial endpoint certificate constitutes a
no-contour integral result.

## Final independent work controls

The final handoff, SHA-256
`46d8b5f91a102c2e3d18557ccf1767d98666190c13c5148a9a815659f1885395`,
closes all twenty predeclared jobs. The coordinator checked 99 pins and verified
that all fourteen previously closed rows were unchanged. Six QMC controls
complete their exact requested work; six MC controls preserve their original
600-second cap and explicit incomplete-work status. No reruns or pooled
estimates were introduced.

At 179,896,320 accepted calls each, the physical fixed/polynomial/sign-aware
QMC controls took 377.81/406.92/328.12 seconds. The tuned covariance traces are
0.9904/0.9790 times fixed, recovering similar per-call variance. The scalar
controls at 71,958,528 calls retain ratios 0.1929/0.2239. These are measured
single-run comparisons, not uncertainty estimates for replicated variance
ratios. The PDF and secondary work columns now use all six complete controls;
the original timed estimates remain unchanged.

The coordinator repeated the native Numerica matrix audit on 26 complete
estimates: 234 complex-block comparisons, with maximum descriptive distances
3.169 (scalar finite), 3.805 (physical pole) and 3.280 (physical finite).
No non-positive covariance or rounding-sensitivity flags occur. Partial MC work
controls are excluded from this final joint set. The final compact record is
`docs/double-box-dynamic-work-overview.json`; the earlier immutable timed
snapshot remains available rather than being silently rewritten.
