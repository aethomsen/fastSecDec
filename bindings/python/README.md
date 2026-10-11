# FastSecDec bindings for HEPKit

This optional Rust crate owns FastSecDec's PyO3 implementation. The
`symbolica-community` extension links it and registers the public
`symbolica.community.hepkit.sector_decomposition` module. It is not an independent wheel.
The numerical workspace and default CLI do not depend on Python or PyO3.

The native API accepts the existing HEPKit `FeynmanDiagram`, `IntegralFamily`
and `Kinematics` objects and Symbolica expressions. Generation and compilation emit native
snapshots through caller-supplied observers. Integration uses caller-requested
QMC packages or native Havana global batches, with native replay, covariance and
checkpoint validation. The
binding introduces no graph, algebra or numerical integration implementation.

Fixed contour generation is opt-in on singleton entrypoints with `contour=True`. Bind
the fixed strength separately from physical parameters using
`template.with_parameters(point, contour=ContourSettings.fixed(0.1,
validation="pilot", pilot_points=256))`. Native `ContourSettings` validates the
strength; rebinding does not repeat symbolic generation or optimization.

Use `integral.generation_family_session(["fixed", "polynomial", "sign_aware"],
compilation_settings=CompilationSettings(backend="eager"))` to generate the
three contour recipes with shared native preparation. Advance its `step()` from
the caller's Generate action; after `.complete`, `.result` is a retained
`RecipeArchive`. Select a template with `.result.select("polynomial")` and bind
`ContourSettings.dynamical(0.8, lambda_cap=1.0, displacement_cap=1.0,
construction="polynomial", validation="pilot")`. The selected recipe must
match the construction; runtime strength and caps do not trigger generation.
The optional `"off"` recipe retains the undeformed path. These mathematical
bounds ensure causal deformation; they do not guarantee good convergence for
every cap. Physical variance and accuracy evidence is recorded in the
[Phase B ledger](../../docs/reviews/contour-phase-b-progress.md).

When validation is enabled, the caller performs it explicitly before creating the integration
session: read `kernels.contour_validation_charts`, supply independent validation
coordinates to `kernels.validate_contour_point(chart.chart_index, coordinates)`,
and call `kernels.finish_contour_pilot()` after the requested points are checked.
Each call also checks the chart's required subtraction faces and fixed-strength
homotopy diagnostics. These observations never enter production statistics.
An optional chart list finishes only that scope; excluded sectors stay locked.
The default `off` policy omits optional causal checks. Explicit `always` checks
production too, while `pilot` removes production causal-checking costs after
preflight. Precision rescue and nonfinite handling remain enabled. A finite
fixed-strength pilot is a diagnostic, not a global causal certificate. Settings,
chart descriptors and reports are immutable native-backed Python views.

For an existing native diagram, kinematics and regulator symbol (declare a
runtime Gram symbol with `S("kinematics::s", is_real=True)` before using it in
the kinematics):

```python
from symbolica.community.hepkit.sector_decomposition import (
    Integral, CompilationSettings, QmcSettings, StabilitySettings,
)

integral = Integral(diagram, kinematics, regulator=eps,
                    runtime_parameters=[s], model_parameters="runtime")
work = integral.generation_session(
    max_order=0, compilation_settings=CompilationSettings(backend="eager"))
# An explicit Generate/Resume action calls one native unit per UI tick.
work.step(max_units=1, observer=on_generation)
# After work.complete; inspection itself performs no generation or compilation.
generated, template = work.generated, work.kernels
# Build an explicit physical point; defaults are inspectable metadata only.
point = {**generated.runtime_parameter_defaults, s: -1.0}
kernels = template.with_parameters(point, stability=StabilitySettings())
session = kernels.session(QmcSettings(points=4096, shifts=16))
# Only an explicit Integrate/Resume action advances sampling.
snapshot = session.step(max_packages=1, evaluation_batch_size=256,
                        observer=on_integration)
```

