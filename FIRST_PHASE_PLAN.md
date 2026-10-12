# FastSecDec first-phase implementation plan

## Final benchmark scope and stopping point (2026-10-12)

The latest user instruction supersedes the double-box no-contour and mandatory
kite completion requirements below. Stop no-contour double-box development and
generation. Retain the completed contour comparisons, document the difficult
algebraic boundaries and distinguish admitted endpoint charts from compiled
sectors. Report the 644,699-cell atlas and qualified illustrative sector-count
scenarios without treating one selected cell as representative of the atlas.

Finish the supported scalar benchmark suite and independently tuned double-box
contour comparisons in `docs/benchmark.pdf`, including the massive kite only if
its remaining no-contour work is practical to complete promptly. Record the
substantial remaining kite interval/whole-atlas gap explicitly. Add a researched,
documentation-only discussion of interior Landau pinches, the limitations of the
current dynamic contour, and potential future blow-up/analytic continuation.
Publish validated code, measurements, Typst and PDF, then stop. Completion of
this explicitly narrowed campaign does not claim completion of the broader
general algebraic resolver in `NO_DEFORMATION_PLAN.md`.

## Threshold benchmark campaign completion (2026-10-12)

The user assigned the following active goal. It prioritizes complete physical
benchmark delivery while preserving the larger no-contour plan and its remaining
general-resolution obligations:

> **Complete the threshold-method benchmark campaign on `no_deformation`.** Resolve the implementation gaps needed for complete no-contour evaluation of the massive kite and both scalar and physical-numerator ggHH double boxes at the specified exact kinematics. Preserve source identity, causal phases, symbolic endpoint subtraction, complete Laurent vectors, covariance and certified algebraic branches.
> Complete the comparison of fixed, polynomial-dynamic, sign-aware dynamic and no-contour methods across the scalar suite, including the massive sunrise, and both double-box variants, using QMC and discrete MC. Use independent calibration and confirmation streams, 25-core serial execution where supported, the authorized ordinary discrete-MC exception, the 100 GB memory allowance and five-hour double-box generation allowance.
> Publish all agreed ε⁰ metrics in the opening overview of `benchmark.pdf`: complete generation time/RAM, artifact size, sector count, raw and checked sampling costs, integration RAM, C, maximum-weight ratio, total calls and relative uncertainty of the complex magnitude near ten minutes. Check compatibility between methods and against independent references where available. Keep failed attempts and incomplete preparation distinct from full-integral results.
> Delegate implementation and independent audits, reuse native ecosystem capabilities, fix practical performance bottlenecks, and commit/push validated code, reproducible inputs, Typst and PDF. Do not declare this campaign complete while required full-integral results are missing, or confuse campaign completion with completion of the broader general-resolver plan.

The reproduction and metric contract remains in
`docs/benchmark-overview-protocol.md`. Every listed topology requires complete
source coverage, native saved results and independent scientific review; a
verified GCAD atlas, local endpoint certificate or missing row is not completion.
Record implementation/resource blockers precisely rather than describing an
unfinished algorithm as a rejected physical integral.

The subsequent 12 October instruction requires a delegated double-box dynamic
deformation tuning study alongside the coordinator's no-contour work. Investigate
the worse variance even at comparable integrand-call counts, including runtime
cap/displacement/safety settings and the small-strength limit that can approach
fixed deformation when the other bounds are inactive. Preserve the full dynamic
Jacobian and causal construction. Use existing artifacts first, scalar before
physical numerator, independent tuning and confirmation streams, and retain the
original settings as a baseline. Report both per-call and per-time convergence
in the benchmark PDF; do not mistake a larger displacement for an improvement.

## No-contour threshold decomposition (2026-10-10)

The approved [no-contour implementation plan](NO_DEFORMATION_PLAN.md) is the
active goal. Its full text is preserved verbatim. Implement on `no_deformation`
after consolidating the validated contour work into `main`; this supersedes the
earlier branch and pause instructions for ongoing work. Keep the contour methods
as supported comparisons. The separate graph exporter below is retained as an
interoperability boundary, not mistaken for an implemented threshold integrator.

Completion requires verified GCAD, general algebraic endpoint resolution and
certificates, symbolic subtraction with auxiliary-regulator continuation,
composed native evaluators, native HEPKit interfaces and bounded execution.
Rational or quadratic controls alone do not complete this phase. Preserve
existing WASM functionality; threshold decomposition in WASM is deferred.

The 2026-10-11 clarification removes the requirement to request or tag Ben Ruijl
on future dependency PRs. Retain the authorized ValentinHirschi publishing and
commit identity. Existing review requests remain historical records.

## Phase B: causal contour deformation (2026-10-09)

The approved [contour-deformation plan](CONTOUR_DEFORMATION_PLAN.md) governs the
next phase: fixed deformation first, then smooth causal dynamic strength, with
optional runtime validation and physical multiloop acceptance tests. Implement
this phase on `contour_deformation`, superseding the earlier `main`-only rule.
Retain the existing native ecosystem, scientific and bounded-memory guarantees.

The 2026-10-10 priority addendum in that plan sets the immediate stopping
checkpoint: the 1000 GeV physical ggHH double box, optimized fixed/dynamic
generation and five-minute 50-worker QMC/discrete-MC comparisons. Preserve
passing LTD evidence without further high-variance refinement. After green
local checks, publication and the requested results table, pause the active goal
and stop for the user's next decision; do not declare the broader phase complete.
The same checkpoint includes the requested Typst source and compiled PDF report,
a 100 GB campaign RSS cap, and investigation of a native dual-evaluated Jacobian
option with measured generation and sampling tradeoffs.

The later 2026-10-10 clarification requires **symbolic endpoint reduction and
subtraction in both Jacobian comparison arms**. Contour-only dual evaluation
must be selectable independently of `GenerationMode::NumericalDual`. Preserve
deform-before-subtraction ordering and every determinant/local-strength
derivative. Earlier numerical-dual endpoint measurements are historical
diagnostics, not the requested comparison; see the verbatim clarification in
the contour plan and the [independent audit](docs/reviews/contour-symbolic-endpoints.md).

The latest 2026-10-10 extension postpones that pause until after a compact
comparison on simpler, different scalar one- and two-loop topologies. Complete
the physical double-box campaign first, then compare fixed, polynomial-dynamic
and sign-aware dynamic deformation, retaining symbolic endpoint reduction and
including both Jacobian constructions where supported. Report generation time,
sampling time, peak RAM and convergence versus time in the same Typst/PDF.
Preserve independent tuning/production identities and full complex covariance;
do not infer a variance advantage from deformation magnitude alone. See the
verbatim extension at the end of `CONTOUR_DEFORMATION_PLAN.md`.

The subsequent dynamic-cap investigation resumes authorized work after that
checkpoint: retain Horner ten as the user-facing default, study larger runtime
caps on existing double-box outputs and a faster scalar two-loop case, inspect
large-weight points and smooth shape parameters, and expose explicit TOML
generation-sector selection. The latest verbatim instruction and investigation
scope are recorded at the end of the contour plan. Historical benchmark
settings and partial-sector scope must remain explicit.

## Future no-deformation graph interface preparation (2026-10-10)

The user explicitly authorizes the separate `no_deformation` branch and
optional external setup clone, superseding older main/reviewer instructions
for this work. Implement a reusable native graph U/F interface and exact
pre-sector JSON/ordinary-problem export with graph provenance, rational
point binding and independent consumer checks. Preserve all native APIs,
physical signed F zero sheets and original denominator order; no duplicate
graph/CAS, forced numerator contraction, numerical work, or full
no-deformation implementation. FastSecDec is never vendored through symGCAD.
The scoped contract and tests are documented in [docs/NO_DEFORMATION.md](docs/NO_DEFORMATION.md).


## Complete one-loop ggHH helicity reproduction (2026-10-08)

Build `example/gg_hh_one_loop_ME/` with separate `fastsecdec/`, `hepkit/`
and `madloop/` reproduction directories. Compare the coherent incoming `++`
amplitude, including all top-loop triangles and boxes, at the documented
300 GeV point. State the top-only flavour specialization explicitly; massive
bottom cuts are outside this no-threshold benchmark. Preserve native diagram
weights, Higgs self-coupling, external wavefunctions, dimension-dependent
rational terms and the full Laurent covariance. Keep colour sums, averages,
loop measures and identical-particle phase-space factors explicit.

Reuse native HEPKit generation and exact colour filtering, one-loop reduction
and OneLOop masters for the analytic reference. Locate and run the installed
MadLoop independently without modifying the shared installation. Retain the
small source inputs, analytic expressions, drivers and numerical results needed
for reproduction; exclude generated binaries, caches and large build trees.
Verify Laurent-pole cancellation, Ward identities and numerical agreement.

For loop-independent tree propagators, add an explicit `SingleTerm` family
preparation policy using the existing native partial-fraction and sector APIs.
Admit a scalar prefactor only after checking the exact integral identity, and
multiply it once. Preserve the existing preparation defaults and `SingleUnitTerm`
semantics. Test and independently review this narrow extension before publishing.

## Bounded-memory serial execution (2026-10-08)

The approved [serial-mode implementation plan](SERIAL_MODE_PLAN.md) is the
authoritative follow-up for universal indexed artifacts, sector-at-a-time
generation and integration, recyclable caller-owned CLI workers, refinement,
checkpointing and mandatory seed/stream-partitioning acceptance gates.
Generation scheduling must not restrict which integration scheduling mode can
consume an artifact. The serial residence time is a minimum, and complete
replicas update the global estimate during that residence. Implement and audit
all acceptance gates before marking the serial-mode goal complete.

## Saved executable restoration and optional validation (2026-10-08)

Investigate and reuse the pinned Symbolica/SymJIT public serialization APIs for
native evaluators and saved machine code. Verify the actual owner behavior with
a focused Rust probe before choosing any new cache format. Where the public
application API lacks guarded machine-code restoration, record that missing
operation instead of substituting raw code bytes with process-local addresses.
Retain exact native instructions for higher precision, eager and portable
consumers. Remove avoidable loading checks, payload copies and re-encoding
without claiming that the remaining native JIT compilation is cached.

Make expensive integrity and semantic revalidation explicitly optional and
disabled by default, as the user now requests. This supersedes the earlier
requirement to repeat all identity/admission checks on every load. Retain the
format, dependency, backend and dimension checks needed to use saved objects
correctly. Keep validation separate from numerical stability policy. Expose the
choice in the CLI and native API, and distinguish verification, decoding and
backend restoration in observations where useful.

Check and report that numerical-dual IBP subtraction formulae use the existing
parallel preparation and shared dynamic recipe cache with complete keys. Do not
change the ggHH subtraction defaults without a user request. Validate saved
execution reuse, enabled/disabled validation, scientific parity, portable/eager
behavior and cache ownership with focused tests and an independent native-reuse
review. No three-loop generation is required. Commit and push a validated
milestone, reporting measured loading limits and any old-artifact constraints.

## Example command scripts and local integration settings (2026-10-07)

Add executable root `generate.sh`, `inspect.sh` and `integrate.sh` wrappers.
Their first argument selects the example; integration also takes a method.
Forward all remaining arguments unchanged at the end of the native CLI command,
retaining caller-relative paths, exit status and cancellation behavior.
Each ggHH example now also includes `qmc.toml` and `discrete_mc.toml`, copied
from the reusable runtime settings examples. This explicitly extends the
previous six-file cleanup to eight useful files per example. Integration uses
these local settings and distinct method-specific checkpoint/result paths.
Validate shell syntax, argument forwarding and TOML cards without running
three-loop generation or integration, then commit and push.

## Minimal ggHH example inputs (2026-10-07)

Reduce the double-box and both triple-box run directories to their README,
`graph.dot`, `model.json`, `parameters.json`, `run.toml` and `point.toml`.
Remove diagnostic text, duplicate diagrams, source snapshots, renderings and
construction reports from those directories. Preserve any source fixture actually
required by a developer exporter in the existing test-fixture area, and update
that exporter so ordinary output no longer recreates the diagnostic clutter.
Write every point-card input as a finite TOML float, preserving the currently
evaluated f64 values rather than changing the physical point or idealizing its
polarization normalization. Keep all model and kinematic evaluator inputs
parametric and retain the existing generation-time on-shell constraints.

Independently verify the actual native graphs: `_bis` is the single outer
eight-top loop with two gluon rungs; the original triple box has two closed
four-top loops connected by two gluons. Explain the color-singlet exchange
precisely from its external states and color structure. Do not contract or
generate three-loop integrals during this cleanup. Validate native imports,
point equivalence and affected exporter/test builds, then commit and push.

## Lightweight inspection and observable evaluator loading (2026-10-07)

Show bracketed elapsed time beside the active generation step and frozen elapsed
times for completed steps, leaving future steps unchanged. Keep these caller-wall
timings separate from the existing native generation phase measurements.

Make default artifact inspection, including a selected sector, metadata-only.
Preserve cached evaluator sizes, sector ranking, generation timings and the
available compact chart information. Older artifacts must explicitly report
missing chart previews instead of loading their binary payload implicitly.
Retain an explicit deep inspection path for native chart data and expressions.
New artifacts may include a bounded, versioned preview index without changing
the mathematical artifact identity or binary format.

Integration loads the saved optimized evaluator program and prepares its native
execution backend; it must not repeat symbolic sector generation or Horner/CPE
optimization. The user explicitly waives backward compatibility: reject old
expression-only evaluator formats instead of rebuilding them. Evaluate saved
exact-offset Atoms directly through native APIs when binding runtime parameters;
mass validation must not construct an expression evaluator either.
Report metadata reading, binary reading, decoding and sector
restoration before sampling starts, with elapsed time, process CPU and RAM.
Reuse the CLI-owned dispatcher for responsive display and cancellation polling;
do not introduce a library-owned pool. Keep all compatibility and identity
admission checks. Defer the optional conditioning evaluator until the validated
stability policy actually requests it. Reuse native evaluator mapping and preserve
unsupported-domain fallback and worker-local ownership. Test round trips,
cancellation, numerical parity, metadata-only I/O and portable behavior, then
record an independent reuse review before publishing.

## Persistent generation step list (2026-10-07)

Keep the applicable generation steps visible in the CLI dashboard, highlighting
the current step and distinguishing completed and pending work. Configure the
plan from the parsed card before expensive input preparation. Group repeated
per-sector operations honestly for the selected generation mode, include artifact
delivery, and retain the aggregate stage gauge, memory and worker activity.
This is caller-owned presentation; numerical execution and status schemas stay
unchanged.

## QMC dashboard uncertainty labels (2026-10-07)

Use the compact `(σ n/a)` label while a central value has no sampling-error
estimate. Size dashboard value/exponent columns from their full terminal
display width, preserving aligned multiplication dots and avoiding silently
clipped uncertainty labels in sector rows and the full-sum panel. Retain the
existing complete-lattice rules: one completed shift supplies a mean, and at
least two independent completed shifts are needed for QMC uncertainty.

## Notebook symbolic default and immediate publication (2026-10-07)

The user's latest correction selects `symbolic` generation in both ggHH Marimo
notebooks, superseding the numerical-dual notebook default below. Preserve
native `to_dots` preparation, runtime parameters, eager compilation, explicit
actions and lazy inspection. Both three-loop CLI cards continue to use
`numerical_dual`, with `dots` for the original and `minimal` for `_bis`.
Commit and push this correction and open the replacement community PR
immediately after source checks, without waiting for the ongoing native link.
Record unfinished validation explicitly; native build validation may follow
publication.

## Publish both three-loop inputs and align the notebook (2026-10-07)

The user now authorizes including the previously untracked
`examples/gghh_triple_box/` alongside `examples/gghh_triple_box_bis/` in the
next validated push. Audit their native graph identities and replace any copied
two-loop source snapshots, documentation or provenance with evidence from the
actual three-loop inputs. Record unavailable enumeration history as unavailable;
do not invent completed generation or integration results. Keep all resources
portable, and do not run three-loop sector generation.

All ggHH cards must impose the incoming gluon mass shells before native support
discovery while retaining the other declared kinematics and contributing model
inputs as runtime parameters. Both three-loop cards use `numerical_dual`.
The original triple box uses `contraction_mode = "dots"` (native `to_dots`),
while the outer-loop ladder with the `_bis` suffix keeps `minimal`.
The modular and self-contained Marimo ggHH notebooks default to `numerical_dual`
and native `to_dots` numerator simplification, preserving eager evaluation,
explicit generation/integration actions and lazy expression inspection.
Validate native imports, card/schema consistency and notebook controls before
publishing, with an independent native-reuse and provenance review.
Commit and push the validated FastSecDec milestone, then update the community
bridge PR or create a new one if the previous PR has closed or merged. Preserve
the host's intervening native dependency fixes while exposing the current
numerical-dual session API and linking the updated notebooks/examples.

## Exact external-gluon mass shells (2026-10-07)

Declare the incoming gluon virtualities as exact zero scalar products in the
ggHH generation cards, rather than runtime symbols whose point values happen
to vanish. Apply the existing native Kinematics rules before parameterization
and sector-support discovery, for both symbolic and numerical-dual generation.
Keep the remaining kinematic invariants and model inputs parametric. Update
the matching example emitter and runtime point cards, and verify with a native
regression that on-shell specialization changes singular polynomial supports
and exposes the additional endpoint poles. Old generic-kinematics artifacts
must be regenerated. Preserve the separately owned untracked triple-box example
and do not run three-loop generation.

## Dynamic subtraction-formula preparation (2026-10-07)

Retain the smallest native ancestor-closed dual shape needed by each individual
sector request, including exact valuation shifts. Never union maximum orders
across sectors merely to create a universal jet. Reuse native source/dual
evaluators only for identical exact effective requirements, including ordered
inputs, compilation settings and structural-zero assumptions. Keep cache
construction generation-owned and permit unrelated keys to build concurrently.

Add a dynamic in-memory subtraction-formula cache to numerical-dual generation.
First discover all required exact recipe signatures from the sector maps and
endpoint data, then build the distinct formulas in parallel through the existing
caller-owned executor before constructing sector evaluators. Reuse completed
formulas without repeating formal coefficient expansion. Preserve complete
epsilon-dependent prefactors, cancellations, Laurent coverage, resource limits,
Taylor/IBP behavior and deterministic admission. Retained generation sessions
must preserve completed templates across pauses.

Expose this as a distinct measured preparation phase in progress, the generation
summary, saved metadata and inspection. Distinguish unique formulas, completed
builds, eligible sector uses and reuse; absent historical counters remain absent.
Report its wall time separately from map/valuation discovery, sector assembly
and evaluator construction. Structure the cache and phase boundary for later
reuse through I/O, but do not implement external cache loading or saving now.
Validate native/portable scientific parity, exact cache-key separation, parallel
construction, cancellation/session behavior, deterministic outputs and measured
ggHH reuse and timings. Record independent native-reuse reviews before delivery.

## Deferred numerical maps and dual endpoint jets (2026-10-07)

Keep the existing symbolic implementation as the default and add an explicit
`generation.mode = "symbolic" | "numerical_dual"` parallel lane. Support both
Taylor subtraction and endpoint integration by parts, exposing their existing
native selection in the run card and notebook API. Reuse exact source supports,
geometry, epsilon Laurent composition and analytic boundary denominators. Avoid
substituting whole sector maps into source polynomial Atoms in the new lane:
compose native evaluator maps and dual jets, shift known monomial valuations
before inversion, and retain complete interior residuals and mixed faces.

Retain runtime parameters, native eager/portable arithmetic, SymJIT O2, batching,
precision escalation, deterministic caller-owned dispatch and artifact loading.
Full expression materialization remains an explicit lazy inspection operation.
Where ordinary jets cannot establish unregulated endpoint admission or represent
signed infinity charts, retain the exact symbolic chart path and disclose it.
Do not infer exact zeros from numerical jets. Native evaluator composition is a
narrow demonstrated missing owner API; test and independently review its native
extension, obtaining separate publication authorization before pushing that
reference repository. The user subsequently authorized that prepared prerequisite;
its tested revision is published in Symbolica PR #54 against `community`.

Validate Taylor and IBP parity on analytic controls and the actual parametric
gg → HH double box. Measure generation including evaluator construction and
compilation, as well as warm scalar/batched sector evaluator time per sample.
Report observed speedups or regressions and precision classifications honestly;
do not assume that deferred generation is faster. Keep benchmark outputs under
ignored output storage and record reproducible evidence and independent reviews
in the maintained documentation.

## Eager Marimo workflow and complete example migration (2026-10-07)

