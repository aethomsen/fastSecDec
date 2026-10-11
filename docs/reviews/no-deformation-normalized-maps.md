# Normalized selected roots and composed open cell maps

This slice constructs an executable open map for a verified positive-projective
cell. It retains the actual source, cell ordinal and original root selectors.
Each accepted prefix is a bijection onto the same original cell prefix; the
next section cannot borrow a selector from a different prefix or from a bounding
box alone. The native proof is shared through `Arc`, not copied per section.

The normalization proposal supplies a positive monomial coefficient chart, a
positive scale and a root bracket. Admission checks the native exponent-matrix
determinant, exact agreement with the preceding prefix, range certificates and
a rational witness inside the original cell. Native substitution and exact
division derive `P = M Q`. The minimum coordinate exponents come from Symbolica's
`degree_bounds`; `ldegree` instead describes the leading monomial. Independent
review caught that API misuse before publication, and a regression distinguishes
the two. The presentation monomial `M` is retained as provenance and is never
multiplied into the physical density.

The normalized equation has a certified derivative margin and endpoint signs
on its whole coefficient box. Original CAD delineation and a checked witness
associate its selected root with the original upper boundary. This permits a
regular normalized root even where the original face polynomial has colliding
roots. It does not transplant the original root ordinal to that face polynomial.
Current admission requires a constant finite lower bound and a positive
normalized upper section; other cases remain explicit refusals.

Execution reuses Symbolica's `EvaluatorComposer` and the existing version-three
algebraic callback. Later coefficients consume earlier root slots: in the
double-box map the second equation receives `h = v Z`, preserving that algebraic
dependence. The full physical numerator is not substituted or expanded. Source
coordinates are returned in preparation order, with the product of positive
CAD-order diagonals once. Saved native IR and callback scope preserve arithmetic
execution; they do not recreate geometric proof authority.

The evaluator owns its scratch space and retains its admitted source. It checks
strictly open unit inputs and valid positive diagonals. In particular, an outer
transform rounding a coordinate to one cannot evaluate a ray at infinity and
then fabricate a zero. A subsequent resolved density must supply the appropriate
analytic endpoint behavior.

The separate test oracle uses native exact real-root isolation for the first
root and directed `RealBall` residual bounds for the dependent second root.
It preserves the interval for `h = v Z`. A native high-precision root candidate
is only a candidate; the residual, certified derivative margin and original
bracket provide its enclosure. Ordinary uncertainty tracking is not described
as certification.

The first physical reference run exposed an invalid harness assumption that
`Float` must preserve its input bit count. Native subtraction deliberately
reduces its loosely tracked precision after cancellation: the isolated ray
`u/(1-u)` reports 127/255 bits from 128/256-bit inputs at `u=2/3` and `3/4`,
while satisfying the unchanged exact-reference tolerance. The physical oracle
therefore reports output precision and checks the certified numerical enclosure;
it does not demand an invariant the native type does not promise. No production
precision rule or owner-library implementation is changed.

The seven focused native groups pass, including the two-level selected-root
control, altered prefix/range/witness/symbol refusals, cancellation, the valuation
regression, symbolic first/second root derivatives, native IR/scope restoration,
128/256-bit evaluation and refreshed tracked inputs. Independent source reviews
`acdb6d25…` and `e5dffda3…` cover the valuation correction and runtime composition.
The consuming checkout passes strict workspace/all-target Clippy with the
threshold feature in 41.91 seconds, formatting and diff checks. The eleven
source postimages match the frozen inventory `c2f5e5de…`; the existing finite
mapper and its admission rules are unchanged.

A separate physical check freshly binds the original 156-term D05 source to
all 644,699 verified cells, then executes the complete six-coordinate map for
cell zero. This includes the two genuine nonlinear sections, three positive
rays and the original source-coordinate ordering. All 42 coordinate/Jacobian
comparisons pass at three rational points and 128/256-bit working precision.
The full original and prepared term ledgers, including monomial powers, remain
attached; no numerator is replaced by a scalar surrogate.

This correctness-build run takes 258.725 seconds under a 600-second/100-GB
allowance. Source preparation takes 3.493 seconds, proof decoding 128.675
seconds, binding/verification 120.548 seconds, map preparation 0.058 seconds
and the independent oracle 0.034 seconds. Sampled aggregate RSS peaks at
11.528 GB. The source postimages remain unchanged. These measurements neither
time a full physical integrand nor supplement the separate GCAD trial's stages.

Actual evidence receipt `76707ff0…` pins the source helper, preserved first
attempt and successful result. Outcome review `7b7d7209…` verifies the saved
association, complete term ledger and all numerical comparisons without another
proof load. The coordinator independently reads the oracle construction and
replays all 42 saved rational comparison inequalities. The source/scientific
review of the eleven final postimages is `9a638118…`.

No new CAS, root solver, AD implementation, graph object, codec or execution
pool is introduced. Existing HEPKit source objects remain authoritative and
community receives no substantive code. Symbolic endpoint reduction, causal
phases, full closed-face coverage and integration artifacts remain separate
required steps. This map slice does not complete physical double-box generation
or provide an integration time or standard deviation.
