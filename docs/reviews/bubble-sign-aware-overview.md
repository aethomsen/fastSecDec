# Bubble sign-aware confirmation

The existing complete massive bubble (mass squared three, s=16, D=4-2 epsilon,
unchanged gamma-normalized measure) now includes the sign-aware prescription.
Frozen release `6647950` freshly generates one symbolic-Jacobian sector in
0.8645 seconds with 50.8 MB aggregate RSS. The complete pole/finite complex
vector is retained. Generation and numerical settings are in
`../bubble-sign-aware-overview.json`.

Independent 600-second QMC and ordinary discrete-MC confirmations use seeds
202610128121/202610128122, separate from calibration. Native drain, 25 workers,
serial QMC, the authorized ordinary-MC exception, optional causal validation
off and mandatory numerical stability handling match the campaign contract.
Both have complete coverage, zero numerical failures and full native covariance.
All eight marginal checks against the retained independent analytic reference
pass; maximum discrepancy is 1.20238 combined SE. The tiny QMC replica SE is
at floating-point roundoff and excludes common numerical bias.

Root review rehashed all 49 handoff pins and checked source association,
continuation settings, native covariance projection, C/max statistics, exact
logical/accepted counts and all normalization mappings. Two earlier supervisor
schema errors occurred before cold inspection and sampling; they are retained
in the private evidence and are not numeric retries.

Review receipt:
`target/no-deformation-bubble-sign-aware-6647950/root-outcome-review-r1/review.json`,
SHA-256 `915a69d283e9f335fedfedc168a96c6897fc0b5198c82a930f7b05525c3aab04`.
No native implementation or ecosystem dependency changes accompany this report.
