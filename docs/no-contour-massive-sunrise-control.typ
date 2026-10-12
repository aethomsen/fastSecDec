= Massive sunrise: complete no-contour correctness control

This new control extends the no-contour implementation beyond the massless
sunrise in the opening benchmark table. It uses three masses equal to one,
$s=16$, $D=4-2epsilon$, and the complete native density
$-Gamma(2epsilon-1) U^(-3+3epsilon) (F-i 0)^(1-2epsilon)$.
The genuine projective source retains its original Gamma prefactor and causal
phase. All seven verified cells contribute through 62 closed endpoint charts.

#table(columns: (0.55fr, 1.225fr, 1.225fr), inset: 5pt,
 stroke: 0.35pt + rgb("cbd5e1"),
 table.header([*Laurent coefficient*], [*Real part*], [*Imaginary part*]),
 [$epsilon^(-2)$], [$1.4999998621 plus.minus 1.87 times 10^(-7)$], [$0$],
 [$epsilon^(-1)$], [$-1.2316465861 plus.minus 4.39 times 10^(-7)$], [$0$],
 [$epsilon^0$], [$10.0161411847 plus.minus 1.72 times 10^(-6)$],
 [$-3.3089881510 plus.minus 6.71 times 10^(-8)$],
)

Fixed contour, dynamic contour and independent pySecDec give compatible
complete Laurent vectors. All 18 component comparisons pass without an added
absolute-error floor; the largest difference is 2.162 combined standard errors.
The pySecDec finite reference is $10.0161430322940-3.3089881368544 i$.
The no-contour relative standard error of the complex magnitude is
$1.55 times 10^(-7)$, retaining the full real/imaginary covariance.

*Preparation and execution controls.* Fresh verification of the saved
proof, supplied-map admission, symbolic continuation and local publication took
12.38 seconds with a 60 MiB observed process peak. This excludes the original
GCAD solve. The saved universal archive is 1.60 MB. Cold restoration and
integration use that archive alone, without loading the original graph or proof.

The single-worker unoptimized Eager integration completed 16 independent
native HKKN-alpha3 shifts, each with 256 points per chart: 253,952 calls in
275.88 sampling seconds, with a 42 MiB observed process peak. Korobov3 and seed
202610124919 were fixed in advance. It recorded 133,764 precision rescues,
up to 256 bits, and no failures, unstable points or cutoff zeros. All 16
complete replica vectors and their 36-entry covariance were independently
checked. Earlier incomplete diagnostic allocations were not pooled.

An optimized Eager build repeated the same archive and points in *97.83848
sampling seconds* and *101.49132 outer seconds*, with a *24 MiB* observed peak.
All non-timing payloads matched exactly. A genuinely issued *SymJIT O2*
archive then sampled the same points in *97.32653 seconds* (*100.95468 outer*,
*30.18 MiB* observed peak): no material timing improvement in this single
comparison. All means and replica vectors agree; the largest covariance
difference is $1.22 times 10^(-24)$. The repeated points are correlated and
are never pooled or counted as additional independent data.

SymJIT preparation took 3.94 seconds with a 33 MiB process peak, including saved-proof verification and
publication of a new 2.05 MB archive, excluding the original solve. All 62
restored sectors retain their native SymJIT backend. The same 133,764 points
required rescue (52.67%); the ten slowest sectors used 62.94 seconds, seven
rescuing every point. These diagnostics do not establish the cause of rescue.

These clocks are not comparable to the optimized 25-core, approximately
ten-minute arms in the overview, and no square-root time extrapolation is
applied. Ordinary and serial CLI generation now produce all 62 charts, and a
separate cold process restores one selected record. Their clocks are outside
this control.
The opening overview now adds a separate fresh 25-core no-contour campaign:
complete CLI generation in 6.70 seconds, followed by independent QMC and
ordinary discrete-MC confirmations near 600 seconds. It does not reuse the
one-worker estimates above. Its compact record is
`docs/no-contour-massive-sunrise-overview.json`; matching fresh contour timings
remain pending.

The general algebraic resolver, mixed-mass kite and complete double-box
no-contour results remain unfinished.

Compact results and exact settings are retained in
`docs/no-contour-massive-sunrise-control.json`. The native no-contour CLI card is
`examples/no_deformation/massive_sunrise.toml`; the contour reference card is
`examples/no_deformation/benchmarks/controls/massive_sunrise.toml`.
