= Numerical compatibility

#let compatibility = json("benchmark-compatibility-results.json")
#let fmt-distance(value) = if value == none { [—] } else { str(calc.round(value, digits: 2)) }

This checkpoint compares #compatibility.accepted_runs completed, accepted production estimates. The tests
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

No disagreement requiring a new tuning choice is indicated by these completed
checks. They include comparisons between samplers, prescriptions and work/time
arms where results are available. The largest nonsingular mutual distance over
all orders is 2.93; the largest reference distance is 3.08. These many comparisons are correlated with one another and are
not independent repetitions of a significance test. Later campaign closures
and instrumented reruns still require the same audit.

== Independent references and numerical limits
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
