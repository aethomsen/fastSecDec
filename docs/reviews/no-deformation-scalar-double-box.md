# Scalar double-box input

The scalar benchmark uses the physical D05 graph at the same exact point:
`s = 1000000`, `t = -97077`, `u = -871673`, `mt = 345/2`, and `mH = 125`.
The maintained exporter calls HEPKit's existing numerator, projector, numerator
prefactor and overall-factor setters. Its native `to_dot`/`from_dot` roundtrip
preserves the full graph JSON. No DOT parser or graph reconstruction was added.

Setting the numerator to one also removes the physical fermion multiplier:
the scalar graph's overall factor is one. Native loop-measure and Feynman
parameterization conventions remain unchanged. The scalar density contains
`-Gamma(3 + 2 eps) U^(1 + 3 eps) F^(-3 - 2 eps)`.

Independent native input checks establish identical denominator families,
unit propagator powers, U and F, and positive causal-factor scale at this point.
The scalar and physical numerators retain distinct source identities. Reusing
geometry does not transfer authority over the other source's density.

The native exporter roundtrip, CLI source-only export, exact canonical U/F
comparison and workspace/all-target Clippy pass. This is input validation;
neither its subsecond runtime nor the existing GCAD proof is a complete scalar
generation or integration measurement. The no-contour endpoint path is pending.
