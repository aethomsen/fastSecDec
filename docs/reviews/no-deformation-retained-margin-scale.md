# Exact positive normalization of retained margin proofs

Native rational-function normalization can move an exact positive rational
constant from a denominator into its numerator. A saved positivity certificate
for the original polynomial must therefore be related to the actual normalized
chart obligation without rewriting that certificate.

`RetainedMargin::with_positive_scale` supplies a proposal `c > 0`. Admission
checks the exact identity `Q - c*m = c*(P - m)` in the genuine chart's native
rational polynomial ring. It then replays the unchanged certificate for `P-m`,
with its original constraints, domain and resource settings. The issued margin
for `Q` is `c*m`; the retained original margin and bytes remain available.
Default scale one preserves the existing interface and behavior.

Symbolica owns rational arithmetic, coefficient multiplication and polynomial
equality; symGCAD owns certificate verification. Public API/source inspection
and focused executable probes confirmed this reuse. No certificate codec, CAS,
graph object, integration scheduler or statistical policy changes. The proposal
itself is not proof authority: wrong scales, roles, domains and resource bounds
are rejected on admission. Rational-size and cancellation checks remain active.

Independent scientific/source reviews passed, including the final explicit
`BTreeSet` type annotation needed by the native Python feature combination.
Private controls pass for 12 factor and three complete-chart tests. The shared
consuming endpoint-transport suite passes all 47 tests; strict all-target core
Clippy with the threshold feature and workspace formatting also pass.

The immutable handoff is retained under
`target/no-deformation-retained-margin-scale/final-r1` (SHA256
`4f77da5d410f16fbabf9aadc11bf03150b92fa87c8dc5e37bbf034dcf61d0118`).
The final independent review has SHA256
`676fa76e290138e971a1c4f7535c7ea488782d7a271eba2abf64818c1ae6c5aa`.
Actual double-box chart admission remains a separate scientific gate. These
controls do not establish a completed double-box integral.