After finishing batching, rebuild the gg → HH notebook around native generation
of all one- and two-loop diagrams at QED order 2 with only Higgs, gluon and top
particles, symmetrizing initial and final states. Select the first one-loop
diagram by default and allow switching graphs to repeat the workflow. Render
the selected diagram and its numerator before/after simplification using native
HEPKit/Symbolica presentation, taking inspiration from the current HEPKit
four-loop gluon notebook.

Expose separate manual Generate, Inspect and Integrate actions, with QMC before
Havana. Generation and integration must automatically open elegant live views
with smooth stop/resume controls; inspection offers rich sector metadata and
expressions. All returned objects need useful Marimo displays. Visible code
should consist of short scientific calls; startup/helper definitions stay hidden.
Use eager evaluators exclusively and one core, including local execution, while
retaining Pyodide/Wasm compatibility. No passive display/import may trigger
generation, contraction, compilation or integration.

Generate the artifact in place in the notebook, retaining native generated and
compiled objects for Inspect and Integrate without a CLI subprocess or mandatory
filesystem round trip. Saving/downloading is an explicit optional action; hosted
browser persistence can follow after the local workflow is validated.

Inspect must expose the actual Symbolica integrand expression for the selected
sector through the existing HEPKit scoped, lazy expression viewer. Reuse its
large-expression handling; do not eagerly expand a massive expression into HTML
or replace the requested view with a metadata summary.

Use that same pretty, lazy viewer for the simplified projected numerator. Audit
preparation so contributing model inputs and kinematic quantities stay symbolic
in both that numerator and compiled evaluator expressions, with values bound
only for integration as in the local ggHH workflow. Remove unintended numeric
substitutions and large binary-rational artifacts from preparation. Exercise the
notebook controls and parameter changes extensively in the live browser.

Launch and exercise the actual notebook extensively through computer use,
primarily on the one-loop box. Then migrate all examples and other notebooks to
the same applicable standards, update configuration/docs/READMEs, and complete
all local gates. This explicitly lifts the previous deferral of example and
permanent test/gate migration. Validate native and portable behavior separately,
record independent numerical/ecosystem reviews, and commit/push the final
verified product. Mark the goal complete only after every requested workflow
and deliverable has current authoritative evidence.

## Batched evaluator execution follow-up (2026-10-07)

Carry configurable point batches through QMC and both Havana integration paths,
with `evaluation_batch_size = 256` by default. Use native SymJIT matrix evaluation
for SIMD across eligible f64 points, including complex outputs and runtime
parameters. Keep native eager, DoubleFloat and arbitrary-precision fallback
available; reuse native evaluator owners without adding a second interpreter.
Preserve sample order, complete Laurent-vector covariance, per-point stability
and previous-maximum semantics, caller-owned workers, responsive cancellation,
and deterministic task admission. Evaluation batches are operational chunks,
independent of statistical batches, lattices and checkpoint compatibility.
Validate tails, heterogeneous precision routing, scalar/batch numerical agreement,
live statistics and checkpoint continuation. Do not commit or push until this
follow-up is implemented and validated. Other examples and permanent test/gate
migration remain deferred.

## Dashboard counter semantics follow-up (2026-10-07)

Explain the per-sector maximum as the largest importance-weighted complex
coefficient magnitude of an individual sample. Distinguish live estimates from
estimates based on complete batches/shifts, show explicit waiting reasons, and
retain a clearly labeled previous completed allocation during refinement.
Historical display values must not enter sampling, checkpoints or convergence.
Keep the maximum-contribution label compact as `|wgt|`.
On macOS, distinguish native free RAM from sysinfo's overlapping available-memory
estimate; retain raw OS counters in structured observations. Reuse native APIs
and validate accounting and displayed estimate provenance independently.

Evaluator generation must use 10 native Horner-optimization iterations by
default, with a user-configurable iteration count. Also expose the native
maximum `cpe_rounds` setting, defaulting to 1000. Preserve deterministic
generation, native eager/portable consumers, local SymJIT O2, and the ability to
read existing artifacts generated with the previous zero-iteration policy.
Expose the remaining meaningful native builder tuning controls under
`[generation.evaluator]`, including maximum common-pair distance, cache size,
Horner variable limit, direct translation and verbosity. Document upstream
controls that are currently ineffective; do not invent an alternate optimizer.

## Integration dashboard presentation follow-up (2026-10-07)

Restore a structured, colorful Ratatui layout around the integration data:
aligned full-integral summary, iteration/lattice progress bar, total process and
system RAM, and bordered global diagnostics. Add a detail inset for the selected
sector above the global statistics, including precision dispatch, assessed and
accepted counts, mean sample/evaluator times and exclusive timing fractions.
Keep column sorting available by keyboard and add clickable header sorting,
with visible direction, stable sector selection and properly restored terminal
mouse modes. Space sector columns and align scientific multiplication dots.
Use ordinary decimal dashboard durations with dynamic µs, ms and s units.
Format sample and operational counts with four significant digits and base-1000
K, M and B suffixes, keeping small counts as ordinary integers.
Preserve cached one-second observations, numerical estimates, sampling and
checkpoint semantics. Other examples and permanent test/gate migration remain
deferred.

## Runtime stability and live integration follow-up (2026-10-07)

Implement the approved runtime/dashboard plan. Refresh dashboard, plain and JSON
observations on one configurable schedule, defaulting to one second, with
independent responsive cancellation and cached table interaction. Show the full
integral and sortable, scrollable sector estimates using normalized Unicode
scientific notation, two significant uncertainty digits and explicit exponents.
MC previews may use current-iteration point statistics, including implicit zeros
for discrete-sector marginals; accepted estimates retain complete-allocation
semantics. QMC means require a complete shifted lattice and errors require at
least two independent shifts. Never admit preview work to checkpoints or stopping.

Default runtime stability routing uses existing cancellation profiles after
coordinate transformations: f64 above effective distance 1e-3, native 106-bit
DoubleFloat above 1e-8, and arbitrary precision at 1000 decimal digits otherwise.
Retain the old validated policy explicitly. Preserve exact original endpoint
powers for optional per-power distance thresholds and normalized logarithmic
loss budgets; do not infer powers from Taylor orders. Allow an explicit distance
cutoff to return zero, disabled by default, with zero counts and bias limitations
recorded. Nonfinite results escalate and persistent numerical failures are errors.
Use previous per-sector, per-complex-Laurent weighted maxima for optional
whole-vector escalation (default f64 fraction 0.9); only final finite values
update maxima and only admitted complete work updates checkpoint state.

Report the final fractions f64, DoubleFloat, Arb<1000> and Unstable separately
from attempted calls. Measure evaluator time, integrand overhead and integrator
overhead with non-overlapping elapsed spans, excluding coordinator waiting.
Report wall time, aggregate worker time, actual process CPU time, overall mean
primary-f64 evaluator time and the slowest sector mean. Include pilot/discarded
work in operational timing, separate from accepted statistics.

Add runtime TOML settings overlays, explicit epsilon-order accuracy targeting
and maximum refinement rounds. Effective precedence is artifact defaults,
runtime overlay, explicit CLI arguments. Targeted complex accuracy uses the
full-total magnitude and sqrt(C_RR + C_II); retain all coefficients and covariance.
Persist effective policy and accuracy target, reject incompatible checkpoint
resumes, and retain historical checkpoints under their original policy. Retain
portable eager arithmetic and local SymJIT O2. Update only the ggHH example
README with eight-worker Havana (1%) and QMC (0.1%) epsilon-zero commands and
document work limits. Validate routing, weighted maxima, preview statistics,
formatting, timing and interrupt cleanup; independently review native ecosystem
reuse. Other examples and permanent test/gate migration remain deferred.

## Discrete MC responsiveness follow-up (2026-10-07)

Diagnose and fix the apparent hang of ggHH `integrate --method discrete_mc`
with one or eight workers. Keep the caller-owned coordinator responsive during
pilot and production batches, show live work without counting unfinished work
as accepted statistical samples, and service keyboard/signal cancellation
promptly. Preserve complete-batch estimator, covariance, RNG, adaptation,
replay-state and checkpoint semantics; partial cancelled work must not become a
numerical failure or accepted estimate. Investigate evaluator/precision costs
without weakening numerical reliability. Restore terminal modes, cursor and
normal screen on supported exit paths, including interrupt escalation. Other
examples and the committed tests/gates migration remain deferred.

## Artifact inspection follow-up (2026-10-07)

Replace the human artifact inspection dump with a compact, colored tabled
summary. Show overall artifact statistics and the ten largest serialized sector
evaluators, ordered deterministically by size with their stable kernel IDs and
native leading endpoint monomials. Add `inspect BASENAME --sector ID` for rich
sector metadata, including retained remapping equations and measure factors.
Use Symbolica's native pretty printer for expressions and omit per-factor
threshold-status noise. Persist the generation summary's facts in human JSON
metadata and display those saved facts in a dedicated inspection section.
Keep original-chart, representative-chart and numerical-kernel identities
distinct, and identify whether statistics describe exact evaluator storage or
compressed SymJIT data. Preserve plain/JSON output, portable artifacts and eager
evaluation. Other examples and the tests/gates migration remain deferred.

## Generation summary presentation follow-up (2026-10-07)

Replace the final human generation metadata dump with a properly aligned,
colored terminal table, readable durations and explicit Laurent orders with
their real/imaginary components. Preserve structured JSON output, portable
paths, plain/no-color behavior and narrow-terminal readability. Live coefficient
counters must distinguish unavailable counts from genuine zero counts, according
to their computation stage. These are presentation changes; numerical generation,
other examples and the deferred tests/gates remain unchanged.

## Generation scheduling and dashboard follow-up (2026-10-07)

Parallelize the independent complete-density construction and graph
canonicalization used to find equivalent sectors, retaining deterministic
representative admission and exact native equality checks. The CLI remains
the worker-pool owner. Reject invalid artifact output basenames before any
input/reference loading or generation. Show aggregate process memory and
system RAM on the generation dashboard, and replace opaque symmetry and
series-attempt labels with clear descriptions. Existing tests/gate migration
and other-example edits remain deferred.

Use descriptive coefficient-expansion names (`coefficient_series` and
`full_expression`), retaining old input aliases only for the deferred migration.
Model parameters, including named propagator masses and independent coupling
inputs, must remain evaluator parameters with values supplied at integration.
Reuse the model's native analytic dependency resolution; metadata defaults must
not specialize generated expressions. Preserve explicit eager generation and
integration alongside the example's SymJIT O2 backend. Runtime binding must
respect the structural mass domain used to generate sectors.

Parallel completion order must not change coefficients, representative sectors,
parameter ordering or evaluator identity. Format each graph-level DOT attribute
on its own line through the native DOT machinery; omit redundant numerator text
only if the complete native diagram contract and round trip remain valid.

## gg→HH diagram clarification (2026-10-07)

The double-box example must use the user's supplied `D05` s-channel diagram:
both incoming gluons attach to one four-edge circuit and both outgoing Higgs
legs attach to the other, sharing the internal gluon. Preserve the supplied
labeled graph, fermion flow and momentum routing through native HEPKit objects.
The previously selected `FK018` mixed-leg channel and its numerical evidence
do not establish results for this corrected example. Other examples and the
test/gate migration remain deferred.

## Current generation revision (2026-10-06)

The latest user request supersedes earlier threshold-certification and artifact
format requirements for this revision:

- Without an explicitly requested threshold regularization strategy, generation
  performs no sign, face or interior threshold certification. Suitable numerical
  kinematics are the caller's responsibility. Contour deformation and other
  threshold regularization remain unsupported. Algebraic map invariants and
  numerical failure reporting remain required.
- Kinematic scalar products may declare runtime symbols. Compile them as native
  evaluator inputs and supply their values at integration time, including exact
  offsets, precision rescue and checkpoint identities.
- Generation uses caller-owned parallel execution with per-worker activity and
  aggregate elapsed time, progress and ETA in the colorful CLI dashboard.
- Users name an artifact by a basename such as `output/gghh_double_box.fsd`.
  Write human metadata to its `.json` sibling and expressions/evaluators to its
  `.dat` sibling using native context-aware binary serialization. Load by the
  same basename, with both siblings in the same directory.
- Persist relative paths so artifacts can be relocated.
- Preserve the eager Symbolica evaluator for both generation and runtime
  integration, including the portable backend used by future Pyodide/marimo
  execution. The local gg→HH example continues to use native SymJIT O2.
- Update `examples/gghh_double_box` in this round. Other examples and the test/
  gate migration are explicitly deferred to the user's later action. Existing
  checks may be run for focused evidence, but do not claim the deferred gates.

## 1. Goal, governing decisions, and working method

**Goal statement:** Build a modular Rust implementation of FastSecDec for scalar, no-threshold Feynman integrals, using native HEPKit inputs and existing ecosystem functionality. Reproduce the scientific content of all FastSecDecPathFinder examples and relevant tests, independently implement sector decomposition, extend Numerica/Havana with caller-driven lattice QMC, provide a polished standalone CLI, and demonstrate correctness and performance parity under matched SymJIT O2 benchmarks.

**The governing criterion is a flexible, efficient design natural to Symbolica and HEPKit. There is no requirement to preserve pySecDec or FastSecDecPathFinder conventions, algorithms, options, sector numbering, serialization, or internal architecture.** Preserve scientific capabilities and end-to-end examples.

At implementation start:

