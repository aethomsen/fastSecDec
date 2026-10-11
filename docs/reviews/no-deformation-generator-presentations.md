# Exact generator presentations for component discovery

Component discovery now reuses the existing quotient-normalization operation
to remove redundant generators before native elimination. Each proposed
removal and the final presentation must generate exactly the original ideal
in its original localized quotient. The original source, mark, quotient
equations and inverse relations remain attached to the receipt.

The shared helper is extracted from the previously reviewed center-presentation
path. It uses Symbolica's native polynomial reduction and Groebner operations
through the existing `QuotientNormalizer`; it adds no algebra implementation.
The center adapter retains its original checked center and its nonempty and
normal-count requirements. Subsequent adaptation still checks rank, boundary
transversality and the complete pivot cover. A shorter presentation does not
certify a minimal generating set, a radical ideal, a regular parameter system
or a maximum of the resolution invariant.

Annihilator and idempotent discovery use the equivalent shorter generating
lists. Their verification still checks products against every original source
generator, all complementary products, idempotency, the complementary sum,
complete ambient opens and boundary division quotients. A batch-local
`PreparedLocalZero` borrows the actual immutable quotient and reuses its native
membership basis. It retains support, cancellation and resource checks on each
query; no process-global cache or worker ownership is added.

The motivating three-variable diagnostic reduced one idempotent elimination
from 30 variables and 86 terms to 14 variables and 40 terms. Its selected
recursive branch then completed within the existing budget. Another branch
remains incomplete. These local diagnostics establish a useful reduction in
the problem size, not an end-to-end generation speedup or a completed general
resolver. The separate experiment replacing discovery relations by another
basis is excluded from this change.

## Verification and scope

Independent source, scientific and ecosystem review passes. The coordinator
checks all seven source pairs, 32 evidence records, 24 native dependency
artifacts and three test/lint binaries in handoff
`579c13b35fe388df071296f618e235fbdbd4ca447006c181e79ccd4c53f302b6`.
The frozen source inventory is
`c91561fc7c86830e36d309e602abb88d0f8eada23a865835a1e522e4756aed15`.

Against the adopted public Symbolica revision, the focused direct Rust build
passes two presentation tests, six component tests, nine contact/normal tests
and one original-history test. Strict library and test Clippy and formatting
checks pass. Controls include quotient-zero and nonconstant-unit ideals,
retained marks and source ownership, incorrect generators, foreign rings and
quotients, resource refusal, complete original product inventories and existing
normal-pivot admission. The earlier expensive all-pivot diagnostic remains
explicitly ignored, with its previous incomplete outcome recorded.

The combined consuming workspace passes the same eighteen resolver groups,
all twenty-nine generation/preparation groups and their invoked children.
Strict workspace/all-target Clippy (52.41 seconds), formatting and diff checks
pass with unchanged dependency revisions. The private timing observations
above are separate from this correctness gate.

This is a native resolver helper. It changes no public input format, artifact
codec, parameter chamber, evaluator, sampling stream or estimator. The global
termination invariant, singular real atlas and complete endpoint resolver
remain separate unfinished gates.
