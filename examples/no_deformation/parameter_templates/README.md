# Runtime threshold templates

These cards compile one template for a certified open parameter chamber. The
bubble uses the native HEPKit graph, fixed masses m²=3 and 15<s<18. The direct
quintic declares `p` explicitly and has 9/4<p<5/2. Integration coordinates remain
in the unit interval; runtime parameters need not lie there.

Build the native CLI, then keep generation and integration explicit:

```sh
cargo build --release --locked -p fastsecdec-cli
fastsecdec generate examples/no_deformation/parameter_templates/bubble.toml -o target/runtime-bubble/normal.fsd --workers 2
fastsecdec generate examples/no_deformation/parameter_templates/bubble.toml -o target/runtime-bubble/serial.fsd --workers 2 --serial
fastsecdec integrate target/runtime-bubble/normal.fsd --parameter cli_uniform::s=16 --points 1024 --shifts 4 --workers 2 --seed 202610120071 --target-order 1 --max-rounds 1
fastsecdec integrate target/runtime-bubble/normal.fsd --parameter cli_uniform::s=17 --points 1024 --shifts 4 --workers 2 --seed 202610120071 --target-order 1 --max-rounds 1 --serial 0.001
```

Both saved artifacts support both integration modes and either admitted point.
Repeat the integration commands with `serial.fsd` for the other generation mode.
`--serial 0.001` sets the minimum sampling residence per loaded sector. Normal QMC
uses shared shifts; serial QMC uses independent sector streams. Their covariance
estimates have different statistical designs.

For the direct card:

```sh
fastsecdec generate examples/no_deformation/parameter_templates/quintic.toml -o target/runtime-quintic.fsd --workers 2
fastsecdec integrate target/runtime-quintic.fsd --parameter cli_uniform::p=2.375 --points 1024 --shifts 4 --workers 2 --seed 202610120071 --max-rounds 1
```

The bubble Laurent vector contains the real and imaginary components at orders
−1, 0 and 1. Its pole is 1. With β=√(1−12/s), its finite coefficient is
`2−log(3)−β log((1+β)/(1−β)) + i π β` in the card's OneLOop rGamma
normalization. At s=16 this is `2−(3/2)log(3)+iπ/2`. No independent analytic
reference is supplied here for the order-1 coefficient. Preserve the full vector
and covariance when comparing runs.

These bounded commands may stop before the requested accuracy. A work-limit stop
is reported as such. The cards explicitly request validated arithmetic, including
precision rescue. Missing, unknown and out-of-chamber values refuse; the chamber
endpoints are excluded. A numerical checkpoint belongs to one bound point:
`--checkpoint FILE --resume` accepts a worker-count change at that point and
refuses rebinding the checkpoint to a different point. Generation `--resume`
reuses the original symbolic source and preparation settings.

This route supports the native regular-family certificates available for these
single-chamber inputs. Runtime model masses, represented runtime values,
unsupported projective covers and general physical double-box resolution remain
outside this capability. Free atoms in direct input do not declare runtime roles;
use `direct.runtime_parameters` and explicit kinematic constraints.
