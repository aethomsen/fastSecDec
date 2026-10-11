# Exact replacement covers for endpoint charts

`EndpointChartCover` replaces one existing endpoint chart with a complete
Cartesian half-cube partition. Selected members can be further partitioned by
the existing native sector decomposition. Other original charts remain
unchanged. Adding the replacement to its original chart would double-count
that region; this operation exposes the parent and replacement relationship.

All halves are generated internally. Caller-selected axes and refinement masks
are checked, and the constructor uses one cancellation/resource context.
Artificial midpoint boundaries remain distinct from original lower/upper
faces. Native monomial maps retain their boundary preimages rather than
claiming a one-to-one correspondence between their coordinate faces.

The complete original cleared numerator supplies both the native support and
the subsequent exact valuation comparison. Original factor numerators and
denominators, source coordinates and root equations are pulled through the
composed map. Original factor links and all term powers are reconstructed;
the extra measure is the half width times the absolute native determinant,
multiplied by the original chart measure exactly once. Polynomial numerators
remain lazy and the monomial removed from a root equation never enters the
physical density.

Three new native test groups pass. They check complete replacement and exact
factor/measure recombination, subset axes in a nontrivial order, unchanged
unselected coordinates, boundary ancestry, wrong owners and resource/cancel
refusals. The original five transport, ten numerator and seven normalized-map
groups also pass. Whole-term rational-power reconstruction is compared at
exact rational points after clearing exponent denominators, alongside the
symbolic identities for each factor and the composed measure. These checks
do not assert a principal-power identity across arbitrary complex branches.
Private strict library/test checks and the consuming workspace/all-target
threshold Clippy gate pass with warnings denied. Formatting and diff checks
also pass. These gates do not reload the complete physical GCAD proof.

The sealed `IssuedEndpointChart` view retains the original transport or a
replacement-cover owner. A selected root's coefficient images can be composed
only when that exact section belongs to the retained owner. Ancestor unit
images extend polynomially from the closed cube to itself. This property does
not apply to physical source-coordinate images, which may contain poles from
compactified infinite intervals.

The implementation reuses the existing sector fan, Symbolica polynomial
operations and the ordinary generator's coordinate and exponent helpers.
Independent source and ecosystem reviews check exact coverage, measure,
source associations and branch ownership. This operation issues no closed-face
unit, subtraction, Laurent or integration certificate. Subsequent consumers
must prove the regularity of the actual physical density.