The free function `sector_decompose(diagram, ...)` is the same entry point.
`IntegralFamily.sector_decompose(...)` uses the family's native kinematics unless
explicitly overridden. Overrides may add auxiliary-vector assumptions; existing
external products must agree after scalar binding because family construction
may already have substituted them into its denominators. Use `scalar_values`
to specialize stored symbolic invariants consistently, or construct a new native
family for a different fixed point. It requires `powers=[...]` and a scalar `numerator=...`:
one signed power per native denominator, including auxiliary entries. Zero powers
drop a denominator; negative powers multiply that denominator into the numerator.
The family numerator must already include the intended projector and diagram
weights. Both inputs use the normalized Minkowski loop measure
`prod_l d^D k_l / (i*pi^(D/2))`, with an optional extra `measure_multiplier`.
The default integration dimension is `4-2*eps`; the native symbolic tensor
dimension is specialized consistently during parametrization.

For cooperative generation from the same native family, construct
`Integral.from_family(family, regulator=eps, powers=powers, numerator=numerator,
kinematics=kinematics)`. Its singleton and recipe-family sessions use the same
native preparation as `family.sector_decompose(...)`. Construction and passive
inspection retain the input objects without doing algebra or starting generation.

The diagram route retains its native numerator, projector and weights and accepts
positive power overrides by native edge ID. It does not accept a second numerator.
Use the existing diagram-expression replacement helper when preparing a different
diagram numerator. Generation returns inspectable sectors and metadata without
compiling kernels or creating an integration session.

`Integral(diagram, kinematics, ...).generate()` remains a synchronous generation
entry, followed by explicit `generated.compile(backend="eager")`. Neither is
called by passive displays. `generation_session()` instead retains native work
through both stages: stop scheduling `step()` to pause and call it again to
resume the same owner. A unit is a chart/cone, mapping/symmetry operation,
representative coefficient expansion, sector evaluator build or final assembly.
An indivisible native algebra operation can exceed a UI tick. Observer `False`
or `KeyboardInterrupt` retains the completed unit; a genuine native error marks
the owner failed. No library thread or worker pool is started.

All generation entrypoints accept the same explicit route selection:

```python
generated = integral.generate(mode="symbolic", subtraction="taylor", progress=None)
work = integral.generation_session(mode="numerical_dual", subtraction="integrate_by_parts")
```

`symbolic` and `taylor` preserve the existing defaults. The alternatives select
native numerical-dual generation and integration-by-parts subtraction; they are
independent of eager/SymJIT compilation and QMC/Havana sampling. The free
`sector_decompose(...)` function accepts these keywords as well. Unknown values
raise `ValueError` before generation observers or parameterization run. Retained
sessions and generated objects expose `.mode` and `.subtraction` without doing
symbolic work, and their rich displays show these settings. In `numerical_dual`
mode, the native coefficient-series composer handles formal endpoint recipes;
`coefficient_expansion` retains the requested choice without changing that
internal recipe engine. This lane retains source charts without symbolic density
symmetry matching. Exact unregulated endpoint admission and signed maps can
require a local symbolic fallback; each sector's `.generation_mode` identifies
the actual route. Compact roots remain cheap recipe views. Only the explicit
`coefficient.expression()` request materializes the actual native expression,
which can be expensive; passive displays do not call it.

Numerical-dual work has a distinct `formula_preparation` progress stage between
endpoint discovery and sector instantiation. `snapshot.formula_preparation`
reports `.completed` and `.total` unique formulas, `.sectors` eligible uses, and
`.reused` shared uses. It is `None` before discovery or without this phase;
observed zero work is a real snapshot with zero counts. The optional
`snapshot.timings.formula_preparation_seconds` records active preparation time,
excluding pauses in retained sessions. Completed formulas survive subsequent
`step()` calls. The cache stays with the native generation owner and creates no
threads or files; ordinary generation remains caller-driven.

