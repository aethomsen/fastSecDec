# Source-bound open physical densities

`threshold::projective::open_density::CompiledOpenDensity` composes every
original prepared term with an admitted normalized cell map. It retains the
native physical source and factor/sign authorities. Source-coordinate inputs
follow preparation order, even when CAD uses a different ordering. Individual
terms, their complete sum, mapped coordinates and the positive Jacobian remain
available for inspection.

The numerator stays in original coordinates. Symbolica's `EvaluatorComposer`
passes the mapped coordinates into its evaluator; it does not expand nonlinear
root expressions inside the numerator. Every term retains its own coordinate,
positive-factor and causal-factor powers. The map Jacobian multiplies each
complete density exactly once.

The lower-lip phase uses the existing native causal-phase arithmetic, including
the regulator-dependent exponent. Review caught a phase-authority defect in the
first private draft: compiling inserted phases under the caller's `FunctionMap`
allowed a pi alias to change the branch. The accepted implementation compiles
phases separately, with only regulator inputs and no caller aliases, then
composes them with the source bodies. Deliberate pi aliases and explicit pi
coefficient slots are regression controls. Caller mappings still apply to the
original source body.

Numeric execution reuses the existing algebraic callback scope, native
`MappingRequirements`, evaluator codec and precision domains. It rejects invalid
dimensions, nonfinite inputs and exact unit boundaries before evaluating rays.
All geometry outputs remain observable even when a numerator cancellation would
otherwise remove them. A failed callback cannot be hidden by subsequent
arithmetic. The evaluator retains its source owner and owns its scratch space;
it creates no threads, sampling streams or integration loop.

Six new native test groups pass, including fractional causal powers with
denominators three and five, different term powers, complex numerators,
128/256-bit composition, restored native IR, the genuine two-root fixture and
propagated input uncertainty. Five existing phase and seven normalized-map
groups also pass. The final source passes strict test-source Clippy and
formatting. Independent corrected-source review is `d714f0ba…`; the source
inventory is `e5ec712b…` and the handoff is `cb606f26…`. The coordinator checked
all seven postimages and 32 evidence-file digests before importing them.

A separate, inexpensive physical capability check evaluates all 156 original
prepared prefactors using native Gamma callbacks at real epsilon values 1/8 and
1/4, in `Complex<f64>` and `Complex<Float>` at 128/256 bits. It takes 4.526 seconds
and peaks at 36,904 KiB child RSS. This checks callback-domain support; it is not
a high-precision amplitude reference. It neither reloads the large proof nor
reruns GCAD.

No CAS, graph, root solver, AD implementation, codec or execution pool is added.
The HEPKit physical objects remain authoritative, and community receives no
substantive implementation. Native source/proof ownership is shared within the
process. This does not yet establish bounded residency for a future pool of
processes loading a complete cell catalogue.

The combined physical check now passes for all 156 source terms on the first
original cell and its four native infinity charts. At epsilon 1/8 and 1/4,
128/256-bit arithmetic passes 6,864 individual complex term comparisons and
44 sum comparisons against a separately built original-source evaluator. The
reference phase uses an independent exponential form. A separate directed-ball
oracle checks 154 coordinate/Jacobian outputs. Independent outcome review
`c4cfd1b6…` replays every saved term, sum and directed inequality exactly in QQ;
all term references are nonzero. Codec and composed-chart assertions also pass
inside the original driver. They are distinct from the saved-center replay.

Preparation of the open map, four-chart transport and complete density takes
0.052, 0.247 and 0.607 seconds respectively. Exact density IR occupies 58,933
bytes. The process takes 254.308 seconds including the one large proof load,
fresh complete verification and teardown, peaking at 11.522 GB aggregate RSS
under a 100 GB allowance. The build uses captured development dependencies and
native high-precision eager arithmetic; these are correctness-stage costs, not
release SymJIT sampling performance. The 1.931-second numerical stage includes
reference construction, composition and evidence output. The consuming Cargo
workspace/all-target strict threshold-feature Clippy and formatting checks pass.

This API evaluates the open density at explicit, nonpole real regulator values.
It does not certify closed-face behavior, issue a convergence strip, perform
Laurent continuation or publish an integration kernel. The complete double-box
integration remains unfinished.
