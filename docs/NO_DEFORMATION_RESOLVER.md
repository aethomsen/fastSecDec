# General algebraic endpoint resolver: proposed algorithm and acceptance contract

Status: research and implementation specification, 2026-10-10. This draft does
not report a working resolver. The [proof addendum](NO_DEFORMATION_RESOLVER_PROOFS.md)
refines its coverage and regulator-restriction contracts. The general algebraic
resolver is a required deliverable; successful rational or quadratic examples
alone will not complete it. Existing contour and no-threshold modes remain
independent, supported paths.

## 1. Mathematical scope and output

The input is a finite sum of densities on a real semialgebraic integration
domain, with exact polynomial/rational data over a characteristic-zero field,
selected real algebraic branches, and exponents affine in the regulators.
Numerical inputs must have an explicit exact interpretation; parameterized
inputs carry an exact parameter-domain predicate. Native floating-point
evaluation is downstream of this exact admission boundary.

The intended physical family includes real masses and kinematics with complex
polynomial numerator coefficients and the declared Feynman boundary value.
Complex masses, arbitrary transcendental boundary functions, and a finite value
at a genuine singular external kinematic point are not implied by this scope.
An exceptional point can require a separate stratum or a distribution-valued
answer. A resource stop must preserve a resumable proof frontier rather than
produce a zero, an incomplete cover, or a successful generation receipt.

The resolver must deliver finitely many integration records with maps
`phi_j`, real-domain ownership, branch certificates, and the **full** density

\[
 \rho_j(t,\epsilon,\eta)=
 e^{i\theta_j(\epsilon,\eta)}
 \prod_i t_i^{a_{ji}+b_{ji}\epsilon+\sum_k c_{jik}\eta_k}
 H_j(t,\epsilon,\eta),\qquad t\in(0,1)^{n_j}.
\]

Here `a`, `b`, and `c` are exact rationals. The normal-form certificate covers
every required closed face, not only a neighborhood of the origin. It proves
that each denominator unit is nonzero there and that `H_j` has the finite mixed
jets needed for symbolic subtraction and the requested Laurent order. Analytic
algebraic units are the preferred stronger contract. Zeros in a smooth
numerator are permitted; they do not require division by that numerator.

The map, its determinant, all source factors, the numerator, and regulator
phases belong to this density. The numerator may be complex, so positive-F
cells cannot generally be described as purely real contributions. All Laurent
coefficients, exact offsets, and complex covariance remain present, including
coefficients expected to cancel.
Real change of variables uses the absolute determinant, or an explicitly
certified orientation making it positive. This volume factor must not be
confused with oriented seam incidence in a residue/gluing certificate.

## 2. Literature choice and what it does not already provide

Use the following primary sources for distinct parts of the design:

