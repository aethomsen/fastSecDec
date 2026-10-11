== Physical double box: verified rational-point geometry
With a one-hour solve-plus-verification budget and a 100 GB allowance, the
*complete fixed-point GCAD problem now succeeds*. The point and physical graph
are those on the preceding page. Native symGCAD retains the entire positive
orthant and both signs of $F$; no negative-only region or selected sector is
substituted for the full geometry.

A projection-free scan of all six ratio variables admits one native
conditional-quadratic elimination, in $x_2$. Its exact positive-coefficient
certificates hold on the complete domain. The successful native prescription
is positive homogeneous reduction at $x_0$, the monotone engine, and order
$[x_0,x_1,x_3,x_4,x_5,x_6,x_2]$, with the optimized engine for the remaining
problem. Solver and dependency revisions are unchanged. The original optimized
order separately reached its one-hour limit without completing.

#table(columns: (2.4fr, 1fr), inset: 5pt, stroke: 0.35pt + rgb("cbd5e1"),
 table.header([*Completed geometry measurement*], [*Result*]),
 [Verified original generic-open cells], [644,699],
 [Native solver total], [36.356 s],
 [Native projection / lifting], [0.446 / 23.186 s],
 [Solve process, including saved raw proof], [156.526 s],
 [Fresh verification process, including reload], [142.072 s],
 [Complete geometry trial wall time], [298.601 s],
 [Maximum child OS peak RSS], [10.688 GiB],
 [Maximum sampled aggregate cohort RSS], [11.875 GB],
 [Saved raw native proof], [5.284 GB],
 [Full integrand generation / integration / SD], [Pending],
)

The helper saves the raw result before starting a verifier in a fresh process.
Exact input, effective order and parameter-prefix association pass, as does
native independent verification of all 644,699 original cells. The reduced
base's 464,265 cells are a separate counter. Its solver total includes more
than projection and lifting; those two stage times must not be summed and
labelled as total generation.

Serialization is substantial: the solve helper's timer is 152.310 seconds
including its raw JSON write, versus 36.356 seconds in native solver statistics.
The process wall includes startup and teardown. Verification includes decoding
the saved proof. Both child processes have one Rayon thread; they are pinned
to CPUs 18–19. These measurements use the same separately identified Rust
driver over release native libraries as the geometry probes, not the release
CLI integration campaign.

Child RSS comes from `wait4`. Aggregate RSS is sampled across the original
extended baseline's supervisor/child and the successful successor's owned
processes; it excludes unrelated host work. The actual enforcement is
100 GB decimal address space per child and a 100 GB aggregate RSS watchdog.
The native 65536 MiB setting is recorded separately. Neither external memory
nor the pooled 3600-second deadline is reached by this successful trial.

Native source binding retains the actual 156-term physical numerator; 604,005
cells have an infinite boundary. The first cell now has an executable open map
with two normalized algebraic sections and three compactified rays. All 42
coordinate/Jacobian comparisons at 128/256 bits pass independent root enclosures.
Map preparation takes 0.058 s; this separate correctness run, including proof
reload and verification, takes 258.725 s and peaks at 11.528 GB aggregate RSS.
It does not time a compiled physical integrand. See
`docs/reviews/no-deformation-normalized-maps.md`.

*This completes the fixed-point generic-open geometry gate, not the physical
integration.* General cell maps, closed faces, causal zero sections,
infinity boundaries and exceptional seams still require treatment.
Native compatibility merges are not silently substituted for certified cells.
Consequently there is no double-box no-contour integration runtime or SD yet.

The exact recipe is in
`examples/no_deformation/gghh_feasibility/fixed-monotone-prepared.toml`;
compact stage measurements, raw-proof digest and scope are in
`docs/no-contour-gghh-verified.json`. Large raw proofs remain untracked.
The rational-point Rust exporter now also supplies consistent physical momenta
and polarization products for a later matched amplitude comparison.
