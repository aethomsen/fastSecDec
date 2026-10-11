# D05 at exact rational Mandelstam invariants

This is the same incoming-`++` physical double box used in the
[1000 GeV contour comparison](../gghh_double_box_1000/README.md), at the nearby
point used by the [GCAD feasibility study](../../no_deformation/gghh_feasibility/README.md):

- `s = 1000000 GeV²`, `t = -97077 GeV²`, `u = -871673 GeV²`.
- `mH = 125 GeV`, `mt = 345/2 GeV`.
- `cos(theta) ≈ 0.799999308810368`, with positive outgoing x momentum.

These invariants are deliberately different from the older `cos(theta)=4/5`
point. No result or uncertainty at that older point is a result for this one.
The graph, projected numerator, model and normalization remain the existing
native D05 assets. The individual diagram uses the unnormalized `delta_ab`
projector, with no diagram sum or spin/color average.

The Rust exporter constructs the outgoing momentum
`(500, sqrt(21093814799)/500, 0, 193649/500)` and its recoiling partner using
native exact arithmetic. Native HEPKit momentum routing, `FourMomentum` scalar
products and incoming helicity wavefunctions supply all external data. The
usual exact transport of their binary floating-point polarization components
is retained; it does not make their normalization an exact algebraic one.
All 15 Gram entries are regenerated consistently, including those involving
polarizations. The geometry has rational invariants even though individual
momentum components can be algebraic.

Reproduce the inputs and their checks from the repository root:

```sh
nix-shell --run 'cargo run --locked -p fastsecdec --example gghh_double_box_400 -- . target/gghh-double-box-rational-input-check --sqrt-s 1000 --t -97077'
nix-shell --run 'cargo test --locked -p fastsecdec --example gghh_double_box_400 -- --test-threads=1'
```

The exporter requires a fresh directory. Its relative graph/model paths target
the maintained example directory depth; temporary outputs are for comparison.
`point-exact.json` retains exact data, `point.toml` supplies native runtime values,
and `validation.json` records checks and source provenance. Unphysical angular
values and energies at or below the two-Higgs threshold are refused.

For a matched contour comparison, use this `point.toml` with the saved parametric
contour artifact and a fresh checkpoint via `--parameters`. Generation defaults
are unchanged. The included `run.toml` is the same contour-capable parametric
input as the existing 1000 GeV example; it is **not** an automatically specialized
threshold-decomposition recipe. The separately maintained GCAD cards fix the
geometry exactly. This directory establishes input consistency only: it does not
claim completed GCAD, endpoint regularization or numerical integration.
