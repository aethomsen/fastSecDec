# Development

Implementation follows [FIRST_PHASE_PLAN.md](../FIRST_PHASE_PLAN.md). The
numerical product is Rust; the isolated FastSecDec Python binding crate is
linked by HEPKit's community extension. Historical Python/pySecDec references
remain outside the production dependency graph.

## Toolchain and ordinary Cargo builds

Use Rust 1.96 or newer. `nix-shell` provides a compatible Rust compiler and the
native C/GMP/MPFR build tools. The numerical workspace does not need Python,
FORM or a generated C++ integrand backend.

```sh
nix-shell
cargo metadata --format-version 1 --locked
cargo test --workspace --locked -- --test-threads=1
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo tree --locked --duplicates
```

On macOS, if `cc` resolves to MacPorts GCC, use Apple's Clang linker for Rust:
`CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=/usr/bin/clang cargo test --workspace --locked -- --test-threads=1`.
The GCC-linked test executable on our macOS validation host aborted even for a
minimal `catch_unwind`; relinking with Clang restored panic recovery. This is an
environment setting, not a reason to disable cancellation or worker-panic tests.

Cargo obtains dependencies from their public Git repositories and the registry.
There is no dependency-preparation script, generated path overlay or local
Symbolica source patch. The standard `[patch.crates-io]` entry selects the public
Symbolica Git revision for every consumer of the registry package; it does not
modify that source. Library dependency declarations remain registry-based so
the final consuming workspace owns this single shared Symbolica identity.

The numerical workspace, standalone portable validation package, standalone
Python binding package and community host each own their root manifest and
lockfile. Dependency-local Cargo patch tables are not inherited by consumers.
Use `--locked` for normal builds; deliberately update and review lockfiles when
changing dependency revisions. Old bootstrap-generated Cargo configurations
must not be carried into ordinary-source checks.

