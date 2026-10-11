# No-contour threshold decomposition

## 1. Goal, scope and repository workflow

**Goal statement**

> Implement `threshold_decomposition` in Rust/FastSecDec using verified symGCAD decompositions, a general algebraic endpoint resolver, and efficient composed coordinate maps. Preserve causal analytic continuation, symbolic endpoint subtraction, complete Laurent vectors and covariance, fixed and parametric kinematics, and bounded-memory execution. Deliver validated native CLI and HEPKit interfaces, scientific comparisons through the physical ggHH double box, and reproducible generation, memory, sampling and convergence measurements.

Use the advanced GCAD construction from the [March 2026 paper](https://arxiv.org/abs/2603.05444), together with the constructive examples in the [2025 full paper](https://arxiv.org/abs/2506.24073). The supplied 2024 PDF remains background. The newer publication explicitly identifies general algebraic endpoint treatment as additional work; implementing that general capability is included in this phase.

The scope covers real masses and real kinematics, including complex numerators. Native generation, integration and HEPKit use are required. Preserve existing WASM functionality; threshold-decomposition support in WASM is deferred.

**Upon starting implementation:**

1. Consolidate the validated contour work into `main`, run the relevant regression gates, and push. The current branches permit a fast-forward; recheck their state before acting.
2. Fetch and rebase the latest `no_deformation` branch onto that published main, preserving the other agent’s exporter and interoperability changes.
3. Publish the rebase with an explicit lease against the observed remote commit. If another agent advances the branch, incorporate its changes and repeat validation. Never reset or modify the other agent’s checkout.
4. On `no_deformation`, write this approved plan **verbatim** into `NO_DEFORMATION_PLAN.md`, reference it from `FIRST_PHASE_PLAN.md`, and assign the goal above **before changing implementation code**.
5. Commit and push validated milestones on `no_deformation`, using the authorized ValentinHirschi identity.

Coordinate dependency revisions and branch changes through recorded commit IDs and handoffs. Freeze dependency revisions for each benchmark campaign.

Delegate separate slices for mathematical foundations, symGCAD integration, algebraic resolution, evaluator composition, artifacts/interfaces, and benchmarks. Assign independent scientific, ecosystem-reuse, memory and statistics audits. The root agent primarily coordinates and reviews.

## 2. Generation pipeline and general algebraic resolution

### Verified geometry and cell construction

The default full-integral pipeline is:

**HEPKit input → projective parametrization → verified GCAD → cell maps → endpoint resolution → symbolic IBP/Taylor subtraction → Laurent expansion → evaluator optimization.**

- Reuse native graph, family, Symanzik, numerator and kinematic primitives.
- Prefer compact simplex/projective representations before full sector decomposition. Compare admissible gauges and variable orders using polynomial degree, projected complexity, cell count and boundary complexity.
- Solve each canonical polynomial/domain problem once, sharing its result across numerator terms, Laurent orders and descendant sectors.
- Call symGCAD directly as a Rust library. Require both `CompleteGeneric` and successful independent verification.
- Preserve raw cells, root selectors, exceptional polynomials and proof provenance. An incomplete decomposition cannot produce a complete integration artifact.
- Preserve existing `source_sectors` semantics by using the ordinary, unsubtracted source charts first when a subset is requested. Record the resulting strategy explicitly.

For a triangular cell
\[
a_i(x_{<i})<x_i<b_i(x_{<i}),
\]
use
\[
x_i=a_i(x_{<i})+
\bigl[b_i(x_{<i})-a_i(x_{<i})\bigr]t_i.
\]
Its Jacobian is the product of positive interval widths. Include any projective, compactification and subsequent sector Jacobians exactly once.

Transform the entire density, including numerators and prefactors. For every negative causal factor,
\[
(F-i0)^{q(\epsilon)}
=
|F|^{q(\epsilon)}e^{-i\pi q(\epsilon)}.
\]
Retain the full regulator-dependent phase through Laurent extraction. Complex numerators remain complex; sign-definite denominators do not imply positive amplitude integrands.

### General resolver

Implement a general algorithm for algebraic cell pullbacks, with rational and simple algebraic cases as optimized paths.

The mathematical foundation is **Bierstone–Milman marked-ideal resolution**, followed by **Abhyankar–Jung ramification** after establishing its normal-crossing hypotheses. These provide constructive resolution and fractional-power branch normalization, respectively. [Resolution algorithm](https://ems.press/journals/prims/articles/2154), [Abhyankar–Jung construction](https://math.univ-cotedazur.fr/u/parus/publis/A-J.pdf).

Before production implementation, complete an independently reviewed algorithm specification covering:

- Algebraic towers, square-free branch polynomials, retained multiplicities, leading-coefficient zeros and poles introduced by monicization.
- The divisor formed from domain boundaries, discriminants, denominators, root gaps, Jacobian factors, and relevant \(F/U\) zeros, using native elimination or norm operations where needed.
- Admissible blow-up centres, exceptional-divisor bookkeeping and the published termination invariant.
- Finite power substitutions, real-branch selection and normalization throughout algebraic towers.
- Exact disjoint chart coverage or equivalent multiplicity accounting, orientation and boundary treatment.
- A separately checked endpoint certificate for every accepted chart.

The output contract is
\[
\rho(t,\epsilon,\eta)=
\prod_i t_i^{a_i+b_i\epsilon+\sum_j c_{ij}\eta_j}
\,H(t,\epsilon,\eta),
\]
with exact rational exponents and sufficient finite boundary derivatives of \(H\) for the requested subtraction.

Inspect all relevant closed faces and their intersections. Interior sign certification alone is insufficient. Handle root collisions, changing polynomial degree, same-sign zero seams and zeros away from cube corners.

Use ordinary sector decomposition wherever it proves the required normal form efficiently. Retain the general resolver for structures that require more than monomial sector maps. Resource exhaustion must produce an explicit incomplete result with resumable evidence; it cannot become a successful integral.

### Analytic continuation and regulators

Establish the partition identity for an analytically regulated family before continuing it. Excluded measure-zero sets cannot simply be discarded from a distributional integrand.

Where epsilon alone does not regulate individual cells:

- Introduce a common auxiliary analytic family before partitioning.
- Extend the native symbolic endpoint interface to affine dependence on multiple regulators.
- Remove auxiliary regulators from the combined contributions at generic epsilon, then expand in epsilon.
- Verify auxiliary-pole cancellations with the appropriate face or integral identities; do not assume unrelated cell integrands cancel pointwise.

Preserve all physical Laurent coefficients, exact contributions and covariance. Exceptional physical kinematics for which the requested boundary value is undefined must receive a precise diagnostic.

## 3. Efficient nonlinear maps and native reuse

**Avoid full symbolic substitution of the transformed density by default.**

Maintain two connected representations:

- An exact algebraic description used for coverage, branches, valuations and endpoint certificates.
- A compact evaluator graph for coordinate maps, root sections, interval widths, smooth factors and requested derivatives.

Use Symbolica’s native function maps, symbolic differentiation, Puiseux series, algebraic coefficient domains and evaluator composition.

- Keep \(N\circ\phi\), \(U\circ\phi\) and smooth residuals as composed functions.
- Resolve and expand only the singular geometry and finite endpoint coefficients actually required.
- Use Symbolica’s chain rule for every subtraction derivative, including map and Jacobian derivatives.
- Derive moving-root derivatives implicitly on certified regular branches. At collisions and faces, use the resolved branch and its native series rather than an invalid simple-root derivative.
- Keep endpoint reduction symbolic. Do not introduce numerical-dual endpoint jets as its implementation.
- Compile the complete Laurent vector together, sharing map values, roots and derivative intermediates.
- Use the triangular Jacobian product where applicable; use native matrix facilities for other maps.
- Retain exact root functions for bulk evaluation. Truncated series supply certified local coefficients, not uncontrolled approximations to the entire integrand.

Prepare root-polynomial evaluators once. Use stable closed forms for suitable low degrees and native bracketed/refinement facilities otherwise. Root selection must follow the CAD branch certificate; a proof sample’s isolating interval is not a universal runtime bracket.

Reuse existing callback ownership, serialization and precision-rescue patterns. Keep mutable root state worker-local and make results independent of previous sample order. No evaluator reconstruction, generic all-root search or production RNG consumption should occur unnecessarily in the sampling path.

Benchmark compact function calls, selective inlining and instruction-level composition. Current Symbolica composition/inlining limitations must be checked against the adopted revision; do not assume every combination is supported.

**For every CAS operation, triple-check public APIs, source/tests and a focused executable probe.** Record the reuse evidence before adding functionality. Symbolica already supplies Puiseux arithmetic and algebraic relations; the substantial new work is resolution orchestration and certification. Missing owner-library primitives should receive narrow, tested upstream changes.

Align symGCAD and FastSecDec on one compatible Symbolica dependency identity. Use a local override during development and reproducible public dependency declarations for delivery, without installation-preparation scripts.

## 4. Interfaces, parameters, artifacts and execution

Expose:

```bash
fastsecdec generate run.toml --threshold-decomposition
fastsecdec run run.toml --threshold-decomposition
```

```toml
[generation]
threshold_decomposition = true
```

Add native `ThresholdDecompositionOptions`, mirrored through an advanced `[threshold_decomposition]` input table and thin HEPKit bindings. It contains kinematic constraints, strategy, existing symGCAD settings and resolution resource limits.

- Default: disabled.
- Strategy `auto`: GCAD first for complete inputs; source-chart-first when existing source selection requires it.
- Explicit `gcad_first` and `sector_first` remain available for controlled comparisons.
- Threshold decomposition and contour deformation are mutually exclusive within one requested generation recipe.
- Integration consumes the saved threshold recipe without rerunning GCAD or symbolic optimization.
- Replace existing “anything undeformed is contour” assumptions with explicit recipe capabilities.

For parameters:

- Respect exact values and symbolic choices in the native graph input.
- Represent supplied floating-point values faithfully when exact polynomial input is required; never silently guess simpler rationals.
- Put genuinely runtime parameters before integration variables in the CAD order.
- Persist chamber constraints and exceptional parameter conditions.
- Require resolution compatible with parameter specialization. Refine parameter strata when necessary; do not mix parameter directions into integration coordinates and silently change the family.
- Rebinding inside an admitted chamber requires no regeneration. Excluded or uncovered values require exact specialization and regeneration.

Extend universal artifacts with proof identities, parameter guards, map/root programs, endpoint certificates, phases and hierarchical lineage:

**projective patch → threshold cell → endpoint chart → residual kernel.**

Preserve existing source-selection meanings and add explicit threshold-cell selection for focused studies. Selected artifacts remain visibly partial. Introduce versioned one-to-many lineage rather than weakening existing artifact invariants.

Reuse disk-backed preparation, atomic publication, selective loading and resumable worker receipts. All normal/serial generation–integration combinations remain supported under existing method restrictions.

Run GCAD and resolution inside caller-owned generation workers. Report their preparation peaks separately from sector-worker residency; do not claim global CAD memory scales only with resident sectors. Persist completed preparation and cell records so serial workers release heavyweight objects.

Extend status and inspection with solver/verification progress, cell and chart counts, branch complexity, endpoint powers, resolution progress, evaluator sizes, timings and checkpoint state. Native snapshots drive CLI and HEPKit presentation.

Preserve seed reservations, checkpoint compatibility, full covariance, QMC, Havana and ordinary discrete MC. Changes to cells, maps, chambers or regulator prescriptions change mathematical artifact identity.

## 5. Delivery gates and performance study

Deliver in validated milestones:

1. **Repository and dependency consolidation:** contour regression gates, safe rebase, one-owner dependency build, direct symGCAD solve/verify probes.
2. **First complete path:** fixed and parametric bubbles and massive triangles, causal phases, composed maps, artifact restoration and symbolic subtraction.
3. **General algebraic resolver:** audited algorithm, independently checked certificates, algebraic towers, ramification, all-face treatment and auxiliary-regulator support.
4. **Execution and ecosystem integration:** normal/serial combinations, recovery, native HEPKit interfaces and unaffected existing portable builds.
5. **Physical validation and optimization:** scalar suite, ggHH subsets, full physical results and published benchmark report.

Required tests include:

- Exact threshold controls with cancelling cell poles and a nonzero finite imaginary part.
- Repeated propagators, complex numerators, Taylor/IBP parity and complete Laurent cancellations.
- Boundary controls such as \(x+(2y-1)^2\), \((x-a)^2\), \(r^2-y\), and \(\sqrt{x^2+y^2}\).
- Higher-degree and nested algebraic branches, vanishing leading coefficients, close roots and every supported root-selector convention.
- Independent map/Jacobian/derivative checks against small explicit-substitution controls.
- Parameter rebinding across several admitted points and rejection of invalid chambers.
- Tampered proofs, incomplete solves, interrupted writes, resource exhaustion, fresh-process restoration and checkpoint resume.
- Deterministic sampling under reordered work, changed worker counts and process replacement.
- General resolver invariants and generated families of algebraic examples, beyond a catalogue of hand-written physics transformations.

The physics suite progresses through massive bubbles, triangles and boxes, the genuinely algebraic massive sunrise, mixed-mass kite, the paper’s non-planar two-loop example, existing physical multiloop fixtures, and ggHH.

For ggHH, first use selected sources, then the complete physical-numerator double-box example at \(\sqrt{s}=1000\) GeV. Retain the one-loop incoming-\(++\) HEPKit/MadLoop comparison as an independent physical control.

Compare threshold decomposition with the existing fixed and tuned dynamic contour modes. Measure:

- GCAD, verification, resolution, subtraction, optimization and load times separately.
- Aggregate peak RSS, artifact size and evaluator size.
- Average and maximum sampling cost per sector.
- Root-evaluation cost and reuse across Laurent outputs.
- Full complex estimator covariance, convergence versus work and convergence versus time.

Use independent tuning and confirmation runs. Include matched-work tests and **five-minute, 50-core QMC and discrete-MC campaigns** for the full double box. There is no mandatory per-mille stopping requirement for this phase’s ggHH benchmark.

Investigate gauge/order choices, stable root-gap evaluation, shared map programs, symmetry reuse and endpoint power transformations. Successful GCAD must lead to a carefully regularized, efficient numerical representation; performance and variance improvements must be demonstrated rather than inferred from sign separation.

Update the existing Typst benchmark report and commit its compiled PDF, reproducible inputs and compact results. Keep raw outputs and build products untracked.

Complete the goal only after the general resolver, required native interfaces, scientific checks and independent audits pass. Passing a small set of rational or quadratic examples alone does not complete this phase.

## Subsequent benchmark checkpoint (2026-10-11)

The user requests a new campaign with **all reported quantities targeting the
epsilon-zero coefficient**. Retain the previous measurements as historical
results, with their original settings and coefficient orders.

- Pull the latest symGCAD changes and freeze reviewed dependency identities for
  the campaign. Keep earlier proof and benchmark evidence under its original
  identity.
- Add a scalar double box with the same exact fixed kinematics, masses and
  propagators as the physical-numerator ggHH double box. Reuse a verified GCAD
  solve only after checking native geometry identity; the two source-density
  owners remain distinct.
- Compare fixed contour, dynamic contour and no-contour threshold decomposition
  across the existing scalar suite and both double-box variants. Include a
  meaningful above-threshold massive two-point kite. Both no-contour double-box
  cases use fixed exact kinematics.
- Use 25 workers with serial generation and serial QMC integration, initially
  capped at 100 GB. The user explicitly authorizes ordinary discrete MC's
  existing global scheduler as a labeled exception to serial residency. Reduce
  workers or deliberately raise memory limits when necessary and record it.
  Concurrent host activity must not prevent launching the campaigns.
- Start `benchmark.pdf` with a clean landscape-A4 **Overview**, grouped by
  topology and method, with adjacent QMC/discrete-MC rows. Report complete
  generation time, generation RAM, artifact size, sector count, maximum/average
  raw f64 per-sector sampling cost, integration RAM, summed-sector raw f64 and
  checked wall sampling costs, fixed-work relative precision, estimated-result
  standard error, and relative precision after ten minutes. Select the fixed
  work budget using independent pilots to target roughly ten-minute runs.
  The repeated generation-RAM item is provisionally interpreted as integration
  RAM; the standard-error and discrete-MC choices were explicitly confirmed.
- Put column definitions, exact topology/kinematic descriptions, deformation
  settings, covariance conventions and reproduction instructions after the
  opening table. Do not relabel old runs as the new campaign or present missing
  no-contour results as completed integrations.

Once all requested scalar and double-box results and the compiled PDF are
delivered, fix any remaining implementation/generalization gaps that can be
addressed quickly and stop at the user's requested checkpoint. This checkpoint
does not waive the scientific distinction between a completed benchmark and
any still-unfinished general-resolver acceptance gates above.

The user subsequently authorizes increasing physical endpoint-preparation time
beyond the initial twenty-minute diagnostic budget. A two-hour follow-up keeps
the 100 GB memory bound and stage measurements; both outer process supervision
and cooperative preparation limits must permit it. A timed-out preparation is
an incomplete measurement, not evidence that the requested integral is
infeasible. Continue compact symbolic-series optimization in parallel.

The user also permits omitting IBP reduction when its generation cost becomes
excessive. Compare it with the existing direct symbolic Taylor subtraction,
retaining every required endpoint counterterm, boundary contribution and Laurent
cancellation. This does not authorize leaving divergent endpoints unsubtracted
or replacing symbolic endpoint treatment with numerical-dual jets. Record the
chosen strategy and measure both generation cost and any resulting increase in
precision rescues and integration time. Expensive finite endpoint coefficients
shared by both strategies still require compact symbolic preparation.

The subsequent user instruction makes optional runtime causal validation
**disabled by default** in native settings, CLI/TOML and HEPKit constructors.
Explicit `always` and `pilot` policies remain available. Preserve explicitly
saved policies when loading existing artifacts or settings. All sampling and
convergence metrics in the new benchmark table use explicit validation `off`;
separate validation probes are not part of those timings. The reported checked
wall cost includes ordinary stability checks and precision rescue, but excludes
optional causal validation. Essential solver and nonfinite handling remain on.

The subsequent user instruction makes mutual and reference compatibility an
explicit benchmark acceptance gate. Compare every accepted integration estimate
across prescriptions, samplers and confirmation arms at the same integral,
kinematics, normalization and Laurent order. Retain the full complex covariance,
seed relationships and numerical limitations when assessing discrepancies.
Use existing analytic and native HEPKit references wherever available, checking
their point and normalization rather than borrowing a value from another case.
Keep stochastic standard errors separate from reference uncertainty and numerical
roundoff. Significant discrepancies remain visible and require independent
diagnostics; do not tune on confirmation values, silently inflate their errors,
or label incompatible results as validated.

The subsequent benchmark simplification makes relative error after approximately
ten minutes the primary integration comparison. Preserve actual errors, elapsed
times and accepted work, and label any square-root work/time normalization as a
heuristic rather than a measured QMC convergence law. Keep accepted timed runs
within a 25% max/min elapsed-time spread for each topology; rerun outliers rather
than correcting large differences. The secondary work metric uses a round target
for average evaluations per sector, since discrete MC does not allocate equal
work to every sector.

Add streamed, saved and tabulated weight diagnostics: the estimated absolute
sector-weight integral divided by the magnitude of the total complex integral,
and the largest observed absolute sector-local weight divided by that same
magnitude. Define continuous/discrete proposal corrections, complex coefficient
pairing and exact-offset grouping explicitly. Accumulate accepted production only
through complete replica/batch and checkpoint semantics. Neither a pilot maximum
nor marginal component maxima may substitute for the complex production maximum.
At a zero or unavailable total mean, report the ratio as unavailable. These
diagnostics do not bound unseen weights or replace covariance-based uncertainty.

The final opening-table column is the relative uncertainty of the magnitude of
the complex epsilon-zero result. Propagate its complete real/imaginary covariance
along the radial direction; retain the joint complex covariance and uncertainty
for compatibility checks. At a zero magnitude report the relative quantity as
unavailable, and identify first-order propagation when uncertainty is large.
Immediately before this final column report the total number of sector-integrand
calls made during the timed measurement. Distinguish measured calls from accepted
production points, and label any legacy interrupted-run count as an observed
lower bound. Timed stopping should drain already assigned complete replicas or
batches so newly collected measurements have complete call accounting.

The subsequent implementation priority is to complete no-contour generation and
integration for the currently unsupported benchmark cases, from simplest to
hardest. A dedicated subagent continues the benchmark table and its new
absolute-weight/maximum-weight columns while the main implementation team starts
with the massive triangle, then the box and two-loop scalar topologies, followed
by both double boxes. A source-domain refusal before GCAD is an implementation
gap, not evidence of solver infeasibility. Each case must produce a complete
artifact, a checked full complex result, and working reload/integration before
its overview row is accepted. General reusable sufficient-class improvements
are appropriate; hard-coded physics answers or partial cell coverage are not.
