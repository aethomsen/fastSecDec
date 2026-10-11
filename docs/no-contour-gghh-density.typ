== Physical double box: density and infinity preparation
The same exact physical input now passes an open-density check on original
cell zero, using all 156 source terms and four native infinity charts. The
complete 644,699-cell proof is loaded and verified once. Native evaluator
composition keeps the numerator in its original coordinates; the cell map
feeds it without expanding the algebraic roots inside the numerator.

#table(columns: (2.4fr, 1fr), inset: 5pt, stroke: 0.35pt + rgb("cbd5e1"),
 table.header([*Preparatory correctness measurement*], [*Result*]),
 [Open cell map preparation], [0.052 s],
 [Four-chart endpoint transport], [0.247 s],
 [Complete 156-term open-density preparation], [0.607 s],
 [Native exact density IR], [58,933 bytes],
 [Term / summed-density comparisons], [6,864 / 44; all pass],
 [Independent coordinate/Jacobian comparisons], [154; all pass],
 [Proof decoding / source-bound verification], [126.148 / 116.082 s],
 [Process wall time, including teardown], [254.308 s],
 [Sampled aggregate peak RSS], [11.522 GB],
 [Full integrand generation / integration / SD], [Pending],
)

The density checks use real epsilon values 1/8 and 1/4 at 128/256 bits. A
separate evaluator uses the original source factors, exponents and complex
numerator, with an independent exponential representation of the lower-lip
phase. Each chart's additional Jacobian appears once. The exact evaluator codec
roundtrip retains its live algebraic-root owner; detached restoration is a
separate gate. These are ordinary high-precision arithmetic comparisons, with
independent directed-ball checks of the coordinates and Jacobian.

This correctness driver uses the private combined FastSecDec build and captured
development dependencies. It is *not a release SymJIT sampling benchmark*.
The 1.931 s numerical-check stage also constructs reference/composed evaluators
and writes evidence; dividing it by the point count would not give sampling
cost. The test uses one process under 600 s / 100 GB limits. Its shared proof
owner establishes no cross-process residency bound.

For numerator behavior at the common infinity face, native finite-depth series
completes all *128 distinct numerators × four charts* in *20.602 s*, at
*38,588 KiB* child peak RSS. Twelve independent exact specialized coefficient
controls pass. Joining the original powers and measure gives $c^3$ for all
624 term/chart combinations, with epsilon cancelling. This calculation avoids
materializing the fully cleared numerator: an earlier expansion trial reached
roughly 9 MB for one 12 KB source and was stopped at 265.8 s / 6.65 GB.

*Endpoint regularization and full integration remain unfinished.* The common
infinity calculation does not resolve direction faces, lower $U=F=0$ strata,
finite causal seams or all other original cells. It supplies exact local
coefficients, never a truncated bulk approximation. No missing contribution
is treated as zero. Scope, source identities and compact measurements are in
`docs/no-contour-gghh-verified.json`; source and reuse reviews accompany it.

*Implementation checkpoint, `cf3cc91`.* Complete replacement chart covers,
original-factor obligations on every relative face, and compact symbolic
numerator derivatives through moving algebraic roots are now implemented.
Their maintained scientific checks, independent reviews and strict consuming
workspace checks pass. Exact boundary zeros remain pending refinement;
finite numerator coefficients are not a valuation or a complete density
certificate. The physical preparation measurements above are unchanged.
Full endpoint closure, common analytic continuation and integration remain
required before a no-contour double-box SD or runtime comparison can be given.