| Owner | Selected source |
|---|---|
| Symbolica | Public `community`-based fork revision `ae0c82b7bda670fdb9f6229226bd47b13bb49a20`, retaining native evaluator composition, ball domains, prepared roots, shared direct-vector evaluation, independent callback environments and single-pass elimination of dependent duplicate instructions, plus [fractional-series, F4 and resultant corrections](reviews/no-deformation-dependencies.md) and [checked native uncertainty range](reviews/no-deformation-numerical-range.md); [upstream contour PRs](reviews/contour-upstream-prs.md) |
| FeynKit / Linnet / Spenso ecosystem | Public fork revision `9f8b1e50016a58d04e712c34569ea936c44e179a`, retaining the native kinematics mapper in [PR #131](https://github.com/alphal00p/gammaloop/pull/131) and citation URLs in [PR #128](https://github.com/alphal00p/gammaloop/pull/128), plus the immutable numerator-prefactor setter in [PR #132](https://github.com/alphal00p/gammaloop/pull/132); [adoption evidence](reviews/no-deformation-represented-graph-payloads.md) |
| Numerica | Same public owner revision as Symbolica, with narrow tracked `hypot`, certified real-ball square-root fixes and checked uncertainty range from [PR #9](https://github.com/symbolica-dev/numerica/pull/9) |
| symGCAD | Optional native geometry dependency, public revision `a1132d4f4545c7784b3ec61239a05485e544b403` from [PR #1](https://github.com/alphal00p/symGCAD/pull/1); shares the consuming workspace's Symbolica identity |
| Graphica | Registry 3.0.1 |
| SymJIT | Compatible minimum `2.27.0`; public Git revision `d74993ffd76a6fc322a7bcf3963fa786783a38a8` supplies the complex callback fix, existing Rust API entrypoint and compatible long-label codec pending upstream release |
| OneLOop | Public `main`, locked at `27c3723434b7d99cf70ce612b0b8041d3f5c0e78`; development-only reference provider |
| one-loop-reduce | Public source, locked at `b53a70776a43bd14c6562c52a03bc4909568e473`; development-only reference provider |

The CLI reads public Git revisions and registry checksums from its consuming
workspace's resolved lockfile. No `FASTSECDEC_*_SOURCE_ROOT` variables are needed
for normal builds. Developers deliberately selecting local path dependencies
must supply matching source roots for their source-state fingerprints.

The three FastSecDec consumer roots also select that public FeynKit revision
through `[patch."https://github.com/alphal00p/gammaloop"]`. This keeps direct
and transitive native graph, kinematic and algebra types on one owner; changing
only a direct dependency to a revision would leave duplicate owners through
other ecosystem crates. A downstream host must make the same selection in its
own root manifest. No unpublished checkout or source-rewriting script is used.
The newer upstream Python graph renderer adds rendering/font/image packages to
the binding lockfile; the numerical and portable validation roots remain free
of Python and that rendering stack.

## Portable and Python consumers

The native `fastsecdec/threshold-decomposition` feature exposes verified GCAD,
local resolution operations and certified rational-fiber continuation. The
native `KernelSet::compile_threshold_fiber` factory compiles the currently
admitted one-dimensional continuation into standalone v15 kernels. This is
complemented by detached `ThresholdCompilationPlan` jobs and indexed v3
archives containing local v16 programs. These retain caller-owned scheduling
and selective sector loading. `threshold::generation::{prepare,resume,resume_evidence}`
provides the corresponding native preparation and recovery adapter, with compact
receipts for caller-owned processes and fresh verification of saved solve output.
The CLI exposes threshold preparation through `--threshold-decomposition`
and the corresponding run-card setting. In addition to the fixed,
one-dimensional rational path, automatic preparation admits the verified
regular algebraic families described below; see
[`examples/no_deformation/README.md`](../examples/no_deformation/README.md).
Both generation schedules publish the same indexed format for ordinary or serial
integration. Native HEPKit inputs also expose this path through
`Integral.generation_session(threshold_decomposition=True)`: explicit `step()`
calls prepare and compile native records, returning the existing recipe archive.
The binding accepts exact fixed inputs and the explicit represented-value
paths described below. Existing diagram/family
`sector_decompose(threshold_decomposition=True)` calls return a native prepared
owner with a separate `compile()` action. The general algebraic resolver remains
a separate gate. See the
[prepared-owner review](reviews/no-deformation-hepkit-prepared-owner.md).
The native `threshold::represented::graph::ExactRepresentedGraphInput` owner
now accepts supported represented floating-point kinematics, scalar bindings
and measure factors before native family arithmetic. It preserves the exact
binary value, source association and numerical meaning; it does not guess
simple rationals. Raw geometry replay requires the original graph point and
re-verifies its evidence. Graph numerator/projector Float payloads require the
explicit primitive policy described below; uncertainty-bearing input remains
unsupported. See the
[native graph review](reviews/no-deformation-preparametric-graph.md).
The existing Python graph entrypoints now select this owner with
`ThresholdSettings(numerical_meaning="represented_values")`. Native
`prepare_graph`, `resume_graph` and `resume_graph_evidence` share the existing
preparation and recovery pipeline. `Integral` retains the original graph point
without family arithmetic; explicit generation/parametrization performs the
full native basis and scalar-point checks. See the
[HEPKit represented-input review](reviews/no-deformation-hepkit-represented.md)
for validation timing, progress, recovery and selected-result serialization.
For numerical graph coefficients, explicitly add
`graph_payloads="primitive_fragments_v1"` to `ThresholdSettings`. Each stored
native fragment and global factor converts before the native aggregate is
recomputed. This convention preserves the original rounded aggregate as
provenance; it need not equal exactification of that aggregate. Float literals
inside function arguments remain unsupported, including scalar `f(0.1)` and
tensor/momentum labels. Native callers select
`PreparationOptions.graph_payloads = Some(GraphPayloadPolicy::PrimitiveFragmentsV1)`.
The default and legacy source identities remain unchanged. See the
[graph-payload review](reviews/no-deformation-represented-graph-payloads.md).
The native `threshold::represented::family::ExactRepresentedFamilyInput`
similarly retains an existing exact `IntegralFamily` and its original numerical
point. It converts numerator and measure separately before native specialization,
preserving complete kinematic assumptions and precision. Existing family/free
HEPKit entrypoints use the same explicit represented-value setting and native
`prepare_family`/`resume_family` recovery. Stored Float family construction,
uncertainty-bearing input and implicit runtime specialization remain unsupported.
See the [native family review](reviews/no-deformation-represented-family.md).
The separate `KernelSet::compile_threshold_family` factory accepts an issued
regular algebraic secant family. It shares detached compilation and indexed
publication, saving record-local root helpers in native v17. This path keeps
full defining polynomials and Symbolica implicit derivatives, with a validated
stability policy. `StaticExactF64BracketV1` remains the default; explicit
`NormalizedFamily::admit_affine` selects `AffineRationalBracketV2` for certified
constant rational brackets through native normalization to `[0,1]`. Both
recipes retain generic polynomial degree within resource limits. See the
[rational-bracket review](reviews/no-deformation-affine-root-brackets.md).
The shared preparation factory now discovers the admitted fixed native-cube
two-/three-cell families and uses shifted affine root programs (V3). This
normalizes the defining polynomial, retaining its source-derived endpoint
constant to avoid cancellation. It does not expand the complete transformed
density. All four normal/serial generation/integration combinations pass for
the sextic example in
[`examples/no_deformation/regular_sextic.toml`](../examples/no_deformation/regular_sextic.toml).
These are generic-degree simple-root constructions within the documented
geometry class, not arbitrary algebraic endpoint resolution. See the
[automatic-factory review](reviews/no-deformation-regular-family-factory.md).
Default native consumers restore these programs without GCAD; portable
root-bearing records remain unsupported. Issued regular native-cube families
also support checked runtime parameters through
`NormalizedFamily::admit_parameter_affine_shifted` and the same native compile
and detached-publication APIs. They save v18 programs with a shared exact
parameter guard and an explicit v4 archive capability. Rebinding admitted
points retains compiled IR; rejected points leave the previous binding intact.
See the [compiled-template review](reviews/no-deformation-compiled-parameter-templates.md).
Explicit native paired-family admission also accepts the actual one-coordinate
affine-projective massive bubble with a uniform parameter chamber; see the
[projective-template review](reviews/no-deformation-projective-parameter-templates.md).
Native `threshold::generation::prepare` now discovers these uniform cube and
one-axis projective families from explicit runtime roles and strict chamber
constraints, with versioned no-Solve recovery. See the
[automatic-parameter review](reviews/no-deformation-automatic-parameter-preparation.md).
Automatic CLI/HEPKit parameter-chamber preparation, more general projective
sources, moving brackets and general root collisions remain separate gates. The
existing native HEPKit owner forwards to the shared preparation factory;
the new sextic's recorded end-to-end execution gate is the CLI. See also the
[algebraic-kernel review](reviews/no-deformation-algebraic-kernels.md).

The direct solver call is synchronous;
the caller owns scheduling and hard resource limits. No setup script or private
checkout is required. The feature is excluded from portable consumers:

```sh
cargo test -p fastsecdec --features threshold-decomposition --locked --lib threshold::gcad -- --test-threads=1
```

Positive homogeneous projective sources also have native verified cell owners,
normalized algebraic maps, and `CompiledOpenDensity` evaluation of the original
complex numerator at explicit nonpole real regulator values. The density uses
native evaluator composition, preserving the unexpanded source body and causal
phase. `EndpointTransport` attaches a complete native sector fan, exact clearing
identities and per-term powers to such a map. These APIs retain pending closed
faces; they do not yet publish a general multi-dimensional physical integration
kernel. See the [density](reviews/no-deformation-open-density.md) and
[transport](reviews/no-deformation-endpoint-transport.md) reviews. The complete
fixed physical ggHH geometry passes within the enlarged 100 GB budget, while
its full endpoint regularization and integration remain unfinished.

Select exactly one arithmetic backend: `native` provides GMP/MPFR and supports
either SymJIT O2 or Symbolica's eager evaluator; `portable` uses the eager
evaluator with Malachite/Astro. The CLI defaults to SymJIT O2 locally. Python
notebooks explicitly choose eager compilation and one caller-owned worker, also
when running locally. `CompilationSettings.backend` selects evaluator generation
without changing expressions or runtime parameter ownership. Portable host tests
exercise that arithmetic backend, but do not establish actual Wasm execution.
The standalone validation consumer excludes native-only reference providers:

```sh
cargo test --manifest-path tests/portable-kernel/Cargo.toml --locked
cargo check --manifest-path bindings/python/Cargo.toml --locked
```

The Python binding crate has its own workspace so root checks remain Python-free.
HEPKit owns the extension-module ABI and wheel. Follow the
[HEPKit build guide](../examples/hepkit/BUILD.md) for native/Pyodide tooling,
installed-wheel tests and notebook export. A matching FastSecDec checkout supplies
examples/tests; it does not configure the host's dependencies.

Provide a Symbolica license through the environment when required by the chosen
workload. Never put it in source or browser assets. Numerical workers evaluate
native programs without constructing Atoms; execution and parallelism remain
caller-owned. QMC tests do not require Symbolica.

## Ownership and compatibility

QMC lives in `crates/fastsecdec-qmc`; Numerica supplies the existing RNG,
numerical types and ordinary Havana Monte Carlo. The earlier Numerica
[PR #8](https://github.com/symbolica-dev/numerica/pull/8) remains historical
implementation evidence, not a required fork.

Native and portable evaluators use upstream Symbolica mapping APIs. Unsupported
numerical domains are checked through native metadata before mapping. The
error-tracking conditioning shortcut is optional; a flagged point without that
shortcut uses the existing increasing-precision rescue. FastSecDec does not
supply a generic special-function-to-error-tracking conversion.

Kernel artifacts retain native evaluator IR and are application caches from a
trusted producer. Format, dimensions, compatibility and byte-consumption checks
remain. Expensive content hashes and semantic/geometric revalidation are off by
default; enable them with `--validate-artifact` or native `KernelLoadOptions`.
The loader does not certify arbitrary native instruction streams. The
experimental structural-decoder patch is no longer a build requirement. No
faulty IR from the native generator was observed.

Newly compiled native kernel records retain Symbolica's native primary JIT
payload alongside the authoritative exact IR in a v13 wrapper. Loading a
compatible payload restores it inside the current callback factory scope,
without translating the exact IR into SymJIT again. The native application
codec still compiles executable code on restore; these are not saved machine
code pages. Missing or incompatible primary caches use the admitted exact
program; malformed applicable payloads return an error. No restoration path
rebuilds expressions or runs Horner/CPE. The CLI's load-completion counts and
`SectorKernel::primary_evaluator_restoration()` describe initial restoration,
separately from later binding, causal pilots and callback-policy remapping.

`GeneratedIntegral::to_kernel_bytes` deliberately remains an exact-program
export without compiling a primary evaluator, and portable production remains
exact-only. Loading an existing artifact and calling `KernelSet::to_bytes`
preserves its original bytes. Native `IndexedReader::write_with_native_cache`
and `ProgramArchiveReader::write_with_native_cache` explicitly refresh existing
indexed records one at a time; mathematical identities stay unchanged while
transport sizes and digests change. The caller owns atomic publication. See
the [primary-cache audit](reviews/symjit-primary-cache.md) for callback ownership,
optional integrity checks and the trusted-producer shape boundary, and the
[application-codec audit](reviews/symjit-cache-loading.md) for native compilation
on restore. Expression-only v1/v2 kernel formats are rejected; regenerate those
artifacts. Saved exact-offset Atoms are evaluated directly through Symbolica at
parameter binding, with native precision escalation, rather than compiled into
another evaluator. Mass constraints use native indeterminate discovery on load
and native evaluation at the physical point; unsupported function forms fail
before successful binding. The CLI reports loading progress independently of
native work, and ordinary `inspect` reads only JSON metadata. See the
[loading review](reviews/lightweight-inspect-loading.md) for the supported
boundaries and measured inspection costs.

The [reuse audit](REUSE_AUDIT.md) and
[ordinary dependency review](reviews/regular-hepkit-build.md) distinguish current
checks from historical patched builds. Original dependency-patch narratives and
the standalone series reproducer remain historical records; their fixes are not
applied by the build. Keep reference checkouts and generated artifacts untracked.

Native one-loop master and numerator-reduction references share the same public
Symbolica and graph owners. They introduce no Python or Fortran runtime and do
not restrict production Gaussian parameterization to their reduction limits.
See the [numerator reference review](reviews/hepkit-numerator-reduction.md).

## Reference comparison

The frozen Pathfinder source revision is
`582d8c7f6dde9bf750750d4c2a2d85a94ce940cd`.
Its external environment can be created in its own directory using
`uv sync --frozen --python 3.12`; building QMCPy needs a C compiler on PATH.
Never add this environment or its packages to FastSecDec dependencies.

Historical timing reports are not performance acceptance evidence for this host.
The [benchmark protocol](BENCHMARK_PROTOCOL.md) defines paired measurements, and
the [regression matrix](REGRESSION_MATRIX.md) records coverage. Pending rows and
unmeasured cases must remain visibly pending.

## Delivery validation boundary

Native tests, portable-feature host tests, actual Wasm execution and installed
Python wheels are separate evidence. Earlier notebook and ggHH results keep
their original source identities; an ordinary dependency migration does not
relabel them as fresh runtime validation. Current delivery is tracked in
[merged HEPKit PR #18](https://github.com/symbolica-dev/symbolica-community/pull/18)
and the [dependency review](reviews/regular-hepkit-build.md).
