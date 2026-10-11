# Exact immutable constants in selected-root evaluators

Root-helper rational constants previously received a rounding-error allowance
unless they were small integers. That assigned artificial uncertainty to exact
binary fractions such as `3/4` and `31/128`, including the affine bracket offset
and width. It also attempted an unrepresentable uncertainty for an exactly
representable subnormal value.

The constructor now reuses the native rational-to-f64 conversion and exact
f64-to-rational conversion. A finite, identical round trip proves that an
immutable literal is exact in the supported domains with at least 53 bits.
Only those conversion errors become zero. Nonbinary rationals retain the
existing rounding allowance; values outside the admitted finite range do not
acquire an exactness claim. A separately called lower-precision Float is also
excluded from this proof.

Runtime coefficient and coordinate uncertainties never pass through this
conversion. Their real and imaginary errors remain present in callback inputs
and propagated outputs. This is not the removal of data error tracking or a
certified enclosure for uncertain inputs. Solver tolerances, stopping policy,
precision rescue and accepted-statistics rules are unchanged.

## Bounded diagnostic

An independently instrumented degree-six, three-cell family evaluated four
fixed coordinates in each of four charts, twice. Original and instrumented
results and acceptance decisions were identical. Whole-vector tracked error,
rather than inaccurate root centers or callback failure, triggered rescue.

For the affine recipe, exact immutable literals reduced the upper root's
reported absolute error from about `4e-13..9e-13` to `1.6e-14..3.7e-14`.
At the first point the largest Laurent error-to-tolerance ratio fell from
`2511.14` to `111.71`. The legacy recipe was unchanged. Both recipes still
rescued all 32 vectors, with unchanged final vectors, iterations and stopping
reasons. This cleanup does not establish a reduction in rescue frequency.

The uninstrumented warm second pass of 16 vectors took 0.174333 seconds in a
debug build. Native stage counters attributed 1.136 ms to primary evaluation,
63.955 ms to plain precision rescue and 108.356 ms to tracked conditioning.
These fixed-query diagnostics are not a representative sampling benchmark.
Remaining coefficient conditioning and iterative error correlation have not
been separated quantitatively.

## Reuse, reviews and gates

The API/source/probe audit reuses Symbolica's existing exact rational
conversion, already used by the original static bracket admission. No numeric
conversion, root solver, error estimator or CAS is reimplemented. Maintained
controls cover exact fractions, large exact powers, the smallest subnormal,
the largest finite f64, nonbinary values, overflow/underflow and low precision.
Saved legacy/affine root programs preserve aliases and incoming real/imaginary
uncertainty after restoration.

The independent producer reviewed the two-file change and accepted its limited
scientific contract. Eight focused native groups and strict core-test lint
pass in the private coherent public-dependency build. The coordinating agent
verified 21 evidence pins, 592 source pins and exact source pre/postimages.
The adopted-dependency kernel gate passes all 14 enabled groups in 51.30
seconds, including native uncertainty controls, full quintic Laurent-vector
oracles and fresh-process restoration. The separate child entrypoint is marked
ignored for direct harness execution and is exercised by its parent test.
Strict workspace/all-target Clippy passes in 20.17 seconds, followed by
formatting and diff checks. These gates do not rerun the full physical campaign.

Frozen cleanup handoff:
`ec427b74e5dafbdd36abef694695d5fef43965807eeceb888542ac2e359a35fc`.
The separate bounded diagnostic handoff is
`81bbbc9c00c800a11c1bdfa6aa9370d1b0dcacbee000eb7297a92c52149c8ed4`;
the coordinator also verified its 68 evidence pins and 1,184 private source
pins. Raw traces and build products remain untracked.

The change stays in FastSecDec's native root callback, with no HEPKit/community
implementation, executor, random stream or artifact-format change. It does not
complete moving-bracket support, collision resolution, general algebraic
generation or the physical performance campaign.
