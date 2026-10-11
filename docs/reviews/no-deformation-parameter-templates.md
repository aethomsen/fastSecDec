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
