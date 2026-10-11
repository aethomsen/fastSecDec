# Explicit numerical graph payloads

The optional `PrimitiveFragmentsV1` policy interprets the numerical coefficients
stored in each original native vertex and edge fragment before recomputing the
numerator. It is a deliberate input convention: the result need not equal an
independently exactified, already rounded aggregate. The original aggregate is
retained as provenance and is never multiplied into the density again.

## Native ownership and scope

The existing `PreparationOptions` and `prepare_graph` interfaces carry this
policy alongside the explicit `RepresentedValues` convention. The default
remains the existing exact graph-payload path. Native HEPKit callers use
`ThresholdSettings(numerical_meaning="represented_values",
graph_payloads="primitive_fragments_v1")` through their existing diagram,
integral and generation-session entrypoints. Family and already-parametric
inputs reject this graph-specific setting.

The bounded represented-value converter processes each primitive fragment,
projector, numerator prefactor and overall factor independently. Native graph
setters recompute the aggregate and apply each global factor once. The original
graph/model owner, topology, routing, cuts, IDs and input precision are retained.
Native graph validation, Idenso tensor inference, kinematics and family
parameterization continue to own those operations. No replacement graph,
tensor contraction, parser or CAS is introduced.

Conversion rejects Float literals inside any function argument before native
tensor admission. This prevents numerical tensor indices or momentum labels
from becoming apparently valid exact metadata. It is intentionally conservative:
legitimate scalar `f(0.1)` and numerical coefficients nested in shorthand
arguments are also refused. The current tensor controls do not establish full
ggHH polarization support. Earlier rounding within an individual stored fragment
cannot be recovered.

## Source identity, replay and memory

Policy-free source identities and configuration bytes retain their v1 form.
The explicit policy introduces a distinct v2 source witness binding the
original and effective native diagrams, the interpretation, precision-bearing
conversion records, the full original numerical point and the resulting exact
density. A matching numerical answer cannot erase different source provenance.

Replay requires the original native input and a separately supplied resource
cap. It reconstructs the bounded conversion, checks the request/configuration
policy and source association, and runs the verifier on saved raw GCAD evidence
without solving again. The Python owner accepts source association from native
progress rather than treating arbitrary imported receipt JSON as authority.
Completed integration artifacts remain usable after releasing the input owner.

Cancellation and conversion-limit failures leave the original input unchanged.
The original and effective diagrams plus occurrence evidence reside during
preparation; these limits do not bound native tensor contraction, family
parameterization or GCAD process RSS. Caller-owned execution remains necessary.

## Owner-library change

Public API, source and executable probes identified one missing immutable
operation: replacing the diagram's existing global numerator prefactor. The
63-line setter/test patch is proposed in
[GammaLoop PR 132](https://github.com/alphal00p/gammaloop/pull/132), authored and
published by ValentinHirschi with `benruijl` requested as reviewer. The consumer
revision `9f8b1e50016a58d04e712c34569ea936c44e179a` applies exactly that patch on
the already adopted `c81fa327` owner. It preserves the kinematics mapper and
citation fixes; directly adopting the PR head would lose those changes.

No owner manifests change. Core, Python and portable consumers use one coherent
public owner identity. Substantive conversion and policy code stays in
FastSecDec; community registration is unchanged.

## Validation

Independent runtime and coordinator reviews accept this scoped interpretation,
native ownership, replay association and caller-owned scheduling. Maintained
controls cover 53- and 192-bit inputs, scalar and tensor fragments, projectors,
all global factors, structural Float rejection, cancellation, altered-source
rejection and fresh-process restoration. The complete bubble comparison retains
four Laurent components and all 16 covariance entries, and uses an independent
analytic reference multiplied by the native tensor contraction.

The coordinator verified all 77 evidence digests, 13 source preimages/postimages
and six public dependency files in handoff
`fb8a0876fc57b60236547bb588655118f84266b62bddcd581d74fa11be94a983`.
Four payload controls pass, including their separately invoked fresh child;
nine request-wire controls, 15 generation controls, seven graph and seven
family controls pass. The two existing scalar-conversion controls also pass.
The old/new native identity probe retains byte-identical policy-free source
and exact-density IDs and conversion evidence.

The real HEPKit host passes in 9.64 seconds. It exercises tensor input through
the existing entrypoints, interruption/replay without solving, complete complex
vectors and artifact use after releasing the original owner. Strict native and
portable binding checks pass, as do both actual stub-emission/AST tests.
Portable metadata excludes GCAD and threshold generation. The host uses the
clean path checkout of the exact published owner; separate public metadata
checks confirm one dependency identity and no unrelated package drift.

The shared checkout passes the four payload controls and fresh child against
the public Git owner, the two scalar-conversion controls, nine request-wire
controls and 15 generation controls. Strict workspace/all-targets lint passes
in 61 seconds; both source-format checks and the diff check pass. The native
host and portable evidence above applies to the exact frozen source; these
joined gates do not claim a rerun of the complete physics suite.

Raw evidence remains untracked under
`target/no-deformation-graph-payload-implementation/`; joined logs are under
`target/no-deformation-graph-payload-joined/`. No installed wheel or release
binary was replaced. This capability does not complete general algebraic
preparation, the real closed-face atlas, or the physical ggHH gate.
