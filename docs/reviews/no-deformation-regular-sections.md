# Certified regular algebraic sections and composed maps

`threshold::maps::regular` admits generic-degree implicit boundaries from an
actual verified GCAD cell. It constructs native composed map programs without
expanding the whole transformed density. It does not yet certify that density's
endpoints or produce a complete integration artifact.

## Exact admission

The caller proposes a rational root bracket and a positive derivative margin.
The native cell supplies the full polynomial, selector, lifting roles and
original domain. Each prefix bound is checked as a positive multiple of an
original one-variable affine constraint; later coordinates and target signs
cannot silently shrink that domain.

Existing symGCAD domain certificates establish derivative and endpoint signs.
Continuity extends the derivative margin to the closed cylinder, and weak
endpoint signs give a unique root there. Native exact root isolation at the
saved anchor associates it with the cell's actual selector. All these receipts
remain attached to the immutable source. Interior positivity or a sample's
isolating interval alone does not establish this admission. Unsupported
domains, collisions and insufficient margins remain explicit refusals.

## Native execution

Symbolica's `EvaluatorComposer` wires coefficient programs, root values, interval
widths and coordinate images together. Subsequent stages consume earlier slots;
they do not recursively substitute large nonlinear expressions. The full root
polynomial is evaluated, with a prepared polynomial/derivative program and
native `nsolve_bracketed`; no all-root search occurs during map evaluation.
Certified linear sections lower directly to native `-c0/c1` arithmetic, avoiding
both a root solve and a callback lock for constant or linear bounds.

For coefficient `c_i`, the callback's symbolic derivative is `-r^i/P_r`.
Symbolica differentiates the root and minor again for higher derivatives.
No numerical endpoint AD is introduced. Immutable helpers and scoped factories
own callback metadata, with clone-local mutable scratch and explicit error
boundaries. The numerical root result is not a certified interval.

Every width is checked separately. Interior widths must be positive; a closed
face may collapse, but that zero is a map value, not an endpoint-unit proof.
The triangular Jacobian is multiplied once. The existing projective delta gauge
contributes its certified unit measure.

## Review and evidence

Root and the independent resolver agent reviewed the equations, domain/selector
association, callback ownership, full chain rule, measure and failure paths.
Seven isolated native groups pass in 0.17 seconds, covering actual cubic/quintic
cells, all four cells of a nested example, first/second derivatives, f64,
DoubleFloat and 192-bit Float, complex callbacks, clone lifetime, native codecs,
invalid proposals, resource limits and cancellation. Strict isolated Clippy
passes in 26.425 seconds.

After import, the joined native threshold suite passes 188 tests with no failures
and one pre-existing ignored test in 29.10 seconds. Strict library/test Clippy
passes in 32.24 seconds; workspace formatting and diff checks pass. These gates
include the direct linear-bound lowering described above.

A further native control solves and verifies actual degree-7, degree-11 and
degree-17 cell boundaries. At four rational points, including both closed
endpoints, numerical roots agree with independently isolated exact fibers and
the first two symbolic derivatives agree with implicit differentiation. This
control passes in 0.09 seconds; independent scientific review, strict Clippy
(16.59 seconds) and formatting pass. The bracket is certified anew on `[0,1]`:
the quintic's narrower `[0,4/5]` bracket would not contain these higher-degree
roots throughout the domain. These are regular-section controls, not a claim
that singular high-degree integration is complete.

The supervised test process took 1.348 seconds with 30,109,696 bytes observed
peak RSS at 100 ms sampling. This is a small correctness control, not a sampling
performance result or a hard memory bound. Native certificate operations still
need caller-owned hard process limits.

API/source/probe checks reuse native polynomial substitution and division,
symGCAD domain/root facilities, Symbolica evaluator composition, callback
derivatives, root refinement and serialization. No replacement CAS, AD, root
solver, graph or numerical estimator was added. Ordinary linear cell APIs are
unchanged; HEPKit generation continues to own its existing native objects.

## Remaining boundaries

Map programs currently reject runtime-parameter axes pending their typed chamber
admission. Proof owners retain the full GCAD decomposition; detached sector
records must remove that residency before claiming bounded-memory integration.
The codec tests restore main and helper IR in the same process with the same
live proof owner. Fresh-process callback registration, saved proof admission,
whole-kernel tracked-error precision rescue and indexed integration artifacts
are separate gates.

Closed-face normalization, singular-root transport, common analytic regulators,
multidimensional symbolic subtraction and the general integration atlas remain
unfinished. The [secant-unit construction](no-deformation-secant-units.md) is the
reviewed next bridge for regular sections; its stronger interval conditions must
be checked independently rather than inferred from the root bracket.

## Complete degree-seven control

The later automatic preparation and saved-kernel path also passes the complete
unit-square integral of `(y^7+y-x-i0)^(-1-epsilon)`, through finite order. The
reproducible input is
[`regular_degree_seven.toml`](../../examples/no_deformation/regular_degree_seven.toml).
It produces two verified cells and two endpoint charts with full-integral scope,
then loads their saved programs in a separate CLI integration process.

For fixed `y`, native Symbolica verifies the primitive
`(y^7+y-x)^(-epsilon)/epsilon`. Its causal imaginary boundary value integrates
to `pi*r`, where the unique root satisfies `r^7+r=1` in `(0,1)`. Independent
native exact root isolation gives `2.5024178911883874`. The QMC estimate is
`2.502417893750741 +/- 1.1013230273037872e-8`; both epsilon-pole components
cancel. The complete real/imaginary vector and all 16 covariance entries are
retained. The finite real estimate has no independent oracle in this control.

The bounded run uses 1,024 points, four shared shifts, Korobov3 and two workers:
8,192 sector evaluations, 5,829 precision rescues, at most 256 bits, and zero
failed or cutoff-zero samples. The existing debug binary takes 0.816 seconds
for generation and 17.753 seconds for the integration command. These are
correctness-run wall times, not release performance or a generic-degree cost
claim. No native source or dependency changes were needed.

Independent scientific review accepts the analytic phase, pole cancellation
and scope. Raw commands, native oracle source, complete outputs and executable
identities are retained under `target/no-deformation-generic-degree-seven/`.
This extends the earlier degree-7/11/17 map controls to a complete degree-seven
regular integral. Colliding roots, changing polynomial degree, moving brackets
and general singular algebraic integration remain separate obligations.