- [Bierstone–Milman 2008, *Functoriality in Resolution of Singularities*](https://ems.press/content/serial-article-files/41043), §§2–7: the marked-ideal algorithm and local-to-global construction.
- [Bierstone–Milman 1997, *Canonical desingularization in characteristic zero*](https://arxiv.org/abs/alg-geom/9508005): the invariant and constructive termination framework. The arXiv record exposes only the first chapter; the full published construction must be used when implementing its detailed invariant.
- [Bierstone–Milman, *Desingularization algorithms I. Role of exceptional divisors*](https://arxiv.org/html/math/0207098v1): exceptional-divisor history, principalization, and the distinction between permissible and invariant-selected centers.
- [Parusiński–Rond, *The Abhyankar–Jung theorem*](https://math.univ-cotedazur.fr/u/parus/publis/A-J.pdf), §5: constructive ramification and root splitting after the discriminant has normal crossings.
- [Bierstone–Milman 1988, *Semianalytic and subanalytic sets*](https://www.numdam.org/item/PMIHES_1988__67__5_0.pdf), §§4–5: the real rectilinearization framework. A finite covering by rectilinearizing charts is not automatically a disjoint integration formula.
- [Encinas–Nobile–Villamayor, *On Algorithmic Equiresolution and Stratification of Hilbert Schemes*](https://arxiv.org/abs/math/0010228): parameter stratification and simultaneous resolution under its stated algorithmic hypotheses. Those hypotheses are not automatically established for this proposed implementation.

The [2025 Feynman-integral paper](https://arxiv.org/html/2506.24073v1) gives useful
massive-sunrise algebraic maps and explicitly discusses spurious poles and
cancellation across regions. It does not justify deleting such coefficients
from FastSecDec. The [2026 generic-CAD paper](https://arxiv.org/html/2603.05444v1)
separates threshold signs but leaves general algebraic endpoint decomposition
as additional work. Consequently, neither paper supplies the complete general
resolver requested here.

This specification combines these foundations into a proposed implementation.
The finite real-atlas adapter and general regulator-restriction certificate
remain design proof obligations. They are not claimed as consequences of one
ordinary sector-decomposition call.

## 3. Ordering and stable source semantics

For an unselected full integral, the preferred route is:

1. Native graph/parametric input and a finite primary-projective or simplex
   domain, retaining the original density and causal prescription.
2. Exact symGCAD sign decomposition before extensive ordinary sector
   decomposition, avoiding unnecessary polynomial degree and chart growth.
3. Algebraic cell pullback, general endpoint resolution, and real atlas
   construction.
4. Ordinary polynomial SD only where its representation and proof assumptions
   are actually satisfied; it may implement a certified monomial substep.
5. Symbolic endpoint subtraction/IBP of the complete pulled-back density.
6. Common-family regulator assembly/removal and complete epsilon expansion.
7. Native evaluator compilation, persistence, and caller-driven integration.

For explicit existing `source_sectors`, compute the existing source geometry,
apply the original-source filter, and threshold-decompose those selected,
unsubtracted charts. The alternative ordering is part of generation identity.
One-to-many threshold descendants need explicit nested lineage; do not weaken
the existing v14 strictly increasing local-to-original mapping to accommodate
duplicate parent IDs. Partial-source results remain partial physical results.

No endpoint subtraction is moved in front of a nonlinear map without a
separate equivalence proof. The new strategy uses real maps and causal phases;
it is independent of the contour-Jacobian option.

## 4. Exact cell and algebraic-tower preparation

Each cell retains its original signed-factor IDs, exponents, constraints,
ordered axes, root selectors, and exceptional conditions. Projection factors
serve root geometry; they do not replace the original signed density factors.
For an original factor `(F-i0)^a`, a negative-F cell contributes
`exp(-i*pi*a) * abs(F)^a`. In denominator notation `a=-nu`, the phase is
`exp(+i*pi*nu)`. Preserve positive content factors raised to `a`.

A root selector must describe a branch over the whole admitted cell, not only
an isolating interval at one sample. Refine leading-coefficient zeros,
discriminant zeros, root-order collisions, and piecewise selectors such as
`max(0,root)`. Omitted lower-dimensional GCAD sections have measure zero only
in the initial convergence domain; their limiting contributions still enter
the analytic continuation and endpoint data.

For each algebraic tower:

1. Retain defining equations, variable order, square-free factors, denominator
   conditions, and a compatible real embedding.
2. Check separability over the correct function field. A degree drop is a new
   stratum, not a root-index adjustment.
3. Make the defining polynomial monic without losing its leading divisor. For
   `P(z)=a0*z^d+a1*z^(d-1)+...+ad`, set `w=a0*z` and use
   `Q(w)=w^d+a1*w^(d-1)+a0*a2*w^(d-2)+...+a0^(d-1)*ad`.
   Recovering `z=w/a0` introduces an explicitly tracked valuation.
4. Either retain the tower or use a native primitive-element presentation with
   certified transport in both directions. Flattening is an optimization, not
   permission to forget real branch selection.
5. Build a finite divisor inventory containing domain/patch boundaries,
   leading coefficients, denominator factors, discriminants, and relevant
   elimination divisors for causal factors, positive factors, root gaps, and
   the map determinant. Only nonzero factors are admitted to this inventory.

For algebraic expressions, native resultants can provide an elimination
polynomial whose zero set contains the required divisor. Extraneous conjugate
divisors cause extra work but are safe if the selected branch is retained.
The inventory must cover the entire tower. Resolving just the first defining
polynomial's discriminant is insufficient.
For polynomial representatives `f_i`, its union divisor is encoded by the
principal ideal `(product(f_i))`, or by a proved equivalent simultaneous
principalization. The ideal generated by the separate `f_i` instead describes
their common zero locus and is not a substitute. Keep factor identities even
if native polynomial operations materialize the product internally.

## 5. Marked-ideal resolution engine

The chosen BM state is `(M,N,E,I,d)`: smooth ambient/working spaces, ordered
SNC exceptional divisor, ideal, and positive mark. A permissible smooth center
lies in `ord(I)>=d` and meets `E` normally. Its controlled transform is
`I'=I_exceptional^(-d) * sigma^*(I)`.

Marked addition is `(I,d)+(J,e)=(I^(l/d)+J^(l/e),l)`, `l=lcm(d,e)`.
Coefficient recursion uses logarithmic derivative ideals relative to the
appropriate exceptional divisor, marks `d-j`, and restriction to certified
maximal contact. It is not an unweighted union of ordinary derivatives.
The companion construction separates the residual and exceptional factors.
See BM2008 §§2–5 and equation (4.1) for these operations.

The corresponding formulas to encode are

\[
 C_{E,P}(I,d)=\left.\sum_{j=0}^{d-1}(D_E^j I,d-j)\right|_P,
 \quad I=M N,\quad e=\max_{\operatorname{cosupp}(I,d)}\operatorname{ord}N,
\]

\[
 G(I,d)=\begin{cases}(N,e)+(M,d-e),&0<e<d,\\(N,e),&e\ge d.\end{cases}
\]

The residual-unit case is the separate monomial branch. These expressions do
not eliminate the maximal-contact and exceptional-history preconditions.

The implementation must expose the following exact operations and witnesses:

| Operation | Required evidence |
|---|---|
| Smooth ambient/local chart | defining ideal, local denominator guard, Jacobian-rank witness, chosen local coordinates |
| Order locus | derivative-ideal equations and exact membership/reduction witnesses |
| Exceptional extraction | powers removed, native quotient identity, remaining residual ideal |
| Maximal contact | defining section, order-one/transversality witness on the relevant open set |
| Coefficient ideal | source derivatives, marks, common-mark powers, restriction map |
| Center selection | invariant value, maximal-locus predicate, boundary history and tie-breaking data |
| Blow-up chart | exact forward map, local denominator/ownership guard, exceptional equation, full Jacobian |
| Controlled transform | exact divisibility and reconstructed original ideal generators |
| Overlap/gluing | transported center and ideal equality on chart intersections |

Use the published lexicographic invariant with its recursive lower-dimensional
entries and exceptional history. BM1997's maximum strata select centers;
the algorithm's proof, rather than a heuristic claim that polynomial degree
decreases, supplies termination. A recorded invariant can remain constant
through a preparatory part of the algorithm: do not invent a per-blow-up strict
decrease assertion that the chosen construction does not make.

The exceptional/residual distinction and chronological exceptional data must
survive every transform. The monomial terminal case has a separate
combinatorial center rule. BM's principalization algorithm resolves the total
ideal to a monomial exceptional factor and a unit; an empty cosupport of one
intermediate marked ideal is not by itself the final factor-by-factor
normal-form certificate. These requirements follow the principalization and
exceptional-divisor discussion in *Desingularization algorithms I*.

Proposed driver skeleton:

```text
resolve(divisor_inventory, parameter_stratum, boundary):
    establish smooth local algebraic presentations
    construct BM marked-ideal problem with ordered boundary history
    while the published principalization stopping condition is false:
        compute companion/coefficient presentations using native polynomial ops
        compute invariant maxima and glue compatible local centers
        independently check each selected center
        create all required blow-up charts and exact transforms
        retain overlaps, domains, lineage, and resource checkpoints
    independently verify every inventoried divisor is monomial times a unit
    return resolution certificate, not yet an integration acceptance
```

An existing ordinary SD strategy can be a fast path only when the resulting
maps pass the same final certificate. It cannot replace the general algorithm
or its termination argument.

## 6. Abhyankar–Jung branch normalization and finite jets

After monicization and certified normal-crossing discriminant, use
Parusiński–Rond §5.1: eliminate the penultimate coefficient by translation,
extract the rational monomial scaling, introduce finite power ramification,
split the constant polynomial into coprime factors, and lift them by the
algebraic-series implicit-function construction. Recurse on strictly smaller
factor degrees. A finite algebraic coefficient-field extension may be needed.
The relevant coefficient Jacobian is invertible because the constant factors
are coprime. Their §5.2 retains algebraic/convergent-series structure. Their
Remark 5.2 also shows why a weaker Newton-polyhedron condition does not suffice:
an unresolved branch divisor can prevent multivariate Puiseux roots.

The runtime representation should retain polynomial equations, selected
constant roots, transformations, and finite jets. It must not require an
infinite series or a fully substituted numerator expression. Proposed steps:

1. Certify each ramification exponent and transformed polynomial identity.
2. Select the real root continuation using the original symGCAD selector and
   compatible sample/isolation evidence.
3. Obtain only the multiindices requested by boundary subtraction, evaluating
   coefficient recurrences with native exact polynomial/series/matrix types.
4. Certify the remaining algebraic unit and its nonzero denominator on every
   required face patch. Continue using the unexpanded implicit object for
   numerical evaluation in the interior.
5. At a collision face, use the ramified analytic branch representation and
   its jets. Dividing by the original vanishing `P_z` is not an admissible
   face-evaluation algorithm.

For a simple branch in the interior, `r_x=-P_x/P_z` and its repeated native
derivatives provide the chain rule. The full derivative of a pulled-back
density includes the map determinant and every dependent root. Sharing these
values across all Laurent outputs is desirable; dropping dependencies is not.

## 7. Exact real atlas and all closed faces

Resolution charts usually overlap. The integration layer must prove
coverage and multiplicity, not assume that the list of charts is a partition.
The proposed atlas procedure starts from the GCAD ownership predicates and
propagates exact predicates through each map. For an ordinary blow-up in
adapted real coordinates, select a maximal absolute pivot with a deterministic
tie rule; retain its sign. This gives disjoint pivot sectors away from the
exceptional set. General local coordinate patches require analogous explicit
ownership guards.

The complete atlas design must additionally establish:

- A finite exact refinement of overlapping patch images into owners, with
  rational/algebraic branch guards retained.
- Injectivity or a known finite multiplicity on each open integration record.
- Certified coverage of the original domain up to the identified
  lower-dimensional sets, initially where the density is integrable.
- Rectilinearization of the ownership guards and cube boundaries while
  preserving or re-establishing all required normal forms.
- A finite termination argument for this refinement, not an uncontrolled
  loop that alternates arbitrary CAD and arbitrary resolution.

This last adapter proof is **open work in this draft**. The existence of real
rectilinearization does not by itself certify this particular proposed
finite-domain driver. It must be resolved at the algorithm-design milestone
before the driver is accepted as general.

Every original endpoint, artificial seam, exceptional face, and root-collision
stratum is part of the face inventory. Faces at `t_i=1` can be covered with
`1-t_i` coordinates; intersections need simultaneous certificates. Values at
sample points are diagnostics, never the unit-nonvanishing proof. Lower
dimensional faces needed by analytic subtraction remain represented even
though they have zero ordinary volume.

## 8. Parameter strata and specialization

The implementation must not resolve by mixing external parameters with
integration variables and then advertise the resulting maps as fiberwise
integration domains. Each map retains the original parameter value.
Derivative/coefficient ideals and maximal contact are relative to integration
variables over the parameter-stratum function field. Centers must be smooth
over that stratum and meet the boundary with relative SNC; differentiating in
parameter directions cannot be used to lower a fiber's order.

A proposed constructive route is generic resolution over the function field
of an irreducible parameter stratum, followed by spreading that finite
sequence to an open part of the stratum. Record all denominators, degree
conditions, relative-smoothness and SNC witnesses, and identities needed for
specialization. Resolve the complementary lower-dimensional parameter strata
separately; their generic defining factors can have different degree or vanish
identically. Refine real connected sign chambers and root order within each
algebraic stratum.

This route needs an explicit proof that the computed open conditions are
sufficient and that the exceptional recursion gives a finite cover. Merely
tracking denominators seen during generic arithmetic is insufficient.
For a proper algebraic compactification, BM's *Role of exceptional divisors*
§6.3, Theorem 6.11 supplies the generic-smoothness/induction framework for a
finite stratification; include compactification boundaries in the divisor
inventory. Its more precise invariant-equality proof uses a specified
Villamayor variant, so that stronger compatibility must not be silently
attributed to the unmodified BM invariant. The implementation must compute and
verify the chosen proper-family conditions. Exact rational/algebraic numerical
points exercise the same certificates on zero-dimensional strata.

For runtime binding, crossing a certified stratum boundary must trigger
re-specialization/re-generation or a precise error. It cannot reuse the old
root index or old causal phase under a generic-validity label.

## 9. Analytic family, causal boundary values, and regulator removal

Begin with a **single family defined after a fixed compact projective
parametrization and before the cell partition**, using that parametrized
density's factors and measure. Independently shifting the powers of homogeneous
`U`, `F`, and `x_i` before fixing the projective gauge can break the required
homogeneity and hence Cheng–Wu equivalence away from `eta=0`. An alternative
homogeneous-family construction must include compensating projective powers
and prove its degree/measure identity. Any later gauge change must transport
the entire same regulated family, not silently choose a new one.

Independent analytic exponents can supply a common convergence region when
the compactified domain, numerator, factors, and any algebraic poles satisfy
the required bounds. Compactness alone does not bound a rational or algebraic
function with poles. Clear and track every such pole, and supply a nonempty
convergence-region witness: for example, a rational regulator point satisfying
all certified face-exponent inequalities with bounded residual units. If this
cannot be certified, do not assert the existence of a common convergence
region. Where certified, identities follow from ordinary changes of variables.
Integrate/sum the complete atlas there, then continue that identity
meromorphically. This is the justification for omitting volume-zero sets during
the initial real-cell partition; it is not a claim that a singular causal
distribution has no contribution there. The general complex-power
continuation framework is described in [Bernstein's primary paper](https://www.mathnet.ru/php/archive.phtml?jrnid=faa&option_lang=eng&paperid=2532&wshow=paper)
and, for multivariate polynomial powers, [Borcherds, *Renormalization and quantum field theory*, Corollary 17](https://msp.org/ant/2011/5-5/ant-v5-n5-p.pdf).

Preserve the causal phase before the regulators are expanded. At a regular
one-dimensional crossing, for example,

\[
 (x-i0)^{-1}=\operatorname{PV}(1/x)+i\pi\delta(x).
\]

A split into positive and negative open intervals must recover that imaginary
term through its common analytic continuation. Setting the exponent to its
integer value on each open interval first loses it. This provides a minimal
non-negotiable regression, independently of any Feynman benchmark.

After certified normal form, symbolic subtraction uses exact affine forms
`a+b*epsilon+sum(c_k*eta_k)`. Pole factors are affine regulator hyperplanes;
there is no canonical multivariate Laurent expansion that ignores their
ordering. Keep affine pole denominators unexpanded and regular regulator
dependence symbolic until the physical restriction has been established.
In particular, never set `eta=c*epsilon` as a shortcut.

Required removal procedure:

1. Build boundary counterterms and residuals for every cell from the same
   family, retaining full face lineage and oriented seam incidence.
2. Assemble meromorphic contributions at generic epsilon.
3. Prove the restriction `eta=0` exists for the admitted physical family,
   possibly as a meromorphic function of epsilon.
4. Remove auxiliary regulators on that assembled family; then compute the
   requested epsilon Laurent coefficients, including all spurious poles.

There are two acceptable routes for step 3: an applicable theorem with checked
hypotheses, or a certificate reducing the potentially singular polar parts to
verified identities, such as compatible seam pullbacks and exact boundary/IBP
relations. The certificate must cover all pole orders and intersections of
regulator hyperplanes through the restriction locus; a vanishing simple
residue alone does not establish regularity. It must describe the integration
domains as well as the expressions. Cancellation between distinct cell
integrals need not be pointwise cancellation of their integrands.

The general residue/gluing and regular-restriction construction is **open work
in this draft**. Numeric near-zero residues are tests, not its proof. Taking a
finite part when restriction does not exist is an extra regularization or
renormalization prescription and is not silently authorized by this strategy.
The algorithm-design milestone must resolve this interface. Until then it
must report `RegulatorRestrictionUnproved` with retained evidence rather than
accept a claimed general answer. This diagnostic is a research frontier, not
a replacement completion target.

## 10. Symbolic endpoints and native evaluation

For `a>-1`, the coefficient integrals can be integrable without subtraction
when the certified units are regular; logarithms from Laurent expansion do
not automatically change that conclusion. A positive power map `t=u^m`
changes the exponent to `m*(a+1)-1` and can improve smoothness/variance. It does
not turn `a<=-1` into an ordinary integrable density.

For divergent endpoints, use symbolic IBP/Taylor subtraction of the **full**
pulled-back density. Native function definitions/derivative hooks may preserve
an unexpanded numerator and map. The resolver must supply all requested
algebraic branch jets; a numerical moving-root callback without these
semantics is insufficient. No full-density endpoint dualization is introduced.

Keep map/root evaluations shared within a point and across outputs. Cache keys
include the exact branch certificate, ordered input/parameter identities,
parameter stratum, face, derivative multiindex, and compilation policy.
Compiled artifacts must restore the same branch/jet definitions in a fresh
process. Runtime samples do not confer a new proof of cell validity.

## 11. Native API evidence and proof probes

Inspected owner: Symbolica/Numerica
`74225696cd445247fa81c499c5110decd19257ed`. symGCAD's new documentation/example
commits through `8030f7ef` do not change its previously inspected library API.
The common Symbolica identity and any owner changes require separate build
evidence; this draft does not update dependencies.

| Need | Existing native source/API | Boundary to prove or implement |
|---|---|---|
| Polynomial ideals and exact identities | Symbolica `poly/groebner.rs`: `GroebnerBasis::new`, polynomial `reduce`, `change_order`, `is_groebner_basis`; native derivative, replacement, quotient/remainder | resolver ideal/localization recipes, order loci and smooth-center certificates; no separate CAS |
| Divisor preparation | native GCD/factorization and `poly/resultant.rs` | exact tower transport and elimination divisors, with leading factors retained |
| Generic algebraic towers | `solve_parametric`, `ParametricExtension::generic_conditions`, native quotient and specialization; tests at `groebner.rs:3241–3390` | zero-dimensional generic algebraic solving is not a marked-ideal resolver or uniform fiber certificate |
| Norms | `domains/algebraic.rs:2722` polynomial `norm()` exists | implementation calls a shifted factorization helper; probe an unshifted resultant identity before using it as a divisor norm |
| Puiseux arithmetic | `poly/series.rs`: rational trailing exponents, ramification, coefficients; `AtomCore::series` | native series arithmetic does not discover a general algebraic branch's normal form |
| Algebraic-series factor lift | native `poly/factor.rs:8359` has public `hensel_lift` | that routine lifts integer factorization modulo prime powers; it is not the characteristic-zero multivariate AJ lift |
| Lazy symbolic jets | `atom.rs` derivative/series hooks; `derivative.rs:486` custom singular/regular series composition; native FunctionMap/EvaluatorComposer | certify equations and branch jets, not opaque root calls; built-in formal ROOT must not be used as a moving branch with constant derivative semantics; current EvaluatorComposer requires bodies to be inlined before composition |
| Exact real branch cells | symGCAD `output::{RootBound,Cell}` and `roots::{roots_in_open_interval,isolate_union,refine_once}` | full closed-face validity, sign-index provenance, exceptional strata and complete real-atlas verification |
| Endpoint integration | FastSecDec `generation/subtraction/endpoints.rs` | currently exact rational `a+b*epsilon`, not the proposed multi-regulator exponent vector |
| Resolved residual admission | FastSecDec `generation/domain.rs::check_residual` | polynomial nonzero-constant check is not a certificate of arbitrary algebraic units on all faces |

These are source/API observations. The following focused Rust probes remain
required before the corresponding new helper is accepted:

| Probe | Exact acceptance target |
|---|---|
| Ideal/transform | native Groebner reduction verifies a blow-up identity and controlled divisibility for a cusp and a non-principal ideal |
| Marked coefficient ideal | independent direct derivatives agree with marks/LCM construction; changed maximal contact gives compatible center data |
| Tower/norm | two-level root tower, unshifted resultant divisor, specialization away from and on the excluded collision locus |
| AJ branch | `z^2-x*y`, a cubic ramification, a coprime factor split, and a non-quasi-ordinary example that must first be resolved |
| Finite jets | requested mixed jets from lazy algebraic branches equal independent native symbolic values on a simple explicit example |
| Collision face | original `P_z=0` but ramified analytic jets exist; no division by zero in the face path |
| Real ownership | overlapping blow-up charts, root-order changes, same-sign seams; exact counts/volume for polynomial test densities |
| Phase | split `(x-i0)^(-1+eta)` recovers the imaginary delta contribution; wrong-sign/index controls are rejected |
| Regulators | paired cells with cancelling auxiliary poles; generic-epsilon removal agrees in two admissible regulator coordinate systems; mutations retaining a higher-order pole or a mixed hyperplane pole must fail even if a selected simple residue vanishes |
| Persistence | fresh-process decode preserves all equations, root selectors, stratum conditions, face jets and identities |
| Parametric | specialize a generic family on both sides of a discriminant and on its zero stratum; reject stale generic certificates |

All substantive polynomial, series, matrix, numeric, and evaluator operations
belong to the native ecosystem. A missing general algebraic-series operation
should be proposed to its owning crate after these focused probes; do not hide
a second algebra engine inside FastSecDec.

## 12. Independent certificate checker

Discovery and verification must have separate entry points and data ownership.
They may share trusted native arithmetic, but the checker must not accept a
discovery success flag as evidence. A prospective interface is:

```text
verify_cell(source, cell, signed_factor_map, parameter_stratum)
verify_blowup(marked_ideal, center, chart_map, transform_witness)
verify_root_tower(equations, selectors, ramification, factor_lift_witnesses)
verify_atlas(domain, charts, ownership, coverage_and_multiplicity_witness)
verify_faces(density, normal_forms, branch_jets, requested_multiindices)
verify_regulator_restriction(common_family, atlas, residue_witnesses)
verify_resolver_bundle(source_identity, all_certificates, options)
```

Checks include native polynomial quotient identities, smoothness/SNC minors,
branch isolation/order, all required divisor valuations, sign/phase mapping,
nonvanishing conditions, disjointness and coverage, exact Jacobian identity,
and regulator restriction. Real quantified checks require a complete exact
certificate path, including exceptional sections; generic CAD coverage alone
is insufficient for every checker query.

The checker need not reproduce heuristic searches. It must verify every
mathematical claim needed for the delivered integral. The general algorithm's
termination argument is reviewed separately; a valid final certificate does
not by itself prove that discovery terminates on all admitted inputs.

Proposed structured failures include `ParameterStratumMismatch`,
`IncompleteCellCover`, `UnresolvedBoundaryDivisor`, `BranchCollisionUnresolved`,
`MissingFaceJet`, `RegulatorRestrictionUnproved`, and `ResourceLimit` with the
last verified stage. None is a numerical zero. Exact names remain an API design
choice rather than an implemented interface.

## 13. Implementation milestones and current status

Status after the native rational-fiber artifact milestone. Independently checked
local operations may be implemented before the complete general driver is
accepted. Their narrower certificates do not close the open G0 adapters.

| Step | Deliverable and independent gate | Status |
|---|---|---|
| G0 | Freeze precise BM variant, real-atlas algorithm/termination argument, parameter specialization construction, projective common-family/convergence certificate, and regulator-restriction proof interface. Resolve the two open adapters identified above. | Design draft only; not accepted as a complete algorithm |
| G1 | Execute the native API probe matrix; record actual missing operations and owner boundaries. | Native ideal, relative derivative, Puiseux, function-map, prepared-root and codec probes recorded in `REUSE_AUDIT.md`; further operations require their own probes |
| G2 | Exact GCAD adapter, signed-factor associations, branch/tower and parameter predicates, persisted descendant lineage. | Verified caller-owned GCAD, request/solver staging, rational cell maps and structural one-to-many lineage implemented. Certified generic-degree regular sections, composed nested map programs and restricted complete two-/three-cell endpoint-family issuers now pass native controls, including two distinct degree-six roots and closed second-secant units. Singular branch transport and general branch towers remain pending |
| G3 | Marked-ideal objects, native ideal recipes, coefficient/maximal-contact construction, invariant history, centers, transforms, and gluing. | Local exact algebra, contact/coefficient construction, SNC and component checks, monomial frontier, companion cycles and relative recursive blowups implemented. Original physical factors now have an entrypoint before the first pivot and a caller-stepped inventory through repeated embedded transforms and positive residual-cycle restarts. Actual source openings, coefficients, exceptional multiplicities and full Jacobians are retained; completed companions retire at their last admissible frames. Native controls cover a complete local cusp cycle and selected deeper cubic descendants, with explicit resource frontiers. A separate same-ambient principal-open atlas verifies all local-center overlaps and refuses incompatible local maxima. A monomial selection owner now restricts one whole-ambient choice without recomputing local maxima; empty selection and empty cosupport remain distinct. An exact principal-power receipt preserves the original source while exposing an equivalent smaller auxiliary marked presentation. Its opt-in initial recursive adapter readmits centers against the original source and retains the power identity through every first physical pivot. A retained-power owner now follows subsequent issued maps, checks both marked mates, retires completed sources/companions at their owning frames and preserves all support opens; its optional physical-frontier connection now accounts for all issued source/open slots, distinguishes physical termination from completed bookkeeping and retains source-bound retirement through actual later cycles. Unsupported consumers return an explicit incomplete outcome. Canonical nonmonomial global invariant selection, general overlap gluing, relative parameter strata and the complete BM driver remain pending |
| G4 | AJ algebraic-series lift, ramification, real branch transport, and finite mixed jets with owner-level controls. | Exact local coprime/simple root germs, implicit jets, checked discriminant-SNC admission and automatic local ramification/degree recursion implemented, including quintic, nested algebraic and repeated-cluster controls; adapted noncoordinate ramification, global real branch transport and closed-face atlas remain pending |
| G5 | Disjoint real atlas and all-closed-face normal-form certificate; independent checker with adversarial mutations. | Not implemented |
| G6 | Multi-regulator symbolic endpoint engine, common-family assembly, certified auxiliary removal, and complete epsilon output. | Affine multi-regulator symbolic subtraction primitives, rational-fiber continuation and restricted regular-algebraic two-/three-cell continuation with a common epsilon strip implemented. Native finite-face reduction and optional proved rational coefficient zeros preserve ambient derivatives and complete vector/covariance controls; general auxiliary-family restriction/cancellation pending |
| G7 | Native lazy evaluator composition and fresh-process artifact restoration; caller-owned serial/resident execution and resource accounting. | Native rational-fiber factory, composed evaluators, v15/v16 restoration, detached compilation jobs, indexed v3 selective loading and native/CLI preparation/recovery with fresh verification implemented for the first fixed one-dimensional path; normal/serial QMC and MC controls pass. Native HEPKit Integral sessions and existing diagram/family entrypoints expose preparation/compilation for exact fixed inputs. A native pre-parametric represented graph owner now preserves supported Float inputs and source-aware raw replay; native HEPKit graph sessions now expose this represented-value policy with original-input recovery and lazy family arithmetic. An original-point owner now supports represented numerical points on existing exact native families through the same preparation/recovery and HEPKit entrypoints. An explicit represented primitive-fragment policy now uses native graph setters to recompute the effective numerator while retaining the original rounded aggregate as provenance; source-aware replay and native HEPKit tensor controls pass. Float function arguments, full ggHH payload coverage, measured residency and global proof replay remain pending. Native regular-section root programs and the restricted symbolic endpoint family now share complete-vector compilation, detached jobs and indexed v17 records. The quintic native QMC control and default-native fresh-process restoration pass; an explicit affine numerical recipe also admits certified constant rational brackets at generic degree, retaining the V1 default and old artifacts. Two-root family metadata, finite endpoint faces and strict local-identity replay now pass complete-vector and fresh-process controls. Moving-bracket coverage and range-limited validated tracking remain explicit numerical limits. General algebraic preparation and portable root execution remain pending |
| G8 | Scalar, literature/LTD-derived, and physical ggHH acceptance, including complex numerator, original-source selection, parametric strata, and exceptional failures. | Rational pole controls and above-threshold bubble match native OneLOop; required higher-dimensional/general algebraic and physical suite pending |
| G9 | Bounded matched performance and five-minute 50-worker QMC/discrete-MC physical runs; report generation/RSS/full covariance without mandatory per-mille accuracy. | Not run |

The [recursive-driver review](reviews/no-deformation-bm-driver.md) records the
accepted boundary of the next fixed-parameter recursion slice and the remaining
global-center, transform and relative-family obligations. The
[nested-continuation review](reviews/no-deformation-bm-nested-continuation.md)
records the subsequent supported residual drop, retained physical chart frontier
and unresolved terminal component covers. The
[weighted real-atlas review](reviews/no-deformation-real-atlas.md) supplies a
conditional compact-cover construction with native quintic probes. Neither
review changes the incomplete status of G0, the full G3 driver, or G5.

The [original-factor continuation review](reviews/no-deformation-original-factor-continuation.md)
adds a checked original-source inventory through two physical maps, separate
per-factor local terminal tests, and a new ambient companion cycle after a
proved complete residual-order drop. Selected third-map controls retain every
original generator, exceptional multiplier and composed Jacobian. A resolved
generated ideal is explicitly insufficient to certify its individual factors.
The subsequent [physical-lineage review](reviews/no-deformation-physical-lineage.md)
removes that fixed ancestry depth. It retains the actual initial opening and
keyed factors through repeated maps, independently proves per-factor terminal
forms and permits a new cycle only after the active companion is complete and
the full residual maximum drops. This does not supply a simultaneous physical
divisor constructor or a real integration atlas.

The [center-descent review](reviews/no-deformation-center-descent.md) adds exact
overlap checks for local admissible centers on one full ambient principal-open
cover. The source, history and boundary identities remain native owners.
Admissible local centers need not agree: the control with different local
maxima is refused. Canonical global selection and its restriction to opens are
still required; this limited descent receipt does not establish the full BM
invariant or termination argument.

The G3–G6 gate family must include arbitrarily parameterized test families that
exercise nontrivial centers, higher-degree algebraic towers, intersecting
discriminants, and seam/regulator cancellation. Passing the earlier scalar
fixtures or one massive double box cannot substitute for the general algorithm
and certificate argument. Conversely, finite resource limits are legitimate
diagnostics and do not require pretending that every physical input is cheap.
G2 adapter work and individually reviewed local G3 operations proceed in
parallel with G0. Accepting a general production driver still requires the
complete algorithm-design gate.

No overall performance or phase-completion conclusion follows from these
partial milestones. The per-subsystem reviews record exact capabilities and
limitations; they do not replace the general acceptance gates above.
