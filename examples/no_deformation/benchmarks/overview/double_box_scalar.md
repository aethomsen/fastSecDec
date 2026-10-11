# Scalar double box at the exact rational 1000 GeV point

This card uses the native D05 topology, seven unit propagator powers, mt = 345/2, mH = 125, s = 1000000, t = −97077 and u = −871673. Every fixed Gram entry is copied exactly from the maintained rational-point fixture. Auxiliary polarization entries remain as provenance; the scalar numerator and projector are one. Model/parameter assets are shared with D05. The original physical overall factor −1 is replaced by one. Native Feynman/loop-measure normalization remains: parameterization gives −Gamma(3 + 2 eps) U^(1 + 3 eps) F^(−3 − 2 eps), with delta-simplex measure and no extra normalization.

Reproduce the DOT through HEPKit's native serializer and validated roundtrip, from the repository root:

```sh
nix-shell --run 'cargo run -p fastsecdec --example double_box_scalar_export -- examples/no_deformation/benchmarks/overview/double_box_scalar.dot'
```

The exporter invokes the native numerator/projector/prefactor/overall setters and `to_dot`/`from_dot`; no DOT rewrite or graph reconstruction is used. Native input-only checks establish exact U/F and denominator-family equality with the physical point, positive F scale and U authority, and a distinct scalar source identity. Geometry equivalence never transfers the physical numerator's source authority.

Run the native CLI input/export check without generation or sampling:

```sh
fastsecdec export-symanzik examples/no_deformation/benchmarks/overview/double_box_scalar.toml -o target/overview/scalar-uf
```

The native DOT roundtrip and historical CLI source-only export both passed. The exported canonical U/F bytes equal the independent scalar input probe; no second CAS comparison is used. The CLI assigns its own regulator/source roles, so equality of U/F does not imply equality of the complete source identity.

Generation and integration are separate explicit actions. This epsilon-zero card is preparatory; it is not a timing result or an integrated artifact. Apply the campaign's frozen recipe, executable, scheduler, resource and integration overlay as specified in the overview protocol. No-contour endpoint completion remains pending.
