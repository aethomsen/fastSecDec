# Exact graph export and threshold implementation status

This preparation lives on the explicitly authorized `no_deformation` branch.
For this work, that instruction supersedes the older main-branch publication
and reviewer guidance in `AGENTS.md`. FastSecDec remains a separate repository;
consumers may optionally clone this branch and pin a tested commit. No copy of
FastSecDec is vendored into symGCAD.

The exporter documented here produces exact native graph U/F and provenance.
Export itself does not perform integration or supply a sign/coverage certificate.

The ongoing native implementation is governed by
[the approved threshold plan](../NO_DEFORMATION_PLAN.md). Its optional
`threshold-decomposition` library feature now provides caller-driven native
solve/verify requests, compact projective preparation, causal cell phases,
composed linear cell maps and durable proof staging. A restricted, certified
rational-fiber continuation also reuses symbolic subtraction and has a native
above-threshold bubble comparison with OneLOop. The corresponding CLI path now
supports fixed one-dimensional rational cells, indexed artifacts, ordinary and
serial execution, and recovery. A bounded automatic route also supports regular
algebraic families on the unit cube, as detailed below.
Fixed affine-projective inputs with at least two integration coordinates also
have a [resolved-family route](reviews/no-deformation-resolved-projective-family.md):
constant algebraic bounds on the first CAD axis, rational linear bounds on every
later axis, and independently replayed residual margins on every closed chart.
The [dimension-general route audit](reviews/no-deformation-generic-projective-route-audit.md)
covers the complete triangular prefix, every half-cube and orientation.
The original massive-triangle graph card generates 18 kernels and its saved
artifact's full Laurent vector agrees with OneLOop in a native QMC control.
The [massless sunrise](reviews/no-deformation-scalar-sunrise.md) generates six
kernels and agrees with its analytic Laurent vector, including its imaginary
finite part. The [massive box](reviews/no-deformation-scalar-box.md) now generates
54 kernels from its native graph card and agrees with OneLOop after fresh-process
restoration. Its explicit native solver order is recorded in the runnable
[`box_no_contour.toml`](../examples/no_deformation/benchmarks/overview/box_no_contour.toml).
This is a sufficient class, not the general algebraic endpoint resolver.
The [runnable examples](../examples/no_deformation/README.md)
and [CLI review](reviews/no-deformation-cli.md) describe that supported subset.
These capabilities are separate from the exporter contract documented below.
Explicit single-chamber runtime templates now reach the CLI, including normal
and serial generation/integration, point rebinding and checkpoint identity checks;
see the [parameterized examples](../examples/no_deformation/parameter_templates/README.md).
The general endpoint resolver, broader parametric chambers, complete HEPKit
generation interface and broader physical suite remain under development. A
verified GCAD result alone is not an executable integral. See the
[reuse and acceptance ledger](REUSE_AUDIT.md#no-contour-foundation-and-consolidation-2026-10-10).

## Algebraic boundaries beyond quadratics

The native map interface retains a boundary as its exact polynomial and a
certified real-root selector. It does not require a formula in radicals.
Prepared polynomial evaluators and native bracketed refinement evaluate the
selected root; symbolic implicit differentiation supplies its derivatives.
For a regular branch `P(x,r(x)) = 0`, the first derivative is
`dr/dx_j = -P_x_j/P_r`, with higher derivatives obtained through Symbolica.
Composed map programs keep these root calls separate from the full density.

The regular-section controls include degrees 7, 11 and 17, and a complete
fixed sextic three-cell family now has an [automatic preparation and CLI control](reviews/no-deformation-regular-family-factory.md).
All four normal/serial generation–integration combinations pass, retaining the
complete Laurent vector and covariance. The [narrow-bracket arithmetic correction](reviews/no-deformation-affine-root-brackets.md#source-derived-constant-coefficient)
uses exact source-derived coefficients and preserves native error tracking.
This factory supports complete fixed unit-cube families with two or three cells;
it does not yet provide automatic generation for arbitrary algebraic cells.

Root collisions and vanishing `P_r` need the general algebraic endpoint resolver
and branch normalization; they cannot use the regular implicit derivative above.
Uniform branch selection, every subtraction face and the entire cell inventory
remain proof obligations regardless of polynomial degree. See the
[regular-section review](reviews/no-deformation-regular-sections.md) and
[general resolver contract](NO_DEFORMATION_RESOLVER.md).

## Native interface

`fastsecdec::input::GraphSymanzik::from_graph(&graph, parameters)` delegates to
HEPKit's `IntegralFamily::symanzik`. Parameters follow
`GraphIntegral::propagator_edges()` in stable native denominator order. Native
HEPKit owns graph parsing, momentum routing, external Gram substitutions and
U/F determinant algebra. Graph powers remain positive integers. Pinched or
signed sectors belong to the separate family preparation API and are not
silently introduced here.

U/F do not depend on numerator, positive propagator powers, or measure factors.
The new interface therefore accepts loop-dependent numerators without
contracting them. It rejects a singular U and exposes zero F as algebraic data;
the CLI explicitly rejects zero F rather than silently omitting its split.

## Exact export

```sh
cargo run --locked -p fastsecdec-cli -- export-symanzik \
  examples/no_deformation/bubble.toml --output output/bubble-symbolic
# Set a runtime invariant with an exact rational, not integration's f64 defaults:
printf '[parameters]\ns = "5"\n' > /tmp/bubble-point.toml
cargo run --locked -p fastsecdec-cli -- export-symanzik \
  examples/no_deformation/bubble.toml --point /tmp/bubble-point.toml \
  --output output/bubble-point
```

The supplied massive bubble has m²=1 and the point s=5 crosses physical F=0
sheets. `sunrise.toml` / `sunrise-point.toml` provide the two-loop m²=1, s=10
case. Export does not read `[integration.parameters]`:
that numerical interface remains unchanged. The optional point file accepts
only integer TOML values or bounded strings `integer` / `integer/integer`.
Floating TOML values and decimal/exponential expression strings are rejected.
Point keys may be native symbol names or exported kinematic aliases.
Feynman coordinates cannot be point-bound. Missing kinematic bindings remain
symbolic; no Euclidean or physical region is inferred.

The fresh output directory contains `symanzik.json` and `problem.toml`. Existing
directories are refused. The JSON names and hashes the companion TOML. The
ordinary symGCAD problem splits **F with its exact sign and rational scale**,
requires each Feynman parameter positive, and puts remaining kinematics before
Feynman axes. Additional `--kinematic-constraint 's > 4'` options explicitly
specify strict polynomial inequalities in native names or exported aliases.
The native parser converts their sides exactly to the wire map; ambiguous
native-name/alias spellings, undeclared variables, functions, weak comparisons
and oversized expressions are rejected. Fully qualified native names resolve
alias ambiguity. No graph metadata is a proof.

## Version 1 contract

The schema tag is `fastsecdec.symanzik`, version `1`, stage
`pre_sector_projective`. `symbols` gives distinct ASCII aliases, fully qualified
native symbols, and roles `kinematic` / `feynman`. Kinematics precede Feynman
parameters; Feynman order is native denominator order. `graph.propagators`
binds each `denominator_index` exactly once to its stable `edge_id`, parameter,
endpoints, power, and native denominator. Stable DOT, native routing, external
Gram products, admitted scalar bindings and uncontracted weights are retained.

`symbolic.{u,f}` and `specialized.{u,f}` use the **same complete alias map**.
A polynomial is `{variables, terms, expression}`. Each term has a nonzero
signed rational `coefficient` string and a complete unsigned `exponents`
vector. Terms are sorted lexicographically by exponents. Expression spelling
is deterministic: `(coefficient)*alias^exponent`, with factors in map order,
including exponent one, joined by `+`; zero is `0`. No primitive-part
normalization is applied. Consumers reconstruct sparse terms independently
and need not parse arbitrary expression syntax.

`substitutions` records the exact rational point. Consumers check exact
symbolic-to-specialized equality and companion Problem equality, including
constraints, groups and fixed parameters. The producer checks nonzero U/F and
termwise Feynman degrees `deg U = loops`, `deg F = loops + 1` before and after
the point. Exact cancellations are retained. Unsupported nonrational
coefficients/functions are rejected.

`domain.source = projective_simplex` records the native parametric origin;
`domain.analysis = positive_orthant` is the homogeneous sign-analysis domain.
Positive rescaling preserves U/F signs and zeros; no simplex-to-CAD output
transport or integration Jacobian is implemented. Graph provenance, source
BLAKE3 hashes, producer source identity and dependency revisions are retained.
No primary-sector or bounded-box map is silently applied.

The first CLI version caps input files at 32 MiB, output at 32 MiB, maps at
64 symbols, sparse polynomials at 100,000 terms, and native admission at
32 propagators / 8 loops. Algebra still needs an external wall/memory limit.
The API itself remains caller-driven and creates no worker pool.

## Optional independent consumer test

The Linux integration test is ignored by default and adds no symGCAD Cargo
dependency. Set an explicit tested executable and run it with an external
memory/CPU limit:

```sh
SYMGCAD_BIN=/absolute/path/to/symgcad \
  cargo test --locked -p fastsecdec-cli --test symgcad_interop -- --ignored --nocapture
```

It invokes the real native massive-bubble exporter, checks the bundle through
`import-fastsecdec`, applies only operational solve limits (30 s, 2 GiB, one
worker), solves the original sign problem and invokes independent verification.
A common deadline defaults to 180 s (`FASTSECDEC_INTEROP_SECONDS`, at most
600 s). The supervisor retains fresh artifacts and kills its owned process
tree on expiry. `FASTSECDEC_INTEROP_OUTPUT` can name a new evidence directory.
This acceptance test is outside all CAD benchmark timings. Neither successful
interchange nor sampled signs substitute for full solver verification.

The [A446 fixture](../examples/no_deformation/a446/README.md) supplies the
intended dressed HEPKit graph, a native Standard Model preparation command,
and an analogous ignored `a446_symgcad_interop` test. It checks the exact
signed U/F correspondence and edge assignment before importing, then requires
eight independently verified cells carrying both F signs. Its only changes
to the checked problem are explicit solver configuration and resource limits.

The first validated milestone passed 24 native graph/input tests, eight
existing CLI admission tests, six export tests, one native model-preparation
test and two deadline-supervisor regressions. Actual fixed-bubble and A446
process workflows passed with three and eight verified cells respectively.
A separate symbolic bubble with the explicit constraint `s > 4` also passed
with three verified cells. Formatting and scoped core/CLI/test Clippy checks
with warnings denied passed. These checks prepare graph-driven sign analysis;
they are not no-deformation integration results. Exact evidence hashes and
limitations are recorded in the [independent review](reviews/no-deformation-interface.md).
