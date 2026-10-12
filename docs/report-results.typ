= Benchmark findings and scientific checks <benchmark-findings>

The opening tables support comparisons at a common finite Laurent order and
approximately ten minutes, with the metric and clock qualifications in
@campaign. The main lesson is that *a permitted contour displacement, a fast
raw evaluator and a verified sign decomposition are each insufficient to
predict the final uncertainty*. Sector proliferation, full Jacobians,
importance sampling and precision rescue can dominate different cases.
The following conclusions concern the recorded settings and independent
confirmations, not universal rankings of the methods.

== Scalar topologies: accuracy versus representation cost

The bubble, triangle, box, massless sunrise and massive sunrise have complete
no-contour results with both samplers. Their no-contour kernel counts are
6, 18, 54, 6 and 62, respectively, versus one or two kernels in the
corresponding contour recipes. The extra sign/endpoint charts can make a
single kernel cheap while increasing the work of a full sweep. Fixed and
dynamic contour kernels likewise have different arithmetic costs; the
largest allowed deformation need not minimize either variance or runtime.

For the triangle and box, the dynamic construction improves QMC convergence
at the recorded $S=0.8$, $L=R=1$ settings. The box's normalized radial relative
SE is about $1.02 times 10^(-8)$ for both dynamic variants, compared with
$5.68 times 10^(-5)$ for fixed strength and $5.33 times 10^(-10)$ for
no-contour. Ordinary MC gives a different balance: its box values are about
$8.5 times 10^(-5)$, $1.39 times 10^(-4)$ and $9.73 times 10^(-4)$,
respectively. QMC and MC therefore cannot be ranked solely by raw call count.
Polynomial and sign-aware bounds coincide for the relevant quadratic
residual causal factors with no higher terms to distinguish their envelopes.

The massless sunrise is a non-crossing control, not evidence for treating an
interior singularity. Its contour results nearly coincide after monomial
extraction leaves no active deformation direction. The massive sunrise is
a genuine above-threshold test: all seven verified cells enter through 62
closed endpoint charts in the no-contour representation. Complete generation
takes 6.70 seconds, versus 1.44–1.61 seconds for the one-kernel contour
recipes. At the recorded settings fixed deformation has the smallest reported
SE for both samplers. The independently generated pySecDec Laurent reference
is retained as a scientific control, rather than using this ranking as a
test of correctness.

The all-massive kite favors fixed deformation over both dynamic baselines
in this campaign. It has no complete no-contour result. The earlier
three-massive/two-massless kite at small momentum is a separate tuning
experiment: its results cannot be used as either a reference or a substitute
for the physical $p^2=16$ row.

*Roundoff is visible in the easiest cases.* Bubble QMC replica SE reaches
the binary64 rounding scale. Its value excludes common numerical bias and
must not be read as accuracy beyond f64. The finite analytic value
$2-3 log(3)/2+i pi/2$ supplies an independent discrepancy check. Even a
visually excellent SE or smooth QMC trend does not calibrate uncertainty
coverage from one confirmation.

== Precision rescue changes the effective cost

Long integration streams and deterministic f64 profiles exercise different
points. In the triangle no-contour confirmations, QMC rescues 115,307,844
of 340,410,368 logical calls, while MC rescues all 61,915,136 calls. Both
reach 256-bit precision with zero failures; the deterministic profile
triggered no rescue. The corresponding massless-sunrise and box QMC fractions
are 33.9% and 32.2%; their MC runs again rescue every logical call. Box
maximum precision is 384 bits for QMC and 256 bits for MC, with zero failures.
The recorded clocks include this work. These workloads have no per-trigger
reason histogram, so rescue frequency alone does not identify its cause.

The massive-sunrise profile illustrates the same issue particularly clearly:
its no-contour summed raw f64 cost is 47.4 microseconds per sweep, whereas
the checked sweep costs approximately 23.7 milliseconds. Consequently,
native root, branch and stability handling must be assessed together with
the compiled expression. These observations motivate optimization but do not
justify disabling mandatory precision rescue or dropping difficult points.

#pagebreak()
== Double boxes: tuning restores fixed-like behavior

The scalar and physical D05 cases have 18 and 30 stochastic kernels. For
the scalar input, complete fixed/polynomial/sign-aware generation takes
9.77/19.2/34.1 seconds. For the physical numerator the corresponding values
are approximately 253/1800/1536 seconds, with six resident generation
workers and measured peaks of 8.99/34.8/36.7 GB. These are complete
publication measurements, not GCAD geometry timings. Tuned rows reuse the
same artifacts and therefore repeat those generation costs.

The untuned $L=R=1$ dynamic double-box results are substantially noisier than
fixed $lambda=10^(-6)$, including large-uncertainty warnings. A separate
calibration chooses $R=10^6$ and small $L$ as specified in @campaign.
This weakens an overly restrictive displacement cap while controlling the
strength and its spatial derivatives. No Jacobian or local-strength
derivative is omitted. Sampling profiles are measured again at the chosen
runtime settings; all eight final confirmation streams are independent of
calibration.

#table(columns: (1.5fr, .75fr, .85fr, .9fr, .5fr), inset: 5pt,
 stroke: .35pt + rgb("c7d4df"),
 table.header([*Finite coefficient*], [*Fixed*], [*Dynamic tuned*], [*Sign-aware tuned*], [*Sampler*]),
 [Scalar double box], [1.02%], [0.665%], [0.755%], [QMC],
 [Scalar double box], [0.246%], [0.262%], [0.233%], [MC],
 [Physical double box], [1.98%], [2.09%], [2.14%], [QMC],
 [Physical double box], [0.594%], [0.662%], [0.648%], [MC],
)

