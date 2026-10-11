# Mixed finite numerator coefficients

`MixedNumeratorFace` retains a genuine `NumeratorFace`, its original chart,
source catalogue and selected algebraic branches. It supplies lower-normal
Taylor coefficients of the already cleared numerator coefficients. The
existing clearing-normal root-independence requirement remains in force.

For clearing factor `c^k D` and formal cleared valuation `v`, the exact identity
is `c^k D N(phi) = c^v D sum_j c^j A_j`. The new coefficients describe
`G = c^(k-v) N(phi)`, with the original source record, clearing order and
valuation retained. A coefficient with lower-normal order `m` and clearing
order `j` corresponds to the derivative multiplied by `m! j!`. A finite set of
zero coefficients never establishes a new valuation.

## Reuse and joint-face admission

Symbolica performs native series expansion of tagged finite coefficient calls,
including the actual finite coordinate maps and selected-root dependence,
before restricting the lower normal. Original-coordinate partial derivatives
are registered only when the native series demands them. The existing normal
admission, definition collector and staged evaluator compiler are shared; the
old normal API retains its original arguments and behavior.

Every required finite argument and inverse direction divisor is admitted at
the joint face, retaining the selected-root extension and nonzero denominator
proof. Remaining directions are open. Caller aliases execute in the scalar
coefficient stage; source coordinates, both normals, root roles and geometric
maps remain protected. Literal integer powers preserve native evaluation at
zero. Native complex evaluators, precision tracking and serialization retain
their live callback/source owner. No CAS, root solver, numerical endpoint AD,
execution pool or serializer is introduced.

## Validation and limits

All 19 related numerator tests pass: five mixed-face groups and 14 inherited
source, face, selective-input and normal-jet groups. Controls include exact
clearing, 128/256-bit values, native restoration with the live scope, explicit
factorial comparison against independent native etale derivatives, a nonzero
cubic moving-root term, nontrivial direction divisors, joint-pole refusal,
alias isolation and typed source/normal/resource failures. A genuine case with
`v=1`, `k=0` gives `G=u sqrt(1+u^2)`, preventing accidental use of the simpler
`v=0` identity. Independent source, mathematics and ecosystem reviews pass.

Exact-source native library and test Clippy pass with warnings denied, as do
formatting and patch checks. Initial test-fixture homogeneity and import errors
were corrected before the final gate; the valid fixture uses the native
projective preparation and verification path.

The consuming workspace also passes `cargo clippy --workspace --all-targets
--features threshold-decomposition --locked -- -D warnings` (25.39 seconds)
and `cargo fmt --all --check` after applying the independently reviewed source.

This first API consumes original transport charts. It does not silently adapt
child charts that mix the clearing normal, establish complete physical U/F or
measure regularity, prove a common regulator strip, or issue a continued
physical density. Whole-cell subtraction and integration remain separate gates.
