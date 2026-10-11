# Caller-selected native certificate budgets

The physical first-cell refinement has eight leaves with no identically zero
factor-face residual. Whole-box native margin proofs succeed for their U,
denominator and measure factors; the attempted F proofs remain incomplete.
The F Bernstein tensor sizes range from 13,608 to 122,472 coefficients. A
bounded miss is not a zero or a counterexample on the selected algebraic branch.

## API, source and executable audit

FastSecDec reuses symGCAD's `CertificateOptions`, `certify_nonzero` and
`verify_certificate`. On both the adopted `a1132d4` and upstream `053b3e9`,
`domain.rs` has identical hard validation ceilings of 10 million work units
and 65,536 Bernstein coefficients. A focused native constant-polynomial probe
confirms that requests above those ceilings are rejected before arithmetic.
Larger external time and memory allowances alone cannot change that behavior.

[symGCAD PR #2](https://github.com/alphal00p/symGCAD/pull/2), authored by
ValentinHirschi, raises only the opt-in ceilings to one billion work units and
262,144 coefficients. The existing defaults remain one million and 4,096.
Discovery stores the selected limits, and independent replay validates and
enforces them. Checked degree-plus-one arithmetic precedes checked tensor-size
multiplication, including on 32-bit targets. No positivity algorithm, CAS,
root solver or numerical approximation is added.

## Validation and scope

All 34 native controls pass: 31 inherited domain tests, two new budget/replay
tests, and an allocation-free overflow regression. The positive polynomial
`2-(x0*x1*...*x8)^3` requires an actual 4^9 Bernstein tensor. The test checks
discovery, independent and serialized replay, and refusal with insufficient
work or coefficient storage. Tampered replay limits and invalid options are
rejected. Independent source, allocation and replay review passes.

Owned-source formatting, diff checks and focused Clippy pass. Unqualified
upstream strict Clippy reports 50 inherited library findings and one inherited
test finding on unchanged lines; whole-repository formatting also has three
unrelated baseline files. Those files were left unchanged. Focused lint uses
explicit command-line allowances for the recorded inherited categories.

The PR head is `45246b7b27a5c21293b829458470cd34b03f199e`, tested on upstream
`053b3e9` with its own unchanged dependency graph. A separate isolated carry,
`ae778874ed2eb2fab184f8b49d5bf0b04bbaa986`, applies exactly the three-file patch
to FastSecDec's adopted `a1132d4`. Its physical probe and consuming validation
are separate work; FastSecDec's dependency declaration is unchanged here.
No physical completion or performance improvement is claimed by the owner tests.

Structural arithmetic limits do not bound rational coefficient height or
replace caller-owned process time and aggregate-memory supervision. Subsequent
physical measurements must record the amended dependency identity and retain
the earlier limited attempts unchanged.
