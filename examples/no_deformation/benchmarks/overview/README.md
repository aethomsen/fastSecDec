# Finite-coefficient Overview campaign

These are the inputs used by the current 25-core integration campaign. Exact
executables, generation recipes, work budgets and confirmation seeds are pinned
in its receipts. Completed results and explicit missing or refused routes appear
in the Overview; ongoing runs are not measurements. See
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

The one-second residence above is the campaign's QMC setting. It is not a
one-second or ten-minute total deadline. Ordinary discrete
MC is the explicitly permitted scheduling exception; it cannot use `--serial`.
The frozen caller observes the native integration clock and requests one graceful
stop at 600 seconds. Actual elapsed time, completed work and any drain are
retained. The commands above remain templates: use each row's recorded options
and seeds to reproduce a specific run.

All table sampling uses explicit `--contour-validation off`. Native stability
checks and precision rescue remain active. Separate causal admission pilots are
scientific controls whose times never enter table sampling costs or convergence
windows. The double-box fixed setting is `--lambda 0.000001`; the earlier scalar
`0.1` attempt failed its independent causal check and is excluded. Physical
numerator generation uses six resident workers under the 25-core affinity
allowance, as recorded explicitly; integration uses 25 workers.

`sunrise.dot` is the maintained native sunset topology with its tensor numerator
replaced by one, matching the existing Rust fixture's native `with_numerator(1)`
operation. Its retained native export confirms the scalar U/F and unit numerator
and measure. The triangle/box kinematics and one-loop measure multipliers match
the maintained native reference fixtures. The reference ledger distinguishes
these checks from independent integrated values: no applicable maintained
reference is available for the new all-massive kite or either exact-point
double-box integral.
