# Native massless sunrise without contour deformation

The original massless sunrise benchmark now generates and integrates through the
complete resolved projective family. The native HEPKit graph, exact point,
propagator powers, dimension `4-2*eps` and unit measure multiplier are unchanged.
This is the CLI's native graph-to-parametric source path; it does not claim the
separate strict `GraphPoint` preparametric owner identity.

The first run exposed a native algebraic-context conversion defect: an entirely
rational field could use cached zero as a preferred generator and form `0^0`
when converting a newly produced constant coefficient. The preserved native
reproducer forms `2*(1+x)` in a trivial `AlgebraicContext`. FastSecDec now uses
Symbolica's existing polynomial `to_expression` for that trivial field. The
frozen `3cf81e` sunrise acceptance binary retained contextual conversion for
nontrivial selected fields. The subsequent box candidate instead uses native
`to_expression_with_coeff_map` and `element_to_atom`, preserving the selected
primitive field without simplifying each algebraic coefficient; its exact
round-trip, exact full-density box tests and saved-publication gates are recorded
separately. Actual box CLI acceptance remains pending its expression allowance
diagnosis. The final converter also rejects collisions with native root-polynomial
roles and mismatched selected coefficient fields. There is no replacement
algebra implementation. A genuine source-bound sunrise test
checks complete chart coverage, the Gamma prefactor, causal phase and common
regulator strip. The independently reproduced upstream correction is
[Symbolica PR 74](https://github.com/symbolica-dev/symbolica/pull/74); the adopted
FastSecDec dependency remains unchanged.

The native input has `U=x0*x1+x0*x2+x1*x2` and `F=-x0*x1*x2`. One verified
projective CAD cell produces six complete endpoint charts. The original full
regulator prefactor and the lower-lip phase are retained before native symbolic
subtraction and Laurent expansion. Positive closed residual certificates and
the common convergence strip remain mandatory. This result covers the stated
massless source, not a massive sunrise with a general cubic moving root.

The frozen release candidate generated all six kernels in 0.488104 seconds
(native elapsed). A fresh process restored the saved artifact and evaluated
196,608 accepted QMC sector points, using 25 workers, eight shifts and a fixed
allocation of 4,096 points per shift and chart. Integration took 0.819608 seconds,
with artifact loading measured separately at 0.049532 seconds. The pole is
`-0.25000000000354/eps`; the finite coefficient is
`-1.33639216756958 - 1.57079632681714 i`. All four returned real/imaginary Laurent
components satisfy the predeclared comparison against the native Symbolica
analytic reference: absolute difference at most six reported standard errors
plus `2e-9`. The full covariance is retained; an existing Numerica exact-rational
matrix helper supplies a separate descriptive covariance diagnostic.

This was a short scientific acceptance run, not the ten-minute benchmark and
not tolerance convergence. It ended at the complete fixed allocation with
`WorkLimit`, no failed or unstable points, and no cutoff-to-zero samples. Native
precision rescue was active. Optional contour validation was off; required
geometry and endpoint certificate checks were still performed. Sampled own-job
RSS was 25.36 MB for generation and 392.58 MB for integration. Shared-cohort peaks
were 298.60 MB and 668.95 MB respectively; they include peer processes and are
not interchangeable with own-job measurements.

Private immutable evidence is sealed at
`target/no-deformation-projective-sunrise-actual/handoff.json` (SHA-256
`203956d4f3f28b7cd49c15ac3af651fd1355fe5bb3f153a16eb3a11c4a7caa3f`).
It retains the original conversion failure, unchanged source pins, generation
and integration commands, native results, full covariance, process/resource
receipts, comparison criteria and the upstream regression receipt. No raw
benchmark artifacts are tracked.
