# Represented numerical points on native integral families

This extends the native threshold input boundary to an existing exact HEPKit
`IntegralFamily` and a separately supplied numerical point. It does not recover
rounding already performed while constructing that family. The stored family
basis and its context must be exact; unsupported earlier Float arithmetic is
reported explicitly.

## Native ownership and numerical meaning

`FamilyPoint` retains the original native family, signed propagator powers,
separate numerator and measure, complete point kinematics, scalar bindings and
formal coordinate roles. The existing bounded represented-value converter
maps the stored point values before any product or specialization. Numerator
and measure convert independently. HEPKit's immutable scalar-value mapper,
`prepare_family_input` and `ParametricIntegrand::from_family` own specialization,
signed-power projection and parameterization. No additional CAS, kinematics
representation or family reduction was introduced.

The source witness includes unused kinematic assumptions, original precision,
the ordered family basis, separate scalar slots and the resulting exact
density. Equal numerical output does not erase different source provenance.
Existing external-product compatibility checks remain authoritative; the
adapter does not silently override an already fixed product.

The new family provenance is exclusive of exact, represented-parametric and
graph provenance. Replay requires the caller's original native family point
and an independent conversion cap. It repeats bounded conversion and source
association, then verifies saved raw solver evidence without solving again.
Saved integration kernels continue to work without the original family.

## HEPKit boundary and caller-owned execution

The existing `Integral.from_family`, family/free `sector_decompose`, and
generation-session interfaces dispatch this native owner only with explicit
`numerical_meaning="represented_values"`. Typed progress exposes conversion
and native parameterization. The binding accepts source association from live
native-issued progress, not arbitrary receipt JSON, and reuses the existing
preparation, detached compilation and artifact workflows.

The original inputs remain alive until preparation finishes. Cancellation and
Python exceptions retain their identity and recovery behavior. Presentation
does not start arithmetic automatically. No additional worker pool, Python
algebra or community implementation was added.

Independent native API, source, executable-probe and runtime reviews accept
these ownership and reuse boundaries. Conversion limits are not hard native
parameterization RSS limits. Represented graph numerator/projector payloads,
general algebraic preparation and runtime-parametric geometry remain separate
work.

## Validation

The frozen native controls cover pre-arithmetic conversion, signed and zero
powers, original-context and precision identities, unsupported stored inputs,
cancellation, replay, and publication of the complete Laurent vector. Existing
graph and request-wire controls also pass. A rebuilt real HEPKit host exercises
all three family entrypoints, interruption and raw-evidence recovery without a
new solve, changed unused context, and artifact evaluation after original-owner
release. Its complex bubble retains all four Laurent components and all 16
covariance entries and agrees with the independent analytic control.

The coordinator verified all 18 source preimages and 48 evidence digests in
handoff `8f3f3ec6af80d94442a75d06d59f0500a9a3ad37c83174977144dd7a1550026a`.
Seven native family controls pass, including their separately invoked fresh
process; eight existing request-wire controls, seven graph controls and 14
generation controls pass. The real HEPKit host passes in 7.80 seconds.
Native and portable strict binding checks and both actual stub-emission/AST
tests pass. Portable metadata retains exactly the `portable` core feature and
does not resolve symGCAD. One equivalent let-chain change addresses a strict
lint diagnostic in the existing source-identity acceptance method.

The consuming shared checkout passes the complete native library suite:
692 passed, no failures, and 22 explicitly ignored manual controls in 263.80
seconds. Its seven family integration tests pass, including invocation of the
fresh-process child. The real HEPKit host passes against these shared sources
in 7.96 seconds. Strict workspace/all-targets lint passes in 52.11 seconds;
strict native binding/all-targets plus stub-generation lint passes in 28.90
seconds. Both source-format checks and the diff check pass. These joined
results include the final let-chain correction.

Private source and raw evidence remain untracked under
`target/no-deformation-represented-family/`; consuming gate logs are under
`target/no-deformation-family-joined/`. Portable binding and stub-emission
evidence refers to the frozen source above, rather than a new joined rerun.

## Related native graph API gap

The next graph-payload adapter needs the existing global numerator prefactor
to be replaceable without rebuilding the physical graph. A narrow native
`FeynmanDiagram::with_numerator_prefactor` addition is proposed in
[GammaLoop PR 132](https://github.com/alphal00p/gammaloop/pull/132), based on
`aa2ec7b38363c01971916c178a79d7f27e6b8ce3`. The draft is authored and published
by ValentinHirschi, with `benruijl` requested as reviewer. Two new controls,
five existing graph controls, and strict native leaf lint/format checks pass;
this is not a full owner-workspace CI claim.

At this family milestone, FastSecDec's dependency remained `c81fa327`. The setter
does not decide whether
rounded aggregate numerators or their primitive fragments are authoritative
for a later exact conversion. That policy needs its own explicit contract;
this milestone neither changes graph semantics nor reconstructs lost rounding.
The subsequent [graph-payload milestone](no-deformation-represented-graph-payloads.md)
adopts the setter on that existing owner and supplies an explicit primitive
interpretation, leaving the default exact-payload convention unchanged.
