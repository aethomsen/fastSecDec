#set page(margin: 20mm)

= Repeated dynamic-candidate observations

Independent scientific, safety and ecosystem-reuse review, 16 October 2026.

== Scope and observed failure

One admitted dynamic-root `Bundle` can be evaluated more than once during one
native vector evaluation. Equivalent arithmetic paths can produce different
floating-point centres. Requiring their exact equality in the observation layer
rejects the evaluation before the existing independent certificate can examine
each value. The captured initial binary64 conflict differed by two ULP; a
terminal retry also showed distinct centres within the same 3322-bit domain.
The latter is not evidence that two precision levels were accidentally mixed.

The correction changes observation bookkeeping only. It retains each distinct
exact `(Rational centre, precision bits)` pair for its original request, and
passes every retained pair to the existing certificate machinery. It does not
average, replace, select, clip or memoize numerical return values. Exact
duplicates share a certificate obligation; approximate duplicates do not.

== Scientific acceptance boundary

`Validation::certify` still visits every saved source/face binding of every
retained candidate. `Check::evaluate_projected` reconstructs independent
coefficient enclosures from the saved native arithmetic, and `radius::certify`
checks both conditions:

+ The actual strength lies strictly inside the sufficient causal radius.
+ The relative enclosure of the intended root meets the unchanged precision
  policy. A smaller but inaccurate strength is not accepted merely because it
  is causally safe.

The exact binary value is supplied to these checks. Native outward-rounded
`RealBall` and `ComplexBall` arithmetic remains responsible for enclosures.
The proof is tied to the bound runtime point, source projection and face; no
candidate is certified against a different request. A missing projected axis
continues to mean uniform certification over its full closed interval, not an
invented coordinate.

All candidates must pass before accepted coverage is published. Invalid,
unsafe, unresolved or capacity-exceeding work fails the attempt and follows the
existing retry/error path. The retained collection does not establish a
global contour certificate, integration convergence, or equivalence of a
different deformation prescription. Finite-precision consistency of the full
numerator/Jacobian evaluation remains the numerical stability policy's job.

== Ownership and bounded work

The private limit is 1024 distinct candidates per admitted request and per
attempt. Equality is tested before the bound, so repeating an identical pair
at capacity consumes no additional slot. Overflow is a sticky error, not
truncation. This is an operational bound; it is not a claim that native
callback multiplicity is mathematically at most 1024.

The existing `Requests` owner supplies request indices. Flattening candidates
preserves those indices even when the input insertion order differs from
the `BTreeMap` key order. Missing and unknown requests remain errors. The
existing thread-local `Attempt` guard restores outer state on normal return,
failure and unwinding, preventing lower-precision retry observations from
leaking into another attempt.

Unchecked factories still bypass exact-centre conversion and observation.
Direct native Atom evaluation retains its existing lazy-branch/function-cache
contract. No public settings, persistence schema, integration stream, worker
pool, contour cap or validation threshold is added or changed.

== Native ecosystem reuse

The reviewed dependency owner is Symbolica/Numerica revision
`ae0c82b7bda670fdb9f6229226bd47b13bb49a20`, as pinned in `Cargo.lock`.

- `CandidateNumber` already uses native `Rational::try_from(f64)`, the exact sum
  of both `DoubleFloat` limbs, and `Float::try_to_rational`. The owner APIs were
  checked in Numerica's `domains/rational.rs` and
  `domains/float/multiprecision.rs`; no replacement conversion is needed.
- Existing `ExpressionEvaluator` mapping and saved native coefficient/root-level
  programs remain the arithmetic owners. The correction does not introduce a
  solver, symbolic rewrite, derivative engine or alternate CAS.
- Existing `Requests`, `Attempt`, `Validation`, `Check` and `radius::certify`
  already own association, lifetime, all-context certification and error
  propagation. A standard exact ordered set inside that owner supplies the
  missing one-to-many observation relation; no parallel abstraction is needed.
- FastSecDec's Python contour views in `bindings/python/src/contour.rs` remain thin
  adapters around the same native settings and `KernelSet`. HEPKit/Linnet graph,
  numerator and routing owners are untouched.

The focused executable regression reuses Symbolica's public `FunctionMap` and
`InliningPolicy::Never` to retain the exact identity `(x + 1024) - 1024` alongside
`x`. At the binary64 input `x = 1.3`, where their arithmetic paths round differently, it
requires distinct candidate centres and unchanged returned output values.
This exercises the missing observation relation without a substitute solver
or a test-only numerical evaluator.

== Validation and limitations

The acceptance checks cover exact deduplication, distinct centres and precisions,
the explicit bound, duplicate-at-capacity behaviour, missing/unknown requests,
and nested retry/unwind isolation. The all-context regression composes two
saved native descriptor contexts by the existing clone/remap/merge operations
and requires all six obligations for three distinct observations. Negative
controls reject the whole attempt when another candidate is unsafe or fails
intended-root accuracy. Existing vector, batch, clone and precision-remapping
checks remain relevant.

Independent source, scientific and ecosystem-reuse review found no blocking
issue. The final focused harness passed 59 tests with zero failures: 32 under
`contour::functions::dynamic::`, 25 under `kernel::contour::dynamic::`, and two
under `generation::program::tests::lowering::requested::`. Each filter ran with
`--test-threads=1`. The harness was built with
`cargo test -p fastsecdec --lib --release --locked -j1 --no-run
--config profile.release.package.fastsecdec.opt-level=0`; the configured release
profile retained ThinLTO and release dependencies. This is a correctness test
build, not a performance measurement. The retained test record verifies all
five reviewed source hashes and the executed harness hash
`b85cdab7a2759420ab20d0ca2949acf63f6d74f37d732e17e3b82252db9b9fe1`.

Strict production-library Clippy passes with
`cargo clippy -p fastsecdec --lib --features threshold-decomposition
--release --locked -j1 -- -D warnings`.
Strict workspace/all-target Clippy did not pass on Rust 1.99.0: unchanged CLI
code triggers deprecated `Atomic::fetch_update` at `process/worker.rs:85` and
`clippy::chunks_exact_to_as_chunks` at `process/sampling_tests.rs:47,49`.
Both files were verified unchanged from baseline
`559416f7a82d4d8b80d27d3df44c2fe76362e10a`; this correction does not suppress or
alter those diagnostics.
Strict library/all-test Clippy also encounters eleven occurrences of
`clippy::chunks_exact_to_as_chunks` in six unchanged test files. These files
were likewise verified against the baseline; no strict all-test lint pass is
claimed.
The scoped library/all-test rerun passes with the same command plus `--tests`
and the sole allowance `-A clippy::chunks_exact_to_as_chunks` after
`-D warnings`. All other warnings remain errors; no source-level suppression
or unrelated cleanup was added.
`cargo fmt --all --check` and `git diff --check` pass.

Pointwise success of a small-cap physical probe,
including a 16/16 probe, does not establish complete-integral convergence or a
variance improvement. Previously accepted physical results are not recomputed
or relabelled by this bookkeeping correction.
