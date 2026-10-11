# Source-bound endpoint transport

`threshold::maps::endpoint_transport::EndpointTransport` pulls an admitted
normalized open-cell map through the existing native sector-decomposition fan.
The caller chooses lower or upper unit faces. Source-coordinate, CAD-coordinate
and root roles are joined by symbol, rather than by an assumed common ordering.
Every original term and factor keeps its source index, exponent and authority.

The operation clears rational geometry with Symbolica's native rational
polynomials, checks exact reconstruction, and requires a complete denominator
inventory of nonzero rational scalars and unit/complement powers. It feeds native
polynomial supports to `fastsecdec_sectors::decompose`, validates the complete
fan, and checks its valuations against native `degree_bounds` minima and exact
quotients. It shares the ordinary generator's coordinate construction and
scaled-exponent accumulation. The measure is the pulled parent measure times
the absolute sector Jacobian, once. Factors used only to normalize a root
equation never enter the physical density.

An admission-only inverse-slot preflight avoids counting a common ray
denominator repeatedly in resource estimates. Symbolica collects the scratch
polynomial and supplies the inverse degrees; checked conservative bounds are
applied before conversion of the unchanged original expression. Exact clearing
and reconstruction still follow. No polynomial arithmetic or rational
simplifier is replaced. The general meromorphic preflight is unchanged.

Zero-exponent declarations retain `Inactive` links, including factors that
become zero on the projective anchor. Polynomial numerators retain lazy links
to their original bodies. Fresh chart symbols exclude every source, root and
coefficient-chart role. The geometric sufficient class here has rational
coefficients; a symbolic coefficient outside that class receives a diagnostic.

Five focused transport groups and seven ordinary mapping groups pass. The
existing fixed-width overflow diagnostic remains explicitly ignored. Strict
native, default-feature and test-source checks pass. Independent review
`7afd0c66…` covers the scientific source; the final two changes only avoid an
allocating scalar comparison and use iterator enumeration in a test. The
coordinator verified all seven source postimages and 49 evidence-file digests
before import. Final source inventory: `6ba6cc1a…`.

This is an exact map and exponent transport operation. Every closed face and
compatible intersection remains pending. It does not license evaluating a
numerator at infinity and multiplying by zero, certify a convergence strip,
perform Laurent continuation, or issue an integration kernel. The known lower
U=F=0 strata and unresolved finite F faces remain to be resolved. The combined
physical 156-term density/transport check is a separate acceptance gate, now
passing on four actual charts. It checks all 156 term/power records and the
additional Jacobian with the original density. Independent review exactly
replays 6,864 term, 44 sum and 154 directed coordinate/Jacobian inequalities;
see the [open-density review](no-deformation-open-density.md). The consuming
workspace/all-target strict threshold-feature check and formatting also pass.

The library owns no worker pool, sampling streams or integration loop. It retains
the existing HEPKit source and shared native proof owner. CLI preparation,
detached artifacts, HEPKit exposure and bounded cross-process residency remain
separate delivery work. No dependency patch is required for this operation.
