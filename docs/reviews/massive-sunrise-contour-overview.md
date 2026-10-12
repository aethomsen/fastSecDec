# Complete massive-sunrise method comparison

Six independent 600-second contour confirmations now complement the two
published no-contour runs. Every arm uses frozen release `6647950`, exact
masses one, s=16, D=4-2epsilon and native measure one. Complete Laurent vectors
through epsilon zero and their native covariance are retained. Fresh serial
generation uses 25 workers, symbolic IBP endpoints, symbolic contour Jacobians,
Horner10 and portable SymJIT O2. QMC is serial with one-second residence;
ordinary discrete MC is the authorized exception. Optional causal validation
is off; native precision handling remains enabled.

The fixed strength is 0.1. Both dynamic recipes use S=0.8, L=R=1. All settings
precede independent calibration and confirmation; pilots are not pooled. The
actual native clocks are 600.197–604.078 seconds, with complete deadline drain,
zero numerical failures and available uncertainty. Confirmations use seeds
202610126121/122, 202610126221/222 and 202610126321/322. No previous short run is
relabeled as a ten-minute observation.

The generation, deterministic raw/checked costs, artifact sizes, all C/max
diagnostics, total and accepted call counts, full numerical results and exact
effective settings are in `../massive-sunrise-contour-overview.json`. The
secondary display target is 20 million / 62 evaluations **per sector** for all
methods. Thus the one-kernel contour arms use that total target, while the
62-kernel no-contour arm uses 20 million. These square-root work rescalings are
heuristic and not QMC convergence laws. The near-ten-minute radial norm error
is the primary metric.

Independent coordinator review rehashes 88 supplied receipts, validates the
full native settings against each pilot, checks covariance/report projection,
complete counters, optional-validation policy and normalization formulas. The
producer's 36 exact unpadded marginal comparisons against the retained
independent pySecDec reference pass, maximum 2.0293 combined standard errors.
The reference does not provide cross-component covariance; none is invented.

The unmodified native `benchmark_compatibility` example additionally checks
all eight campaign arms over all three complex Laurent blocks, using existing
Numerica exact Rational conversion and Matrix determinant/solve: 84 comparisons
with maximum defined Mahalanobis distance 2.6114. Five comparisons are
rounding-sensitive and two covariance blocks are not positive definite. Those
flags remain explicit; no covariance regularization, error inflation or
calibrated significance claim is made. Replica SE does not bound shared
floating-point bias.

The old local helper binary refused the new `conditioning_timing` field.
Instead of editing saved results, the exact same checked-in helper source was
linked against the coherent reviewed current native library and then passed.
No replacement matrix algorithm, CAS, graph input or numerical sampler was
introduced for this review.

Private evidence directory:
`target/no-deformation-massive-sunrise-contour-6647950/`.
Final campaign handoff SHA-256:
`15989fcdcb8ed934bed85613ca9608b74e387c421acbdde35a14e6dfb35de253`.
Independent coordinator review SHA-256:
`7b0489ff9cc8fe533982f201aeb0a885e836f43edb1eab4f49654bfd17908ba7`.
Raw runs and native helper binaries remain untracked.
