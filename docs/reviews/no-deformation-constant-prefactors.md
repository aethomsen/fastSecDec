# Exact constant coefficients in threshold prefactors

The physical ggHH input has 156 prefactors containing exact pi and real square
roots in their epsilon-polynomial coefficients. The previous rational residual
owner accepted only Q(i), so these valid source prefactors stopped preparation.
The new constant-polynomial residual preserves the original Symbolica Atoms and
their exact Gamma associations. It does not approximate the numerator or replace
constants with rational surrogates.

## Native reuse and admission

The API/source/probe audit found the required operations in the adopted
Symbolica/Numerica revision `ae0c82b7`: `AtomField` polynomial conversion,
exact coefficient evaluator export, `ComplexBall`, and directed real-ball
square roots. Conversion explicitly disables statistical zero testing and
verifies exact polynomial recombination. Pi and principal real square roots
use native ball enclosures through owned evaluator arguments; exact rational
coefficients remain exact until their directed conversion to balls. Unsupported
IR instructions or rounded constant-function fallbacks are refused.

This is admission and orchestration around those facilities, not a new CAS or
interval-arithmetic implementation. The supported class is rational complex
constants, pi, integer powers and square roots of certified nonnegative real
constants. Every original maximal constant expression and denominator is checked
before epsilon-polynomial cancellation. Free parameters, uncertain input,
other transcendental functions, and regulator-dependent radicals remain outside
this class. An inconclusive ball returns an incomplete-resource diagnostic.

`ConstantCoefficientLimits` exposes a native precision setting, default 256 bits,
separate from the existing persisted meromorphic limits. The original rational
path runs first; only an unsupported-class result permits this alternative.
Invalid input and resource exhaustion are not converted into success. Balls
certify definedness; original coefficient Atoms remain the numerical source.

The shared witness retains the actual rational subset and an explicit map back
to source term indices. It checks regulator identity across both residual
variants and all original Gamma and positive-scale conditions. An entire
epsilon polynomial adds no pole exclusions; its zeros are allowed.

## Evidence and limits

Six new controls cover exact complex coefficients, nested constant branches,
denominators, cancellation before conversion, precision/resource refusals,
foreign regulator roles and mixed rational/constant witnesses. Five inherited
prefactor and two epsilon-scale controls pass. Independent source, consumer API
and final execution reviews pass. The actual source-only probe admits all 156
full prefactors in 0.456 seconds, preserving every Gamma association, with a
common witness 419/628 in (1/2, 1). The independent reviewer recomputed each
Gamma argument at that exact witness and checked positivity.

That witness lies within the earlier necessary endpoint diagnostic and the
radial Gamma convergence conditions. It does **not** certify an unconstructed
full physical endpoint atlas, its common strip, or the integral. Complete
double-box generation and sampling remain separate unfinished gates.

The adopted arithmetic is unchanged. The separately published
[Numerica PR #11](https://github.com/symbolica-dev/numerica/pull/11) documents and
tests the existing pi enclosure; it is not a dependency update or a claim about
unadopted square-root implementations. Native objects and error boundaries stay
in FastSecDec; no Python implementation or caller-owned execution change is
introduced. No serialized schema changes.

Private reproducibility evidence is retained under
`target/no-deformation-physical-prefactor-coefficients/`: final handoff digest
`d494bea42e390cd7eee60727f673ddfa998e40c99f692a51de79903bd7a7a34e`,
independent final review digest
`5f569d95c62796ec3636d4437589e236620cafa267132751d232933dd0bfa829`.
Raw outputs and historical binaries are untracked.

After importing the four reviewed files, the consuming workspace's complete
threshold unit-test selection passes: **433 passed, zero failed, six existing
ignored tests**. Strict core/all-target Clippy with threshold decomposition
enabled and workspace formatting both pass. Validation commands:

```sh
cargo test -p fastsecdec --features threshold-decomposition --locked --lib threshold:: -- --test-threads=1
cargo clippy -p fastsecdec --features threshold-decomposition --all-targets --locked -- -D warnings
cargo fmt --all --check
```