1. Save this complete plan, including the verbatim prompts below, to `/common/dev/fastsecdec/FIRST_PHASE_PLAN.md`.
2. Set the goal statement above as the agent's active goal, with completion governed by this document's acceptance criteria.
3. Work directly on FastSecDec's local `main` branch. Commit the initial plan and repository scaffolding, then commit at each validated milestone.
4. Create `codex/havana-qmc` in the separate Numerica repository. Commit its changes there and record the corresponding revisions in FastSecDec.
5. Ignore the entire reference directory, build products, generated kernels, caches, and benchmark scratch data. Never commit reference checkouts into FastSecDec.
6. Push validated FastSecDec milestones on `main` to `https://github.com/alphal00p/fastSecDec`, as authorized on 2026-10-04. The subsequent authorization also permits publishing the finished Numerica QMC feature branch as a PR against its main branch and requesting review from `benruijl` (the user's corrected spelling); verify readiness and the target branch first. The user subsequently authorized tested shared external-state changes directly on GammaLoop's `feynkit` branch and a PR for the HEPKit bridge. Other reference changes remain local. All subsequent commits and publishing use authenticated `ValentinHirschi` and commit email `valentin.hirschi@gmail.com`; ask the user to authenticate if the active account differs.

**Use extensive subagent delegation throughout implementation.** The primary agent acts mainly as coordinator: it owns interfaces, task allocation, sequencing, integration, acceptance evidence, and milestone commits. Delegate bounded slices for implementation, dependency research, mathematical audits, test migration, debugging, profiling, and performance optimization.

Use the available parallel slots continuously where useful. Give agents explicit ownership of files or modules and acceptance criteria. Assign important reviews to agents other than the implementation author. Integrate and validate each slice before dependent work proceeds; avoid concurrent edits to shared manifests and public interfaces.

**Completion and stopping rule (user clarification, 2026-10-05):** Complete the
first-phase scientific capabilities and establish representative performance
parity, then mark the active goal complete and stop. Keep performance work
bounded to resolving acceptance gaps. Reuse valid measurements, and stop tuning
each case once it meets the agreed criterion. Do not add optional optimization
campaigns or pursue further speedups after acceptance. Threshold support and
its plan await the user's subsequent instruction.

**Reference-resource clarification (2026-10-05):** Do not keep pursuing
pySecDec/FORM cases that exceed this machine's reasonable resources. Preserve
their failures and use those providers only where they complete practically.
FastSecDec must still complete the required cases. FastSecDecPathFinder remains
the performance comparator, using its existing functional routes and explicit
versioned environments. Complete coverage and establish parity or better for
the required generation, highest-requested-order convergence and sector/sample
metrics, then commit, push, mark the goal complete and stop. An infeasible
pySecDec reference is not an additional completion gate.

**On-shell scope clarification (2026-10-05, supersedes earlier acceptance
requirements for this case):** The user accepted completed FastSecDec generation
of the original on-shell triple box as sufficient for phase A:

> The fact that you can now complete generation of the `on-shell triple-box` within fastSecDec is good enough for now, let's not benchmark performance of it too much and instead focus on the faster-to-generate simpler integrals.

Its full generation, portable artifact publication and cold loading are already
demonstrated. Preserve the partial numerical checkpoints and reference attempts,
but stop further integration, reference construction and performance campaigns
for this case. Full-integral numerical agreement and convergence remain explicitly
unverified, deferred rather than claimed successful. They no longer block phase-A
completion. Focus the remaining bounded comparisons on the existing faster
representatives, starting with triangle, box and double box, and retain the
existing numerator and hard-polynomial coverage without expanding the campaign.
The user also supplied an authorized Symbolica license for subsequent runs;
verify the existing eight-worker Pathfinder route and use matched eight-core
measurements when it succeeds. Keep the license outside tracked files and logs.

Confirmed decisions:

| Topic | Decision |
|---|---|
| Inputs | Native HEPKit DOT is primary; direct parametric `U/F` input remains available |
| Compatibility | Scientific parity; no legacy interface or sector-ID compatibility requirement |
| Numerators | Scalar after native contractions/projectors; arbitrary polynomial loop-momentum rank |
| QMC | Worker-local point generation from published or caller-supplied vectors; no CBC construction in phase one |
| Scheduling | Democratic and adaptive QMC |
| Reference execution | Existing Python/pySecDec may run externally for development comparisons |
| Performance | Matched SymJIT O2; at most 5% timing regression per representative case |
| Platforms | Linux and macOS, x86-64 and ARM64 where supported by dependencies |
| Domain uncertainty | Reject inconclusive threshold checks by default; allow an explicit recorded no-threshold assertion |
| Python bridge | Public HEPKit module registered in community; substantive Rust/PyO3 bindings and FastSecDec-specific implementation belong in FastSecDec under the latest clarification below |

### Authorized HEPKit notebook extension and native example prerequisite

The user approved a browser-first marimo showcase alongside the bounded native
performance work. Register its public Python module on the symbolica-community
feature branch, with substantive bindings owned by FastSecDec as clarified
below, and reuse native HEPKit diagrams, models, kinematics, tensor algebra and
typed FastSecDec events. Demonstrate a massive triangle, massless box, rank-two
box numerator and coupled two-loop sunset numerator. Expose masses and scalar
products through explicit forms, require a deliberate generation/integration
action, and show actual streamed Laurent estimates, uncertainty, complete-shift
coverage, convergence and cancellation. Retain native O2 by default; investigate
Symbolica's existing interpreted evaluator and supported WASM numeric backends
for the browser. Native feasibility is not evidence of browser responsiveness.

Add an advanced single-diagram `g g -> H H` double box, generated by HEPKit from
the Standard Model with six massive internal top lines and one internal gluon.
Select the topology with native graph primitives. Preserve the generated diagram
weight, couplings and dimensional numerator; contract color with a documented
delta projection and external gluons with GammaLoop's existing numerical helicity
convention, exposed through shared HEPKit primitives if necessary. The shared
extraction must include the complete existing GammaLoop scalar, massive/massless
vector and spinor wavefunctions, including longitudinal states and conjugates;
do not implement only the transverse subset needed for this example. Use native
`simplify_algebra` to obtain scalar dot products. Do not replace the helicity
contribution with a spin sum or a scalar toy, or expand factored numerators without
a demonstrated need. Label the single diagram's gauge and normalization; it is
not the full gauge-invariant amplitude.

**Run this example natively first.** Put the generated input and steering in the
existing `examples/` collection, provide a reproducible Rust generator, and run
the ordinary CLI before including it in the notebook. The default point is
sqrt(s)=300 GeV, m_H=125 GeV, m_t=172.5 GeV with an explicit scattering angle;
retain the native domain check on the actual graph. Record generation time,
memory, complete Laurent coverage and integration convergence. Only then decide
whether the example is suitable for the default browser walkthrough, an optional
longer browser run, or native execution. Do not predeclare it browser-feasible.

**Bounded gg→HH reference feasibility (2026-10-05):** Delegate a comparison with
FastSecDecPathFinder and pySecDec for this exact diagram, scalar numerator,
color/helicity projection, masses and physical point. Inspect each route before
running it. Each implementation has a cumulative attempt budget of ten minutes
and 15 GB of RAM across its owned process tree. Do not pursue completion when
the route appears infeasible within those limits; terminate and reap owned work
at a limit, preserving the stage, timing, memory and failure evidence. Report
measured generation and integration costs and achieved uncertainty only when
available. An incomplete reference supplies a feasibility bound, not a numerical
result, extrapolated timing or new requirement to complete that reference.

### Latest priority: binding ownership and complete notebook interaction

The user's subsequent correction supersedes the earlier requirement to house
all PyO3 API implementation in symbolica-community. Keep the numerical core and
default CLI entirely Rust and free of Python/PyO3 dependencies. Place substantive
Rust/PyO3 binding implementation in FastSecDec, with an optional, isolated
dependency boundary. Community should only link the published FastSecDec binding,
register the HEPKit module, provide the necessary reexports/stubs and point to
the maintained examples. Audit dedicated demo helpers, assets, tests and build
guidance for the same ownership boundary; avoid maintaining a second FastSecDec
implementation inside community.

Prioritize this architectural audit and the notebook over further performance
experiments. Preserve accepted benchmarks and prepared protocols, but park new
performance runs until these requirements are implemented and reviewed.

Every showcase case, including gg→HH, needs separate explicit **Generate** and
**Integrate** actions. Opening the notebook, changing a form or exploring sectors
must not start generation or integration. The Generate action prepares and
compiles the input, then stops with an inspectable result. Integration begins
only through its own action; cancellation/resumption must preserve accepted
native coverage and keep input/result identities consistent.

Both stages must stream information-rich, polished views of actual native
events. Generation needs detailed phase progress and counts; integration needs
complete Laurent estimates, uncertainties, coverage, convergence and meaningful
timings. After generation, provide an all-sector statistics view and expandable
or selectable detail for individual sectors. Expose these through high-level
HEPKit Python objects wrapping the corresponding native FastSecDec objects;
do not reconstruct numerical or algebraic state from display strings or add
duplicate Python algorithms.

Acceptance includes exercising the real notebook actions and visible streaming,
checking that controls do not trigger work automatically, checking sector
exploration before integration, and completing the gg→HH native workflow with
the existing scientific safeguards. Preserve native/browser distinctions and
label rendering replays and unpaced API-driver timings accurately. A screenshot
or an API-only run does not alone establish a smooth live notebook interaction.
Finish this review and implementation before returning to the bounded remaining
performance gates. The stopping rule remains: complete the required features
and parity, commit and push, mark the active goal complete, then stop for the
user to plan the next phase.

Latest user clarifications, verbatim:

```text
Ok with the plan above, you can go on with the implementation of the plan, but can you have an example of a double-box example with two on-shell and two off-shell legs, along with a couple internal masses and non-trivial-numerator? i.e. essentially the double-box contribution from g g > h h (for di-higgs production through a double-box top-quark line and vertical gluon)?
Basically use the HEPKit diagram generator to build that double-box contrib to g g > h h, then use `simplify_algebra` to get a scalar numerator in terms of dot-products (using colour delta and numerical polarization vectors as external data (using their numerical implementation in gammaloop which is hopefully already nicely exposed in hepkit)  ) and then use that as an input for fastSecDec to compute that "helicity amplitude" from that one diagram.
```

```text
(simply set sqrt(s)< 2 m_t (about 340) but > 2*m_H (about 250) which will ensure no thresholds, so for instance pick sqrt(s)=300)
```

```text
Continue as planned, but before integrating this example in the notebook, make sure to verify that it is reasonably doable by running this example locally and natively within the fastSecDec package (encode it first as an entry in ./example which you run using the cli).
```

```text
Can you delegate to subagent to see what kind of performance fastSecDecPathFinder and pySecDec themselves would have on this exact same double-box gg_hh example? (If it does not appear to be feasible within 10 min and 15 GB of RAM then do not try to complete this example within these two different implementations).
```

```text
Once you're feature complete within what's stated in the goal, and reached parity or better everywhere according to the metric I mentioned earlier vs fastSecDecPathFinder, then commit+push, set the goal as completed and stop yourself so that we can plan together for the next step.
However make sure that the demon notebook (also g g > h h) is solidly implemented with great visuals and runs smoothly, with explicit buttons for generating and integrating so that marimo does not automatically run those cells. Both generation and integration must have very elegant and information-rich visuals streamed during the runs to follow the generation in details as well as the integration.
It should also be able to visually elegantly explore the resulting generation (looking at stat for all sectors, but also expand details of particular ones etc...) all with rich high-level HEPKit python primitives wrapping the fastSecDec corresponding objects.

Also Ben ruijl noted this:
"""
It is pushing fastsecdec code into community repo. It should only be registering the module and linking to the fastsecdec repo and adding adding the stubs
"""
So indeed, make sure that the community repo only ever contains the necessary python API glue, but all heavier implementation must remains in the fastsecdec repo directly, make sure to thoroughly review all the above instead of chasing more performance for now.
```

### Resumed notebook and gg→HH validation requirements (2026-10-06)

The scalar triangle stays first and selected by default. Include gg→HH as an
optional longer browser calculation using the same HEPKit input helper and
physical assets as native execution. Slow execution or imperfect keyboard
interruption does not justify excluding it. Keep separate explicit Generate
and Integrate actions, preserve accepted work on interruption where supported,
and describe actual browser interruption limits without claiming immediate
response during long native algebra calls.

Increase the native gg→HH integration statistics until the epsilon-zero
coefficient reaches a relative standard error of at most 0.001. Preserve the
complete Laurent vector, real/imaginary components, covariance, precision
diagnostics and reproducible settings. Reuse the accepted generated artifact.
Expose meaningful integration settings through the notebook; a larger planned
allocation alone is not proof of convergence.

Also provide ordinary Havana Monte Carlo with discrete importance sampling
over sectors, reusing Numerica/Havana's existing grids and accumulators rather
than implementing another sampler or estimator. Demonstrate statistical
compatibility with the QMC calculation at the identical physical point and
normalization. Audit the actual sector-selection probabilities and weights;
fixed round-robin per-sector Monte Carlo is not this requested capability.
Keep the scientific implementation native and the HEPKit binding thin.

Perform basic independent sector sanity checks against the existing partial
pySecDec generation and its native decomposition tools. Compare scientifically
meaningful geometry, factorization, singularity and multiplicity information;
different valid decomposition algorithms need not produce identical sector
numbers. Preserve the existing infeasible full-reference limits. Record any
comparison that partial output cannot establish instead of inventing a
completed reference.

Continue feature and notebook validation before further performance tuning.
The original completion criteria and stopping rule remain in force.

**Sector explainability extension:** Retain the mapped integrand structure
before endpoint subtraction in the generated result and portable metadata.
Expose each chart's variable substitution and Jacobian, epsilon-dependent
endpoint powers and prefactor, the associated subtraction requirements, and
the size of the actual evaluator input passed to SymJIT. Distinguish compact
symbolic size, evaluator operation counts and compiled-code size where the
backend provides them; do not label an alias count as expression size. Reuse
Symbolica's native introspection and preserve factored expressions. The HEPKit
objects and notebook must make these details available on demand for a chosen
sector, chart and term, with bounded previews and no automatic expansion.
Older artifacts must explicitly report unavailable metadata rather than infer
it from their finite kernels. Complete this notebook extension before returning
to the remaining goal work.

Latest user requests, verbatim:

```text
Continue as planned, but I'd like this statement of yours:
"""
since the finite compiled kernels no longer show those original powers directly.
"""
To be changed, and have richer meta-data and information in the exploreable result of the fastSecDec generation, also having the viewer nicely exposing on demand for a given sector info like the change of variables, expression size compiled by SymJIT, epsilon-powered prefactor form that needs to be expanded into plus distributions, etc..
Then continue as planned with the goal once the notebook is complemented as instructed above.
```

```text
Continue as planned, but yes include the gg>hh as runnable in the notebook. Even if keyboard interupt does not behave so well for now, keep it in (just make sure the scalar triangle comes first).
```

```text
Continue as planned. (Also making sure to update the integration statistics for g g > h h double-box so that you get one per mil accuracy in the eps^0 coefficient).
Also make sure there is also a havana non-qMC integration (with discrete importance sampling over the sectors then) and show that it is compatible with the earlier qMC run.
Also do some basic sanity checks that at least the sector you identified there are in-line what what you'd expect from the pySecDec generation (even though it does not complete).
Then continue the goal (without overemphasizing performance gains yet) until all is covered and you can consider the goal completed.
```

### Native HEPKit entry points and visible notebook workflow (2026-10-06)

Use `symbolica.community.hepkit.sector_decomposition` as the canonical Python
namespace. Add `sector_decompose()` to HEPKit's existing `FeynmanDiagram` and
`IntegralFamily` classes, forwarding to the same public function. The user's
later compatibility-removal instruction drops the unused `hepkit.fastsecdec`
namespace. These methods return the existing inspectable `GeneratedIntegral`;
compilation and integration remain separate explicit operations.

Reuse the native diagram, family, kinematics and Symbolica expression owners.
Diagram input preserves its numerator, projector and weights. Family input
requires an explicit signed power vector and an already weighted scalar
numerator; auxiliary denominators must not silently receive unit powers.
Use native sector selection and Symbolica operations for nonpositive powers,
and share any input specialization orchestration with the existing graph path.
Inspect existing ecosystem APIs before adding a primitive. If an existing
HEPKit structure lacks a necessary feature, extend its owning crate minimally
instead of introducing another graph, family or algebra representation.

The notebook must visibly define the actual decomposition, compilation,
session creation and bounded stepping calls using this API. Its helpers may
own presentation, controls and lifecycle state. Visible functions are invoked
only by the existing Generate/Integrate actions and active integration ticks;
displaying the code must not duplicate scientific work or start it on load.
Keep the scalar triangle first, the optional gg→HH browser case, typed streamed
events, lazy sector inspection and accepted checkpoint ownership.

Delegate the binding, native-owner delivery and notebook slices separately,
then independently audit reuse, normalization, signed powers, namespaces and
action ownership. Validate native and portable entry points against their
actual compiled sources; previous wheel results do not validate a changed API.

Latest user requests, verbatim:

```text
Explain the layout of the notebook structure. In principle I expect clear integration:

from symbolica.community.hepkit.sector_decomposition import *

and perhaps stuff like IntegralFamily.decompose(), FeynmanDiagram.decompose()

perhaps sector_decompose istead of decompose, not sure.
```

```text
Yes this sounds good! Make sure to use the hepkit structures that already exist whenever you can (don't reinvent the wheel), modify them if they lack a feature.
```

### Self-contained gg → HH walkthrough (2026-10-06)

Provide one notebook using `Model.standard_model()` and native graph generation,
with masses, widths and couplings specified inline. No helper scripts or input
files are required by this walkthrough. Keep the native diagram/family API
visible, inventory genuine missing primitives, and extend their existing owners
where necessary. Reuse the existing Rust scalar-binding logic through Model
rather than copying its dependency resolution into Python. Prefer ordinary
marimo cell execution to a custom Generate/Resume workflow in this compact
walkthrough. Use `diagram.sector_decompose(...)` for the gg→HH walkthrough,
retaining the contracted numerator and graph weights on the native diagram.
Retain the richer dashboard as a separate example.

User clarifications, verbatim:

```text
Try to do the whole gg→hh in one notebook, no other python scripts and with minimal code. Can you use Model.standard_model() and the graph generator to get the double-box? Inventorize the challenges. I am not sure buttons of the workflow are needed, especially for Generate. It should be clear that with little code the decomposition should be set up
```

```text
Is the parameter card really needed? Can this not be in the notebook?
```

```text
Can it not use diagram.`sector_decompose` instead?
```

## 2. Architecture, ecosystem reuse, and inputs

Use a Cargo workspace with three crates:

| Crate | Responsibility |
|---|---|
| `fastsecdec` | Public library; HEPKit ingestion, parametric construction, subtraction, kernels, integration sessions, persistence, status and results |
| `fastsecdec-sectors` | Exact sector geometry, coordinate maps, symmetry identification and decomposition progress |
| `fastsecdec-cli` | Configuration, commands, worker execution, terminal dashboard and reporting |

Organize each crate into small modules by responsibility. Keep graph ingestion, numerator conversion, domain analysis, subtraction, compilation, numerical rescue, scheduling, statistics, persistence, and presentation separate. Avoid large files that combine the pipeline.

The sector crate owns neutral domain/map types and does not depend on the main library. The main library reexports appropriate public types. CLI dependencies must not enter either numerical library.

The later HEPKit ownership correction adds an isolated `bindings/python`
package, `fastsecdec-python`, outside this three-crate workspace. It has its own
workspace/lockfile and exports an `rlib` registration entrypoint. Community owns
the Python extension initializer and ABI selection; the leaf binding forwards
native/portable backends and shares the existing FeynKit-Py, Symbolica and PyO3
owners without depending back on community. This preserves the core/default
CLI dependency graph and its `--workspace` test boundary. FastSecDec maintains
the notebook under `examples/hepkit/`, with binding tests alongside the isolated
crate and notebook tests alongside the showcase. See
`docs/reviews/hepkit-binding-ownership.md` for the migration and validation gates.

### Reuse before implementation

Before implementing any computer-algebra or graph capability:

1. Search the relevant public Rust APIs and community bindings.
2. Inspect implementations, examples, and tests on the correct dependency revisions.
3. Build a focused Rust probe to determine whether existing APIs, composed together, provide the capability.

Record the evidence and the narrow missing operation. Implement new functionality only after these checks establish the gap.

Repeat a broad HEPKit integration and ecosystem-reuse audit at every major
subsystem milestone, before accepting a new dependency patch, and before final
phase-one acceptance. Assign the review to a subagent independent of the relevant
implementation. Review native graph, model, kinematics and Atom ownership;
public library interfaces; typed status, errors and persistence; caller-owned
execution; unnecessary serialization; and any duplicated graph, algebra or
numerical functionality. Include existing one-loop scalar masters and reduction
as numerical cross-check providers, without limiting the audit to those examples.
Require API, source/test and executable evidence for reuse decisions. Record
findings, fixes, justified gaps and the next audit boundary in `docs/reviews/`
and `docs/REUSE_AUDIT.md`. Resolve avoidable duplication and integration blockers
before accepting the affected milestone; do not claim future compatibility from
an unchecked interface alone.

Preserve factored Symbolica expressions throughout the pipeline. Avoid expansion
unless a specific operation requires it, and document those boundaries. Exact
support extraction may require polynomial coefficients; it does not justify
expanding the surrounding density, subtraction terms, Laurent templates, or
kernel expressions. Reuse native differentiation and series with opaque
subexpressions and late substitution where that avoids expression growth.

Use the following existing owners:

| Capability | Existing implementation to use |
|---|---|
| Physical graph and DOT interpretation | HEPKit/FeynKit `FeynmanDiagram` and `feynkit-graph` |
| DOT parsing and graph primitives | `linnet` |
| Models and parameter cards | `feynkit-model` |
| Routing, scalar products and integral families | `feynkit-kinematics` and graph APIs |
| Tensor, Lorentz, color and gamma algebra | Idenso, Spenso and applicable `feynkit-tensor` operations |
| `U/F` construction | `IntegralFamily::symanzik` |
| Numerator rewriting and denominator bases | Existing integral-family operations |
| Polynomial algebra, differentiation, series and exact linear algebra | Symbolica |
| Gamma-function Laurent expansions | Symbolica transcendental functions and series |
| Graph canonicalization | Existing Symbolica/Graphica APIs |
| JIT and batch evaluation | Symbolica's SymJIT interfaces |
| MC integration and numeric types | Numerica/Havana |
| Symbolic integration, if needed | Native Symbolica rational-polynomial integration; existing ecosystem integration APIs for other required classes |
| One-loop scalar master references and reduction | Existing native HEPKit/FeynKit integral APIs |

**Use Linnet directly.** `FeynmanDiagram` already owns a Linnet `HedgeGraph` and exposes its underlying graph. Reuse stable edge/vertex IDs, connectivity, bridges, cycles, subgraphs, spanning forests and traversal primitives. Do not create another physical graph representation, DOT parser, momentum-routing implementation, or graph canonicalizer.

Use local dependency worktrees at the revisions matching the inspected community environment:

- FeynKit initially: `8f834d9c62ae06fb327e4ef0b14abffda755b610`; the authorized
  HEPKit bridge now aligns all consuming owners to published shared-wavefunction
  revision `6c707c6b77a437256eb1180da13d4d327b371d13` in an isolated worktree.
- Symbolica community: `98794d0d7337ba2b08e4c046dde584ad7fc1ce10`.
- Numerica QMC branch starting from `a8a8fcb8941752e265e3c4fa507c02a3f06bb70e`.

The supplied FeynKit and Symbolica directories currently have other branches checked out; prepare isolated worktrees without disturbing them. Root Cargo patches must resolve direct and transitive dependencies to one compatible Symbolica, Graphica, and Numerica instance.

The initial published Rust backend was `symjit = "=2.26.0"`; the current pin is
`=2.26.4`, verified against the live registry on 2026-10-04 before the requested
function-map experiments. Symbolica's pinned upstream revision includes the
latest published 3.0.1 release. Recheck published releases before subsequent
evaluator experiments and record the exact source revisions and local patches.
The supplied SymJIT checkout has different library packaging; that alone does
not justify patching it.

Disable Python features and unnecessary dependency defaults in the core and CLI.
Verify their production dependency graphs exclude PyO3, Python bindings,
pySecDec, and the full GammaLoop application. The separately selected HEPKit
binding is the explicit Python/PyO3 boundary; it must not enter default core or
CLI builds. Existing numeric backends used by Symbolica remain dependencies;
all new FastSecDec and QMC implementation code is Rust.

Keep dependency patches minimal and isolated: a demonstrated defect, focused regression test, small fix, and upstream-ready explanation. Numerica's requested QMC extension is the intentional larger exception.

### Native input boundary

Library callers supply native `FeynmanDiagram`, model, `Kinematics`, Symbolica atoms, and polynomials. Do not serialize these objects to strings and parse them again internally.

The CLI uses one TOML steering format referencing:

- Native HEPKit compact or stable DOT.
- Existing model JSON and native parameter-card JSON.
- Scalar-product declarations mapping directly to HEPKit `Kinematics`.
- Numerical parameter assignments.
- Expansion, generation, integration, and output settings.

Resolve paths relative to the run card. CLI overrides apply to steering settings. Preserve model-fingerprint checks. Use HEPKit's `P(i)`/`K(i)` symbols and routing conventions; do not require Pathfinder's additional propagator or loop-momentum annotations.

Reproduce examples using native HEPKit DOT, minimal model fixtures, and TOML run cards. Non-default propagator powers belong in integral settings keyed by stable internal edge IDs, without extending the DOT dialect.

The old DOT files are scientific references, not files to preserve unchanged.
Deliver a physically equivalent set using current native HEPKit conventions.
Validate every fixture by native parsing and export/reload; additionally verify
propagators, numerator routing, graph weights, and kinematics. Do not require the
old frontend's `mass`, `mom`, explicit propagator lists, or custom momentum lists.

Support:

- Standard quadratic scalar propagators with positive integer powers.
- Scalar polynomial numerators, including mixed-loop products, arbitrary rank, and epsilon dependence.
- Native tensor expressions that existing contractions and supplied projectors reduce to such scalars.
- Rational coefficients in dimension and external parameters.
- Gram-degenerate external kinematics when scalar processing does not require Gram inversion.

Reject unresolved free indices, unsupported loop-dependent denominators, custom nonquadratic propagators, finite widths, and complex masses in phase one.

Define the default measure explicitly:

\[
I=C_{\mathrm{graph}}\int\prod_{\ell=1}^{L}
\frac{d^Dk_\ell}{i\pi^{D/2}}\,
\frac{N}{\prod_j(q_j^2-m_j^2+i0)^{\nu_j}},
\qquad D=4-2\epsilon.
\]

An explicit symbolic measure multiplier defaults to one. Add no implicit scale, Euler-gamma, or \(4\pi\) factors. Preserve and apply native numerator, numerator-prefactor, projector, and overall-factor semantics exactly once. The diagnostic graph symmetry factor must not introduce another division.

Direct parametric input specifies its expressions, powers, prefactor, and domain explicitly:

- `ProjectiveSimplex`
- `UnitCube`
- `PositiveOrthant`

The hard four-loop reference is a positive-orthant integral, not an original unit-cube integral.

## 3. Symbolic and numerical generation pipeline

The pipeline is:

**Native input → normalized parametric expression → domain assessment → sector maps → endpoint subtraction and Laurent expansion → complete direct evaluators → integration.**

Preserve Symbolica expressions in factored form where possible. Expand only when required for polynomial supports, coefficient extraction, or another specific operation.

### Parametric construction

Reuse HEPKit's propagator families and Symanzik construction. Build measure factors and regulator-dependent powers explicitly around those APIs.

The principal missing physics operation identified by inspection is general polynomial-numerator conversion to Feynman parameters. After the required executable reuse probe, implement only that operation using Gaussian generating functions/moments and existing Symbolica arithmetic, differentiation, and matrices.

Do not make external Gram inversion a compulsory stage. Validate numerator conversion against propagator-cancellation identities, routing changes, odd/even moments, and independent analytic examples.

Compute intermediate expansion depth from the requested final Laurent orders and the poles of every prefactor. Convolve prefactors and coefficients exactly, including covariance propagation after numerical integration.

### Native sector decomposition

Implement one exact geometric engine based on Newton polytopes and normal fans. This gives a finite decomposition strategy without reproducing pySecDec's collection of methods. The mathematical basis is the [Kaneko–Ueda geometric construction](https://arxiv.org/abs/0908.2897).

- Map projective input through primary projective charts.
- For unit-cube inputs, restrict the normal fan to the nonnegative logarithmic domain.
- For positive-orthant inputs, retain the full normal fan, including maps resolving infinity.
- Obtain sparse supports through Symbolica; construct Minkowski support sums without expanding symbolic products.
- Implement the missing sector-specific exact cone enumeration using double description and deterministic pulling triangulation.
- Reuse Symbolica integers, rationals, determinants, ranks, and solves throughout.
- Represent each monomial map by its signed integer exponent matrix and exact determinant/Jacobian.
- Extract endpoint valuations and retain residual expressions, numerator information, and provenance separately.
- Identify equivalent complete sector integrands through existing graph canonicalization. Include numerator and exponent data; merge only verified equivalents.

Use deterministic content-based sector identities for diagnostics and caching. No existing `PSD` numbering is contractual.

Cache geometry by domain and canonical support. Parallelize independent charts and cone work. Provide progress, cancellation, and resource-limit diagnostics: formal termination does not guarantee acceptable memory or speed.

Geometry failures must never silently become zero integrals. Use existing scalelessness certificates when available; otherwise report the unresolved condition.

### Direct subtraction and evaluators

Implement endpoint Taylor subtraction, analytic endpoint terms, and the integration-by-parts reductions needed by the examples. Use Symbolica for the underlying algebra and series.

Substitute coordinate changes directly into complete Laurent-coefficient expressions. Compile a multi-output evaluator per sector or verified equivalent group. Preserve cancellations within each complete coefficient; do not integrate projector pieces independently.

Default compilation explicitly selects **SymJIT O2**. Retain an interpreted evaluator and arbitrary-precision evaluation for diagnostics and rescue. Validate JIT results against them at interior and near-boundary points before trusting the compiled path.

Also investigate representing coordinate substitutions with Symbolica's native
evaluator function maps and aliases, comparing inlined and non-inlined forms.
Measure symbolic generation, evaluator construction/compilation, expression/IR
size, and numerical runtime, including precision rescue and portable reload.
Preserve the same complete direct Laurent coefficients and Jacobians. Choose
the representation from measured correctness and performance; do not assume that
the smallest symbolic expression gives the fastest numerical evaluator. Verify
the latest published Symbolica and SymJIT releases before this work and retain
exact dependency identities in its evidence.

When generation or evaluation performance is uncertain, dissect the reference's
**direct integrand** implementation first: its subtraction and IBP formulas,
symbolic expansion order, reuse and caching, and compiled evaluator layout.
Use those findings to guide an efficient Rust design without retaining Python,
pySecDec, or unnecessary architecture conventions. The single dual evaluation of
U and F is explicitly out of scope: the user found that runtime strategy too slow.

Fuse periodization into the direct kernel when beneficial, and verify it against the unfused transformation. Whole zero-dimensional sectors are evaluated analytically.

Precision handling must detect nonfinite values, unstable endpoint evaluations, and suspicious weights. Retry at higher precision using existing numeric facilities. Never silently discard failed samples or substitute zero.

### Domain assessment and phase-two readiness

Use inexpensive sufficient sign/domain checks and runtime diagnostics. Distinguish:

- Certified no-threshold input.
- Explicit user assertion with inconclusive certification.
- Detected unsupported threshold behavior.

A user assertion is recorded in artifacts and results and does not suppress detected violations.

Keep input domains, coordinate maps, Jacobians, branch/phase metadata, and numerical evaluation separate. Retain general Symbolica map expressions at the domain boundary, even though phase-one sector maps are monomial.

This permits later domain splitting and complex contour maps without redesigning subtraction, kernels, or integration. Implement only the no-threshold strategy now. Future references are [Numerical Loop-Tree Duality: contour deformation and subtraction](https://arxiv.org/abs/1912.09291) and the [GCAD-based no-deformation approach](https://arxiv.org/abs/2603.05444).

## 4. Numerica QMC, CLI, status, and persistence

### Numerica/Havana QMC lane

Add `numerica::numerical_integration::qmc` as a separate library lane. Preserve existing MC interfaces and behavior; do not add QMC samples to accumulators that assume independent Monte Carlo points.

Expose distinct types for:

- `Rank1Rule`: modulus, vector, dimension, provenance.
- Randomization plans: seed and stable shift identities.
- Work packages: immutable rule/randomization references and point ranges.
- Worker-local point generation into reusable buffers.
- Partial results: per-shift counts and vector sums.
- Accumulators and estimates: complete replicate means, covariance, errors, and completion state.

Numerica owns neither the integration loop nor a thread pool. Callers choose workers, dispatch packages, evaluate integrands, merge results, and stop.

Generate points by index inside workers with exact modular arithmetic. Do not materialize full lattices centrally. Keep point and shift identities independent of worker count and completion order. Use canonical work packages and ordered compensated reduction for reproducibility.

Bundle the Kuo extensible generating-vector data distributed with QMCPy, with its license, attribution, source revision, construction range, and checksum. The author documents these as randomly shifted rank-one rules with power-of-two sizes. Production requires only numeric data and Rust code. [Author's catalog](https://web.maths.unsw.edu.au/~fkuo/lattice/), [QMCPy distribution](https://github.com/QMCSoftware/qmcpy).

Default to 4,096 points, 64 shifts, and Korobov exponent 3. The bundled rule supports refinement through \(2^{20}\) points; further automatic refinement adds independent shifts. Caller-supplied vectors support other moduli and larger rules. Reject unsupported explicit size requests clearly.

Estimate uncertainty from completed randomized lattice means, never from individual correlated points. Track covariance among Laurent coefficients. Reject duplicate or overlapping contributions, and retain unfinished work for resumption.

### FastSecDec integration sessions

Provide caller-driven integration sessions with operations equivalent to `next_work`, `submit`, `estimate`, and `checkpoint`. CLI code supplies execution and parallelization.

- **Democratic QMC:** evaluate all sectors with shared shifts; sum sectors within each shift before estimating total uncertainty.
- **Adaptive QMC:** use a separate pilot to estimate sector cost and variance; freeze each production allocation, use independent sector randomizations, and propagate independent sector covariance correctly. Do not treat pilot observations or unequal lattice stages as interchangeable production replicas.
- **Havana MC:** reuse current grids and adaptation. Require meaningful sector coverage before certifying errors or allowing accuracy-based stopping.

Integrate complete Laurent vectors over complete sector support. Preserve real/imaginary covariance where applicable. Stop only on completed, statistically valid estimates satisfying the requested tolerances or explicit work/time limits.

### CLI and status interfaces

Provide a compact command set:

- `generate`
- `integrate`
- `run`
- `inspect`
- `benchmark`
- `check-boundaries`

Use `clap`, `ratatui`, terminal support, and table rendering in the CLI crate. Avoid legacy backend-selection switches.

Core status objects contain structured data without ANSI escapes or terminal dependencies. Include generation progress, stage timings, sector counts, kernel counts, integration iterations, Laurent estimates, uncertainty validity, coverage, rule sizes, precision rescues, throughput, and stopping reasons.

Expose an event/observer interface and coherent snapshots. Each completed generation unit and integration iteration updates state. Render at a bounded rate so presentation does not become a bottleneck.

Interactive terminals receive a live dashboard with clear colored tables, progress, numerical results, and diagnostics. Noninteractive execution receives stable plain output or JSON events. Respect monochrome output, terminal resizing, and cancellation. Restore the terminal reliably on errors and interruption.

### Artifacts and checkpoints

Persist versioned manifests, symbolic/evaluator data, dependency identities, numerical conventions, compiler settings, and input hashes.

**O2 alone does not make machine code portable.** Portable artifacts retain expressions/evaluator IR and rebuild JIT kernels for the current architecture. Machine-code caches are optional and target-specific.

Checkpoint integration settings, rule and shift identities, completed ranges, statistical state, and precision diagnostics. Write atomically. Resumption must reject incompatible inputs or settings and must not duplicate work.

## 5. Milestones, audits, and acceptance criteria

Commit each milestone after its implementation and independent review pass. Record evidence and remaining work in the saved plan.

| Milestone | Deliverable and required gate |
|---|---|
| 0. Foundation | Saved plan and active goal; ignored references; modular workspace; reproducible Rust environment; pinned dependency identities; subagent ownership |
| 1. Reuse and baseline | Executable reuse probes; full test/example inventory; frozen external reference results and matched O2 benchmark protocol |
| 2. Native input and parametric construction | HEPKit/Linnet ingestion; model/kinematics adapters; `U/F` reuse; numerator conversion; independent normalization and graph-factor audit |
| 3. Native sectors | Exact geometry, maps, Jacobians, symmetry reduction; analytic domain checks and early hard-example generation benchmark |
| 4. Direct kernels | Subtraction, Laurent expansion, prefactor convolution, O2 kernels and precision rescue; independent mathematical and endpoint audit |
| 5. Numerica QMC | Worker-local generation, randomization, reduction, covariance, serialization; unchanged Havana MC tests |
| 6. Integrated application | MC and both QMC schedulers, checkpoint/resume, typed status, polished CLI dashboard |
| 7. Scientific parity | All example families and relevant regression behaviors reproduced in Rust |
| 8. Performance and completion | Profiling and optimization; independent final audit; documented platform evidence; final milestone commits |

Numerica work can proceed alongside symbolic and sector work once interfaces are agreed. UI work can proceed against recorded status events. Keep mathematical reviews and performance work as explicit agent assignments, not informal final checks.

### Scientific tests

Build a traceability matrix for the 182 inspected Pathfinder test functions and all examples. Port scientific requirements; document intentional retirement of tests tied solely to removed Python, pySecDec, backend, or serialization conventions.

Reproduce the scientific content of the 15 DOT examples, 11 kinematics fixtures, 17 run cards, stored targets, and hard-polynomial report under `/common/dev/fastsecdec/examples`.

Required coverage includes:

- Massive/massless triangles and boxes; double and triple boxes.
- Off-shell and rank-one/rank-two/high-rank numerators.
- Two-loop kite, three-loop self-energy, and multiloop three-point examples.
- Direct `U/F` double box, hard four-loop positive-orthant integral, and `issue_1` regressions.
- Native graph weights, numerator prefactors and projectors applied exactly once.
- Routing invariance, raised powers, propagator cancellations and Gram-degenerate scalar inputs.
- Exact map/Jacobian identities, cone coverage, non-simplicial geometry, and infinity charts.
- Analytic cube, projective, and positive-orthant integrals.
- Endpoint subtraction, prefactor poles, requested expansion depth, and zero-dimensional sectors.
- Eager/JIT/high-precision agreement, including the reference's documented double-box JIT concern.
- Worker partition invariance, covariance-sensitive cancellations, incomplete shifts, interruption, and resumption.
- Statistical coverage across independent seeds and unchanged existing Havana behavior.
- Dashboard rendering, plain/JSON output, terminal cleanup, and structured error reporting.

Normal tests and builds must run without Python, pySecDec, FORM, Normaliz, or a reference installation. External reference execution is confined to development comparisons; saved numerical fixtures carry provenance.

### Performance acceptance

Complete the plan's capability and scientific coverage before the broad
optimization campaign. Performance work needed to make a required example
generate or evaluate successfully remains part of closing that capability gap.
Retain the current baselines and all unsuccessful experiments; do not replace
missing full-vector coverage with a favourable throughput measurement.

After coverage, assign focused generation, convergence and evaluator work only
where needed to resolve a remaining acceptance gap, with independent reviews.
Profile factored generation and compiler costs; compare published lattice rules
and periodizing transforms on frozen multi-seed workloads when convergence
requires it; and prioritize reducing both average and maximum observed sample
evaluation time where necessary for parity. Reuse accepted evidence and do not
broaden the representative set to pursue marginal improvements. Keep sector IDs, full coefficient
vectors, dimensions, sample/batch sizes, ordinary arithmetic and rescue counts,
precision, and point-generation/transform costs with those observations. Report
batch-amortized time separately from individually timed sample latency, quantify
instrumentation overhead, and retain tail observations rather than dropping
slow boundary samples. Use caller-owned structured diagnostics so the same
measurements can later be exposed through HEPKit. Distinguish a faster numerical
kernel from fewer samples needed for verified accuracy.

Benchmark on the same host with matched O2, worker count, numerical precision, transforms, lattice rules, shift counts, coefficient orders, and statistical targets.

Measure separately:

- Cold generation and compilation.
- Warm artifact loading.
- Complete-vector kernel and integration throughput.
- Fixed-work integration.
- Time to verified accuracy.
- Peak memory, point-generation cost, reduction cost, and rescue rates.

Report an explicit eight-physical-core time to one-per-mille estimated relative
uncertainty for the coefficient at the **largest signed requested epsilon
power**, together with independent reference checks and the complete vector.
The user clarified that this is what they meant by the hardest coefficient:
usually the finite part for a request through order zero, and the highest
positive order when requested. It is not the most singular pole. Record the first observed crossing,
actual work, seed/rule/transform and worker affinity; separate generation and
loading from integration. If the selected coefficient is known to vanish, use a
labelled absolute zero check rather than dividing by zero or a noisy near-zero
estimate; do not silently switch to another coefficient. Do not
infer eight-core timing by scaling a one- or two-worker measurement. Keep
per-sector average worker cost, maximum sector-average cost and instrumented
individual-sample maximum latency as distinct metrics.

Use at least seven paired repetitions for ordinary cases and three for expensive cases. Alternate execution order. Require timing medians within the agreed **5% band per representative case**; investigate and resolve larger regressions.

Because sector partitions may differ, scientific time to accuracy and total work take precedence over misleading sector-by-sector timing comparisons. Use identical points for evaluator comparisons wherever expressions can be matched. Use Pathfinder's normal boundary-support, optimized QMC configuration while retaining its complete physical coefficient vectors and correlated all-sector sum. Record actual evaluator dispatch; requesting optimized evaluators does not prove that a fused path ran. Earlier full-support, optimization-disabled runs remain explicitly labelled historical observations.

Representative gates cover small one-loop cases, double box, a numerator-heavy multiloop case, and the hard four-loop input. Fast results with inaccurate estimates or underestimated errors fail.

Provide Linux/macOS target configuration and testing where hosts are available. Clearly distinguish executed tests from unverified targets. Do not claim performance parity or platform validation without evidence.

The goal is complete only when the scientific suite, required numerical audits, dependency separation, example delivery, and performance gates are satisfied. Partial implementations and unresolved benchmark regressions remain unfinished work.

## 6. Verbatim original prompt and subsequent requirements

The following original prompt must remain verbatim in `FIRST_PHASE_PLAN.md`:

````text
So our goal now is essentially to re-implement the path-finder experimental fastsecdec pathfinder code hosted at:


```
/DO_NOT_PUSH_FOR_REFERENCE_ONLY/FastSecDecPathFinder
```



however with many important differences which I will come to list here.



a) Contrary to `FastSecDecPathFinder` the \*entire\* implementation should be in rust, nothing in python. The pyo3 python API will live entirely in the HepKIT API bridge hosted in `./DO_NOT_PUSH_FOR_REFERENCE_ONLY/symbolica-community`

At a later point you'll then be able to add a feature branch there to develop the python API connecting to fastSecDec.



b) Note that we still want a CLI for `FastSecDec` allowing for use and tests independently of HEPKit. There, the conventions can be similar to `FasSecDecPathFinder` except that many options should be removed (as we should now only aim at the "direct" implementation of sector integrand with change of variables substituted in directly). Also the input format should really be centered around the HEPKit Graph dot format, meaning in particular that any numerator conversion needed should happen without assuming any difference in the current way mult-iloop graphs are being supplied.



c) There should be *NO* dependency on pySecDec anymore, meaning that you must independently re-implement the sector identification tools used by pySecDec in fastsecdec.



