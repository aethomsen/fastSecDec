# Threshold-decomposed bubble

`threshold_bubble.toml` uses the native HEPKit scalar bubble graph with
`s=16`, equal squared masses `m²=3`, and `D=4-2*eps`. Its causal polynomial
has the two interior zeros `x=1/4,3/4`.

```sh
fastsecdec generate examples/no_deformation/threshold_bubble.toml \
  --serial --workers 2 --output bubble.fsd
fastsecdec integrate bubble.fsd --method qmc --workers 2 \
  --points 4096 --shifts 8 --target-order 0
fastsecdec integrate bubble.fsd --method mc --serial 0.1 --workers 2 \
  --points 4096 --shifts 8 --target-order 0 --max-rounds 3
```

`generation.threshold_decomposition=true` in the card enables generation;
`--threshold-decomposition` is its CLI equivalent. Generation and integration
serial settings are independent and use the same artifact format.

With the measure multiplier specified in the card, the analytic result is

```text
eps^-1:  1
eps^0:   2 - 3*log(3)/2 + i*pi/2
         0.3520815669978355 + 1.5707963267948966*i
```

This example exercises the fixed, one-dimensional rational cell path.
[Runtime templates](parameter_templates/README.md) also support explicit
single-chamber parameter rebinding. General algebraic endpoints and broader
parameter families remain under development; unsupported inputs fail explicitly. It is not a substitute
for the general resolver acceptance gates in `NO_DEFORMATION_PLAN.md`.

## Regular sextic boundaries

`regular_sextic.toml` integrates `(y^6-y+(1+x)/16-i0)^(-1-eps)` on
the unit square. Native preparation finds its two degree-six root sections and
retains all three cells and four endpoint charts. No root formulas or manual
brackets are required.

```sh
fastsecdec generate examples/no_deformation/regular_sextic.toml \
  --serial --workers 2 --output sextic.fsd
fastsecdec integrate sextic.fsd --points 1024 --shifts 4 --workers 2 \
  --seed 202610119061 --target-order 0 --max-rounds 1
# Add --serial 0.001 to integrate one resident record per worker.
```

The pole cancels in the complete result. The finite imaginary coefficient is
`3.8521160461762443`, independently obtained from the endpoint root differences.
The card enables tracked whole-vector validation; no cutoff-zero handling is
used. Ordinary and serial generation produce the same artifact identity, and
both integration modes pass the [four-way correctness control](../../docs/reviews/no-deformation-regular-family-factory.md).
This is the fixed regular-branch subset. Colliding roots, general algebraic
towers and broader parameter families remain under development.

## Regular degree-seven boundary

`regular_degree_seven.toml` integrates `(y^7+y-x-i0)^(-1-eps)` over
the full unit square. The same automatic path constructs its two cells using
the defining polynomial and root selector. The pole cancels; its finite
imaginary coefficient is `pi*r = 2.5024178911883874`, where `r^7+r=1`.

```sh
fastsecdec generate examples/no_deformation/regular_degree_seven.toml \
  --workers 1 --output degree-seven.fsd
fastsecdec integrate degree-seven.fsd --points 1024 --shifts 4 --workers 2 \
  --seed 202610119073 --target-order 0 --max-rounds 1
```

The [complete-vector control](../../docs/reviews/no-deformation-regular-sections.md#complete-degree-seven-control)
checks the saved-artifact path, causal imaginary part, pole cancellation and
full covariance. These regular boundaries require no radical formulas;
singular branch collisions still require the general resolver under development.
