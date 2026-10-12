# Endpoint resource settings and native HEPKit routes

The CLI and HEPKit now call the same native
`PreparationOptions::enable_automatic_endpoint_routes` policy. It fills an
absent `regular_family` setting, preserves explicitly supplied settings and
leaves route classification and coordinate roles to the existing native issuer.
The one-dimensional legacy path remains supported. This corrects a HEPKit
integration gap: its former preparation entrypoint never enabled the admitted
multidimensional routes.

`EndpointPreparationResources` is a shared serde DTO for partial geometry,
discovery, prefactor and chart-count overrides. It changes resources rather than
mathematical roles, margins, input identities or proof algorithms. Scalar and
discovery checks, including checked durations and integer conversions, complete
before either effective setting is assigned. Native symGCAD certificate
construction and replay retain responsibility for their private resource
ceilings. Tests pass zero and excessive limits to that native operation; no
duplicate ceiling table or certificate implementation was added.

The resource clocks cover their respective cooperative native operations. They
do not bound continuation, staging, compilation or aggregate host work. A caller
can separately supervise total wall time and memory. Defaults remain unchanged.
The configuration codec already persists the effective policy.

This slice applies to fresh preparation. Ordinary resume restores the original
configuration and retains issued-settings and source checks. A resource-only
retry must be a separately identified preparation; it is not implemented by
loosening ordinary resume. The CLI fingerprints original TOML, so an absent
new setting does not insert a serialized null into existing identities.
HEPKit likewise omits the absent setting from `ThresholdSettings.to_json()`.

## Validation and evidence boundaries

The broad native generation gate passes 35 tests, with two intentionally ignored
child-process entrypoints. This includes resource defaults, effective-policy
restoration, atomic refusal, graph/family preparation, source checks and recovery.
The five CLI input tests and six serial threshold/recovery tests pass; the latter
also has two intentionally ignored child entrypoints. Strict feature-enabled
core/CLI all-target Clippy passes. The binding's native and stub-generation
all-target strict Clippy gate, generated constructor-signature/AST test, and
both formatting checks also pass. The generated native constructor includes
`endpoint_resources_json`; no manual implementation was added to Community.

The first broad run exposed two old configuration tests that incorrectly treated
version 3 as unsupported. Their test-only unknown version now uses `u32::MAX`;
valid version 3 still refuses restoration through an incompatible route. No
production decoder was changed. The earlier narrower dependency-adoption gate
did not include these tests.

The maintained native Python test exercises above-threshold triangle and box
inputs through both a HEPKit diagram and its retained integral family. It uses
the maintained native graph/model/Gram/measure and native numerator contraction,
then prepares, compiles, restores the archive into fresh runtime ownership and
integrates all Laurent components with covariance. All four paths pass their
OneLOop comparisons at eight standard errors plus a `2e-9` binary64 allowance.
They produce respectively 4 cells/18 charts and 6 cells/54 charts. Runtime was
89.22 seconds for the frozen native host test. Archive reconstruction is within
the same process, not a fresh operating-system process.

The private host lock was seeded from the current Python binding lock; only its
consumer package was added. It retains the adopted Symbolica, Numerica, FeynKit
and symGCAD identities. The failed initial host attempt that picked up unrelated
registry changes is retained as failed evidence, not part of acceptance.

Independent source/interface review covers caller-owned work, native graph and
algebra reuse, covariance, error boundaries, and the strict distinction between
fresh preparation and resume. Raw logs and source inventories remain untracked
under `target/no-deformation-endpoint-resource-implementation` and
`target/no-deformation-endpoint-resource-runtime-review`. Community stub/wheel
publication and portable threshold support are separate delivery work.
