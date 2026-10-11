# Exact rational-invariant ggHH input audit

Independent source, fixture and native gate review passes for input preparation.
The new point is the D05 incoming-`++` double box at
`s=1000000`, `t=-97077`, `u=-871673` GeV², with `mH=125` and
`mt=345/2` GeV. It is distinct from the previous `cos(theta)=4/5` point;
results or uncertainties at that previous point cannot be transferred to it.

The existing example `Point` helper adds `with_mandelstam_t`, using native
Numerica `Rational`, Symbolica `Atom`, HEPKit `FourMomentum`, diagram routing
and incoming helicity wavefunctions. Both constructors share the existing
Gram-data path. For `E=sqrt(s)/2`, the new constructor sets
`pz=(t-mH²+2E²)/sqrt(s)` and `px²=E²-mH²-pz²`, refusing a negative
transverse square and energies at or below the two-Higgs threshold. No new
graph representation, polarization implementation or algebra system is added.

The outgoing momentum is
`(500,sqrt(21093814799)/500,0,193649/500)`. Native checks establish all mass
shells, momentum conservation and the requested invariants. The independent
physical Gram entries are `p0·p1=500000`, `p0·p2=56351`,
`p1·p2=443649` and `p2²=15625`; they give `s+t+u=2mH²`.
All 15 physical/polarization Gram entries are retained. Binary floating-point
wavefunction components keep their existing exact rational transport and
approximate normalization; this is not an exact algebraic normalization claim.
Model inputs, projected numerator, loop measure, unnormalized `delta_ab`
projector and absence of spin/color averaging or diagram summation are unchanged.

An independent byte audit found the old 400 and 1000 GeV `run.toml` and
`point.toml` unchanged. Each old `point-exact.json` changes only 22 native
momentum-label occurrences, adding the already-adopted display attributes
`component-labels:subscript` and `tensor-label:p`. All physical values and
coordinate ordering remain unchanged. The sole `validation.json` change is
the source hash of `point.rs`. All three maintained fixture directories match
their final native exporter outputs. The new rational fixture's run card is
the existing parametric contour input, not a specialized threshold recipe.

The coherent native Cargo gate passed six maintained tests: two existing scalar
transport checks, reproduction of both old fixtures, the new rational point
and its refusal cases, and complete equality of both constructors at the
650 GeV rational-angle control `t=-39625`. Strict example Clippy, formatting
and diff checks passed. The original incorrect four-test harness expectation
and a subsequently corrected comparison-style lint failure remain in private
evidence; source-sensitive fixtures were regenerated after the lint correction.
The independent reviewer inspected these receipts without rerunning the native
campaign. The frozen review receipt is
`de577f82abc00892393cc6a6e87294a5c9ecb673e3eed98096d66d33e5305bcf`.
A final shared-example dead-code annotation changes no arithmetic. Its refreshed
fixture provenance and repeated six-test/strict gates pass the separate amendment
`029e0ad5696a5d9aea2f992fd07118b424f9548a1df6813fcebac96781233a6a`;
all physical fixture bytes remain unchanged from the preceding audit.

This establishes consistent external input only. It does not establish tensor
contraction, completed GCAD, endpoint regularization, contour admission,
numerical integration or an independent amplitude reference. A future matched
comparison must use this point in every arm with a fresh numerical checkpoint.
