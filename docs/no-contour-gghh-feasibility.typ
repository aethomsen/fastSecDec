== Physical double box: initial GCAD feasibility trials
A follow-up tests the same physical D05 graph at $sqrt(s)=1000$ GeV before
attempting full no-contour generation. Native HEPKit/FastSecDec exports its
exact Symanzik geometry in about 0.2 seconds. It retains the graph and
numerator data, but these probes perform no numerator contraction, endpoint
resolution or integration. The fully symbolic export has 15 terms in $U$
and 80 in $F$.

The first solve keeps only $s,t$ parametric, with exact masses
$m_H=125$ and $m_t=345/2$. Its open physical chamber includes
$999000 < s < 1001000$, $-97100 < t < -97050$, $s>4m_H^2$, $s>4m_t^2$ and
$t(2m_H^2-s-t)>m_H^4$. Free mass parameters were not tested.

The fallback specializes *all geometry coefficients rationally*:
$
 s=1000000, quad t=-97077, quad u=-871673, quad
 m_H^2=15625, quad m_t^2=119025/4.
$
This differs slightly from the original
$t=-484375+100000 sqrt(15)$ at $cos(theta)=4/5$.
The new $cos(theta)$ is approximately $0.79999930881$;
on-shell conditions, momentum conservation and Gram products pass independent
native exact checks. Any later amplitude comparison must use this same point
and regenerate its external polarization data.

#table(columns: (1.8fr, .75fr, .8fr, 1.4fr), inset: 5pt, stroke: 0.35pt + rgb("cbd5e1"),
 table.header([*Geometry / order*], [*Solve wall s*], [*OS RSS MiB*], [*Outcome*]),
 [Parametric $s,t$, baseline], [60.037], [179.63], [Projection timed out],
 [Exact rational point, baseline], [180.043], [198.84], [Projection timed out],
 [Exact rational point, alternate], [180.037], [169.48], [Projection timed out],
)
These three initial scouts are single-worker native solver probes, not complete generation times.
RSS is the child process's OS-reported maximum from `wait4`; separate periodic
RSS observations are retained in the JSON. Build time and U/F export are
excluded. All three stops are imposed by the external wall deadline. The
process address-space bound is 10 GiB; observed memory is far below it.
Configured native time/memory settings do not cap eager projection in this
direct library path. An external supervisor enforces the stated limits;
raising the memory allowance alone would not address these observed stops.

Every trial uses symGCAD's optimized engine, positivity pruning and its checked
homogeneous reduction of the positive $x_0,...,x_6$ domain with anchor $x_0$.
The baseline requests coordinate lifting order; the alternative uses
$[x_0,x_5,x_4,x_3,x_2,x_1,x_6]$. Parameters precede integration variables.
The alternative is selected by a native cheap ordering heuristic, with no
unreported trial projections. The final completed projection records contain
15 polynomials / 406 terms for the parametric case, 22 / 1393 for the fixed
baseline and 25 / 1643 for its alternative. These are partial diagnostic
records, not verified cells.

*None of these initial trials returned a SolveResult.* They do not prove
mathematical infeasibility. The next page reports the successful independently
verified rational-point geometry obtained with a different native prescription
and a larger allowance. That result is separate from this initial table;
endpoint resolution and integration are still required afterward.

The runner links release-built public symGCAD `a1132d4f` and
Symbolica/Numerica `ae0c82b7`; its small standalone driver uses opt-level zero
and thin LTO. It is a separately labelled feasibility build, not the release
CLI executable behind the small-input tables. Exact settings, dependency
identities and measurements are in `docs/no-contour-gghh-feasibility.json`.
The native source/algorithm can be replayed from
`examples/no_deformation/gghh_feasibility/`. No later upstream production solver
changes were found when checking symGCAD head `ea9d1cf5`.
