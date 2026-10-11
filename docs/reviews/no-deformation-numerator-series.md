# Compact finite numerator coefficients at infinity

The actual physical double-box input has 156 numerator-factor associations and
128 distinct native numerator Atoms. A bounded Rust probe computes their first
two finite residual coefficients on each of four native infinity charts using
Symbolica's existing series engine. All 512 cases finish in 20.602 seconds at
38,588 KiB child peak RSS. Twelve independent exact specialized coefficient
controls pass. This is preparatory research, not an integration measurement.

The original ray coordinates are x5, x6 and x2. For each original polynomial
numerator N, native polynomial conversion supplies its individual degrees
d5, d6 and d2, rejecting hidden coordinate dependence in coefficient Atoms.
These degrees are fixed before regulator or coefficient specialization. For
the native chart distances R,S,T, the compact expression is

```text
R^d5 S^d6 T^d2 N(x5=(1-R)/R, x6=(1-S)/S, x2=(1-T)/T).
```

`AtomCore::series(c, 0, SeriesDepth::relative(2))` extracts its exact formal
c-valuation and two residual coefficients. Finite source coordinates remain
independent coefficient slots; root expressions are not expanded into N.
Native differentiation supplies the mixed c/source-x1 partial. The common
infinity scale c does not enter the roots in these charts. Physical u/v
derivatives still require composition through the root and coordinate maps.

The denominator triples are `(bc,ac,c)`, `(abc,c,bc)`, `(bc,c,abc)` and
`(c,bc,ac)`. An exact native check joins these maps, the original prepared
156-term exponent ledger and the existing measure. U has c-power -2, F has
c-power -3 and the ray-plus-sector measure has c-power -4. All 624 complete
term/chart powers simplify to c^3, with epsilon cancelling. Root-presentation
monomials never enter this calculation. Leading coefficients may vanish at
further specializations; this formal accounting does not certify the other
faces or establish an integration kernel.

For three representative actual numerators on all four charts, an independent
native univariate calculation sets each ray to z/slope-1 at a fixed exact
rational prefix. Its highest and next coefficients match the finite series
value and normal coefficient after denominator scaling. Repeating this on the
original source-x1 derivative checks the mixed partial. These are exact
specialized controls, not global real-branch certificates.

The API/source/probe audit also checked native
`UnivariatePolynomial::resultant(b*x-a, N)`: its linear-first implementation
uses homogeneous Horner and computes `b^degree(N) N(a/b)`. This supplies exact
full clearing, but materializing successive AtomField polynomials grew one
roughly 12 KB source to roughly 9 MB. An incremental probe completed 23 unique
numerators before being stopped at 265.8 seconds and about 6.65 GB. Its unfinished
rows remain failures, rather than benchmark results. The finite-series route
completes all 128 unique numerators without that materialization.

Native series can use a statistical nonzero check for a generic leading
denominator. This probe only introduces inverse coordinate monomials whose
nonvanishing is known on the open direction domain. It does not admit arbitrary
denominators or infer real signs from formal series. Production admission must
retain the source polynomial, frozen degrees, exact chart, coefficient roles
and requested derivative orders. Existing native FunctionMap/Etale facilities
can compose derivatives; they do not replace real root selectors or face
regularity certificates.

The source identity is `ce87d1ac…`, unchanged from the verified full geometry.
Evidence handoff `3d63ffcb…` freezes source/API pins, successful series controls,
the exact power ledger and both interrupted full-expansion trials. No GCAD solve
or large proof reload occurs in this research. No dependency patch, alternate
CAS, endpoint AD or production RNG is introduced. Lower U=F=0 strata, direction
faces and finite causal seams remain required work.