These radial relative SE values use the small correction to 600 seconds;
actual durations are 600.11–604.79 seconds. Tuned dynamics improves the
scalar QMC observation and recovers near-fixed behavior for the physical
numerator. It does not establish a universal dynamic advantage. All eight
confirmations have complete production coverage, no failed, unstable or
discarded-zero points, and no norm-linearization warning.

*Independent equal-work checks.* Three scalar QMC controls each complete
71,958,528 accepted calls. The polynomial/fixed and sign-aware/fixed
finite covariance-trace ratios are 0.193 and about 0.224. Their times are
145.95 and 136.80 seconds, against fixed's 112.49 seconds. Thus the scalar
variance improvement is also present at equal work, using the full complex
uncertainty rather than only its radial projection.

Three physical QMC controls each complete 179,896,320 calls. Fixed,
polynomial and sign-aware take 377.81, 406.92 and 328.12 seconds, with
covariance-trace ratios 1, 0.9904 and 0.9790. These single independent
controls recover fixed-like per-call variance; they do not establish a
statistically significant advantage. The estimates are not pooled with
timed observations. Six MC work controls stop cleanly at their original
600-second caps and remain explicitly incomplete requested work. Their
actual counts support only the declared heuristic work display.

For the polynomial construction, sufficiently small $L$ approaches
$lambda=S L$ at fixed regular coordinates when the remaining bounds are
inactive. Sign-aware smoothing leaves a small limiting correction retained
in the native tests. Scalar test points with $L=1.25 times 10^(-6)$ and
$R=10^6$ give $lambda/(S L)=0.9792,0.9915,0.9985$; the larger scalar-QMC
cap is not simply a copy of the fixed map. Such point diagnostics do not
certify an arbitrary fixed contour globally or characterize every sector.
In particular, the final weight summaries do not retain maximum-weight
coordinates. The saved cap experiments discussed in the implementation
chapter provide separate mechanism evidence, not new confirmation data.

#pagebreak()
== Compatibility and independent references

Compatibility uses complete real–imaginary covariance for every available
Laurent coefficient. For independent estimates the difference has covariance
$Sigma_1+Sigma_2$, and the descriptive distance is
$sqrt(Delta^T (Sigma_1+Sigma_2)^(-1) Delta)$.
Distinct reserved seeds support independence; no empirical cross-method
covariance is invented. This estimated-covariance distance is not a
calibrated p-value. Comparisons are correlated with one another, and a
large collection of them is not a set of independent significance tests.

#let compatibility = json("benchmark-compatibility-results.json")
#let fmt-distance(value) = if value == none { [—] } else { str(calc.round(value, digits: 2)) }
#table(columns: (1.45fr, .4fr, 1fr, 1fr), inset: 5pt,
 stroke: .35pt + rgb("c7d4df"),
 table.header([*Baseline case*], [*Estimates*], [*Largest mutual distance, $epsilon^0$*], [*Largest reference distance, $epsilon^0$*]),
 ..compatibility.topologies.map(row => (
   row.label, str(row.runs),
   fmt-distance(row.comparisons.maximum_covariance_distance),
   fmt-distance(row.reference_comparisons.maximum_covariance_distance),
 )).flatten(),
)

This baseline audit covers #compatibility.accepted_runs retained native
estimates from work/time arms. Its largest nonsingular distance across all
orders is 3.779 between runs and 3.077 against a reference. The physical
pole comparison with a noisy dynamic-QMC estimate produces the former.
Singular and rounding-sensitive comparisons remain unresolved diagnostics;
they are not regularized, padded or hidden inside these maxima.

The final double-box tuning extension independently checks 26 estimates
over 234 complex Laurent blocks. Largest distances are 3.169 for scalar
$epsilon^0$, 3.805 for physical $epsilon^(-1)$ and 3.280 for physical
$epsilon^0$, with no non-positive-covariance or rounding-sensitivity flags
in that set. This extension does not discard the baseline's noisier
observations or turn consistency into an analytic reference.

The massive-sunrise audit checks all eight fresh timed arms through orders
$-2,-1,0$. Its 84 complex-block comparisons have largest defined distance
2.611; five pole comparisons are rounding-sensitive and two covariance
blocks are not positive definite. Those diagnostics are retained without
regularization. Thirty-six component comparisons of the six contour arms
with the independent pySecDec reference have maximum discrepancy 2.029
combined SE; the two no-contour arms have a separate 36-component audit
against established controls with maximum 2.162. These are descriptive
agreement checks, not calibrated coverage probabilities.

The bubble has the analytic finite value given above. Native
HEPKit/OneLOop supplies triangle and box reference expressions at exactly
the listed point, including their explicit inverse $r_(Gamma)$ measure
factor. The massless sunrise uses Symbolica's Laurent expansion of the
composed analytic bubble identity with its causal phase. The independently
generated massive-sunrise pySecDec finite reference is
$10.0161430322940-3.3089881368544 i$.
There is no independently established reference here for the five-massive
kite at $p^2=16$ or either D05 double box. Their inter-method agreement is
a consistency check; the scalar and physical integrals cannot validate
each other by equality.

Native Numerica matrix operations perform the retained covariance checks.
Reference rounding has no certified enclosure and is not silently converted
into stochastic error. The compact evidence inventory in @campaign preserves
every association and qualification. The resulting campaign is reproducible
without mistaking an unfinished atlas, a tiny sampling SE or a consistency
check for a certified integral.
