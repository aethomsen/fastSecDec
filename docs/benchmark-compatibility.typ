= Numerical compatibility

#let compatibility = json("benchmark-compatibility-results.json")
#let fmt-distance(value) = if value == none { [—] } else { str(calc.round(value, digits: 2)) }

The original campaign audit compares #compatibility.accepted_runs accepted native production estimates. The tests
use the full real–imaginary covariance, retained before rounding for display.
For two independent runs, the difference uses $C_1+C_2$. The diagnostic distance
is $sqrt(Delta^T C^(-1) Delta)$. It is an estimated-covariance diagnostic, not a
calibrated probability or a proof of correctness. Distinct recorded seeds
support the independence assumption; the audit does not manufacture empirical
cross-run covariance. Every available complex Laurent block is checked; the
summary below concerns $epsilon^0$.

#table(
  columns: (2fr, 0.6fr, 1.2fr, 1.2fr),
  inset: 6pt,
  stroke: 0.3pt + rgb("cbd5e1"),
  table.header([*Case*], [*Runs*], [*Largest mutual distance*], [*Largest reference distance*]),
  ..compatibility.topologies.map(row => (
    row.label, str(row.runs),
    fmt-distance(row.comparisons.maximum_covariance_distance),
    fmt-distance(row.reference_comparisons.maximum_covariance_distance),
  )).flatten(),
)

The checks include comparisons between samplers, prescriptions and work/time
arms where results are available. The largest nonsingular mutual distance over
all orders is #fmt-distance(compatibility.semantics.maximum_all_order_pair_distance)\;
the largest reference distance is #fmt-distance(compatibility.semantics.maximum_all_order_reference_distance). These many comparisons are correlated with one another and are
not independent repetitions of a significance test. Later campaign closures
and instrumented reruns require the same audit. The current all-order maximum
comes from the physical double box at $epsilon^(-1)$, involving its noisy dynamic
QMC estimate. The finite-coefficient maxima are listed separately above; no
uncertainty has been enlarged to obtain agreement. A clean capped secondary-work
estimate can be included with its actual count and unfinished planned allocation;
its covariance still comes from complete native batches. An overlapping partial
prefix is omitted when its completed continuation is used.

== Independent references and numerical limits
The added massive-sunrise block has a separate audit of its two fresh timed
estimates. All 36 full-Laurent component comparisons with the previously
validated fixed, dynamic and pySecDec controls pass the existing eight-combined-SE
criterion without an added floor; the largest difference is 2.162 combined SE.
Both complete native covariance matrices and the radial norm uncertainties are
checked independently. These comparisons are recorded in
`no-contour-massive-sunrise-overview.json`; they do not change the original
campaign counts above. The subsequently completed six contour timings and
their separate native joint audit are recorded in the overview details and
`massive-sunrise-contour-overview.json`.

The bubble's finite reference is $2-3 log(3)/2+i pi/2$. Native HEPKit/OneLOop
expression evaluation supplies the triangle and box references at exactly the
listed invariants and masses. Their measure includes the explicit inverse
$r_(Gamma)$ factor. The massless sunrise uses Symbolica's Laurent expansion of
the composed analytic bubble identity, including its causal phase. Fresh native
reference fixture checks passed for the triangle, box and sunrise; the
historical kite reference is deliberately excluded because its kinematics and
masses differ from this campaign.

There is currently no independently established integral reference in this
campaign for the five-massive kite at $p^2=16$ or either D05 double box. Agreement
between implementations there is a useful consistency check and does not
substitute for an analytic reference. The scalar and physical-numerator double
boxes are distinct integrals despite sharing denominator geometry.

Bubble QMC uncertainties can be smaller than binary64 rounding effects. Singular
or rounding-sensitive covariance tests are explicitly retained as unresolved
diagnostics and excluded from the maxima above. Covariance is never padded or
replaced by a diagonal matrix to obtain agreement. The rounding-warning scale
is a heuristic, not an error bound; the reported sampling SE excludes shared
numerical bias. Rounded reference arithmetic likewise has no certified enclosure
and is not silently counted as additional stochastic error.

The compact reference associations and comparison results are committed as
`benchmark-compatibility-references.json` and
`benchmark-compatibility-results.json`. The native
`benchmark_compatibility` example reuses Numerica's exact conversion of recorded
binary64 entries and its matrix determinant/solve. It checks the native saved
result, input-card association and distinct seeds without generating or sampling.
Full covariance and raw estimates remain in the benchmark data; private audit
receipts preserve every pair and coefficient rather than only these maxima.

The final double-box tuning extension has its own 26-estimate audit in
`double-box-dynamic-work-overview.json`. Its 234 complex-block comparisons
include six complete equal-work QMC controls as well as the timed baselines
and confirmations. The largest distances are 3.169 (scalar finite), 3.805
(physical pole) and 3.280 (physical finite), without non-positive covariance
or rounding-sensitivity flags. This does not replace the original broader
campaign audit or discard its earlier high-variance and roundoff diagnostics.