d) In general, \*ONLY\* implement a compute-algebra-type-of-feature in fastsecdec if you have triple-checked that this capability is not already available in symbolica-community or hepkit. Only then can you implement your own version.



e) You can find in `./DO_NOT_PUSH_FOR_REFERENCE_ONLY` a lot of material which serves as useful look-up or, for the rust one, as dependencies you can link against directing in `Cargo.toml` so you can do local edits there when needed. However, in general and except for the lattice QMC implementation discussed later, you should keep all you implementation in fastsecdec and not in any dependencies. However, if there is a bug or a severe limitation, you can apply a local patch to those dependencies which I can escalate to the author (who are close collaborators of mine) and which will be implemented upstream quickly then.

But try to keep these patches short and to a minimum with just what is essential for proper working of your implementation of `FastSecDec`.



f) As said before, you have to re-implement from scratch a quasi monte-carlo integrator, and you should do this in a feature branch of `numerica` (also in the references provided to you). It should be seamlessly integrated within `Havana` without disturbing its current Monte-Carlo capabilities but extending it with a separate lane implementing the lattices needed for quasi monte-carlo. Make it possible to let the lattice generation take place *by the parallel worker* as it may be a significant workload for faster integrands.

As for normal havana, the qMC capabilities should remain a library, i.e. the user is the one steering the integration loop and the parallelization.



g) The CLI should deliver exceptionally clean visuals during integration and generation, using beautiful coloured tabled views, with similar data as already shown in `fastSecDecPathFinder`. Also keep in mind that for future implementation of the pyo3 python API in HEPKit, it's nice if all the information you show through the CLI is abstracted behind clear status structs with display functions, so that the port to a python API can happen later seamlessly by mirroring these structures in python.

In particular during generation, I would be nice to have a `ratatui`-like dashboard streamed environment which updates after each iteration with live results and crystal clean display. Once again, each stream update should be properly abstracted so that later it's easy to integrate in HEPkit.



h) There are a bunch of tests in `fastSecDecPathFinder` and examples (which you should reproduce in an  `./examples` folder) which you should all have available in the end in your implementation of `fastsecdec`. In particular performances *must* be on par or equal to what you get with `fastSecDecPathFinder`. Compiled kernels by default should use `symJIT` O2 (so that the output remains portable).



i) You are starting the implementation of this project from scratch (although you have many references to draw directions from), so you must spend considerable amount of time studying the structure you need with a clean design, many submodules (or even subcrates) and not a large flat collection of large files. This will be a large project and I need you to spend considerable time and effort on it.



