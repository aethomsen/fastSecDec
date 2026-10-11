# Projective physical templates with runtime parameters

The native HEPKit massive bubble now admits one complete threshold template
throughout the strict chamber `15 < s < 18`, with squared mass `3`. Its original
homogeneous coordinates remain distinct from the one coordinate in the native
affine projective preparation. GCAD receives `[s,x]`; the compiled kernels
receive `[u,s]`. Rebinding a point does not rerun geometry, subtraction or
evaluator optimization.

The shared source-patch projection reads the actual preparation's images,
eliminated index, domain and measure. It constructs no new gauge. The paired
family covers three cells with four endpoint charts, retaining both root
branches and their regulator-dependent causal phases. The existing symbolic
subtraction, composed root evaluators, parameter guard, codecs, worker jobs and
QMC session remain their respective owners. No algebra, root solver, graph or
estimator implementation is duplicated.

Runtime parameters contribute no integration coordinate or Jacobian width.
Metadata validates the prepared dimension separately from the original source
dimension; the new projective semantic projection includes the original map
and native measure. The density already contains the preparation's measure,
so metadata projection adds no numerical factor. This particular delta gauge
has measure one: structural source checks, rather than the numerical bubble
value, establish that association. General nonunit gauges are not validated by
this example.

## Scientific controls

The graph's dimension is `4-2*epsilon`, with
`U=x+y` and `F=3*(x+y)^2-s*x*y`. Before parametrization it receives the existing
OneLOop comparison normalization `mu2^epsilon/rGamma`, with `mu2=1` and
`rGamma=Gamma(1+epsilon)*Gamma(1-epsilon)^2/Gamma(1-2*epsilon)`.

One compiled template is bound at `s=16`, `17` and back to `16`. Its program
bytes and template identity remain fixed while the point identity changes and
returns. Native fixed-fiber controls compare all real and imaginary Laurent
components through epsilon order one at three sample points and 128/256-bit
precision. Invalid chamber boundaries, a threshold point, nonfinite and missing
values are rejected before mutating the last valid binding.

The native QMC test crosses resident/detached preparation with whole/selected
record execution at both kinematic points. Each of the eight runs accepts
8,192 chart evaluations: 512 points, four shifts and four charts. At each point
all four paths preserve identical six-component means, errors and all 36
covariance entries, including checkpoint restoration. Selected execution uses
the same detached indexed archive. This is a native execution test, not the
automatic CLI normal/serial matrix, which remains a subsequent gate.

The existing native OneLOop Expression API supplies independent pole and
finite-part comparisons, including both real and imaginary components:

| `s` | Real UV pole | Real finite part | Imaginary finite part |
|---:|---:|---:|---:|
| 16 | 1.0000000000024185 | 0.3520815670127679 | 1.5707963267986957 |
| 17 | 1.0000000000024187 | 0.24252098510795242 | 1.7037678315615228 |

For `s=16`, a further closed control is `2-1.5*ln(3)+i*pi/2`. The UV pole is
retained. Epsilon order one remains in the full vector and covariance and has
pointwise fixed-fiber parity; OneLOop provides no independent integrated
reference for that coefficient. These debug controls are not performance
measurements.

## Execution, review and compatibility

Detached record jobs run after releasing the live geometry owner. Reordered
jobs, complete/selected/exact-only artifacts, parent and guard mutations,
resave and cache refresh exercise the existing transport boundaries. Ownership
assertions do not substitute for aggregate RSS measurements.

A separate default-native reader loads the actual new records without the
threshold-generation feature, with validation both enabled and disabled. After
unrelated symbol registration it checks binding, scope, identities and
resave/reload. Independent source/scientific and ecosystem reviews pass.
The coordinator verifies all 17 source pairs and 113 evidence pins in handoff
`2caeb19d0518b2f90526581aabd79657a94dcc73d083401aaf739092633f7075`.

Five actual historical artifacts, spanning v15, v16 and eager/SymJIT/shifted
v17, retain every baseline vector, order, offset, identity and scope when read
and resaved by the new default-native library. The default-native executable
receipt and comparisons are under `target/no-deformation-projective-consuming/`.
The v18/v4 capability introduced for parameter templates needs no additional
wire change here; old fixed formats retain their representations.

The first consuming run caught a metadata compatibility regression: an issuer's
single-patch restriction had also been applied to the generic structural reader.
Its existing two-patch test failed. The correction checks that every retained
patch has the chamber's prepared dimension, while leaving the live issuer's
narrower admission rule in place. Empty inventories and a mismatched dimension
in either patch are rejected. The original failure remains in the local receipt;
independent review accepts the two-file correction. Its source inventory is
`fe39a28faef990265c7f9ab349307189a32736c4931c868f5a761bed2785bf15`.

The consuming workspace passes five projective scientific/transport groups
(65.85 seconds), five existing cube-template groups, three paired-family groups
and, after correction, all thirteen metadata groups. The corrected build also
rechecks projective role/binding admission and the existing fixed projective
OneLOop integration. This is a focused milestone campaign rather than a full
workspace physics suite. The dependency revisions are unchanged.
Strict workspace/all-target Clippy (32.57 seconds), formatting and diff checks
pass on the corrected source.

## Remaining boundaries

This milestone exposes explicit native family admission and compilation.
Automatic parameterized CLI/HEPKit preparation and recovery remain unfinished.
The sufficient projective class is one prepared integration coordinate with
two uniformly regular boundary roots; multiple projective patches, moving
brackets and root collisions need further work.

Polynomial degree itself is not restricted to quadratics. Existing regular
programs retain defining polynomials and certified selectors, refine roots
through native facilities and obtain derivatives implicitly. The separate
degree-seven complete integral and degree-five compiled parameter control
exercise that generic representation. Singular or colliding roots require the
unfinished general algebraic resolver; this bubble is not evidence that it
has been completed.

The independent mixed parameter/Gamma-prefactor work is excluded from this
milestone. Its higher-order tracked `polygamma` evaluation still requires a
native owner capability; that failure is not hidden by reducing epsilon order.
