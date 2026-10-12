# Massive-sunrise reference controls

The native equal-mass sunrise at mass one and external square sixteen has
independently compatible fixed-contour, dynamic-contour and pySecDec results.
These are short scientific controls for the ongoing algebraic endpoint work.
They are **not a completed no-contour result or ten-minute benchmark**. The
massless sunrise in the Overview remains a distinct topology/point entry.

The [portable input](../../examples/no_deformation/benchmarks/controls/massive_sunrise.toml)
uses the existing native graph and scalar model, unit propagator powers,
numerator and measure multiplier, and dimension `4-2*eps`. Both tools independently
produce `U=x0*x1+x0*x2+x1*x2` and
`F=(x0+x1+x2)*U-16*x0*x1*x2`, with the complete prefactor
`-Gamma(2*eps-1) U^(3*eps-3) (F-i0)^(1-2*eps)`.
The Gamma factor and regulator-dependent causal phase are retained before
Laurent expansion. The imaginary finite part cannot be recovered by taking an
absolute denominator alone.

| Method | Finite coefficient | SE real | SE imaginary |
|---|---|---:|---:|
| Fixed, lambda 0.05 | `10.01614303245649 - 3.30898813687998 i` | `1.20e-10` | `1.16e-10` |
| Dynamic sign-aware, S=0.8, L=R=1 | `10.01614302845062 - 3.30898813141287 i` | `1.44e-8` | `9.47e-9` |
| Independent pySecDec 1.6.6 | `10.01614303229401 - 3.30898813685439 i` | `5.77e-11` | `6.27e-11` |

All twelve real/imaginary comparisons to pySecDec, covering powers -2, -1 and
zero, pass the declared eight-combined-SE criterion, including when its `2e-9`
numerical floor is removed. The largest discrepancy is 2.162 combined SE.
The pole coefficients are compatible with `1.5/eps^2` and
`-1.23164699470456/eps`, with vanishing imaginary poles. FastSecDec retains its
full six-component covariance; pySecDec provides marginal uncertainties, so
no reference cross-covariance is inferred. Replica SE does not measure common
floating-point bias.

## Reproduction and timing scope

The measured FastSecDec executable has SHA-256
`6c570eb9338741e3f3d2ebce5440cc27319eb7a7eeb8d65fcf7ee28d73134463`.
Endpoint reduction and both deformation Jacobians are symbolic; evaluator
settings retain Horner ten and portable SymJIT O2. From the repository root:

```sh
fastsecdec generate examples/no_deformation/benchmarks/controls/massive_sunrise.toml \
  --recipe fixed-v1 --serial --workers 25 --output target/sunrise-fixed.fsd
fastsecdec integrate target/sunrise-fixed.fsd --method qmc --serial 0.01 \
  --workers 25 --points 8192 --shifts 16 --max-rounds 1 --max-seconds 60 \
  --double-points false --target-order 0 --seed 202610120801 \
  --contour fixed --lambda 0.05 --contour-validation pilot \
  --contour-pilot-points 64 --contour-diagnostics disabled
```

For the second arm, generate with `--recipe dynamic-sign-aware-v1` and a
separate output. Integrate with seed `202610120802`, replacing the fixed-mode
arguments by `--contour dynamical=0.8 --lambda-cap 1 --displacement-cap 1
--contour-construction sign_aware`. All other settings remain identical.
Each accepted allocation contains 16 independent replicas of 8,192 points in
one symmetry-representative kernel. The complete source multiplicity is retained.
These runs end at the work limit; the small errors are observations, not a
claim that a requested tolerance was reached.

Native generation took 1.358 and 1.355 seconds respectively; native integration
took 1.181 and 1.992 seconds. These short runs include the explicit 64-point
causal pilots; production causal checking is disabled afterward. They therefore
do not enter the Overview's unchecked-production timing columns. The observed
dynamic error is larger for these particular untuned settings; this is not an
optimization result or a general variance comparison.

The independent reference uses `LoopIntegralFromPropagators` with
`k1**2-1`, `k2**2-1`, `(p-k1-k2)**2-1` and `p**2=16`, unit powers and
`additional_prefactor=1`. `loop_package` uses iterative decomposition and
contour deformation. Its QMC settings are `korobov3`, `fitfunction='none'`,
`generatingvectors='cbcpt_dn1_100'`, `minn=8311`, `minm=16`, `maxeval=2000000`,
`seed=202610120803`, `cputhreads=25`, `epsrel=1e-7`, `epsabs=1e-9`,
`presamples=10000`, `deformation_parameters_maximum=0.05`, and `together=True`.
The physical returned tuple includes the native Gamma prefactor exactly once.
Generation, compilation and integration completed within eight seconds and
about 1.2 GB own-process-tree peak RSS, below the ten-minute/15-GB reference
budget. These clocks are not a cross-tool performance comparison.

The reference wrapper failed only while JSON-encoding dictionary tuple keys
after the completed calculation. Its previously saved raw native series was
converted offline with pySecDec's existing `series_to_json`, without resampling.
Both that original failure and the recovered native result remain in the evidence.

The immutable private handoff is
`target/no-deformation-massive-sunrise-contour-controls/handoff.json`, SHA-256
`85e5c9fbf7cb5d28b415525a646d54e9ee9f0cac9a0808836b718a36348d9440`.
Its independent source, normalization and exact-rational comparison review is
`target/no-deformation-massive-sunrise-controls-runtime-review/review.json`,
SHA-256 `ab50d778db3d8f55ff81eaa83849fdc9577bccafc1a0ee53f8ea8972d02ba5ee`.
Raw outputs, binaries and build products remain untracked.