j) You should for now focus on this first phase-only where you will cover "no-threshold"/"euclidean-like" integrals (same as what fastSecDecPathFinder) currently supports. However, once you'll have completed this first phase, we'll proceed to plan for the second one which will support an advanced contour-deformation idea (similar to `./DO_NOT_PUSH_FOR_REFERENCE_ONLY/literature/1771932.pdf`, but applied to feynman integrals) and also a no-contour deformation which will use a Generalized Cylidrincal Algebraic Decomposition (GCAD) to avoid contour deformation, how it is described in \`./DO_NOT_PUSH_FOR_REFERENCE_ONLY/literature/2603_05444.pdf\`. Although you should not implement any of this yet, the structure of your implementation must keep in mind that this will need to be implemented next, so it should be already ready to welcome it.



Write a plan for the above, which *must include verbatim this prompt in it*. Also add a goal statement in it and the instruction to write it in `FIRST_PHASE_PLAN.md` and set it to yourself as a goal upon starting its implementation. You should also work within the main branch of the local repository and periodically commit to it at various key implementation steps/milestone. I'll give you a remote to push into at a later point.



Ask any question you may have to make sure the path is fully clear and devoid of any ambiguity.
````

Subsequent requirements, also preserved verbatim:

```text
Yes, it's important that you specify in your plan that you DO NOT NEED TO stick to any existing convention of pySecDec or FastSecDecPathFinder, you only want to be able to achieve the same end-to-end examples, but you should chose the design that you find most flexible, appropriate/natural for Symbolic and our design and also most efficient. That is the most important criterion.
```

```text
And with inputs most in-line with the library environment and existing example in HepKit, although for now you focus only on a CLI for steering examples and tests.
```

```text
Also make sure that you link against existing crates of the HEPKit ecosystem to borrow functionality that are needed, In particular linnet will be needed to implement a lot of graph functionalities and primitives that you'll need. Same for the Graph dot objects that you parse where HEPKit toolset will provide many primitives you need already.
```

```text
And include in the plan that you should use many subagents to organise the various implementation slices, their audit, the research, the debug, the performance hunt, etc... you will be acting mostly as a coordinator only!
```

```text
Continue as planned, but note that the `All 15 historical DOT` must not remain identical, you should instead start from a phyically equivalent set of dot file fully compliant with the new HEPkit standards.
```

```text
(And when in doubt of how to achieve good performance, dissect how `FastSecDecPathFinder` achieved its work; we don't want the same structure necessarily, and we want it fully in rust and with no pySecDec dependencies, but its innerworking should give a good close-to-optimal implementation plan, although you can ignore building the sector integrands from a single dual evaluation of the U and F polynomials, this idea proved to be too slow at runtime).
```

```text
(Continue as planned, but indeed avoiding expansions unless absolutely necessary is key for a powerful use of Symbolica (which unlike FORM does not need to ever expand))
```

```text
Continue as planned, but periodically run an audit verifying that your implementation is perfectly setup for a future integration in HEPKit and also uses HEPKit primitives wherever it can, without re-implementing its own versions (e.g. for numerical test/cross-checks the one-loop master scalar integrals are already available there, and even on-loop reduction if need be; that's just an example, your auditing subagents should review that aspect broadly).
```

```text
Ok, continue as planned, but probably a good point at which you can now commit+push fastSecDec to:
[https://github.com/alphal00p/fastSecDec](https://github.com/alphal00p/fastSecDec)
and periodically push there when you reach milestones.
```

```text
(continue as planned, but don't forget that one potential useful way to get a small expression for each evaluator is to encode the change of variables as a function map int he evaluator builder arguments, and play with the possibility of inlining aliases or not (though this need generation and runtime generation check)). Make sure you are on the latest symbolica and SymJIT realeases when doing all that.
```

```text
(continue as planned, but if you already have a final version of the lattice QMC version for numerica, you can open a PR for merging into its main and tag BenRuijl as a reviewer)
```

```text
no the reviewer you must ask for is `benruijl`
```

```text
Continue as planned, but once you have full capability coverage within goal plan, look into optimizing performance of both generation, lattice and transform choices for convergence, and evaluation time per sample and for each sectors (mostly focusing on getting max and average sample evaluation time per sector down).
```

```text
Continue as planned, but summarize where you're at regarding what's planned to being implemented for the plan, and what are your current performance measurement (generation time, convergence time to 1 per mil convergence for deepest pole on 8 cores, and max and avg run time per sample and sector)
```

```text
Continue as planned, but summarize where you're at regarding what's planned to being implemented for the plan, and what are your current performance measurement (generation time, convergence time to 1 per mil convergence for deepest pole on 8 cores, and max and avg run time per sample and sector). Organise the table showing your result putting them side-by-side with the same results from the FastSecDecPathFinder ref. code. Then continue with the implementation as planned.
```

```text
Can you build an MRE for this symbolica bug that I could escalate to the author, i.e. Ben Ruijl? Put it in a standalone folder in the workspace (and make it in the form of a standalone rustscript reproduction, with a README describing it and the patch for the fix you'd want). Delegate an agent to setting up this MRE and continue on your side as planned.
```

```text
Sorry when I meant the "deepest" I always mean the hardest to compute which is the one with the largest signed epsilon power (so often, the finite part), not the deepest pole of course, sorry.
```

```text
Continue as planned, but don't overdo it on the performance part, and once you're on par and feature-full regarding the objectives of the first phase of the active goal, declare it completed and stop yourself.
We'll then draft a plan for approaching the physical cases with threshold.
```

```text
Continue as planned, but don't push the pySecDec/FORM references where it can't complete on this machine. Only make sure the fastSecDec version completes there and only benchmark against cases that complete within reasonable resources in the original pySecDec. You should however always be able to benchmark against fastSecDecPathFinder.

And as I said, once you're feature complete within what's stated in the goal, and reached parity or better everywhere according to the metric I mentioned earlier vs fastSecDecPathFinder, then commit+push, set the goal as completed and stop yourself so that we can plan together for the next step.
```

## Implementation record

- 2026-10-06: the sole existing adaptive-QMC hard-four-loop trial completes all
  699 sectors with independent production shifts and a discarded fresh pilot.
  Its finite coefficient is -149.6820571 ± 0.1683125, or 1.124467 per mille:
  the target is narrowly missed. The numerical process takes 316.246 seconds,
  including loading and pilot; ordinary generation with the declared production
  budget takes 11.399 seconds and reproduces the exact kernel payload. All three
  available reference checks, all 700 covariance checks and cleanup pass, with
  zero evaluation failures. No kernel, geometry, precision or default changes
  follow, and no further allocation is run under this bounded diagnostic. The
  highest-order target and representative parity remain open.

- 2026-10-06: the first hard-four-loop fixed-work pair completes both full
  numerical vectors with all available reference checks passing. Fresh native
  generation takes 12.976 seconds. Eight-worker integration takes 68.735 seconds
  natively and 279.689 seconds in Pathfinder, using different published lattices
  and sector partitions. Finite-part uncertainties are 65.225 and 53.177 per
  mille: neither reaches the target. Native covariance and every Pathfinder
  Laurent row remain intact; its six lower zero rows agree with the separate
  whole-input certificate. The independently verified shared-shift covariance
  motivates one existing adaptive-QMC trial with independent production shifts,
  without changing kernels or the precision policy. See
  `docs/reviews/hard-four-loop-pathfinder-capability.md`.

- 2026-10-06: a native evaluator arithmetic-optimization probe is rejected at
  its strict physical comparison despite reducing operation counts by 24.1%.
  A separate public `optimize_stack()`-only candidate preserves all 7,752 tested
  complete vectors and precision/replay decisions bit for bit, while reducing
  aggregate temporary slots by 76.7% with unchanged arithmetic. Its independent
  MPFR checks retain 60 identical existing ordinary-lane tolerance exceedances;
  no blanket point-accuracy claim follows. The sole full-workload timing pair
  preserves all 835,584 vectors per lane and complete covariance, but reduces
  callback time by only 0.921%, below its predeclared 5% interest threshold.
  Production stays unchanged and this optimization track stops.
  See `docs/reviews/native-evaluator-storage-probes.md`.

- 2026-10-06: cold native-program loading now conservatively recovers literal-zero
  output facts through Symbolica's public instruction export. Deferred constants,
  computed outputs, control flow and non-inlined bodies remain unproved. This
  prevents zero padding from forcing unnecessary MPFR rescue without relaxing
  precision checks; artifact bytes, IDs and schema remain unchanged. Twenty-four
  focused native artifact/replay controls, formatting and scoped strict Clippy
  pass, with independent source/test review. Real and complex tiny nonzero values
  still receive weighted recovery. A separately built and independently reviewed
  repeat of the same complete scalar-double-box K2 allocation retains identical
  kernel payloads and shift plans: elapsed time falls from 172.715 to 152.996
  seconds, and rescues fall from 46.52% to 31.53%. All 103 vector/covariance
  estimates differ only within retained roundoff bounds (maximum mean difference
  1.25e-14), all five reference checks pass, and the 0.713577 per mil target remains
  met. Seventy-one additional weighted replays remain enabled and recorded.
  This is a single observed improvement, still slower than Pathfinder's
  91.584-second target-reaching run. One subsequent trial of the existing IBP
  strategy also passes all five reference checks and reaches 0.978 per mil, but
  takes 164.945 seconds and nearly doubles serialized evaluator size. Taylor
  remains the default; fewer rescues alone do not establish a speedup. No further
  IBP tuning ladder is planned. The accepted earlier notebook wheels retain
  their own source pins.

- 2026-10-06: the corrected full-stream box latency comparison includes lazy
  first-use work in both providers. All 393,216 complete-vector rows per provider
  pass retained scientific-equivalence checks. Native mean/observed maximum are
  9.878479 microseconds / 5.863705 milliseconds; Pathfinder gives 70.390258
  microseconds / 43.051430 milliseconds. The reference maximum is unattributed,
  and finite observations are not worst-case bounds. Independent data/source
  review accepts the corrected ordering, coverage, complete covariance,
  precision/replay and historical numerical identity. The earlier asymmetric
  measurements remain recorded. No further box timing or cache tuning is needed
  for this observed comparison. A single scalar-double-box Korobov2 allocation
  then reaches 0.713577 per mil finite-part relative standard error in 172.715
  seconds on eight workers: 13,369,344 samples, 102 complete sectors, all five
  independent reference checks passing and zero failed evaluations. Generation
  takes 2.810 seconds including 0.367 seconds of O2 compilation; exact numerical
  payload identity with the accepted kernel is preserved. Full vectors and all
  103 covariance matrices are retained. The initial reference-envelope parsing
  failure performs no algebra or sampling and is preserved separately; its
  transport-only correction is charged to the same bounded attempt. The new
  timing is one different-transform observation against Pathfinder's retained
  275.506-second Korobov3 result, not final paired parity. The subsequent same-K2
  reference reaches only 5.062 per mil at prime 8,311 in 42.756 seconds, then
  reaches 0.362230 per mil at prime 17,807 in 91.584 seconds. Both complete their
  five reference checks. Reduced all-sector sample timing gives native/reference
  means 106.314/74.846 microseconds and observed maxima 44.481/52.645 milliseconds;
  full off/on numerical controls and independent data reviews pass. The native
  mean and time-to-target gaps remain real. No further reference ladder follows.
  A source audit identifies native Taylor-conditioning costs and lost literal-zero
  facts after cold loading; an exactness-preserving loader correction and the
  existing IBP strategy are the bounded next investigations. Other representative
  parity gaps remain open.

- 2026-10-06: the notebook now owns an automatic refresh timer only during
  active caller-driven integration. Public marimo state preserves the buttons,
  generated objects and session while disabling timer traffic during generation,
  pause, completion or failure. Twenty-four focused controls and marimo validation
  pass. The actual Wasm triangle verifies timer absence throughout 19 generation
  observations, explicit Integrate/Resume activation, and exact preservation of
  a 3,072-point paused prefix through 16,384-point completion, with zero browser
  errors. A corrected custom-element locator completes this lifecycle in 45.582
  seconds; its earlier 47.295-second driver failure remains retained. The wheel
  remains the separately identified accepted public build. This closes the
  observed automatic-refresh issue, without claiming a new gg→HH browser run
  or universal responsiveness during synchronous native calls.

- 2026-10-06: the original hard-four-loop Pathfinder input now passes generation
  and strict bundle loading within one 600-second/15-GB attempt. The 528.820-second
  supervised sequence retains 3,728 nine-dimensional sectors, all orders
  `[-8,...,0]`, the full orthant and unchanged unit-prefactor density. Peak sampled
  RSS is 1.18 GB; all inputs remain unchanged and all owned processes are reaped.
  Independent review accepts this reference capability, without an integration
  or matched-performance claim. Existing native results and their Laurent-range
  qualifications remain unchanged. The independent scope review confirms that
  a subsequent comparison may use the three common numerical orders and the
  separate exact whole-input zero certificate through minus three. Equal chart
  counts or a nine-row native estimator are not required; preserve the old
  covariance and every provider row without numerical padding.

- 2026-10-06: one existing adaptive-QMC scalar-double-box allocation takes 73.298
  seconds including its four-shift pilot, loading and production. Native
  allocation uses the same N8192/Korobov2 kernels and 320 summed worker seconds;
  the pilot is excluded from production statistics and sector streams are
  independent. Complete vectors/covariance and five independent reference checks
  pass, but the finite-part uncertainty is 1.035 per mil, narrowly missing the
  one-per-mil target. One subsequently authorized allocation changes only the
  production budget to 400 worker seconds and reaches 0.989 per mil in 82.859
  seconds, again including a fresh pilot and loading. Its complete vectors,
  covariance and five references pass. Both attempts cost 156.157 integration
  seconds plus 6.704 generation seconds; their estimates are not combined.
  The successful row is below Pathfinder's retained 91.584-second row, but does
  not establish timing medians or individual-sample parity. No further budget
  ladder or default change follows.

- 2026-10-06: a bounded same-exact-IR SymJIT indirect-translation probe is rejected
  before timing because a complete-vector comparison exceeds the unchanged
  1e-12 relative gate at sector zero, point 37. A fresh public native weighted
  context provides a 128-to-256-bit MPFR check at that point: the existing direct
  result has 5.57e-13 relative error, while indirect has 3.48e-12. The direct
  backend and accuracy policy remain unchanged. This is a one-point numerical
  qualification, not a general backend defect or performance measurement.

- 2026-10-06: retained generation explainability and discrete-sector Havana
  are implemented. Native source charts preserve mapped prefactors, exact affine
  epsilon powers and required endpoint Taylor counts before subtraction;
  actual shared evaluator operation counts, exact-program bytes and serialized
  SymJIT application sizes are exposed through Rust, CLI and thin HEPKit views.
  Legacy metadata absence remains explicit. Fresh gg→HH generation/reload
  retains all 30 charts, 54 terms and 324 powers, with maximum denominator
  power `a=0`, zero required endpoint Taylor coefficients, and unchanged exact
  evaluator programs. Its HKKN QMC allocation
  reaches 0.093706 per mil finite-part relative standard error; the new ordinary
  Havana allocation reaches 0.576957 per mil and agrees within 1.70 combined
  component standard errors. Full vectors/covariances and completed-checkpoint
  restoration are retained. The native notebook passes explicit generation,
  rich inspection, QMC cancellation/resumption, same-kernel New integration,
  and Havana pilot/frozen-production cancellation/resumption. Tests pass:
  419 workspace tests (25 explicit heavy ignores), 81 installed Python controls,
  41 portable-backend host controls, formatting and strict all-target Clippy.
  The final equivalent Option-guard cleanup also passes all five discrete-MC
  controls. Numerica's essential one-line nested-grid clone fix is published
  at `f6ecdac` on PR 8. FastSecDec is published at `a3d09e1`; community PR 18
  pins that implementation and its generated stubs at `c9bacce`. The subsequent
  workflow-only `b8279dd` adds the notebook test dependency missing from hosted
  CI, which now passes all 81 controls. The new public Wasm wheel passes its
  generic smoke and all 58 portable controls. Its actual browser triangle
  completes generation, retained-metadata inspection, QMC and ordinary Havana,
  including exact accepted checkpoint-prefix resumption. Retained full means
  and covariance agree exactly with the native notebook. The raw browser
  harness remains failed solely at a supplemental screenshot after completed
  scientific and lifecycle assertions; an additive independent review qualifies
  those retained results without rewriting the failure. The sole bounded
  browser gg→HH trial reaches generation-ready with all 30 sectors after
  546.667 seconds. It then times out downloading its first report: no browser
  inspection, integration or resumable prefix is validated. Long native calls
  also produce marimo refresh-RPC timeouts, with streamed progress still visible.
  All owned processes are reaped after 596.001 seconds; sampled peak RSS is
  3.86 GB. The example remains selectable; the scalar triangle remains first.
  Native gg→HH coverage is unchanged. Representative parity gaps remain open;
  this milestone does not complete phase A. See the metadata and sampling
  reviews in `docs/reviews/`.

- 2026-10-05: the requested ownership correction is implemented locally.
  FastSecDec owns the isolated optional Rust/PyO3 crate, modular notebook,
  native input helpers, fixtures, dedicated tests and delivery scripts;
  community retains linking, registration, reexports, stubs and thin build/CI
  forwarding. Independent review accepts the boundary and confirms the core
  and default CLI remain Python-free. A normal release wheel passes all
  **61 installed controls**, including six native sector/chart/alias inspection
  tests and nine explicit-action/report tests. Actual native browser lifecycles
  pass for triangle and gg→HH: separate Generate/Integrate actions, native compact
  sector/chart inspection before sampling, streamed generation and full-vector
  displays, and checkpoint-preserving Cancel/Resume. The sole gg→HH run generates
  30 sectors in 122.489 seconds from the button press and completes all 245,760
  points; its four means equal the retained CLI values and all 16 covariance
  entries agree within rounding. Its 89.326-second active integration includes
  refresh waits on one caller CPU, distinct from the earlier eight-worker CLI
  timing. Finite relative error remains 1.027%, not 0.1%. Independent review
  also accepts the final history presentation using seven focused controls and
  a retained-data browser replay, with no repeated gg→HH calculation. Generated
  stubs pass public inventory, installed-export and Python 3.9 grammar checks,
  including all eight new inspection classes. The new public dependency pin
  and current Pyodide execution are still open. Performance work
  stays parked until these requested notebook and ownership gates are reviewed.
  See `docs/reviews/hepkit-relocation-audit.md`.

- 2026-10-05: delegated exact-input gg→HH reference feasibility is complete.
  Pathfinder and direct pySecDec both time out during generation, with charged
  times 597.533/597.402 seconds including a conservative ten-second preparation
  allowance each. Sampled peak RSS is approximately 132.10/76.80 MiB. Neither
  produces a ready artifact or numerical estimate; both process groups are
  reaped, source/read-cache checks pass and independent review accepts the input
  transport and bounded outcomes. No retry is planned. The retained native CLI
  result remains 61.285 seconds generation and 8.781 seconds integration on
  eight workers, at 1.0273% finite-part relative standard error. This is a
  feasibility comparison, not matched performance or independent numerical
  agreement; see `docs/reviews/gghh-reference-feasibility.md`. The separate
  off-shell rank-two Pathfinder attempt also reaches its original 1,800-second
  limit without a bundle; its failure is retained without a retry. The current
  Korobov2 release CLI builds and passes source-owner postchecks, but no new
  physical K2 result is claimed. Native notebook gg→HH validation continues.

- 2026-10-05: optional Korobov-2 is published in Numerica PR #8 at `bb996e4`
  and exposed through native FastSecDec settings and CLI run cards. Numerica
  owns all transform arithmetic; Korobov-3 and Monte Carlo defaults are
  unchanged. Exact Rust 1.89 Numerica suites pass 243/246/223 tests. FastSecDec
  passes 37 focused tests, workspace all-target Clippy, formatting and the
  portable-host compile check. Independent source and bound-evidence reviews
  accept the additive dispatch, weight application, checkpoint identity and
  dependency pin; manifests and lockfiles are unchanged. No physical K2 timing
  or convergence claim is made. The user additionally requested bounded
  gg→HH Pathfinder/pySecDec feasibility checks, limited separately to ten
  minutes and 15 GB; the governing instruction is recorded above. Native
  notebook integration and those reference checks remain in progress.
  See `docs/reviews/korobov2-integration.md`.

- 2026-10-05: the fresh public-owner release passes strict scalar double-box
  generation/loading, with 102 kernels exactly equal to the retained native
  payload. Generation takes 4.441 seconds. One bounded larger-package run
  reaches 0.393 per mille finite-part relative standard error in 479.451 seconds
  on eight workers, preserving every sector estimate, total covariance and
  precision counter exactly. The default package size is unchanged because no
  timing benefit was observed. One existing-catalogue Pathfinder counterpart
  reaches 0.577 per mille in 275.506 seconds; no FORM or package generation is
  repeated. These single, different-rule observations leave performance parity
  open. HEPKit PR #18 additionally links the native gg→HH result in prose-only
  follow-up `a698db6`; its dedicated hosted CI terminates during dependency
  compilation without a reported cause, before tests. Local native 39-test and
  earlier actual browser evidence remain valid for their recorded builds.
  See `docs/reviews/double-box-lattice-followup.md` and the bridge review.

- 2026-10-05: the experimental native-input bridge and four-example marimo
  notebook are published as draft HEPKit PR #18 at `ab84c89`, authored by
  `ValentinHirschi`, on the merged external-state PR #17 base. A fresh installed
  native wheel from public FastSecDec `be9c3d2` passes all 39 focused tests with
  zero skips; source and installed-binary identities are verified. Actual
  Pyodide PEP 517 metadata passes with the helper's isolated Cargo home. The
  earlier actual Wasm tests and browser Run/Cancel/Resume evidence retain their
  original build identities. The draft documents the unpatched OneLOop/SymJIT
  resolution conflict, including when the optional feature is disabled, and
  the required explicit source preparation. Native gg→HH feasibility is closed;
  its browser cost and the remaining representative performance gates remain
  open. See `docs/reviews/hepkit-fastsecdec-bridge.md`.

- 2026-10-05: the bounded double-box follow-up reuses the existing kernels and
  Numerica's published HKKN catalogue. Its prescribed 8,192- and 16,384-point
  levels reduce finite-part relative standard error from 1.323 to 0.393 per mille;
  the final eight-worker process takes 437.039 seconds and both full executions
  total 664.554 seconds. All five independent reference comparisons pass, full
  covariance is preserved and there are no evaluation failures. Independent
  data audits accept both rows. This follow-up stops at the observed target;
  production defaults stay unchanged. These are observations on the frozen
  `8ecc406` build, not current-main timing medians or Pathfinder parity.
  See `docs/reviews/double-box-lattice-followup.md`.

- 2026-10-05: the gg→HH example now completes ordinary native CLI generation
  (61.285 seconds, thirty sectors) and an eight-worker full-vector allocation
  (8.781 seconds, finite-part relative standard error 1.0273%). Native Idenso
  color-invariant closure preserves the generated diagram conventions. This
  satisfies the native feasibility prerequisite; it does not establish browser
  feasibility or one-per-mille accuracy. The four-case HEPKit bridge passes
  39 native and 39 actual Pyodide tests. An actual marimo/Pyodide browser run
  cancels, downloads a checkpoint and resumes the triangle to complete coverage
  and its one-per-mille target, preserving accepted work. The other three cases
  have bounded portable generation/evaluation probes, not convergence claims.
  See `docs/reviews/gghh-native-feasibility.md` and
  `docs/reviews/fastsecdec-showcase-ui.md`.

- 2026-10-05: public dependency manifests and a pinned-source bootstrap replace
  machine-local reference paths. Fresh downloads reproduce all reviewed source
  contents and patches; native all-target Clippy, three CLI provenance tests,
  portable-host checks and locked metadata pass with unique ecosystem owners.
  Missing CLI dependency provenance fails explicitly. The existing lockfiles
  remain unchanged. The community bridge's actual published-Git validation and
  draft PR remain the next delivery gate; upstream OneLOop still requires the
  documented local SymJIT compatibility patch. No representative performance
  gap is closed by this source-delivery change.

- 2026-10-05: the actual FastSecDec portable library now generates, evaluates,
  round-trips artifacts and completes a QMC allocation in Emscripten/Node.
  Native kernel controls pass 52 tests (eight existing diagnostics ignored),
  portable host controls pass 39 tests, and the actual Wasm smoke matches all
  six means and 36 covariance entries with two successful 512-bit replays.
  Native O2 and artifact identities remain unchanged. Independent reuse review
  passes; browser/Python-wheel responsiveness is not inferred from Node.
  Factored Gaussian source elimination and native single-variable homogeneity
  admission also reduce the actual gg→HH parametrization to 1.881 seconds and
  about 69 MiB process peak RSS, with thirty focused controls passing. Shared
  generation snapshot observers now serve both CLI and future HEPKit adapters.
  Ordinary gg→HH sector generation/integration, the notebook bridge and remaining
  bounded performance gates stay open.

- 2026-10-05: the complete installed HEPKit community wheel passes fourteen
  wavefunction cases, its namespace/stub check and eight literal documentation
  examples, with the locked dependency set unchanged. PR #17 is ready for review.
  The shared native wavefunction delivery is complete; the FastSecDec bridge,
  browser execution and native gg→HH numerical prerequisite remain open.

- 2026-10-05: shared numerical external states are published on GammaLoop's
  `feynkit` branch as `6c707c6b7`, after six native physics controls, five existing
  GammaLoop regressions, the installed Python host check, stub generation and
  selected formatting pass. HEPKit PR #17 carries the dependency, generated
  stubs and user examples; it remains draft until the complete community wheel
  checks finish. All commits and publishing use ValentinHirschi. A separately
  reviewed additive Symbolica fallible-mapping API passes ten focused controls
  and keeps interpreter construction inside the existing native evaluator.
  The FastSecDec browser feature and gg→HH parametrization fix are still under
  validation and are not accepted by this dependency milestone.

- 2026-10-05: auxiliary helicity vectors now enter native `IntegralFamily` and
  `Kinematics` through the graph adapter and CLI, retaining canonical tensor
  labels and exact binary numerical values. Independent ecosystem review,
  19 affected native input tests, four CLI input tests, formatting and workspace
  all-target Clippy pass. The Rust gg→HH generator exports the requested
  Feynman-gauge top/gluon double box, its raw native factors, numerical (+,+)
  states and full provenance into `examples/gghh_double_box`. Corrected export
  takes 1.098 seconds; ordinary CLI generation does not finish within 600 seconds
  and peaks at 47.8 GiB, with no sector artifact or integral result. A separate
  native diagnostic finishes tensor contraction/input within 0.37 seconds and
  times out in Gaussian parametrization at 120 seconds. Retain the failures and
  narrow that stage before another full attempt. The
  notebook prerequisite remains unmet. Shared generic wavefunction publication
  tests and the portable evaluator feature split continue separately; no native
  performance acceptance or phase completion follows from this input milestone.

- 2026-10-05: the saved on-shell artifact resumes successfully with eight bounded contexts, reaches 397,312 evaluations with zero failures and peaks at 9.03 GiB. It is intentionally stopped after 1,305.505 seconds for a scheduling handoff, not by its deadline; its checkpoint is retained. Observed heterogeneous batches permit at most 32.09% worker-time utilization in the audited prefix. The CLI now refills free workers through the existing native task/submission/checkpoint APIs, retaining the context bound and coordinator-owned accepted replay. **20 focused driver tests**, formatting and scoped all-target Clippy pass; cancellation, in-flight checkpoint recovery, full covariance and panic draining are covered. An initial overlapping test abort is traced to Symbolica's occupied permit; a separate test-only failed-attempt count is corrected, with both records retained. The next same-allocation continuation has one 7,200-second bound. Pathfinder's supported guarded projector route meanwhile generates a complete 968-sector bundle through order zero in 39.637 seconds. A harness Gamma-metadata assumption is corrected against its native external-prefactor convention, enabling numerical reuse without regeneration. Neither bundle generation nor partial native work closes full-vector agreement or representative parity.

- 2026-10-05: the CLI now bounds QMC evaluator ownership to one active sector per worker and restores each sector's accepted native replay state on revisit. All **16 focused driver tests pass**, including full-vector/covariance equivalence with actual precision rescue, failed-prefix exclusion and eight-worker checkpoint continuation; formatting and scoped all-target Clippy pass. Native libraries and artifact/checkpoint formats are unchanged. The existing full on-shell artifact can therefore be resumed without repeating generation; its eight-worker, 1,800-second numerical continuation remains a separate measured capability gate. Pathfinder's cached attempt reaches its 1,800-second generation bound without producing a bundle or changing the 137 retained formula entries. Its processes are reaped and inputs verified; inspect the stall before another attempt. No infeasible pySecDec/FORM route is reopened.

- 2026-10-05: the committed persistence fix (`c3ec42a`) saves the entire original on-shell prepared graph successfully: 1,026 sectors and all orders −6 through 0, whole generation/compilation/publication **1,695.486 seconds**, peak **16,063,700 KiB**, artifact **957,122,802 bytes**. Separate-process cold inspection succeeds in **136.719 seconds**, peak **8,458,348 KiB**, with the same content identity and coverage. The subsequent original 180-second integration stage times out and is reaped after **186.784 seconds**, with 38,912/8,404,992 points in its last status, zero completed sectors and zero evaluation failures. It retains an accepted checkpoint but no result. All 71 frozen checks pass; artifact publication and reload are now established, while full-integral agreement and parity remain open. Continue numerically from the saved artifact using measured costs; do not repeat accepted generation. A fresh Pathfinder direct attempt resumes after the user pause with the same retained cache provenance. Final representative comparisons permit Pathfinder's actual default boundary/optimized QMC mode; earlier opt-out/full-support observations remain separately labelled evidence, not the final optimized-reference baseline.

- 2026-10-05: the persistence capacity fix passes **380 workspace tests**, zero failures and 23 explicit ignored probes across 63 summaries, plus formatting and all-target Clippy. Independent focused gates pass 16 native and 42 CLI tests. Borrowed native payloads and streaming hashing retain exact v3 bytes/IDs; cold loading avoids re-encoding an unused envelope. The CLI keeps embedded native JSON opaque, streams atomic output, retains legacy v1 identity/loading and explicitly writes a v2 outer identity. No algebra or dependency patch is added. These controls cover cold complete vectors, tamper rejection and interrupted writes; the bounded original full-graph rerun still determines capacity. The prepared Pathfinder direct continuation reuses 137 successfully built formula-cache entries with explicit provenance and unchanged physics; infeasible pySecDec/FORM routes remain closed. Representative performance work is limited to the existing five-case set and missing required metrics.

- 2026-10-05: the prepared native whole graph completes all 1,026 coefficient expansions (1,531.885 seconds) and all 1,026 sector JITs (127.529 seconds). An allocation then fails under the 30-GiB address-space cap before artifact publication. The watchdog subsequently reaches its deadline during core dumping and reaps the process after 1,807.811 seconds; all 60 frozen checks pass and no artifact or integral result exists. The exact failing allocation site is not established. Source review identifies avoidable full-buffer and per-byte JSON-value copies in native/CLI persistence; a focused Serde-based correction is being implemented with explicit legacy identity handling. This is a persistence capacity gap, not unfinished coefficient mathematics or successful full-integral acceptance.

- 2026-10-05: the latest user clarification ends further infeasible pySecDec/FORM attempts for the on-shell case. The existing Pathfinder direct route is the next comparator; its first attempt stops at a missing formula-cache entry after 19.186 seconds, before any complete bundle/result. The documented native fallback builder is the next configuration change, with the same bounded full-integral scope. A separately built matching Python binding imports correctly but its small smoke reveals an old Pathfinder return-type assumption; the existing working versioned environment remains available without an API-porting project.

- 2026-10-05: a fresh independent projected reference using the provider's documented pure-Taylor option avoids the earlier recursive IBP memory failure but reaches its 600-second generation deadline while writing FORM sector 41 of 968. Shutdown completes in 605.098 seconds; the outer process finishes in 608.398 seconds. All 62 frozen checks pass, all owned groups are absent and partial files remain; no complete reference vector exists. The prepared native fullgraph release passes its build and independent review and starts under the existing bounded protocol. No successful whole-graph or parity result is inferred from progress.

- 2026-10-05: the final delivery audit finds no additional missing required subsystem or HEPKit integration blocker beyond the original on-shell example and bounded numerical/parity ledger. Independent review reconciles the last regression-matrix row against its actual historical endpoint-equivalence requirement and existing native identities/full double-box vector: **182 rows, 101 Covered and 81 Retired, with none Partial or Pending**. No new numerical run or universal uncertainty claim is inferred. Full on-shell agreement and representative parity still govern goal completion.

- 2026-10-05: the CLI exposes the existing native family-preparation policy as an explicit opt-in, retaining `Original` by default. It preserves original source identities and records the native preparation/fallback report and actual active parameter count. The focused gate passes **38 tests**, formatting and CLI all-target Clippy, including the complete independently known `210 Gamma(eps)` vector through order one, cold artifacts and integration. No new algebra, graph or numerical implementation is introduced. The next full on-shell trial combines this exact preparation with NativeNamed coefficients.

- 2026-10-05: the original ten-parameter NativeNamed fullgraph trial was intentionally cancelled after 1,006.900 seconds at 372 of 1,026 representatives to use the existing exact prepared-family route. It was not a timeout and produced no artifact. Both independent external reference generation attempts reached their 30-GiB memory bounds: the original representation after 530.288 seconds and the equivalent eight-parameter representation after 431.801 seconds. Their partial output is retained, all children are reaped and frozen inputs pass their checks; neither supplies a numerical reference. Full-integral agreement remains open.

- 2026-10-05: the difficult captured representative now passes the ordinary public input/generation/compile/v3-artifact path and all three independent cold readers. Generation takes 66.752 seconds, compile/save 37.370 seconds, and the complete process 108.356 seconds. All seven orders through zero, the actual coordinate permutation, multiplicity one and literal-zero exact offsets are retained. Independent review accepts 21 signed coefficient/point comparisons and 24 weighted vectors (168 real components), including twelve forced 1,024-bit replays. The first ignored helper's structural expected-expression assertion failed before generation and was corrected without modifying the imported density or production source; that failure remains recorded. This closes public representative acceptance, not the original full graph or performance parity. The next work is the bounded full-graph trial and its exact-input independent reference.

- 2026-10-05: the user clarified the stopping rule: finish first-phase capability and scientific acceptance, establish representative performance parity with bounded effort, then mark the active goal complete and stop. No optional optimization campaign or threshold-phase planning follows automatically. Existing valid measurements should be reused; remaining work is directed at specific acceptance gaps.

- 2026-10-05: the validated native named-coefficient algorithm now has a public opt-in, with the physical route retained as default. Shared native Series/request modules, exact unregulated fallback, checked caller limits, cancellation, conservative conditioning profiles and exclusive typed progress/timing rejoin the existing multiplicity, kernel and artifact owners. The CLI uses the same native option types and preserves prior geometry/final output while coalescing only new coefficient-request JSON updates. **370 workspace tests pass**, with 23 explicit probes ignored; workspace formatting and all-target Clippy pass. Focused public controls include complete Taylor/IBP vectors, Gamma/complex coefficients, context/dispatch, weighted precision replay, worker clones and separate-process cold artifacts. Two test-only compile calls, two test-only lint findings, a wrong-stream CLI assertion and an actual presentation-cadence regression were corrected with their failed evidence retained. The captured representative through the public path, complete original on-shell graph, convergence/calibration and matched performance/platform gates remain open. No performance row is updated by this acceptance.

- 2026-10-05: the unchanged Numerica QMC branch passes its declared minimum Rust/Cargo 1.89.0 on Linux x86_64: **237 default, 240 serde and 217 alternative-backend tests**, with zero failures or ignored tests. Independent review verifies exact commands, versions, all 73 source hashes and both manifest/lockfile hashes. The published PR description now records this gate and retains the requested `@benruijl` mention. No Numerica source changes were needed. macOS/other-architecture qualification, FastSecDec's own platform gates and matched performance remain open.

- 2026-10-05: the first production adoption slice extracts shared native endpoint admission and checks cancellation-degree overflow. The independently accepted focused gate passes **50 tests**, with two explicit probes ignored; formatting and package all-target Clippy pass. Existing physical Taylor/IBP schedules, exact pruning, error precedence and defaults are preserved. A separate native symbol-hygiene probe rejects all 17 foreign metadata/hook conflicts without executing callbacks, retains occupied-input names, and validates stable repeated names and independent simultaneous alias maps. This closes the native API/probe prerequisite; the actual production allocator and public named-route integration remain required. The latest full workspace gate is still the earlier 336-test run. Complete on-shell generation, convergence/calibration and matched performance/platform gates remain open.

- 2026-10-05: all three independent original-expression native oracles are accepted, completing in 28.316, 32.790 and 31.645 seconds after the preserved export-only failure was corrected using native dynamic precision and round-trip formatting. The cold candidate reader independently passes all 21 signed-order/point comparisons and 24 complete weighted evaluations (168 real components), including twelve forced replay vectors and rescue at 256/384 bits; all 5,417 frozen checks pass. This closes the captured representative's candidate agreement, not public-path or full-graph acceptance. The reviewed production module plan is ready; implementation starts with shared endpoint admission while preserving the current default. Native symbol-hook hygiene remains a focused promotion gate.

- 2026-10-05: shared native oracle helpers reproduce every semantic field of the accepted six-case/forty-row controls. The additive diagnostic reader admits four complete layouts and rejects 31 malformed cases. The first actual native-dual oracle completes exact factorization, analytic admission, monomial reversal and all seven two-precision/realness comparisons, but exits after 28.705 seconds at an incorrect export assertion that native Float results retain the requested precision. Numerica intentionally tracks result precision dynamically. The failure and unchanged inputs are retained; no complete oracle is accepted. A transport-only correction records requested and actual precision separately and uses native round-trip decimal text, with new cancellation/growth/zero controls required before a fresh actual attempt. Production generation remains unchanged.

- 2026-10-05: the disconnected native pole-extraction/automatic-differentiation controls pass six cases and forty complete signed coefficient rows at independently repeated 512/1024-bit precision, compared with native Series of the original expressions. Known Taylor normalization, relative-only tiny nonzero coefficients, complex output, negative requested maxima, nine typed rejections and a separate literal-zero path pass independent review. This route reuses native factor collection, Dualizer and MPFR, with no custom Laurent arithmetic. Its analytic Taylor coverage is explicitly distinct from a native Series remainder. The captured-expression adapter and additive diagnostic reader remain subject to source/build review and actual-input gates; production generation is unchanged.

- 2026-10-05: the independently bounded unchanged-math original-expression oracle also times out at 600.315 seconds, after its earlier 180.167-second failure. Both stop during native width-seven series with no complete coefficient vector; all immutable checks pass and both processes are reaped. Later points, cold comparisons and production adoption remain held. Source audits identify existing native factor extraction and high-precision automatic differentiation as a possible independent oracle route, subject to exact six-pole regularization, intermediate-domain admission and small native controls. No custom Laurent algebra or further unchanged-oracle deadline extension is introduced.

- 2026-10-05: the hard four-loop full-positive-orthant reference is Checked through order zero, retaining the external order-minus-three statistical observation and separate native exact-zero proof. The original three-order native estimate and all nine covariance cells remain unchanged; the comparison keeps an explicit missing-order row and separate common-order pulls below 1.629. Seven focused transport tests, formatting and focused Clippy pass. The finite reference uncertainty is 0.57%, above 1‰, with calibration still open. The on-shell representative's native program builds successfully in 43.314 seconds with all seven orders, but the first independent original-expression oracle times out at 180.167 seconds. A fresh identical-math oracle has a separate 600-second bound; production adoption and full-graph acceptance remain gated on independent complete-order comparisons.

- 2026-10-05: the combined CLI and test-only interleaved-resolver workspace passes **336 tests**, twenty-three explicit probes ignored, with formatting and all-target Clippy passing. An initial unused test-only forwarding-call lint is corrected without changing resolver mathematics; its failed log is retained. The hard full-orthant reference returns its complete four-order physical tuple within the fresh numerical bound, with final independent transport pending. A separate native generation proves exact zero through order minus three across all 2,760 charts and 699 representatives; the old numerical estimate and covariance remain unchanged. Actual on-shell evaluator/oracles, full-graph generation, convergence/calibration and performance/platform acceptance remain open.

- 2026-10-05: the restricted test-only constant-face schedule passes ten native exact controls and 96 complete weighted-vector evaluator calls (768 component checks), including precision rescue and cold native IR. It then generates the previously stalled on-shell representative in **54.875504 seconds**, peak RSS 980,816 KiB, with all 101 immutable checks passing and formal orders `[-6,-5,-4,-3,-2,-1,0]` covered by native remainder one. Its actual evaluator and independent original-expression oracles remain pending. Original resolution stays the default; this is representative-level capability evidence, not full-graph, convergence or performance acceptance.

- 2026-10-05: a passive native trace attributes the actual named-coefficient timeout to resolving large mixed derivatives before constant-face substitution. Composition reaches the required native remainder by 19.567 seconds, but the separate 180.112-second bounded process produces no complete vector; all 103 frozen hashes pass. A restricted test-only resolver using earlier native constant substitutions is undergoing exact, evaluator and actual-input checks, with the original strategy unchanged. The hard four-loop external reference completes generation and compilation, then times out numerically without a full tuple at 750.953 seconds. A separately bounded same-science numerical retry is in progress. These diagnostics leave the on-shell capability and hard-reference gates open and do not change the frozen performance tables.

- 2026-10-05: the CLI adopts caller-owned geometry jobs through explicit `--geometry-workers` on `generate`/`run`, preserving serial defaults and independent integration settings. Its bounded Rayon adapter polls typed status on the coordinator, joins cancelled work and handles worker/coordinator unwinding. All **49 CLI tests** pass, three probes ignored; formatting and CLI all-target Clippy pass. Actual PTYs verify resize/cancellation during native geometry, no cancelled artifact, normal colored/monochrome completion and terminal restoration; one failed background-timeout driver is retained. The latest full workspace gate remains 330 tests. Separately, the first compact named-coefficient actual-target generation times out after 180.116 seconds with no vector, program or oracle comparison; all 89 hashes pass. This closes CLI adoption, not the on-shell capability, hard-reference, convergence/calibration, matched-performance or platform gates.

- 2026-10-05: the main generation context now exposes caller-owned chart/cone dispatch through the exact geometry cache, while retaining native per-integral domain and symbolic work. Independent review, 41 sector and eight context tests pass. Small named-coefficient program controls also pass 96 complete weighted-vector calls and 768 component checks, including native precision rescue and cold exact-IR reload; this remains test-only pending the actual on-shell input. The off-shell rank-two independent reference and native transport are checked through order zero, with both original/projected covariance matrices retained and maximum comparison pulls below 1.727; its 1.67% finite-part uncertainty does not meet 1‰. The combined workspace passes **330 tests**, twenty-two explicit probes ignored, with formatting and all-target Clippy passing. CLI dispatch adoption, actual on-shell generation, the hard-orthant reference, difficult convergence/calibration, and matched performance/platform acceptance remain open.

- 2026-10-05: caller-owned native chart/cone dispatch is implemented without a pool or new geometry algorithm. Its 38-test sector gate, independent ten-test rerun and source audit pass. The combined workspace passes **323 tests**, with twenty explicit probes ignored; formatting and all-target Clippy pass, with all eight affected controls rerun after a type-alias-only lint correction. Native map order, exact moments, domain/resource limits, error precedence and cancellation are retained. The named regular-coefficient test-only prototype passes eight small complete-vector controls, avoiding physical-body duplication during native endpoint assembly; its numerical-program/cold-reload and actual-target gates remain open. Existing IBP preparation separately timed out after its exact Taylor identity control, with no IBP evaluator. Main-context dispatch adoption, difficult references/calibration and matched performance remain in progress.

- 2026-10-05: the additive `KernelSet::sector_content_id` API binds immutable native IR, layout, numerical policies and selected retained semantics while preserving parent artifacts, checkpoints and replay meanings. It streams existing canonical transport into BLAKE3 without restoring coefficient expressions. Four focused tests, independent ecosystem review, affected formatting and library Clippy pass. This closes the standalone sector-identity API gate, without claiming algebraic equivalence or a machine-code cache. The last full workspace gate remains 305 tests; parallel geometry implementation and the large-example scientific/performance gates remain in progress.

- 2026-10-05: the off-shell scalar triple box now has a checked independent complete `[-3,-2,-1,0]` reference. The ordinary provider sums all 1182 sectors before estimating each coefficient and applies the native Gamma prefactor once. Exact transport and separate native original/projected full-vector comparisons pass, with maximum pulls 2.816 and 2.023 respectively; their full covariance is retained and the observations are not pooled. Five focused reference tests pass, two explicit recorders ignored. The two earlier bounded timeout attempts remain preserved. The external finite-part uncertainty is about 2.5%, so calibration, one-per-mille convergence and performance are still open; rank-two and hard-orthant independent references remain pending.

- 2026-10-05: caller-owned `GenerationContext` now adopts the exact geometry cache through an additive event API, rechecking every input's domain and rebuilding its expressions/metadata. Existing cache-free generation remains available. Independent source and analytic full-vector controls pass; two initial test-only exact-offset placement mistakes were corrected with the failed log retained. The combined workspace passes **305 tests**, eighteen explicit probes ignored, with formatting and all-target Clippy clean. The CLI now shares its color policy, honors `NO_COLOR`/plain runtime errors and shows compact public status in small windows; actual PTY resize, cancellation and terminal restoration checks pass. The original on-shell Taylor generation still times out at the original 1800-second limit with no artifact, after 80 completed representatives; a source-bound capture of the next input completes. Existing native IBP is the next bounded capability comparison, not an accepted optimization. Geometry parallel dispatch, additive sector IDs, difficult references/calibration and matched performance/platform gates remain open.

- 2026-10-05: caller-owned exact geometry reuse is implemented in the sector crate and independently reviewed. `GeometryCache` retains bounded immutable completed decompositions, keys native ordered supports/domain/resource limits, reports reuse explicitly, and preserves cancellation and failed-request semantics. All 28 sector tests pass (two explicit probes ignored), including six new cache controls independently rerun; sector all-target Clippy and formatting pass. This slice introduces no new algebra, dependency or executor. Main-library generation adoption, parallel chart/cone dispatch and additive per-sector content IDs remain separate deliverables; no cached-generation speedup is claimed.

- 2026-10-05: compact native Laurent aliases and exact-evaluator artifacts are implemented and independently audited. The complete workspace passes **294 tests**, with eighteen explicit diagnostics ignored; final affected two- and six-test targets, formatting, all-target Clippy and the production dependency audit pass. `AliasedAtom` remains the public compact symbolic view; compilation, conditioning and MPFR share one native exact program, while restored expressions are produced only on explicit request. Version-three artifacts retain native IR and semantic metadata; legacy bytes/identities remain supported. A fifth minimal Symbolica patch validates native decoded instructions before use, with native and preserved-byte regressions. The actual difficult representative passes exact pre-series template/image identity, all eighteen independent high-precision oracle comparisons and all seventy-two fresh/decoded weighted component checks. Two earlier structural-comparison failures are retained; five factored outputs differ structurally, so no all-six canonical-identity claim is made. Full on-shell generation remains the next bounded gate. The independent remaining-gates audit also keeps geometry caching, caller-owned parallel generation, sector identity wording and final interactive color/terminal evidence open; general affine upper-boundary inputs remain explicitly unsupported. None of these results changes the frozen performance baselines or completes phase one.

- 2026-10-05: the separately retained Issue 1 sector-sum reference passes native transport and independent audit. The new `issue_1_together.json` preserves all three provider values/errors and the original complete native covariance; its Checked label admits comparison under the audited ordinary-constituent uncertainty path, without claiming calibration or exactness. All three comparison pulls are below 0.857; the epsilon-two relative error remains above one per mille. The original disteval fixture remains byte-identical and Unverified. Four focused transport tests pass, with two explicit probes ignored. The ongoing native alias implementation passes 103 focused scientific tests plus artifact and legacy-CLI checks; the full workspace and difficult-representative production gates are still pending, so this reference milestone does not accept that production change.

- 2026-10-05: a separately instrumented native eight-core diagnostic now measures individual full-vector sample latency without changing native QMC/statistics. Triangle's pooled mean/observed maximum are 4.261 microseconds/3.065 milliseconds; box's are 4.358 microseconds/6.168 milliseconds. Every sector's count/mean/maximum, timer overhead, point/precision provenance and rejected-prefix control are retained. Instrumentation on/off gives exactly equal complete vectors, covariance and accepted replay states; these finite observed maxima include possible scheduling interruptions and do not replace the seven-seed package-amortized means. The independent Issue 1 ordinary-constituent sector-sum attempt also completes orders `[0,1,2]`, addressing disteval's identified omitted sector covariance using the provider's existing API. Its epsilon-two value is 760.9168 ± 0.9781, above the requested 1-per-mille relative-error target; a versioned native fixture is pending. Native alias production integration is in progress, including a small essential upstream-owned decoder-validation fix prompted by a non-evaluating malformed-input reproduction. No complete on-shell or final performance acceptance is claimed.

- 2026-10-05: the original-template/native-alias experiment passes nine small controls, all eighteen representative order/point checks with native high precision, cold exact-IR reload, and a fresh Laurent substage. That fresh substage takes 42.539 process seconds with 547,900 KiB peak memory; its six coefficients equal the cached alias inputs exactly. The separate cached evaluator builder takes 0.123 seconds, producing 1.52 MB of native exact IR without restoring giant coefficient Atoms. Ordinary precision remains unstable at every oracle point, so production precision rescue and full on-shell generation remain mandatory next gates. Issue 1's independent full positive-orthant execution and transport also pass; the fixture preserves orders `[0,1,2]` and reported errors but remains typed Unverified and comparison-ineligible because the external provider omits covariance between kernels sharing shifts. Three focused reference transport tests pass, with two explicit probes ignored. Review of other references using that provider and the existing native sector-sum integration alternative is in progress. No production strategy or sampling default is changed by this milestone.

- 2026-10-05: the separately reviewed native-only seven-seed continuation completes all fourteen eight-core rows, preserving full vectors/covariance, unchanged precision safeguards and zero evaluation failures. Every finite-part target is reached by the first complete 1024×16 allocation. Median reported integration/full process times are 0.03297/0.04307 seconds for triangle and 0.04160/0.05402 seconds for box. Pooled worker mean costs are 3.884/3.841 microseconds per sample; median slowest-sector means are 7.600/5.886 microseconds. These are not individual-sample maxima, and missing Pathfinder eight-worker execution prevents a paired speedup claim. The actual callback experiment completes preparation in 6.617 seconds but times out in native series at the unchanged 180-second bound without a complete vector. Its missing late-template-opacity step is an explicit comparison limitation; native template/alias reuse is the next reviewed proposal. The bounded independent full-orthant Issue 1 reference attempt has started separately. No production strategy or sampling default changes follow from these diagnostics.

- 2026-10-05: the eight-core smoke uses the corrected highest signed order, epsilon zero for triangle/box. Both native complete 1024×16 allocations pass the full-vector residual check and have zero evaluation failures; estimated finite-part relative errors are 4.40e-9/6.50e-7, with observed driver intervals 0.0419/0.0495 seconds and full processes 0.0532/0.0617 seconds. These are single smoke observations, excluded from seven-seed medians; an earlier crossing is unmeasured. Both frozen Pathfinder eight-worker processes abort at Symbolica's concurrent-instance license check, so the paired campaign is not accepted. Fresh generation, complete results, dependency/source hashes, precision policy, physical-core affinity and failed attempts are independently audited. No license workaround or scientific-certification promotion is used. The native polynomial-callback proof also passes four nonempty full vectors, an explicitly exercised exact-zero derivative, native numerical checks and callback-free cold IR reload. Actual-sector generation and bounded callback cache lifetime in a long-lived HEPKit process remain open. All three interrupted agents resumed their existing assignments.

- 2026-10-05: independent double-box reference milestone: the copied native pySecDec package completed after two retained bounded build failures. Its full physical vector through order zero, uncertainties and exactly-once Gamma normalization were independently audited. The existing native comparison API finds every coefficient of the retained 64-shift FastSecDec vector within 1.27 combined standard errors. The new versioned reference preserves all source identities, measured uncertainties and unknown work/covariance; the historical target and run card remain unchanged. Both reference transport tests pass through Cargo, with formatting and all-target Clippy passing; production code remains at the preceding 281-test combined gate. The independent original-expression oracle also now supplies all six Laurent coefficients at three exact points, with 512-/1024-bit agreement and an exact absolute-/relative-series control. The user clarified that the requested 1‰ target is the largest signed requested epsilon power, usually the finite coefficient. An eight-physical-core baseline protocol for the validated small examples is approved under that criterion; no measured crossing is claimed yet. Full on-shell generation, difficult-case calibration, remaining scientific references and final performance/platform gates remain open.

- 2026-10-04: literal-series dependency milestone: **281 workspace tests passed**, seventeen explicit probes ignored; formatting and all-target Clippy pass. A native reproduction proves that generic function-series substitution treated a valid underscore-suffixed regulator as a wildcard and could erase the full Gamma series. The minimal fourth Symbolica patch makes the two substitutions literal; native Gamma/composed-function regressions and a FastSecDec full-vector Gamma/endpoint regression pass. The first two formal-function probe attempts are rejected as empty-vector evidence. The corrected six-case proof passes full-vector native identities, O2, weighted MPFR and fresh-process native-IR checks, with independent review; actual difficult-representative validation remains open. Existing performance binaries retain their original dependency identities. At the user's request an agent delivered the standalone Rust-script MRE, README, patch and observed outcomes in `mre/symbolica-literal-series-variable/`: published 3.0.1 fails and the unchanged script linked to the corrected checkout passes. The patch applies cleanly to the published source. This milestone does not complete the phase-one goal.

- 2026-10-04: native-family preparation and convergence milestone: **280 workspace tests passed**, seventeen explicit probes ignored; formatting and all-target Clippy pass. The production dependency audit retains one Symbolica 3.0.1/SymJIT 2.26.4/Numerica/Linnet ownership chain and excludes Python, pySecDec, CLI rendering and reference providers. The new conservative family-preparation API reuses native partial fractions/sector projection, preserves original labels and weights, and has independent review; automatic CLI adoption remains deferred. All 72 fixed massive holdout rows complete 223,838,208 evaluations without failures, with independently audited full coverage and native joint covariance. The three-pair CLI cadence study preserves identical numerical state and reduces median integration time by 20.0%. All 28 paired triangle/box command rows complete at matching sample counts, with median native/reference times 1.012/2.884 s and 1.232/4.404 s; differing backend/precision/persistence boundaries keep strict parity open. The test-only Series-first experiment passes its controls but is slower and more memory-intensive on the difficult on-shell representative; no production switch follows. Shared native FunctionMap bodies are the next bounded proof. The first independent double-box reference attempt times out during FORM/C++ package preparation at 600 seconds with no numerical result, retaining its complete failure evidence. Eight-core one-per-mille timing and individual-sample maxima are explicitly unmeasured in the new status report. Full capability, difficult scientific references and final performance/platform acceptance remain open.

- 2026-10-04: the user specified the post-coverage optimization priorities: generation, lattice/transform convergence, and lower average and maximum per-sample evaluation cost for each sector. The performance section now sequences that campaign after full capability/scientific coverage, retaining work on generation bottlenecks that currently prevent required examples from completing. Native structured profiling, full-vector accuracy, rescue-tail costs and independent ecosystem audits remain mandatory.

- 2026-10-04: native Laurent attribution milestone: three focused controls, fail-closed native capture, bounded absolute/relative replays and exact comparison of every template/restored coefficient pass; formatting and all-target Clippy pass. The captured on-shell representative has a 5.7 MB template and a 256 MB restored finite coefficient. Native relative depth one followed by the measured required width reduces its series work from 148.974 to 37.332 seconds with identical outputs; this is one-representative attribution, not full-generation acceptance. Independent review resolved a cold-import Gamma warning as a lazy-initialization false positive through native normalization, derivative, series and canonical-identity controls. Production parsing is separately covered, and no dependency patch is needed. The diagnostic is entirely `cfg(test)` and cannot return a partial generated integral. Production series behavior remains unchanged pending whole-input evidence and a compact regular-coefficient design informed by Pathfinder's direct path. Projected on-shell generation, current-artifact holdout preparation and matched benchmark execution remain queued or in progress.

- 2026-10-04: bounded orthant and projected-family evidence milestone: issue-1 completes all 2,686,976 evaluations and orders `[0,1,2]`; the hard four-loop density completes generation/O2 compilation in 36.755 process seconds and all 5,726,208 evaluations of 699 kernels in 124.493 process seconds, preserving orders `[-2,-1,0]`. Both retain complete covariance, zero evaluation failures, work-limit stopping and unverified results. The hard case's 2760 physical F-support charts differ from the earlier combined-U/F geometry stress workload; no speedup is inferred from their counts or timings. Independent saved-result/coverage audit passes. Native partial fractions and sector projection also complete both off-shell triple-box vectors with eight active propagators and seven integration dimensions; exact native denominator equality supports equivalence, while first numerical errors remain preliminary and potentially correlated with original trials. No new production algebra or automatic graph rewrite is introduced. The bounded on-shell failure is retained, with native Laurent capture/replay investigating it. Independent difficult references, prespecified convergence, matched performance and platform evidence remain open. See the hard-four-loop, on-shell/issue-1 and projected-family reviews in `docs/reviews/`.

- 2026-10-04: caller-status cadence milestone: **33 focused CLI tests passed** against the 264-test workspace baseline, followed by formatting and all-target Clippy; independent review passes. A numeric-only profile of the complete triple-box checkpoints measured native snapshots at about 2.2 ms, versus about 0.2 ms for JSON serialization; each original diagnostic had emitted roughly 806 MB of status. The CLI now checks its presentation deadline before requesting a snapshot, with JSON default 100 ms, existing terminal/plain cadences, and `--status-interval-ms 0` for every batch. Initial/stage/round/final/cancel/failure events are forced; package errors, accepted replay, cancellation and checkpoint checks remain per batch. Statistical-range errors discovered only by reduction can surface at the next observation or forced boundary, and failure-save tests cover that explicit latency change. Native estimators and checkpoint formats are unchanged. The first on-shell triple-box generation timed out after 300 seconds plus cancellation grace during Laurent extraction; it produced no artifact or usable partial integral. Native template replay and reduced-family diagnostics now target that bottleneck. Orthant completion, convergence and matched performance remain open; no end-to-end cadence speedup is claimed before the paired run.
- 2026-10-04: qualified-selection and native-family milestone: **264 workspace tests passed**, twelve explicit probes ignored; a final legacy-inspection test-only byte-slice adjustment passed its focused regression, followed by formatting and all-target Clippy. Native manifest projection preserves original kernel IDs, exact-offset policy, covariance and checkpoint scope, with selected-only worker contexts and explicit scoped completion. Artifact inspection exposes the retained owner's domain/chart/map/measure records and legacy absence. Independent HEPKit reviews pass. The new borrowed `IntegralFamily` entry delegates to the existing Gaussian implementation, validates positive powers and native label collisions, and rejects stale dimension-dependent physical coefficients; exact raised-power integration and weight-once proofs pass. Both original off-shell triple-box scalar and rank-two baselines complete all 9,682,944 evaluations and four Laurent orders without failures, but remain unconverged and independently uncertified. Native partial fractions and sector projection exactly identify eight active propagators, preserving two squared lines; the default graph path is unchanged pending a measured probe. On-shell/orthant examples, difficult-case calibration, projected-family/reporting performance and matched acceptance remain open. Production dependencies still retain one Symbolica 3.0.1/SymJIT 2.26.4 ownership chain without Python, pySecDec, CLI presentation or reference providers.
- 2026-10-04: the user corrected the requested reviewer to `benruijl`. Retrying that exact username still received GitHub's permissions denial. PR #8's description now contains the verified exact `@benruijl` mention, with the earlier casing removed; its formal review-request list remains empty.
- 2026-10-04: the user authorized publishing the completed Numerica lattice-QMC feature as a PR targeting its main branch, with `benruijl` requested as reviewer (corrected handle). Upstream readiness, branch ancestry and the complete native test suite are checked before publication; this does not authorize publication of other reference repositories.
- 2026-10-04: opened [Numerica PR #8](https://github.com/symbolica-dev/numerica/pull/8) against upstream `main` from the authenticated user's fork, at `e4638da22a17cfa931fa14c6829d3350b7a8de2b`. Full default, serde and alternative-backend suites passed 237, 240 and 217 tests respectively, plus the serial/threaded example, formatting and package-content checks. No existing MC algorithm changes or new runtime dependencies are included. GitHub denied formal reviewer assignment for insufficient permissions; the verified PR description now explicitly tags the corrected `@benruijl` handle. The CLA check was pending at publication; the feature has not been merged. See `docs/reviews/numerica-qmc-upstream-readiness.md`.
- 2026-10-04: the user supplied `https://github.com/alphal00p/fastSecDec` and authorized the initial push and periodic pushes at validated milestones. This supersedes the initial no-push restriction for FastSecDec only; excluded reference repositories and raw outputs remain local.
- 2026-10-04: implementation authorized; goal activated; plan saved. FastSecDec remains on `main`. Isolated pinned dependency worktrees prepared. Three agents assigned native input/dependency integration, exact sector geometry, and Numerica QMC. Acceptance gates remain pending.
- 2026-10-04: foundation milestone prepared: three-crate workspace and local Nix shell; native HEPKit/Linnet input and scalar U/F normalization; executable CAS/JIT reuse probes; exact normal-fan sector geometry. Native library tests passed (14); geometry author and independent tests passed (11 + 8). The hard nine-dimensional support probe produced 3,496 sectors in approximately 125 seconds in its initial mixed-optimization build. This is feasibility evidence, not performance acceptance. Full numerator conversion, symmetry reduction, subtraction, runtime and CLI remain in progress. See `docs/reviews/sector-geometry-initial.md` and `docs/REUSE_AUDIT.md`.
- 2026-10-04: Numerica QMC implementation committed independently as `e26d3dd3ee0683c5acd9706eb95fb6b66f24147b`, followed by peer-review fixes in `e9b7481d66b8f9d0c5e58ccabc4d6644e7fd479a` on `codex/havana-qmc`. The author reports 174 library, 14 API regression, 23 QMC and 22 documentation tests passing with serde. Independent review identified and drove corrections to large-offset covariance and maximum-modulus periodic shifts; the reviewer executed both reproductions against the fix and verified their expected results. No pushes or changes to existing MC interfaces.
- 2026-10-04: native fixtures now cover all 15 historical DOT examples, plus a bubble, with 19 graph TOML cards and three direct-polynomial cards. The two example-input tests pass, including exact numerator and propagator-multiset checks for all five numerator graphs. This validates native topology and routing, not numerical integration of every card. Historical targets retain provenance; the double-box file's manually supplied target has no usable reference uncertainty. The triangle-numerator card resolves the historical unused-leg inconsistency while preserving its actual explicit-propagator integral. See `examples/README.md`.
- 2026-10-04: direct-generation and standalone-runtime milestone: native Gaussian polynomial numerators, factored endpoint subtraction and full Laurent vectors, portable SymJIT O2 kernels with native MPFR rescue, caller-driven QMC/MC sessions, checkpoint validation, and CLI generation/integration/inspection/boundary diagnostics with typed status and a live terminal dashboard. The complete workspace gate passed 109 tests, with two expensive probes explicitly ignored. Native two-worker bubble, triangle and analytic endpoint runs, fresh-process complex/Gamma artifact reloads, and terminal cancellation/checkpoint restoration were exercised. Three small Symbolica fixes are recorded as upstream-ready patches; Numerica is pinned at `617f7a56f8f168cd7498177c7db4a40b098eb135`. There are now 23 run cards. Boundary assertions cannot override unresolved singular faces. Factored Laurent templates reduced the diagnostic double-box generation time from 100.309 to 26.211 seconds; these development-build observations do not establish performance parity. Complete-density symmetry, further cancellation/IBP and geometry optimization, all-example numerical certification, and matched performance acceptance remain pending.
- 2026-10-04: subsequent workspace gate passed 133 tests with four explicit performance probes ignored. It includes verified full-density Graphica symmetry, independent integral multiplicity checks, all 23 native run-card loads, native massless-box numerical validation, literal caller-symbol regressions, Taylor/IBP identities, per-axis cancellation metadata and typed stage timings. The release geometry stress case improved from 99.351 to 11.276 seconds with matching ordered-map fingerprints. Fresh factored double-box generation produced 102 representatives from 152 charts; Taylor was faster to generate and compile than IBP and remains the default. These are local diagnostic measurements, not matched performance acceptance. The recurring HEPKit audit identified a reproducible stale model-cache bug, CLI-only reusable diagnostics and remaining mapped-factor expansion; their fixes and native one-loop master cross-checks are in progress before accepting the next milestone. See `docs/reviews/hepkit-integration-audit-2026-10-04.md`.
- 2026-10-04: HEPKit reuse/diagnostics milestone validated: **147 workspace tests passed**, four performance probes intentionally ignored; formatting and all-target Clippy pass. The audit's model-cache, reusable-diagnostic and mapped-expansion findings are fixed. Three native master tests cover eleven one-loop points with exact normalization and scales. Boundary diagnostics now expose bounded typed face coverage, streamed results and cancellation; benchmarks preserve partial repetitions. Regular degree-10,000 factors stay compact, and a three-axis Taylor/IBP integral identity passes without forced expansion. Production dependencies still exclude Python, pySecDec and CLI presentation crates; the native master provider is development-only. Future wrapper accessors are documented for the bridge phase. Weighted-sample replay, complete multiloop/numerator/orthant numerical certification, and matched performance gates remain open.
- 2026-10-04: weighted-evaluation and retained-metadata milestone: **176 workspace tests passed**, six explicit scientific/performance probes ignored by the ordinary gate; formatting and all-target Clippy pass. Whole-vector MPFR replay now applies the sampling weight before binary64 conversion; caller-owned QMC/MC workers accept already-weighted vectors. Checkpoint version three preserves accepted replay state across worker-count changes. Portable kernel version two retains native coordinate maps, exact geometry, factor-level domain certificates, and the phase-one branch policy, with semantic load validation and explicit legacy handling. Independent HEPKit reviews drove zero-dimensional and symbol-collision fixes and removed quadratic chart association. Five native reducer tests cover eight numerator points, including rank five and zero Gram determinant, alongside the eleven scalar-master points. Numerica is pinned at `55072895f98be8830bcf6400e32546bc9470de7a`, with independently reviewed periodization range checks. A separately executed native rational-integration proof verifies the generated double-box leading pole integrates to zero; the sampled value is still reported unchanged. The 64-shift double-box convergence study, all-example numerical gates, downstream generation optimization, and matched performance acceptance remain open.
- 2026-10-04: reference-reporting and source-provenance milestone validated after the 176-test workspace baseline: nine library reference tests and 22 CLI tests passed, with final focused repeats after review fixes; formatting and all-target Clippy pass. Native callers can compare typed sparse Laurent/component results directly, with explicit exact/reported/unknown uncertainty and recorded normalization, kinematic and independence evidence. CLI targets remain outside numerical settings and artifact identity; changing a target clears inherited evidence, and native TOML fingerprints preserve non-reference value types. Independent reviews found and closed a missing untracked-dependency-source hash and absent-file Cargo watches. The double-box diagnostic completed 6,684,672 samples and 64 shifts without failures: its leading pole is −0.000181020268 ± 0.000588690140, consistent with the separate exact-zero proof. Higher coefficients and matched performance remain uncertified. The validated history has been published to the user-supplied FastSecDec remote; the next slices address factored monomial extraction, all-example multiloop checks and remaining regression gaps.
- 2026-10-04: latest-backend and factored-mapping milestone: **202 workspace tests passed**, eight explicit scientific/performance probes ignored; formatting and all-target Clippy pass. The live registry identifies Symbolica 3.0.1 and SymJIT 2.26.4; the existing Symbolica revision contains that release, and SymJIT is upgraded to 2.26.4. A two-line development-only OneLOop patch updates its exact backend pin and cache identity, with native cache rebuild/reload and old-header rejection passing. Native monomial collection preserves compact residuals, retaining exact support minima and the signed sparse fallback; independent review found no correctness or reuse defect. Four new tests close graph-entry timelike rejection, signed Gamma depth, negative highest Laurent order and complete native rank-two MPFR replay gaps. The rank-five master comparison passes with much smaller development-build symmetry/Laurent costs; controlled release and reference timings remain pending. The native FunctionMap audit and disconnected probe prepare the requested alias/inlining comparison; no production representation is selected yet. All-example multiloop, boundary-growth, saved-result reporting and matched performance gates remain open.
- 2026-10-04: boundary/contribution and evaluator-measurement milestone: **220 workspace tests passed**, ten explicit probes ignored; a subsequent malformed imported attempt-index display regression also passed. Formatting and all-target Clippy pass. Componentwise boundary growth, bounded retries, typed status and clean CLI tables preserve all attempts, selected-sector coverage and cancellation; independent HEPKit reviews found and resolved two public-formatting robustness issues. Native QMC/Havana contribution reports reuse existing complete-replica statistics, keep exact offsets separate and label shared-sector marginal errors without changing the authoritative total. All six smaller massive multiloop cards generated and integrated successfully; initial independent external references are recorded for all six. A six-line higher-work repeat agrees within one reported combined error but shows slow uncertainty reduction, now under investigation. The controlled same-release rank-five generation comparison improves from 95.328 to 12.066 seconds through factored mapping. Native FunctionMap compatibility passes 32 cases; 119 longer actual-coefficient cases confirm compact alias preparation, essentially unchanged batch throughput and slower retained small functions. Production evaluator representation remains unchanged. The next work includes QMC rule/convergence analysis, remaining scientific examples, native saved-result reporting, lazy support profiling and matched performance acceptance.
- 2026-10-04: six independent multiloop references are frozen in the native versioned format with source identities, explicit real projection and reported uncertainties. Three focused data/comparison checks passed, including the two explicitly invoked ignored record/replay checks; the six-case CLI harness compiles and targeted Clippy passes. A constant-integrand control reproduces the five-dimensional Kuo33002/Korobov3 error plateau. At equal work, alternative published vectors substantially improve the actual six-line result, while broader dimensional controls rule out selecting a universal default from that case alone. Production rule defaults remain unchanged pending further physical comparisons. Separate rank-five attribution finds repeated native support extraction accounts for 99.63% of mapping time; a lazy reuse implementation and its independent acceptance checks are in progress. This milestone adds scientific evidence and reference transport, not a convergence or matched-performance certificate.
- 2026-10-04: native support reuse and published-catalogue milestone: **231 workspace tests passed**, twelve explicit probes ignored; formatting and all-target Clippy pass. The production dependency audit retains one Symbolica 3.0.1/SymJIT 2.26.4 owner and excludes Python, pySecDec, CLI presentation and development-only reference providers. The generation-owned lazy support cache preserves compact factors and complete vectors: three alternating release pairs improve rank-five generation from 11.371 to 2.522 seconds, and an untimed independent pair has identical complete-vector fingerprints. Numerica is committed locally at `5d768eea73c525bb28affea31c298db9358b2f5f` with attributed HKKN/Kuo catalogue choices, native bounds and provenance validation; its 29 QMC and nine MC tests pass. Forty-eight physical comparisons retain native joint covariance and show case-dependent rule quality. The default remains unchanged. The CLI exposes explicit choices, reports the effective native design, and rejects mismatched native/outer checkpoint settings before work. Independent HEPKit audits pass, and the exact massive-box run-card point now agrees with native D0. Saved-result APIs, coupled numerator and remaining triple-box/orthant campaigns, difficult-case convergence and matched performance remain open.
- 2026-10-04: saved-result and coupled-numerator milestone: **253 workspace tests passed**, twelve explicit probes ignored; the final enum-layout adjustment passed eleven native result and five CLI result/reference tests, followed by formatting and all-target Clippy. Saved results preserve full covariance, scope, exact offsets, original references and effective QMC design without symbolic loading. Numerical failures retain accepted coverage and produce nonzero CLI exits; independent reviews closed coverage, metadata-allocation, numeric-range and rendering-cost issues. The minimal Numerica coverage accessor is committed at `e4638da22a17cfa931fa14c6829d3350b7a8de2b`, with 30 QMC and nine existing MC tests passing. The native coupled-sunset fixture passes exact routing/export-reload, nine independent density points, a convergent scalar sign control and complete Laurent vectors through epsilon one at two spacelike scales. All 24 run cards and the 17-DOT inventory pass CLI checks. Production dependencies retain Symbolica 3.0.1/SymJIT 2.26.4 and exclude Python, pySecDec and CLI/reference-provider dependencies. Qualified CLI sector selection, remaining triple-box/orthant examples, difficult-case convergence and matched performance remain open; Numerica upstream PR readiness is being checked under the new publication authorization.


### Additional notebook requirements (2026-10-06, verbatim)

> Make sure that the generation has a progress report similar to `generate_diagrams`

> Add a get_citations at the end of the notebook. Make sure that when secdec is used the relevant papers are cited. Also add a citation for fastsecdec

> The pysecdec paper [https://arxiv.org/abs/1703.09692](https://arxiv.org/abs/1703.09692) should also be cited

Reuse the native HEPKit marimo progress presenter for generation and compilation,
with `progress="auto"`, explicit silence and full native-event callbacks. Keep
tracking and citation metadata in FastSecDec, expose native Symbolica `Citation`
objects through the existing HEPKit collector, and add reactive final
`get_citations()` cells with BibTeX downloads to both notebooks. Cite the
FastSecDec software repository, Kaneko–Ueda geometric sector construction,
Binoth–Heinrich endpoint subtraction, and pySecDec as the reference implementation
that informed this project. Do not invent a FastSecDec publication DOI.
Record actual use, including valid cached kernels, rather than import alone.
These notebook/API additions do not close the outstanding phase-A science and
performance gates.


- 2026-10-06: the standalone gg→HH notebook now starts from the native Standard
  Model with inline scalar bindings, generates the physical diagram, contracts
  native external states and calls `diagram.sector_decompose`. Automatic progress
  shares HEPKit's existing presenter. Both notebooks end with the native
  `get_citations()` collector and BibTeX export, including FastSecDec, pySecDec,
  Kaneko–Ueda and Binoth–Heinrich. The updated wheel passes 118 maintained
  controls, strict binding Clippy and generated-stub validation. Native generation
  produces 30 charts; a saved-kernel continuation completes the small QMC and
  Havana runs with full covariance and no evaluation failures. A diagnostic
  covariance-writer error and its corrected continuation are retained separately.
  Actual marimo generation/compilation, sector inspection, reactive citations
  and BibTeX download are observed. The new notebook's editor interruption/resume
  remains unverified after diagnostic selector failures; the existing dashboard's
  prior lifecycle evidence is separate. The fresh Wasm wheel passes generic
  smoke and all 89 maintained portable controls across nine modules, with zero
  failures or skips. The initial generic rendering-fixture failure is retained;
  its test-only adaptation to the existing native render API passes without a
  wheel rebuild. These controls do not claim a new full gg→HH browser run.
  The tested shared Model/progress owner is published at FeynKit `7f3d6c79`;
  community retains only registration, citation aggregation and generated stubs.
  These API/notebook gates do not close the remaining phase-A performance gates.

### Regular HEPKit integration and scalar representation (2026-10-06)

User clarification, verbatim:

> Once WASM works, there is no need for `#[cfg(feature = "experimental-fastsecdec")]`, it can be a regular feature. Why do the scalar bindings have to be made exact?

Include FastSecDec in ordinary native and Wasm HEPKit/community builds, removing
the dedicated experimental feature and opt-in build switches. Keep core-only
Symbolica builds separate and community limited to linking, registration and
stubs. Validate the default feature graphs and build workflows against the
already tested native/portable implementation; do not repeat scientific runs
for a feature-wiring change.

Distinguish exact representation from physical precision. The current native
family's rational-polynomial preparation and rational/algebraic domain checks
motivate lossless binary64-to-rational transport. This adds no precision,
recovers no intended decimal value, and cannot repair relations between
independently rounded kinematics. It is not a universal requirement for all
HEPKit scalars or numerator coefficients. Retain Symbolica expression overrides
and analytic dependency resolution, without introducing another algebra system
or silently changing the numerical point.

Further user clarification, verbatim:

> What is scripts/prepare_fastsecdec_dependencies.sh? There should be no dependencies

Remove the special dependency-preparation requirement from regular HEPKit
delivery. Keep the previously required ecosystem crates as ordinary Cargo
dependencies; do not replace their functionality. Resolve the outstanding
upstream compatibility fixes before deleting the bootstrap or publishing the
default-feature promotion as a working normal build. The current
[dependency audit](docs/reviews/regular-hepkit-build.md) records the actual
remaining fixes and the distinction between patched-wheel validation and
unpatched public-source readiness.

Further implementation instructions, verbatim:

> Can you move teh QMC logic to fastsecdec itself for now? What other Symbolica changes are needed? You are free to update HEPkit and oneloop, you can find the source in /common/dev/

> Why are they pinning symjit versions? They should only define a minimum version

These instructions supersede the earlier Numerica ownership requirement: move
the existing tested QMC library into a dedicated FastSecDec crate, preserving
worker-local point generation, caller-owned execution, estimators, checkpoints,
catalogue attribution and ordinary Havana Monte Carlo behavior. Keep the earlier
Numerica branch and PR available, without relying on its unpublished QMC APIs
for normal builds. Audit remaining uses of its other local fixes separately.

Update the authorized HEPKit and OneLOop sources directly as needed for normal
Cargo integration. Declare minimum compatible SymJIT versions rather than exact
dependency pins; retain reproducibility in lockfiles and record the actual
resolved backend version in evaluator cache identities. Provide the six
remaining Symbolica fixes as a concrete upstream review bundle; the
literal-series-variable fix is already upstream. No Symbolica publication
authorization is inferred from the HEPKit/OneLOop authorization.

Further upstream review instructions, verbatim:

> Why is IR validation needed? Did you generatw bad IR?

> I am not sure if the constant arg fix is good: the resulting constant will not contain error tracking. What are you sing this for?

> I have updated symbolica/community. Remove patches that are no longer needed.

> The oneloopmaster fix is in main too now

Compare each local correction with the updated owning repository and remove
superseded patches from delivery. Preserve historical reproduction evidence
without applying its fixes again. Evaluator decoding validation addresses
deliberately malformed saved programs, not an observed faulty FastSecDec
generator. Distinguish error propagation through the evaluator from a nominal
error assigned after evaluating a fixed external constant: the latter does not
account for cancellation or rounding inside an arbitrary callback. Keep the
remaining constant-domain proposal qualified pending a suitable upstream
numerical contract.


- 2026-10-06: QMC ownership moves into the dedicated `fastsecdec-qmc` crate,
  preserving old-owner serialized plans, point bits, partial estimates and
  resumed covariance. Registry Numerica continues to supply numeric backends,
  RNG and ordinary Havana MC; a private native-grid adapter covers its released
  child-clone omission. Independent review, 37 native plus 37 portable-feature
  host QMC tests, 43 focused core controls and strict QMC Clippy pass. SymJIT
  compatible minima replace exact requirements; actual linked-version reporting
  and CLI Git/registry provenance pass 46 focused tests and scoped strict
  core/CLI Clippy. Leaf and community metadata retain single shared owners.
  Three FeynKit tests, two OneLOop owner controls and the downstream one-loop
  cache regression pass. OneLOop PR #1 is merged into `main` at `27c37234`.
  The updated Symbolica `community` revision `473b4b8` incorporates the alias,
  complex-product and literal-series corrections; those three patches and the
  superseded FeynKit/OneLOop patches are removed. All 23 affected tests pass
  against the updated source with four remaining Symbolica patches. The
  fixed-argument constant fallback's missing callback-internal uncertainty is
  explicitly documented; no general error-bound claim is made. The regular
  community feature promotion remains locally prepared pending the outstanding
  owner requirements. No new Python/Wasm wheel or performance result is claimed,
  and phase A remains incomplete.


Further upstream integration instructions, verbatim:

> Which fix is still needed to be able to use upstream symbolica? Some seem soptinal

> I added it to symbolica/community. Use that version and once no more patches are needed, commit and push to main

Use the public Symbolica revision supplying configurable fixed-variable
coefficient fields. Adapt FastSecDec to the existing evaluator mapping APIs;
keep unsupported-domain errors explicit and make error-tracking conditioning
optional, using the existing precision rescue when needed. Remove local source
patches and dependency-preparation scripts, resolve public Cargo dependencies
with a single owner for shared native types, and validate native and portable
behavior before committing and pushing FastSecDec main. Keep native artifacts
as trusted application caches without claiming structural validation of arbitrary
instruction streams. The regular HEPKit feature promotion and its dependency
selection must use this same public-source implementation.

- 2026-10-06: Ordinary upstream dependency delivery passes with Symbolica
  `58652fabc2f736302a570deaaf8d517679f7fe6e`, tested FeynKit `259df879`, merged
  OneLOop `27c37234`, and registry Numerica/Graphica. All remaining local
  dependency patches and preparation scripts are removed. Public native
  callback metadata supplies typed mapping admission; optional conditioning
  preserves the existing precision-rescue policy without a generic constant
  fallback. Native workspace tests pass 475 controls (25 existing diagnostics
  ignored), portable-backend host tests pass 58, strict workspace Clippy passes,
  and the Python binding passes all-target checking with stub generation enabled.
  Root/leaf source-identity and formatting checks pass. The HEPKit PR's ordinary
  feature promotion follows the published main revision. Historical actual
  native/Wasm wheels retain their original source identities; no new wheel,
  convergence or performance result is claimed. Phase A remains incomplete.

Further namespace and publication instructions, verbatim:

> Remove the compatiblity [`fastsecdec/__init__.py`](https://github.com/symbolica-dev/symbolica-community/pull/18/changes#diff-346603d84f04b9841cd398b49116481ed655aaf26f90947652247ead1ee16b5c). It is not in use. Then push it to main directly using the token.

Remove the unused Python compatibility package, its generated stub and stub
generation output, reexports and compatibility-only import controls. Use
`symbolica.community.hepkit.sector_decomposition` throughout maintained callers
and tests; retain the supported `Integral(...).generate()` entry point within
that canonical namespace. Preserve historical audit records. Validate the
namespace cleanup and integrate current upstream community main without
discarding its newer modules or dependency owners, then publish the reviewed
HEPKit changes directly to community main using the authorized account. This
supersedes the earlier requirement to retain the compatibility namespace and
the earlier PR-only publication restriction for these changes.

### Self-contained copy of the current gg → HH notebook (2026-10-07)

Latest request, verbatim:

> Create a self-contained gghh.py (no local Python imports) , name it gghh_complete.py.

Add `examples/hepkit/gghh_complete.py` and a copy in community's `examples/hep`.
Keep the current notebook's native Standard Model catalogue, parameter binding,
explicit caller-stepped generation/integration, rich inspection, pause/resume
and citations. Inline only the necessary example helpers as readable notebook
cells, with no local Python imports, source-path injection or fixture files.
Installed marimo and HEPKit/Symbolica remain library dependencies. Its browser
export requires the matching community wheel but no helper/input-data archive.
Check relocation by running the single notebook from a fresh directory; keep
scientific/runtime validation limits separate from packaging and startup checks.
