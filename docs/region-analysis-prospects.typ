// Prospective research discussion. No region-analysis implementation claimed.
#let grad = math.op("grad")
= Prospect: certified expansion by regions

*The opportunity is a certified asymptotic expansion that also captures
interior cancellation regions.* A fast critical-locus solver could make its
geometric preparation practical. It would not alone prove completeness.
The desired output is a possibly redundant expansion to a requested order,
with certified coverage and remainder control. A minimal nonzero-region list
is a separate problem. The preceding scratch calculations are the only new
experimental evidence for this research proposal.

== Position relative to existing methods

The geometric method of regions associates scalings with lower faces of
Newton polytopes, following
#link("https://arxiv.org/abs/1011.4863")[Pak and Smirnov's geometric approach].
The #link("https://arxiv.org/abs/1206.0546")[asy2 work of Jantzen and the
Smirnovs] already automates the exposure of potential and Glauber regions
in its examples. #link("https://arxiv.org/abs/2108.10807")[Heinrich et al.,
Expansion by regions with pySecDec] describes its numerical implementation
and difficulties from cancellations in Minkowski kinematics. A region hidden
by such a cancellation may become a usual facet region after a suitable change
of variables. “Interior” here refers to the integration domain; it is not an
invariant classification by location inside the original exponent polytope.
The crown dissection in
#link("https://arxiv.org/abs/2407.13738")[Gardi, Herzog, Jones and Ma,
Dissecting polytopes] provides an explicit example.

#link("https://arxiv.org/html/2609.29442#S2.SS4")[Gardi's Hidden Region Finder
(HRF), September 2026] relates cancellation loci to local dissection and
asymptotic scaling. Its §2.4.3 proposes a scaling-first search over exposed
faces and positive Landau solutions, including boundary strata, as a theoretical
alternative rather than the procedure used for its negative audits.
The paper develops massless,
unit-numerator examples; massive propagators and numerator-dependent
contribution analysis are stated extensions.

#link("https://arxiv.org/html/2503.16601v2#S5")[Correia, Giroux and Mizera's
SOFIA] addresses a different problem: candidate kinematic singularities and
symbol alphabets via Baikov representations and Fubini reduction. Its future
directions distinguish singularity locations from local exponents and physical
sheets. A certified pinch-manifold and density analysis could complement these
outputs.

The homogeneous conditions $F=grad F=0$ with $U!=0$ concern finite-momentum,
first-type candidates. Second-type singularities involve loop momenta at
infinity and require additional strata. The critical equations for the
Lee--Pomeransky polynomial $G=U+F$ have a different interpretation; see
#link("https://arxiv.org/html/2311.14669")[Fevola, Mizera and Telen,
Landau Singularities Revisited]. A real-domain calculation for a specified
causal limit does not classify singularities on every complex sheet.

== The limiting family contains more information than its critical set

Let $rho arrow.r 0^+$ specify the kinematic limit. As a local illustration,
$
 F_2=(x-a)^2+rho, quad F_4=(x-a)^4+rho,
 quad x-a tilde rho^(1/2) " or " rho^(1/4),
$
respectively. Neither polynomial has a real stationary zero for $rho>0$.
Their limiting critical sets coincide, while their normal scalings differ.
Thus a solve at one finite kinematic point, or just the zero set at $rho=0$,
cannot determine the expansion. Vanishing orders, normal directions and
all competing orders in $rho$ matter. The numerator, Jacobian and powers of
$U$ also contribute to power counting.

A proposed construction would combine the following obligations:

1. Fix an exact algebraic family, its physical chamber, causal sheet and
   requested asymptotic order. Cover projective boundaries and infinity,
   including relevant $U=0$ strata and their intersections.
2. Reduce candidate leading equations with native Symbolica and use certified
   real solving on their admissible domains. Include scaling-selected leading
   polynomials: the full unscaled $F$ need not itself be stationary on an
   asymptotic cancellation locus. Ordinary boundary regions remain necessary.
3. Construct exact local charts around each accepted component. Retain
   multiplicities and tangent parameters; resolve degenerate normal geometry,
   collisions and nested strata until the family admits controlled local
   expansions. Record branches, Jacobians, chart coverage and transitions.
4. Extract scalings in the resolved coordinates and compose them back to the
   original variables. Include the regular complement. Use a partition, or
   explicit overlap terms when expanded regions are extended beyond their
   original domains, so contributions are not counted twice.
5. Bound the integrated remainder in a common convergent regulator domain,
   then justify the required analytic continuation. Retain factors such as
   $rho^(a+b epsilon)$ until the Laurent expansion; interchanging asymptotic
   and regulator limits without justification can lose logarithms.

