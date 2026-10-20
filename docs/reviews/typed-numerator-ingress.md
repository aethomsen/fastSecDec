# Typed numerator ingress

FeynKit's typed diagram API returns `SymbolicTensor<PartialStructure>` for the
numerator, projector, and numerator prefactor. The former unchecked Atom
product no longer compiles against that API. Stripping those objects back to
Atoms would also lose unresolved ports and the interface of a ranked zero.

The adapter now composes the existing typed owners through Idenso's
`SymbolicTensor::multiply`, changes Lorentz dimensions through
`with_lorentz_dimension`, and validates native momentum routing through
`with_checked_expression`. Existing simplification and scalar admission remain
authoritative. The overall Atom factor is admitted through native inference
before composition. No contraction, routing, dimension, or zero algorithm is
added.

Python input delegates to the public `spynso3::expression::as_tensor` and
`TensorExpression::structured` APIs. Existing tensor subclasses retain their
interfaces; plain Symbolica expressions receive native inference. This uses the
same Spenso Python owner already linked by FeynKit.

Reuse was checked in the public API, implementation, and tests of FeynKit
`588148851e3e989ecbb7478279db7d46727e23a8`, including:

- `crates/idenso/src/tensor/composition.rs::SymbolicTensor::multiply` and its
  scalar-composition/interface tests.
- `crates/idenso/src/tensor/inference.rs::{with_lorentz_dimension,
  with_checked_expression}` and ranked-zero interface handling.
- `crates/spynso3/src/expression.rs::{as_tensor, TensorExpression::structured}`.
- `crates/feynkit-graph/src/lib.rs` typed diagram payload accessors and setters.

Validation used a separate consuming workspace selecting that typed FeynKit
owner plus its pending generic-conjugation and symbol-renaming fixes, and one
Symbolica source `74c5a5d23bc0dd064399aed344e5f10c3f536f48`. Three native regression
tests pass: both contraction modes preserve every graph weight exactly once;
an indexed projector contracts the numerator; and a rank-one zero is rejected
as a scalar integrand. The focused consuming graph passes Clippy with
`-D warnings`. The combined community Python host passes `cargo check`.
Rust formatting and Python regression syntax were checked.

The Python regression additionally checks unresolved and ranked-zero projector
interfaces; installed-wheel execution remains pending. Historical root,
portable, and binding lockfiles are not advanced by this narrow compatibility
change. Full tests under those older dependency selections are not reported
as passing; the typed FeynKit dependency migration is a consuming-owner decision.
