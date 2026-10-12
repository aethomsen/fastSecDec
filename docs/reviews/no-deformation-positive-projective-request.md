# Native positive-projective requests

`threshold::gcad::PositiveProjectiveRequest` builds the original homogeneous
GCAD problem from a complete native parametric density or an existing
`ExactRepresentedGraphInput`. This removes the need for callers to assemble
that problem by hand before obtaining a `PositiveProjectiveQuotient`.

The request retains the original density, anchor and source interpretation.
Its `solve()` method is synchronous and returns untrusted native evidence.
Callers can save that evidence before calling `verify()`, including when the
solver reports an incomplete result. Verification checks the complete original
problem association and invokes the existing independent homogeneous verifier.
A reduced-only solve cannot stand in for the original homogeneous proof.

## Mathematical and ecosystem boundaries

The implementation reuses `PositiveProjectivePreparation::anchor` for the
homogeneity and measure identity. It reuses the existing `anchor::source_poly`
conversion, Symbolica's rational polynomial content/division, and symGCAD's
problem normalization, homogeneous solve dispatch and independent verifier.
The adopted public APIs, their implementation/tests, and focused executable
controls were checked. No new CAS, graph parser, root algorithm, derivative
engine or serialization format is introduced.

Only active causal factors enter the split inventory. Positive rational content
is removed without reversing their signs. Repeated causal polynomials share a
split. Declared positive factors remain obligations of verification: they are
not extra domain inequalities that could discard part of the integral. The
domain is the complete original positive orthant.

The request requires one native solver worker and a complete given lifting
order with the anchor first. A missing order receives that explicit default;
conflicting orders, anchor choices, partial homogeneous blocks, implicit order
searches and negative-only coverage are rejected. The caller retains process,
time and memory ownership. Native problem validation still applies.

The graph path shares HEPKit's existing graph, kinematics, represented-value
conversion and exact density owners. Its source identity retains the original
numerical interpretation. It does not replace the graph by a polynomial-only
identity or construct a synthetic reduced request.

## Validation and limits

Independent source and ecosystem review passed. The shared native GCAD test
module passes 32 controls; its separately invoked fresh-process helper remains
ignored as a standalone test. Strict library Clippy and workspace formatting
also pass. Five new focused controls cover:

- Both sign cells of a homogeneous causal denominator; rejection of a changed
  problem, incomplete cell inventory and reduced-only evidence.
- A falsely declared positive factor, rejected without pruning its domain.
- Invalid worker, ordering, anchor and partial-coverage settings.
- All seven generic cells of the massive sunrise at mass one and momentum
  squared sixteen, preserving original and quotient coordinate orders.
- A real HEPKit bubble graph with represented floating-point kinematics, mass
  and measure; all three causal cells and the original graph/exact-input
  ownership identities survive verification.

These are geometry and source-association controls. The sunrise request control
has prefactor one and does not claim to evaluate the physical integral. The
separate [massive-sunrise reference controls](no-deformation-massive-sunrise-controls.md)
retain the full physical normalization.

This API currently admits exact fixed rational singular geometry; numerator
and prefactor handling remain the responsibility of the complete downstream
density owner. It does not issue endpoint certificates, perform subtraction,
publish integration artifacts or make the new positive-projective route an
automatic CLI/HEPKit generation option. Durable request staging and routing
remain subsequent integration work. Existing affine requests and their saved
formats are unchanged.
