# Physical factor obligations on relative faces

`FactorFacePortfolio` retains an issued original or refined endpoint chart and
enumerates all compatible nonempty assignments of its coordinates to lower,
upper or free faces. It derives the active singular factors and their signs
from the original preparation and verified cell. Shared factors keep every
original term/factor association; numerator bodies, prefactors and transported
monomial powers remain in their existing owners.

Symbolica performs exact polynomial face restriction. The existing bounded
native polynomial/domain adapter and symGCAD certificates prove positive
margins for the signed residual numerator and its denominator. Polynomial
continuity extends these bounds to the closed box of remaining coordinates
and retained selected-root brackets. Exact constant vertices use native
Rational comparison, avoiding a zero-variable polynomial iterator edge case.
Two visibility-only changes expose the existing adapter inside FastSecDec;
there is no new algebra or certificate engine.

The zero-variable iterator panic is reproduced on upstream Symbolica and has a
tested owner-library fix in [Symbolica PR #73](https://github.com/symbolica-dev/symbolica/pull/73),
authored by ValentinHirschi. Three new iterator tests and 57 existing polynomial
tests pass. The fix preserves zero rows for zero and one empty row for a nonzero
constant; its concrete iterator return type changes. Upstream strict lint debt
is reported in the PR. The review request tags `benruijl` in a comment because
GitHub denied the formal reviewer-assignment operation. FastSecDec's dependency
revision is unchanged, and its exact constant path does not require this patch.

The outcomes distinguish a signed unit, an exact zero residual with a proved
unit denominator, and an unresolved obligation. A zero denominator is never
treated as a removable limit. A failed margin proposal is not evidence that
no positive margin exists; a smaller configured fallback is also attempted
when applicable. Resource, cancellation and native errors return without
publishing a partial portfolio.

Inventory completeness is separate from factor regularity. Every factor row
must be a signed unit before the factor-face completion flag is true. A known
zero residual still requires refinement. Independent review caught and fixed
an earlier predicate that counted these zeros as completed factor work.
Even complete factor faces do not establish numerator/measure regularity,
a common regulator strip, subtraction or an integration kernel.

Original source/root owners and per-face proof boxes are shared. This native
preparation API consumes no sampling streams and owns no execution pool.
Geometry, source and selected-root associations remain available to HEPKit
through the native objects; no substantive code is added to community.

Four maintained test groups pass, covering genuine original/refined charts,
source association, negative causal signs, zero-only pending frontiers, exact
constant vertices, proof replay, denominator failures, cancellation, resource
limits and a smaller fallback margin. Independent source and scientific review
passes. The final registration-order adjustment changes formatting only.
The consuming workspace/all-target Clippy gate passes with warnings denied
in 41.77 seconds; formatting and diff checks also pass.

A separate compact scan of the actual first physical ggHH cell's four infinity
charts attempts all 5,824 U/F face rows: 3,719 positive-margin proofs, 176 exact
zero residuals with unit denominators and 1,929 attempted margin misses. The
misses contain 970 distinct residual pairs; domain and branch identities must
also agree before any proof is reused. This scan takes 25.07 seconds as a whole
process, with 198 MiB peak RSS, using a native certificate work bound of 10,000
and Bernstein depth two/node allowance 15 per query. It consumes previously
extracted exact factors, not a freshly reconstructed live physical owner, and
does not measure generation of an integration kernel or its sampling cost.
