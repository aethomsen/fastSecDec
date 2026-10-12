# Physical double-box sign-aware confirmation

The full physical incoming-++ D05 source at the existing exact kinematics is
unchanged, including its numerator and prefactors. Frozen release `6647950`
generates 30 sectors in 1536.103 seconds, with 36.669 GB sampled aggregate RSS
and 99,710,967 artifact bytes. Six serial generation workers under 25-core
affinity are the documented memory exception. Symbolic IBP endpoints,
symbolic Jacobian, Horner10 and SymJIT O2 remain enabled.

The predeclared dynamic sign-aware settings S=0.8, L=R=1 were not tuned to the
confirmation. Seeds 202610127121 and 202610127122 are independent of each
other and their 30-second pilots. Both samplers use 25 cores; QMC has one-second
serial residence and discrete MC uses its authorized ordinary scheduler.
Optional causal validation is off; stability handling and the 100 GB campaign
supervisor remain active.

QMC drains at 600.135 seconds with 331,702,272 logical calls and 306,839,552
accepted calls. MC drains at 603.712 seconds with 43,585,536 logical calls and
43,487,232 accepted calls. Full coverage and valid committed covariance are
required even when a larger planned fixed-design allocation remains incomplete.
Neither stop is an accuracy success. The normalized radial relative errors are
0.7078 and 0.6797, both with native norm-linearization warnings.

Independent review checks full four-component vectors and 16 covariance
entries, the off-diagonal contribution to radial uncertainty, counters,
absolute/maximum weights, exact source association, native convention,
pilot-to-production settings, deadline/drain and report formulas. All 32
marginal comparisons with the published fixed/polynomial arms pass the stated
eight-combined-SE diagnostic, with maximum 2.64241 combined SE. No covariance
or observations are pooled. This is statistical compatibility of noisy results,
not proof of accuracy or of a variance advantage.

The coordinator also applied the unchanged native `benchmark_compatibility`
example to all six current physical confirmations. Numerica's exact-rational
matrix solve retains each complex covariance, without diagonalization or error
inflation. Across 30 pair/order comparisons the maximum descriptive Mahalanobis
distance is 3.778 in the pole coefficient and 2.547 in the finite coefficient.
All summed covariances are positive definite; none trigger the helper's
rounding-sensitive flag. These are estimated-covariance diagnostics, not
calibrated p-values, and the larger pole difference remains recorded.

The compact record is `../physical-double-box-sign-aware-overview.json`.
It retains exact timings, memory, weights, full covariance, seeds and settings.
The secondary six-million-per-sector display is only the requested square-root
normalization, not a separate measured matched-work run.

Independent receipt:
`target/no-deformation-double-box-sign-aware-6647950/physical-collector-r1/runtime-outcome-review-r1.json`,
SHA-256 `281c91ab8a4781056f0f8a5b29809bb77d7f237a7eaef3b8ded83eabb7d01e4f`.
Raw measurements stay untracked. This report introduces no CAS, graph, sampler
or arithmetic implementation.