This is a proposed certificate contract, not an existing general theorem or
implementation in FastSecDec. Complete equality-aware CAD can reduce work on
irrelevant ambient cells, as studied in
#link("https://arxiv.org/abs/1903.08999")[England, Bradford and Davenport,
CAD with equational constraints] and
#link("https://arxiv.org/abs/1401.0647")[Wilson et al., cylindrical algebraic
sub-decompositions]. Degeneracy, dependent equations and vanishing projection
polynomials require their hypotheses or a complete fallback. The current
symGCAD open-cell interface is not that equality solver.

== Where our present work could improve the practical method

The promising comparison is the cost of obtaining a verified regional
description, including failures and verification. It is not the cost of a
critical-point probe versus somebody else's complete integral calculation.
The following opportunities use actual native capabilities, but their benefits
for region analysis remain to be measured.

#block(breakable: false)[
#set text(size: 8.3pt)
#table(
  columns: (1.0fr, 1.65fr, 1.8fr), inset: 5pt,
  stroke: 0.35pt + rgb("cad5de"),
  fill: (x, y) => if y == 0 { rgb("e7eff5") } else if calc.even(y) { rgb("f5f8fa") },
  table.header([*Opportunity*], [*Present evidence*], [*Potential gain and remaining work*]),
  [Target the critical set],
  [Exact scalar exclusions; crown reduction to four free tangent parameters.],
  [Avoid partitioning every regular threshold sheet. General component discovery and completeness are still needed.],
  [Reject impossible candidates early],
  [Native nonvanishing certificates and an explicit positive kite gradient identity.],
  [Filter inadmissible real loci before expensive chart construction. A failed certificate remains unresolved.],
  [Reduce projection cost],
  [symGCAD certified positivity, linear/quadratic reductions and positive-root covers.],
  [Simplify residual domain checks and massive quadratic structures, while preserving zero seams and exceptional cases.],
  [Choose and reuse algebra],
  [Bounded variable-order search, exact operation/proof caches and invertible factorization preconditioning.],
  [Share work across related faces and branches. Heuristics may prioritize work, never silently remove candidates.],
  [Keep the whole density],
  [Native numerator support, symbolic derivatives, composed root/map evaluators and endpoint machinery.],
  [Compute numerator-dependent orders and test regional coefficients. General family resolution and remainder certificates remain missing.],
)
]

The algorithm inventory is pinned to
#link("https://github.com/alphal00p/symGCAD/blob/36bfd9ebedf617a884518458d51cf1c2adcd30bd/reference/FINAL_CHECKPOINT.md")[symGCAD's
implementation checkpoint]. Some techniques are optional or experimental.
Its #link("https://github.com/alphal00p/symGCAD/blob/36bfd9ebedf617a884518458d51cf1c2adcd30bd/README.md#results-and-validation")[matched
generic-CAD study] reports a 6.82-fold geometric-mean reduction in matched
algebra-phase time against the original implementation. That result is neither
an end-to-end region-analysis comparison nor a guarantee for every problem;
the checkpoint still records unresolved fixed and parametric cases.

The most concrete extension beyond the HRF study's stated scope would be
*massive families with algebraic critical branches and realistic numerators*.
Our exact root/domain infrastructure and symbolic density machinery make this
a plausible target. They do not establish that the difficult general algebraic
resolver is complete, or that our approach is already faster than HRF or SOFIA.
Shared geometry could be reused across numerator terms and Laurent outputs
only with matching kinematic and branch guards.

== Surplus candidates, exact cancellation and the completeness test

Projection artifacts or solutions outside the physical domain can be excluded
algebraically. Other candidates describe equivalent charts, scaleless
contributions, numerator cancellations or overlap terms. Each exclusion needs
its corresponding exact argument and regulator prescription. A region that
vanishes at leading order can still contribute at a later order. It is safer
to retain a harmless redundant contribution than to discard it on the basis
of a small numerical estimate.

#link("https://arxiv.org/abs/1111.2589")[Jantzen, Foundation and generalization
of the expansion by regions] derives regional identities with explicit
covering/convergence assumptions and overlap terms. Cancellation of regulator
poles does not prove completeness: an omitted region can be finite. Nor does
agreement at a finite set of kinematic points establish that all orders or
parameter strata were covered. Exact coefficient cancellations can require
integral identities, making a minimal nonzero-region list harder to obtain
than a complete redundant expansion.

A useful first comparison would reproduce ordinary Euclidean regions, the
massive threshold controls and the crown's known hidden scaling, followed by
HRF's moving and boundary examples. Record discovery, local resolution,
verification, cell/chart counts and aggregate memory separately; compare
exact scalings and coefficient expansions before comparing speed. Include
negative cases and deliberately incomplete searches. The central acceptance
criterion must be *certified coverage plus a controlled remainder*, not just
finding more candidate regions or observing their cancellations.
