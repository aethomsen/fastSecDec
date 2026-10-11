# Generic-degree roots with constant rational brackets

The explicit `AffineRationalBracketV2` recipe removes the old requirement that
a certified physical root bracket have exactly representable f64 endpoints.
It retains the original polynomial and certified selected branch. For a
constant bracket `[a,b]`, the numerical solver works with

\[
z=a+(b-a)s,\qquad Q(s)=P(z),\qquad 0\le s\le1.
\]

Symbolica's native evaluator composer prepares `Q`, `(b-a)P_z`, and `z`
together. Native bracketed refinement finds the selected root. There is no
radical formula or quadratic restriction, nor a full substitution of the
integrand. Degree, coefficient size and transport remain resource bounded;
the degree-5, degree-7 and shifted degree-17 tests are controls, not dispatch
special cases.

## Derivatives and numerical meaning

The callback returns the physical root. Symbolica differentiates its original
coefficient dependence through `partial z / partial c_i = -z^i/P_z`, including
higher chain-rule derivatives. The internal affine normalization adds no
integration Jacobian. Its final complex sensitivity converts `-P/Q_s` back
to physical units by multiplying by `b-a` exactly once.

Preparation also derives `P(a)` and `P(b)` from the same exact coefficient
expressions. These arithmetic controls prevent cancellation at closed faces;
they are not independently supplied mathematical variables. Only exact
zero-error solver coordinates select them. Coordinates merely centered at
zero or one retain their full uncertainty and use `Q`. Rational conversion
uncertainty and incoming real and imaginary uncertainty remain tracked.

`NormalizedFamily::admit_affine` explicitly chooses this recipe. Existing V1
construction and saved kernels retain their behavior. A distinct callback
head and version check prevent mixing owners or argument layouts. Inspection
reports V1, V2 or mixed helper inventories. Mathematical identity includes the
recipe and branch contract; a separate digest binds executable bytes.

Fresh native restoration imports the saved optimized program without GCAD,
root isolation, symbolic generation or Horner/CPE. Backend translation may
still occur. Arithmetic restoration does not import a geometric proof.

## Reuse and measured representation choice

The API, source and executable audit uses native Symbolica composition,
differentiation and bracketed refinement, plus native Numerica arithmetic and
error tracking. No separate solver, CAS or AD was added. Exact endpoint
normalization is confined to the finite geometry coefficient expressions.

For the degree-33 arithmetic probe, expanded, factored and composed helpers
have 1,271, 102 and 164 instructions respectively. Nevertheless, complete
620-query f64 runs take 0.571, 0.0338 and 0.0145 seconds. Arithmetic ordering
affects the native numerical-zero stopping decisions; fewer instructions alone
do not predict root-solving cost. The measured choice is native composition.
These are small unoptimized diagnostic runs, not production throughput claims.
All three forms pass the same fixed tolerances and tracked-error comparisons.

## Acceptance evidence and remaining scope

The coordinator verifies all 51 source/evidence digests and 12 source preimages
in handoff `94f258a1495385ebf3baacd744137f00ff873e77230aea3adaa86c12fa851702`.
Independent resolver, ecosystem and coordinator reviews accept the scoped design.
Six maintained native controls pass, covering rational closed faces,
nonmonotonic queries, first and second implicit derivatives, uncertainty,
version/arity failures, mixed inspection and native artifact restoration.
The final default-native child has no GCAD dependency and restores both new
and historical programs, including selective loading and cache refresh.

The complete quintic control uses 8,192 matched QMC points. V2 and V1 have
bit-identical six-component means and all 36 covariance entries; independent
checks cover pole cancellation and the finite imaginary value `pi*r`, where
`r^5+r=1`. They do not provide an independent oracle for every coefficient.
V2 takes 24.495 seconds with 5,766 rescued points, the same rescue count as V1.
This normalized fixture demonstrates parity, not a speedup. Native tracked
validation remains costly and is not an interval certificate. Science evidence
precedes only documented equivalent lint changes and an untriggered resource
preflight; final linked focused and restoration controls cover the frozen code.

The shared checkout passes all 12 focused algebraic-kernel controls in 51.42
seconds, including both complete quintic runs and the separately invoked
restoration child. All 88 portable tests pass. The portable build exposed an
unused 53-bit preparation wrapper whose four callers are native-only; adding
the matching native feature guard changes no execution path. Final strict
portable/all-targets lint passes in 82 seconds, and strict native workspace/
all-targets lint passes in 27.74 seconds. Formatting and diff checks pass.

Raw evidence and the representation comparison remain untracked under
`target/no-deformation-affine-root-v2/`; shared gate logs are under
`target/no-deformation-affine-v2-joined/`. The 12 frozen source postimages
remain unchanged; the feature guard is the coordinator's separate one-line
cleanup.

This is a generic-degree evaluator for an already certified regular branch in
a constant rational cylinder. Moving brackets, collisions, nested branch
towers, parameter strata, the full closed-face atlas and general algebraic
preparation remain separate obligations. At `P_z=0`, implicit differentiation
is not accepted as a substitute for resolution. Existing portable builds
retain their capabilities; root-bearing threshold execution in WASM remains
deferred.
