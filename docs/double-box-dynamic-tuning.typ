== Double-box dynamic-strength tuning

The new *Dynamic tuned* and *Sign-aware tuned* rows retain the same artifacts as
their original counterparts. Their generation costs therefore repeat the
original measured receipts; no symbolic regeneration was necessary. The original
$L=R=1$ results remain visible. Sampling-cost profiles were measured again at the
selected settings, separately for each sampler when those settings differ.

Independent short calibrations selected $S=0.8$, displacement cap $R=10^6$, and
strength cap $L=1.25 times 10^(-5)$ for scalar QMC. Scalar discrete MC and both
physical samplers use $L=1.25 times 10^(-6)$. The eight confirmation streams were
fixed before their runs and are disjoint from calibration. They use 25 workers,
the existing serial QMC/ordinary discrete-MC scheduling, and shared 100 GB memory
limits. All runs keep the complete position-dependent strength derivatives,
Jacobian and mandatory numerical precision handling. Causal validation is off.

#table(
  columns: (2fr, 1fr, 1fr, 1fr, 1fr),
  inset: 5pt, stroke: .3pt + rgb("c7d4df"),
  table.header([*Finite coefficient*], [*Fixed*], [*Dynamic tuned*],
    [*Sign-aware tuned*], [*Sampler*]),
  [Scalar double box], [1.02%], [0.665%], [0.755%], [QMC],
  [Scalar double box], [0.246%], [0.262%], [0.233%], [MC],
  [Physical double box], [1.98%], [2.09%], [2.14%], [QMC],
  [Physical double box], [0.594%], [0.662%], [0.648%], [MC],
)
These are relative standard errors of the complex magnitude, corrected to 600
seconds with the stated square-root display convention. Native timed durations
are 600.11–604.79 seconds. The tuned physical results recover performance close
to the fixed map; the scalar QMC results improve on it. This is a demonstrated
improvement over the original dynamic settings, not evidence of a universal
dynamic advantage. All eight confirmations have complete production coverage,
no failed, unstable or discarded-zero points, and no norm-linearization warning.

*Why the caps matter.* The original displacement bound can control the radius
when dimensionful gradients are large. Increasing $R$ while decreasing $L$
changes both the allowed strength and its spatial derivatives. For the polynomial
construction, sufficiently small $L$ approaches $lambda=S L$ at fixed regular
coordinates when the other bounds become inactive. The sign-aware smoothing
introduces a small limiting correction, retained in the native tests. Neither
statement certifies an arbitrary fixed-strength contour globally.

At three native scalar-sector test points, $L=1.25 times 10^(-6)$ and $R=10^6$
give $lambda/(S L)=0.9792,0.9915,0.9985$ for the polynomial bound. The Jacobian
still includes the strength gradient. At the tested midpoint, the larger scalar
QMC cap instead permits displacement 1.20, versus 0.742 in the old setting;
it is not simply a copy of the fixed map. These are coordinate diagnostics,
not a claim about every sector or the largest observed integration weights.
The saved native weight summaries do not retain the coordinates of those maxima.

*Independent work controls and compatibility.* Three scalar QMC controls each
completed exactly 71,958,528 accepted calls. Tuned polynomial/fixed covariance
trace is 0.193; sign-aware/fixed is about 0.224. Thus the scalar improvement also
appears at equal accepted work, using the full complex error rather than just
radial relative uncertainty. These independent estimates are not pooled with
the timed runs. Three physical QMC controls completed exactly 179,896,320
accepted calls each: fixed, polynomial and sign-aware took 377.81, 406.92 and
328.12 seconds. Their covariance-trace ratios to fixed are 1, 0.9904 and 0.9790.
The physical tuning therefore recovers approximately the fixed per-call variance;
these independent single controls do not establish a statistically significant
variance advantage. Scalar fixed, polynomial and sign-aware controls took
112.49, 145.95 and 136.80 seconds at their common work target.

All six MC work controls stopped at their original 600-second caps, with
40–41 million scalar or 20–21 million physical accepted calls. They remain
explicitly incomplete requested work; there was no retry to force equal counts.
All twenty predeclared final jobs are closed without failed, unstable or
cutoff-zero evaluations.

For the secondary overview column, completed work controls receive the small
explicit correction to 72 million scalar or 180 million physical calls. The fresh
fixed QMC controls supply the corresponding secondary baseline values; original
timed baseline results remain unchanged. Otherwise the timed estimate is
scaled to the designated 72 million scalar or 180 million physical calls using
the stated MC-equivalent heuristic. This does not relabel unfinished allocations
as equal-work measurements. The final ten-minute column is the primary metric.

Native Numerica matrix checks compare the 12 retained baseline estimates, eight
new timed estimates and six completed QMC controls: 234 complex-block
comparisons. The largest descriptive Mahalanobis distances are 3.169 for scalar
$epsilon^0$, 3.805 for physical $epsilon^(-1)$ and 3.280 for physical $epsilon^0$.
There are no non-positive covariance or rounding-sensitivity flags in this set.
Component checks are also retained. These estimated-covariance diagnostics are
not calibrated p-values or an independent analytic reference for this diagram.

The compact record `docs/double-box-dynamic-tuning-overview.json` preserves exact
native arguments, independent seeds, elapsed times, accepted/total calls,
full Laurent means and covariance, $C$, maximum-weight ratios, memory and source
identities. The completed work controls and final 234-pair audit are in
`docs/double-box-dynamic-work-overview.json`. Both use frozen executable source
`6647950` and the original D05 graph, model, parameters and generation cards.
Only runtime contour caps change.