`Integral` defaults to independent runtime model inputs. HEPKit resolves analytic
dependencies; named masses remain symbolic before sector support analysis.
Declare real Symbolica kinematic symbols with `runtime_parameters`; their
attributes must be specified when the symbols are first created. The generated object's
`runtime_parameters`, `model_parameter_defaults` (string keys) and
`runtime_parameter_defaults` (native symbol keys) describe the required point.
Explicit `scalar_values` specialize selected inputs; `model_parameters="fixed"`
selects a deliberately fixed model for reference calculations. Widths must be
explicitly zero and runtime masses must remain finite, real and nonzero.
`with_parameters()` returns independent native evaluator owners and preserves
immutable artifact bytes. It never recompiles or mutates the template.

The API remains available in the canonical
`hepkit.sector_decomposition` namespace. HEPKit's object methods are
optional-backend forwarding hooks; all
FastSecDec generation, bindings and numerical work remain in this repository.

Use the maintained [HEPKit example build instructions](../../examples/hepkit/BUILD.md)
to build and install the host wheel. The host's ordinary `community` feature
includes this crate for native and Wasm builds; core-only Symbolica builds remain
separate. Cargo selects the public dependencies without local source patches.
See the [delivery review](../../docs/reviews/regular-hepkit-build.md) for current
validation and publication status. The historical metadata/MC pin,
`a3d09e177196013326fd1532eb938f559be87401`, passes dependency bootstrap and locked
native/portable ownership checks. Its actual portable wheel, built with the
community host at `c9bacce12dd4fffd171aecfbc4f76a2a273d39e7`, passes the generic
Pyodide smoke test and all 58 collected controls from the five maintained
binding, inspection, MC, input and shared-wavefunction modules. No tests failed
or were skipped. The actual browser triangle completed metadata inspection,
QMC checkpoint resume, and same-kernel MC pilot/production resume; its full means,
errors and covariance match the native run. A final supplemental screenshot
scroll failed after the complete reports/checkpoints were saved; that failure
remains recorded. The sole optional browser ggHH attempt reached 30 compiled
kernels in 546.67 seconds, with zero samples. Its report download timed out near
the bounded deadline; no generated artifact, selected-sector inspection or
numerical prefix was accepted. Repeated RPC timeout messages also exposed a
responsiveness limitation during long synchronous generation. The complete
attempt exited cleanly after 596.00 seconds; no retry or browser convergence
claim follows.

That optimized native wheel passed 81 controls and an actual triangle notebook
lifecycle covering metadata, QMC, and Havana pilot/production pause and resume.
Its later source qualifications (one test fixture, an equivalent Option guard,
and presentation CSS) are recorded in the delivery reviews; the portable wheel
includes the published final sources. Hosted native CI for community `b8279dd`
builds the public-pin development-profile wheel and passes all 81 controls plus
the host import check. That workflow installs the documented
`marimo==0.24.2` presentation-test dependency.

Earlier portable evidence at `539019a` covered 50 controls and a triangle
browser lifecycle using showcase `0cf08c6`. It does not validate the newer
endpoint/statistics or discrete-MC APIs. The build guide records compiled and
showcase revisions separately when only notebook assets change.

Python compilation defaults to Symbolica's existing eager evaluator on both
native and portable builds. `CompilationSettings` exposes the native Horner/CPE
controls, with deterministic `cores=1`; an explicit `backend="symjit"` is
available only on native hosts. The CLI/core's default `auto` policy remains
SymJIT O2 natively and eager on portable builds. Native and portable features
are mutually exclusive. The host owns PyO3's ABI and extension-module settings.

