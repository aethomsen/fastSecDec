# Restricting one global monomial selection

This successor to [admissible center descent](no-deformation-center-descent.md)
adds selection authority for the monomial terminal-invariant case on one actual
ambient. It reuses the existing native monomial producer once, then restricts
that choice to every open in the retained cover.

The original source, history, ordered divisor ledger and selection event are
immutable owners. Local ideal presentations can change only through checked
two-way ideal equality. The selected cover requires every local receipt to
belong to the same event before using the existing complete overlap checker.
This prevents an independently recomputed local maximum from being substituted
for a restriction of the global center.

## Empty selection is not completion

For `(y^2*z^3,2)` with ordered boundaries `y,z`, the native whole-ambient
producer selects `y`. On `D(y)` that selected center is empty, even though the
local marked cosupport contains `z=0`. Recomputing a maximum on that open would
select `z` and change the global step.

The restriction therefore distinguishes four states: a selected center, an
empty selected center, an empty marked cosupport, and an empty ambient open.
Empty cosupport requires a fresh native proof for the pulled original source.
An empty selection alone never certifies a resolved physical factor or a zero
contribution.

This slice creates no physical blow-up or identity map, no exceptional birth,
and no cycle reset. The future execution adapter must preserve those
distinctions when advancing inactive regions.

## Native reuse and controls

Selection uses the existing checked monomial witness, exhaustive SNC incidence
inventory and ordered choice. Restrictions use the actual native localization
maps, marked-cosupport operations and ideal membership. Pair checks reuse the
preceding common-frame and prepared-membership operations. There is no alternate
CAS, graph, numerical integration loop or random stream.

Four control groups cover the global/local counterexample, source/history/event
transplants, missing or duplicate cover entries, changed ideal presentations,
unit and empty cases, resource retry, reordered work and owner release. The
coordinator and an independent producer accepted the limited source and
scientific contract. The seven-file frozen handoff has SHA-256
`28720a039c93b2352bc7ade815d948bab933aea8c654e408d446f2234404f304`.
The coordinator verified its 43 evidence pins, all source pre/postimages and
156 snapshot files before importing the exact source postimages.

The private native diagnostic graph passes all 135 resolver controls in
255.616 seconds, including the four new groups. Strict library/test lint and
the external public API probe pass. The adopted-dependency combined gate with
the principal-power slice passes all 142 resolver controls in 286.07 seconds,
strict workspace/all-target Clippy in 35.69 seconds, formatting and diff checks.
These are debug scientific checks, not generation or sampling benchmarks.

The monomial selection is not the complete nonmonomial `(inv,J)` construction.
Functorial coefficient selection, complete invariant strata, the physical
execution adapter, general overlap gluing and the real integration atlas remain
open. Caller budgets apply to preparation and proof inventories; native algebra
still needs process limits. Acyclic owner release is not a measured RSS bound.
