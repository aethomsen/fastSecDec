# Latest symGCAD adoption

The requested reference checkout was fast-forwarded to upstream `f1f6fb6`.
The public consumer revision `644a560e69a064f811fa93aff33a247ea7fcdfca`
adds the existing public dependency declarations and the three-file
[caller-selected certificate-budget patch](no-deformation-certificate-budgets.md)
to that upstream revision. The other agent's worktree and branch were not
rewritten. No new geometry algorithm is introduced by this carry.

The consumer continues to own the single Symbolica/Numerica `ae0c82b7`
identity. symGCAD's registry dependency is resolved by the FastSecDec root;
its standalone patch table is not inherited. Native owner tests were linked
against that exact consumer identity before adoption: 211 unit tests, 31 domain
tests, two budget/replay tests, 15 monotone tests, seven conditional-monotone
tests and four cache-admission tests pass. One inherited unit test is ignored.

Independent review verified the source and native dependency identities,
defaulted/omitted configuration additions, cache limits, and proof replay.
Historical absent fields retain their documented defaults. Benchmark settings
will be frozen explicitly; increasing cache admission is an optimization choice,
not proof authority. Old physical solves, proof receipts and timings retain
their original dependency identities.

The first consuming build found one source incompatibility: a former public
`Algebra` struct literal now lacks a private owner-policy field. The adapter
needs only univariate conversion of an already checked single-variable map.
It now calls Symbolica's existing `to_univariate_from_univariate` directly,
retaining the wrapper's explicit `u16` degree limit. No symbol is renamed and
no factorization policy or new polynomial implementation is introduced.
The initial failure and independent review amendment are retained.

Root and Python-consumer lockfile updates change only symGCAD's Git identity.
The consuming dependency tree has one Symbolica and one Numerica owner.
Workspace/all-target strict Clippy passes after the narrow adapter correction
(89 seconds).
The consuming native tests also pass: 12 regularization tests exercise
moving fibers, closed units, continuation, phases and the complete bubble;
eight GCAD groups cover identity, parameter binding, source association,
incomplete/tampered proofs and domain admission. Formatting and the locked
Python-consumer metadata check pass. The first release build completes in
7 minutes 24 seconds; build time is excluded from integral-generation metrics.

The original incomplete physical factor proofs and the later successful
margin proofs are reported separately. A successful GCAD solve or factor
certificate is not complete endpoint preparation or an integration benchmark.
