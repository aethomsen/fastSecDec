# Finite-coefficient Overview campaign

These are preparatory inputs for the new 25-worker campaign. No new benchmark
has run from this directory. The source/dependency/executable freeze and native
input admission are required before production. See
[the protocol](../../../../docs/benchmark-overview-protocol.md) and
[fixture inventory](inventory.json).

All cards generate through ε⁰. Contour recipes are explicit generation overrides;
the base cards do not silently select a contour strength. Each accepted recipe
must get a separately frozen integration overlay and causal admission. A possible
no-contour generation refusal is an outcome, not permission to bypass endpoint
certificates. The kite uses five unit masses and exact p²=16. The double-box
cards and remaining native source checks are listed in the inventory.

Run generation and integration separately with the frozen release executable:

```sh
fastsecdec generate examples/no_deformation/benchmarks/overview/bubble.toml \
  --serial --workers 25 --recipe fixed-v1 \
  --contour-jacobian symbolic -o target/overview/bubble-fixed.fsd

fastsecdec generate examples/no_deformation/benchmarks/overview/bubble.toml \
  --serial --workers 25 --threshold-decomposition \
  -o target/overview/bubble-no-contour.fsd

fastsecdec integrate target/overview/bubble-fixed.fsd \
  --serial 1 --workers 25 --method qmc --target-order 0 \
  --integration-settings FROZEN_QMC_OVERLAY.toml \
  --seed FROZEN_SEED --save-result target/overview/bubble-fixed-qmc.json

fastsecdec integrate target/overview/bubble-fixed.fsd \
  --workers 25 --method discrete_mc --target-order 0 \
  --integration-settings FROZEN_MC_OVERLAY.toml \
  --seed FROZEN_SEED --save-result target/overview/bubble-fixed-mc.json
```

The one-second residence above is a proposed value, subject to the campaign
freeze. It is not a one-second or ten-minute total deadline. Ordinary discrete
MC is the explicitly permitted scheduling exception; it cannot use `--serial`.
The observer, exact work allocations, ten-minute stop and resource supervision
must be frozen before these templates become campaign commands.

`sunrise.dot` is the maintained native sunset topology with its tensor numerator
replaced by one, matching the existing Rust fixture's native `with_numerator(1)`
operation. The new card still requires a native U/F/prefactor equality check
against that maintained fixture. The triangle/box kinematics and all one-loop
measure multipliers likewise match the existing native reference fixtures; path
and TOML parsing alone are not scientific admission.
