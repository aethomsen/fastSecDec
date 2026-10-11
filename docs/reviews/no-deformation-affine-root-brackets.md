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

## Narrow-bracket limitation found during factory integration

A later automatic-factory control exposes a limitation of the composed V2
arithmetic which the earlier wide-bracket tests did not exercise. For
`P=1+x-16*z+16*z^6`, its certified upper bracket `[31/32,127/128]` fails at
`x=0.9999325137757114`. The helper rounds the physical image `z=a+(b-a)*s`
before evaluating `P`. Relative refinement in `s` can then demand finer
physical resolution than that evaluation provides. The native solver correctly
refuses stagnation instead of returning a midpoint or a fabricated zero.

The failure occurs in both ordinary and error-tracked arithmetic. A matched
448-evaluation diagnostic covers both roots, wide/narrow brackets and several
precision tiers; only narrow V2 fails. A separate exact-helper trace reproduces
the failed point through 4096 bits. Merely escalating precision is insufficient
because the coordinate-scale mismatch persists. V1 physical-coordinate
evaluation succeeds in these controls.

An exact Symbolica lowering of the small polynomial `Q(s)` succeeds at the
failed point with unchanged solver options. Its generic-degree cost, tracked
accuracy, saved-program compatibility and full integration still require
acceptance before replacing the measured representation above. The full density
need not be expanded. No native root-solver defect or accepted production fix
is established by this diagnostic.

The frozen diagnostic handoff is
`3c963f13e6cf68b8985e66897e19cd2c8b8c9fb28f300a9fd1955442f925c270`;
the coordinator verified all 24 source, evidence and executable pins. The
automatic normal/serial numerical matrix remains blocked on this correction;
generation success alone is not reported as an integration pass.

## Coefficient-shift successor

The bounded follow-up comparison distinguishes direct expansion from a
coefficient boundary between two evaluators. Write

`Q(s) = P(a + (b-a)*s) = sum_k q_k(c)*s^k`.

Symbolica prepares the exact `q_k` expressions using substitution and native
coefficient extraction, and verifies exact recombination. At runtime, one
evaluation computes the vector `q(c)`. A separate polynomial evaluator accepts
that vector as opaque inputs and evaluates `Q`, `Q_s` and the physical image.
Optimization therefore cannot reconstruct the rounding-sensitive physical
`P(a + (b-a)*s)` evaluation across this numerical boundary. Only this small
univariate helper is expanded; the composed integral density remains separate.

The frozen comparison runs 720 queries per representation. The historical
composed representation succeeds on 675, direct expansion on 705, and the
coefficient boundary on all 720. A fresh native arithmetic-pair process passes
378 further queries across ordinary, DoubleFloat and tracked arbitrary-precision
domains. These counts include repeated and reordered inputs. They are bounded
arithmetic controls, not a universal convergence proof or an interval enclosure.
The handoff and its 39 verified evidence/native-source pins are recorded in
[the reuse audit](../REUSE_AUDIT.md#normalized-root-coefficients-native-arithmetic-correction).

The polynomial body grows linearly with degree; the coefficient shift can grow
quadratically. At degree 33, the diagnostic body has 163 instructions and the
shift has 625. This motivates explicit preparation bounds before expansion.
The proposed versioned recipe saves both optimized programs and retains
derivatives with respect to the original coefficients, `-r^i/P_r(r)`.
Earlier V1/V2 artifacts keep their original recipes. Complete-kernel and
automatic generation/integration acceptance of the successor remains separate
from the representation study.

The private V3 implementation subsequently passes 14 maintained callback and
codec tests. A matched 8,192-point quintic comparison has exactly equal complete
six-component means and all covariance entries for V2 and V3, including 5,766
precision rescues in both. Independent controls check pole cancellation and
the finite imaginary part; the other entries have numerical parity coverage.
The observed 24.172/24.245-second timings establish no speedup. A fresh default
native child restores both complete-family backends and six root/derivative
archives without linking the GCAD dependency. Working-precision root comparisons
at 128/256 bits use exact native rational comparisons, not conversion to f64.

Automatic normal and serial sextic generation also succeed with equal artifact
identities. The subsequent first ordinary integration fails after 112
evaluations on tracked coefficient uncertainty, before accepting an estimate.
It is reproducible from the saved artifact at sector 0, point
`[1.9821906045953033e-6, 0.009410550281948912]`. No later integration lane is
reported as passed. This distinct failure remains under investigation, and the
private factory/V3 source was not adopted while this failure remained unresolved.

### Source-derived constant coefficient

The subsequent diagnostic locates the first rejection at 256 bits, before the
final tracking-range refusal: rebuilding `q_0` from separately rounded original
coefficients reduces its stored precision to 239 bits. Native Float arithmetic
intentionally reduces precision after cancellation. The normalized root then
stagnates under the unchanged solver target. This is not evidence of a native
solver or arithmetic defect.

Exact preparation already supplies `E_0=P(a)=Q(0)` from the same source
coefficient expressions. The narrow correction uses this entire native complex
tracked value for `q_0`, avoiding that repeated numerical cancellation. It does
not clear uncertainty, reset precision, change root selectors or relax solver
tolerances. The original coefficient derivative hooks remain applicable because
`E_0` is a source-derived dependent control, not an independent polynomial input.

Fourteen maintained callback and codec groups pass in 0.97 seconds, including
the captured failing coordinate, independent exact-fiber comparisons at 128/256
bits, non-dyadic closed faces and incoming real/imaginary uncertainty. Strict
library/test lint and formatting pass. A fresh default-native process restores
the saved families without GCAD; a second process evaluates the formerly failing
saved row and accepts its complete vector at 256 bits. These checks establish
the scoped correction, not full CLI acceptance or a general conditioning bound.

The coordinator verified two source pairs and 49 additional pins in handoff
`7f4779daff83c1765484a4f95899a2a9dab398b9e1ade71734139b6896acbaa5`,
and 56 diagnostic/native-source/executable pins in
`d9feeb9775992f5e35db906b6f1ee1c26ed493bd0aa2ae20840570baccf9b609`.
Independent reviews confirm the exact identity and preserved tracking. The
complete automatic-factory matrix was rerun separately against a fresh binary:
[all four lanes pass](no-deformation-regular-family-factory.md), including full
vectors and covariance. Earlier failures remain retained evidence.

The existing shift-output validity check still precedes replacement of `q_0`.
It can therefore conservatively refuse an invalid reconstruction even if its
replacement would be valid; no reachable instance was demonstrated in this
slice. Other shifted coefficients and genuinely uncertain source inputs can
still be ill-conditioned. No universal accuracy or runtime improvement follows
from this particular correction.