On Linux x86_64, select an absolute native linker with
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER` before invoking Pyodide, whose
compiler wrappers redirect generic `cc` to Emscripten. The build guide shows
the command; it preserves the separate Wasm target linker.

For isolated Rust development, use ordinary Cargo from the FastSecDec repository:

```sh
cargo test --manifest-path bindings/python/Cargo.toml --locked --lib
```

Use the Rust/toolchain environment described in
[DEVELOPMENT.md](../../docs/DEVELOPMENT.md). This package has a separate workspace
and lockfile so root `--workspace` checks remain Python-free. Do not copy its
Python dependencies into the numerical core.

`scripts/check_dependencies.py` checks native-object ownership in Cargo
metadata. Its default mode requires the actual published Git source for this
crate and all three native library crates. `--allow-local-fastsecdec` explicitly selects
development with local overrides. Metadata is not evidence of a successful
target compilation or runtime. In particular, Cargo can report a union of
native and portable numeric features from target-specific community integrations;
the actual WASM build/runtime is a separate gate.

`scripts/test-native.sh COMMUNITY_CHECKOUT` runs the maintained binding/example
tests and the host's shared wavefunction controls against the installed wheel.
`scripts/test-pyodide.mjs WHEEL_DIRECTORY COMMUNITY_CHECKOUT` runs the small
binding/input/inspection controls in an actual Pyodide runtime selected by
`PYODIDE_DIST_DIR`. The maintained showcase also offers optional browser ggHH;
its cost is separate from these small controls. The corresponding community
workflow invokes its tests from the exact pinned binding checkout.

The public module/class names and native serialized representations remain
available through the legacy import path. The canonical namespace change has
its own [entry-point review](../../docs/reviews/hepkit-sector-entrypoints.md);
the historical wheel results above do not validate the new owner methods.
Compatibility still follows native content,
precision and checkpoint identity validation; moving the wrappers does not
override those checks.

The API retains source-chart endpoint information at
`generated.metadata.charts[i].pre_subtraction`: exact regulator, prefactors and
affine endpoint powers, with the native number of required Taylor subtractions.
These are facts before symmetry multiplicity and subtraction, not a prediction
of surviving poles. An older artifact can lack the record. Expressions remain
Symbolica objects; inspection does not expand the regular density.
`generated.sectors[i].aliased_coefficients[j]` is a cheap retained view of one
compact root and its native alias definitions. Its explicit `expression()`
method restores only that selected coefficient through Symbolica's alias owner;
call it from an Inspect action when a full expression is wanted. Passive rich
views do not perform this restoration.
`kernels.sector_statistics` reports each complete shared evaluator's native
program bytes and pre-SymJIT operation counts. `symjit_ir_bytes` measures the
compressed compiled application, not machine code, and is absent for eager
evaluators. Complex evaluator outputs precede the real/imaginary
component split.

`kernels.mc_session(HavanaDiscreteSettings(...), pilot=True)` starts native
sector and coordinate importance training. The caller advances bounded
`step(max_batches=1, evaluation_batch_size=256)` calls and can pause by retaining
the session object. QMC uses the same native batched weighted-evaluation path.
Native SymJIT handles matrices; eager and high-precision tiers use the native
owner's row evaluation because those owners have no matrix API. Numerical
batch size is operational and can change across checkpoint continuation.
After a complete pilot, `adapt_pilot()` starts another pilot epoch, or
`freeze_production(points_per_batch=..., batches=...)` freezes both grids and
starts independent production. Pilot observations never enter production
estimates. `checkpoint_available` distinguishes in-memory pilot pause from
persistent production checkpoints, restored with `kernels.restore_mc(bytes)`.
Per-sector `planned_points` is `None` for this stochastic allocation;
`discrete_allocation` contains the actual native selection probability and
global batch size. The total planned budget remains an integer. The existing
QMC `session`/`restore` methods and checkpoint format are unchanged.

Both sessions expose `observation()` for accepted full-vector estimates,
complete covariance, per-sector contributions and exact offsets. Their
`live_observation()` is explicitly provisional: QMC uses complete shifted
lattices; Havana uses native point statistics for the current phase, or only
work since a restored checkpoint. Previews never enter convergence or
checkpoints. `StabilitySettings` transports native distance/validated policy,
including optional cutoffs and per-power thresholds. Diagnostic final-class
counts remain separate from attempted evaluator points and matrix invocations.

Optional contour work observations are enabled with
`kernels.with_contour_diagnostics("aggregate")`; the default is `"disabled"`.
The copied owner preserves physical identity, pilot evidence and sampling
streams. Sampling snapshots expose
`.evaluation_diagnostics.contour_runtime`, with separate `.adaptation` and
`.production` reports. Rich views group evaluation, conditioning, preparation,
exact-contribution and pilot work, with collapsible strength/displacement and
solver details. Ranges are approximate observed centres, not certified bounds.
Missing observations are explicitly unavailable. Reading or rendering a report
does not evaluate, drain observations or advance the retained session.

`kernels.contour_recipes` exposes the retained native contour maps after recipe
selection or artifact restoration. Each view supplies its original
`source_index`, images, endpoint ratios, Jacobian and subtraction-face coverage.
Dynamic maps can keep repeated coefficient expressions as explicit Symbolica
functions: `function_definition_count` is a cheap summary, and
`function_definitions` returns their formal calls and native bodies on demand.
Derivatives in the map refer to these same bodies. Inspection does not expand
the functions, differentiate them or construct evaluators. Views retain their
native owner, so they remain usable after the original Python wrapper is
released. A missing historical metadata record gives `None`; a known map-free
artifact gives an empty list.

### Contour Jacobian construction

Generation accepts `contour_jacobian="symbolic"` (the default) or `"dual"` on
`generate`, `generation_session`, `generation_family_session`, and
`sector_decompose`. For example, create inert retained work with:

```python
work = integral.generation_family_session(
    ["fixed", "polynomial", "sign_aware"],
    mode="symbolic", contour_jacobian="dual",
    compilation_settings=CompilationSettings(backend="eager"),
)
assert work.contour_jacobian == "dual"
assert work.mode == "symbolic"
```

Endpoint reduction and Jacobian construction are independent choices. With
`mode="symbolic"`, endpoint subtraction and integration by parts remain
symbolic for either Jacobian choice. Dual construction uses native first
derivatives of the contour images and determinant composition, currently for
at most six contour coordinates. Symbolic endpoint derivatives may request
higher derivatives of the local contour strength through its native hooks;
they do not introduce numerical-dual endpoint jets for the complete density.
The separate `mode="numerical_dual"` path remains available. Unsupported
requests raise an error without switching modes.
For symbolic endpoints, Dual-J currently requires a straight-line native body
after unused inputs are pruned. A retained conditional or sub-evaluator using a
private Jacobian entry is rejected explicitly; select Symbolic-J for that case.
Exact-only contributions keep native symbolic materialization.
On a newly compiled owner, each sector's
`symbolic_endpoint_contour_partials` statistic counts the surviving image-partial
inputs supplied by this route. Zero is a known cancellation; `None` means the
observation is unavailable, including after restoration. This statistic does
not change the saved mathematical identity or perform an evaluation.
Construction does no generation or compilation until the caller steps the owner.
Synchronous `generate` and `sector_decompose` remain explicit generation actions.

The generated owner retains this choice. Default compilation settings adopt it,
and `kernels.compilation_settings.contour_jacobian` reports the saved value.
Setting `CompilationSettings(contour_jacobian="dual")` cannot retrofit a
Symbolic-J generated owner; select Dual-J during generation. Strengths, caps and
validation are still bound separately at runtime. This is a computational choice,
not an accuracy or speed guarantee.


### Native threshold generation

The existing input/session API also exposes the admitted rational threshold path:

```python
work = integral.generation_session(
    threshold_decomposition=True, subtraction="integrate_by_parts",
    compilation_settings=sd.CompilationSettings(),  # Horner 10
    threshold_settings=sd.ThresholdSettings(),      # GCAD workers=1
)
# Construction is inert. Run this only from an explicit Generate/Resume action.
while not work.complete:
    work.step(max_units=1)
