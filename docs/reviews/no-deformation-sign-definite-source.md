# Whole-source causal sign extraction

This is a narrow native preparation capability, not completion of the general
threshold resolver. `SignDefiniteSource` retains the original independently
verified GCAD owner and issues an identity for the full regulator-dependent
density when every singular factor has a uniform coefficient sign on the
positive orthant. Ordinary sector decomposition and symbolic subtraction are
still required afterward. The owner cannot be deserialized as a certificate.

## Native reuse and scientific checks

The public symGCAD domain API already supplies `derive_domain`,
`certify_nonzero`, and `verify_certificate`. Its source and tests implement the
`ShiftedCoefficients` proof method; the focused Rust tests exercise those same
operations. Symbolica's native rational-polynomial conversion checks the actual
source variable roles before assigning GCAD aliases. No polynomial parser,
coefficient sign algorithm, branch implementation, or serialization is added.

The admitted orthant is stronger than the original simplex or unit-cube
interior. Closed-face regularity is deliberately not inferred from this sign
proof. Only the native coefficient-sign proof class is admitted by this route;
other successful sign proofs do not establish its endpoint geometry.

Each singular factor retains its original term/factor association. The source
sign is cross-checked against all corresponding verified cell signs through
the native `FactorOrigin`, rather than assuming that prepared factor indices
still equal original indices. Exact represented input and fixed kinematic
bindings are preserved. Undeclared coefficients cannot be captured by an alias.

A negative factor requires explicit causal semantics. The existing native
`causal_negative_phase` keeps the complete exponent dependence before Laurent
expansion. Polynomial numerators, complex prefactors and monomial powers are
preserved. Missing sign evidence is an unsupported result, never a zero.

## Validation and independent review

All three focused native tests pass:

- The full integral of `(1 + 2 i) (-x - i0)^(-1 + epsilon)` has the expected
  pole and finite imaginary contribution after existing symbolic subtraction.
- A sign-changing factor cannot enter the whole-source route.
- A negative factor with generic branch semantics is rejected.

The scalar implementation agent independently reviewed source binding,
orthant-vs-closed-face scope, native reuse and full epsilon-dependent phases.
The root agent checked the same code and the native test receipt. The tests
are in the Rust module; raw receipts remain untracked under
`target/no-deformation-recursive-endpoint-cover/`.

The caller owns execution. This native identity introduces no worker pool,
Python numerical implementation or HEPKit graph wrapper. It is not yet an
automatic CLI route or an executable threshold artifact. A physics result
still needs complete endpoint coverage, native compilation, reload and an
independent numerical comparison.
