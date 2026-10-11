# Parameterized regular algebraic families: implementation boundary

This is an API and mathematical-design review, not an implemented parameterized
threshold kernel. The research source inventory is FastSecDec `1f05fd5`; the
focused executable controls use the frozen paired-family dependency graph
recorded in their receipt. No dependency revision changes are required by this
design.

The required result is **one compiled template with checked runtime rebinding**.
Compiling a new fixed-fiber artifact for each kinematic point would not meet the
approved plan. Existing `KernelSet::bind_parameters` already preserves evaluator
programs and template identity, updates input tails and derives a point identity.
Precision caches refresh every input, and worker clones retain separate binding
and scratch state. These mechanisms should be extended rather than duplicated.

## Missing authority and interfaces

The regular-root owner already proves a nonlinear section uniformly over a
parameter domain. However, exact parameter admission currently lives inside
`LinearCellMap`, whose constructor first lowers all integration boundaries to
linear roots. Extract its shared prefix checks so an algebraic integration
boundary does not prevent parameter admission. Keep parameter, original-axis,
integration-coordinate and evaluator-slot roles explicit. Parameters are not
sampled coordinates and contribute no integration Jacobian.

An admitted family must retain the entire cell inventory, original source,
selectors, chamber and uniform continuation proof. A saved compact chamber
descriptor must license the actual runtime inputs before the existing atomic
binding commit. A successful numerical root evaluation alone is no admission:
the probe evaluates a root outside its original chamber without error.

The first useful class keeps kinematic parameters in exact polynomial
coefficients, numerators and function bodies, with parameter-independent
rational-affine regulator powers and Gamma arguments. This permits uniform
subtraction orders and Laurent support. Root gaps, units, finite-face identities
and causal phases must hold over the complete admitted chamber. Parameter
strata where these claims fail require refusal or a separately issued template.
General parameter-dependent exponents are not covered by this first class.

Runtime binding currently stores finite `f64` inputs. Exact chamber admission
must therefore check each float's actual rational value, using native
conversion, rather than a nearby intended decimal or rational. Failed admission
must leave the old binding, point identity and worker state usable. Neither
restoration nor rebinding should rerun GCAD, symbolic subtraction or evaluator
optimization. Proof-heavy preparation owners must be releasable before jobs
are distributed.

## Evidence and next gate

Four focused native controls passed in 0.06 seconds:

- Exact linear parameter-prefix checks retain strict boundaries and the exact
  input key set, while sharing immutable evaluator arithmetic across rebinds.
- A quintic section `y^5 + y - p = 0` admits the chamber `1/4 < p < 1/2`; current
  fixed-family interfaces correctly refuse unsupported parameter plumbing.
- Degree-five and degree-seven root values and first/second implicit derivatives
  agree with independently isolated fibers and analytic derivative identities
  at reordered parameter points.
- Replacing an outer function call does not bind parameters captured in its
  body. All saved definitions and derivative functions need explicit parameter
  inputs or an audited native capture mechanism.

An optional fifth standalone compile/save/rebind probe could not link against
an old frozen core after a concurrent leased build replaced its transitive
cache artifacts. This is recorded as incomplete; no fifth pass or new runtime
performance claim is made. Existing maintained binding tests were inspected,
but this study does not claim to rerun them.

The next implementation gate compiles the complete quintic two-cell family
once, saves/restores it in a fresh native process and binds at least two admitted
points. It must preserve the template and programs, change the point identity,
refresh all precision-tier inputs, reject invalid points atomically and retain
the complete Laurent vector and covariance. The finite imaginary coefficient
has the independent control `pi / P_y(r(p))`; that one coefficient alone does
not validate all Laurent outputs. Additional controls cover source/selector
tampering, cell omission, captured parameters and selective loading.

The frozen research handoff is
`79c65bcd07dfdcfa8ebfb83f42b4bf15c07fe196eb3bdb90626695648d57d3ca`.
The coordinator verified its 19 evidence files, 35 shared-source pins and two
native-source pins and reviewed the existing binding and prefix implementations.
An independent scientific review accepted the stated design scope. This work
does not close the general parameter-stratification or algebraic-resolution
gates.

## Detached guard follow-up

The focused codec probe found that the adopted rational polynomial ring does
not meet the native polynomial codec's `Encode` bound. The implementation
therefore reuses the existing exact evaluator codec, maps its restored constants
to the native rational ring once, and uses native exact ring evaluation at
admission. Numerator and denominator outputs remain separate until a nonzero
denominator is established. No polynomial codec or arithmetic engine is added.

An intermediate private implementation passes five native parent tests and an
explicit fresh-process helper. These cover exact polynomial agreement, negative
denominators, strict boundaries, exceptional predicates, degree-drop and positive
selector distinctions, actual floating-point values, resource/shape refusals,
and native State restoration after reversing symbol registration. Saved native
IR bytes remain unchanged across that restoration. The coordinator verified
four source preimages/postimages and 24 evidence digests in handoff
`57e798176b4662632b8788211a95070d0ea52be130e9f217b39fe7be2c2be583`.

This checker does not issue a uniform chamber or mutate a kernel binding.
Source-derived alias mapping and whole-family authority still belong to the
geometry owner. Integration must admit the chamber before evaluating exact
offsets, then use the existing atomic binding commit. Default-native template
restoration, two-point full-vector evaluation and rejected-binding atomicity
remain pending. Native allocation limits are not inferred from post-evaluation
output-size checks.

## Compiled-template follow-up

The [compiled-family implementation review](no-deformation-compiled-parameter-templates.md)
records the subsequent native runtime, artifact and binding work. The research
refusals above describe the earlier snapshot; the successor review states the
implemented scope and the remaining frontend and general-resolution gaps.
