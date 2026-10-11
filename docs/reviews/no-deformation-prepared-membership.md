# Scoped reuse of native ideal membership

Status: independently reviewed resolver optimization, 2026-10-11. This changes
verification work reuse, not the mathematical admission rules or the general
resolver's completion status.

## Operation and reuse audit

Profiling the next physical center for `z^2+x^3+y^3` identified repeated native
GrevLex basis construction and verification for the same immutable localized
ideal. One observed 29-variable, 18-equation presentation produces a 122-row
basis: native construction takes about 0.43 seconds, with the existing explicit
Buchberger and input-inclusion checks bringing the call to about 3.8 seconds.
Successive relative-derivative and commutator checks repeat that work.

Public API, source and executable probes confirm that Symbolica's existing
`GroebnerBasis::new(...).system` and native polynomial `reduce` already provide
the reusable basis and division operations. No algebra or reduction algorithm
is added. The inspected `1ac765f` CAS sources equal their adopted `ae0c82b`
counterparts; native F4's constructive source combinations remain the existing
reverse-inclusion trust boundary.

`PreparedMembership` borrows the exact ideal and quotient relations, combines
them once and lazily obtains the existing fully checked native basis. A direct
zero remainder against the original generators is still a valid membership
proof. A nonzero remainder does not decide nonmembership until the verified
basis is used. Failed construction never leaves a partially checked cache.

Each query retains variable-map and local-support admission, polynomial and
remainder limits, and operation accounting. Source and used-basis limits are
rechecked against a tightened caller budget. `PreparedLocalZero` binds the exact
localized owner; a basis from another guard or localization cannot be reused.
The one-off API delegates to the same helper. Explicit Lex elimination and
`native_basis` itself are unchanged.

Reuse lasts for one relative-frame verification batch (all relation-preserving
derivatives and commutators) or one graph-coordinate identity batch. There is
no process-global cache, persistent basis receipt, new public API, thread pool
or HEPKit object. All owned generators and the optional basis leave scope with
that batch. Lifetime tests establish object release; they do not establish that
the allocator returns RSS to the operating system. Native intermediate
allocations still require caller-owned process/resource limits.

## Validation and measured limitation

The standalone registration-shaped tree passes 117 tests in 150.780 seconds,
strict library/test Clippy in 13.446 seconds, and scoped formatting. Four new
groups exercise known membership/nonmembership, repeated basis use, tightened
budgets, failed-construction retry, foreign rings, incompatible localizations,
unsupported coordinates, remainder limits and owner lifetime.

On the same selected next-center diagnostic, the original path completes none
of its ten adapted graphs within 120 seconds. The complete-frame reuse path
finishes all ten adapted graphs within that cap and proceeds to standard-chart
verification. **Neither run completes the first center's full physical pivot
inventory.** A separate 600-second correctness run also stops incomplete during
standard-chart verification (600.040 seconds). These observations show progress
through one preparation stage, not a measured speedup for a completed integral
or proof construction. No timing or resource check is relaxed into acceptance.

The latter run observes a sampled process high-water value of 27,984 KiB. That
is neither a guaranteed final peak nor an aggregate residency bound. The very
short focused tests can finish between memory polls; their sampled values are
not used as peak-memory evidence.

The producer and root independently reviewed the helper, call sites and
regressions. The root verified 34 evidence digests and all 130 source files in
the tested snapshot, then applied the six-file registration patch. Its private
module declaration is combined with the separately reviewed physical-cycle
exports. The joined consumer run against adopted Symbolica `ae0c82b` passes all
120 resolver tests in 212.45 seconds; strict workspace/all-target Clippy passes
in 26.80 seconds. Workspace formatting and diff checks pass. Logs are retained
under `target/no-deformation-original-factor-joined/`. These are the focused
joined gates; no full physics-suite rerun is claimed.

Untracked evidence: `target/no-deformation-prepared-membership/`, handoff SHA-256
`2e50818861dc273766762a6a1b13a64a073b6ac3ceca8e86384216048487c915`,
patch SHA-256
`6b65684b1b7c7707e8c909eec5da74171137cfa35ff02f6091b7e436d46bfc00`.
Raw logs, diagnostic binaries and copied source trees are not published.