archive = work.result
kernels = archive.select("threshold")
# Starting and advancing kernels.session(...) is a separate integration action.
```

This returns a `ThresholdGenerationSession`; the ordinary default continues to
return `GenerationSession`. The explicit `integral.threshold_generation_session()`
constructor is equivalent. Its `prepare()` and `compile_next()` methods allow
separate phase control. Preparation counts as one indivisible `step()` unit;
each subsequent unit compiles one local native record. No pool is created.
False callbacks or callback exceptions pause at native boundaries, preserving
completed records and raw evidence for native reverification. An indivisible CAS
call cannot be interrupted by this wrapper. The caller owns hard process/RSS
limits. Transport byte limits do not bound decoded native memory.

Threshold preparation requires fixed physical inputs (`model_parameters="fixed"`
for graphs, no runtime physical parameters) and the currently certified
one-dimensional rational-cell path, including the affine projective 2-to-1
bubble. Unsupported geometry fails explicitly; the general resolver remains
under development. Ordinary calls use exact expressions. Explicit
`ThresholdSettings(numerical_meaning="represented_values")` preserves the actual
finite Float values before native family arithmetic, with their precision and
original source associations. It does not infer uncertainty or simpler rational
values.

For graphs, represented slots are kinematic assumptions, scalar bindings and
the loop measure; graph numerator/projector payloads must still be exact. For an
already exact native `IntegralFamily`, the new explicit point may also contain
represented values in its separately supplied scalar numerator:

```python
settings = sd.ThresholdSettings(numerical_meaning="represented_values")
prepared = family.sector_decompose(
    regulator=eps, powers=[1, 1], numerator=E("0.5"),
    kinematics=point_kinematics, scalar_values={s: E("16.0"), m2: E("3.0")},
    measure_multiplier=E("2.0"), threshold_decomposition=True,
    threshold_settings=settings,
)
kernels = prepared.compile()  # integration is still a separate action
```

The stored family basis and its original assumptions must be exact. This entry
cannot recover numerical arithmetic that happened before constructing that
family. A point must agree with already stored external products after the same
closed scalar binding. Original unused assumptions and all numerator/measure
slots remain in provenance. Resume requires the retained original family/point,
repeats bounded conversion and parameterization, and reverifies raw evidence
without rerunning the solver. Loading completed integration artifacts does not
need that original owner.

`ThresholdSettings(gcad_limits_json='{"memory_mib":1024}')` overlays the native
one-worker defaults. The native solver JSON schema and transport limit are
exposed without a second settings implementation; workers other than one are
refused. `snapshot()`, preparation receipt/progress JSON and catalogue inspection
are inert. Saved certificate identities do not imply global proof replay.
The archive owns its storage after the session is dropped. The existing portable
archive reader is unchanged; this generation session is native-only.

### Preparation through existing diagram and family methods

```python
prepared = diagram.sector_decompose(
    regulator=eps, dimension=4-2*eps, kinematics=kinematics,
    scalar_values=exact_values, model_parameters="fixed",
    threshold_decomposition=True,
    compilation_settings=sd.CompilationSettings(),  # immutable Horner 10 policy
)
# Preparation finished; no evaluators or integration have run.
prepared.snapshot()                    # inert
work = prepared.generation_session()   # identical retained session, inert
kernels = prepared.compile()           # explicit compilation and resident loading
archive = prepared.archive            # existing RecipeArchive
# kernels.session(...) is another explicit integration action.
```

The same flag works with `sd.sector_decompose(diagram, ...)` and the existing
family method, whose signed `powers` and weighted `numerator` remain explicit.
It returns `PreparedThreshold`; ordinary calls still return `GeneratedIntegral`.
The fixed-input and rational-geometry limits above apply. Compiler settings are
chosen during preparation because the native plan identity binds them.

`compile()` retains completed records after cancellation and caches the returned
native `Kernels` object. `complete` means all records have been published to the
archive; it does not promise that resident kernel loading succeeded. A failed
load can be retried without discarding the accepted archive. Callers avoiding full
resident loading may instead step the retained session with `compile_next()`, then
use `prepared.archive` directly. Snapshot and representation methods only read
status. Preparation and compilation callbacks preserve existing cancellation and
exception behavior; resumable preparation uses `Integral.generation_session()`.

This interface is native-only. The portable backend rejects threshold generation
and its generated stubs exclude native-only classes. The pinned HEPKit owner
methods already forward these keyword arguments at runtime; their older owner
text signatures may omit them, while the backend stub describes the new options.
