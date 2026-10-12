# Expansion-by-regions discussion review — 12 October 2026

This milestone adds `docs/region-analysis-prospects.typ` and recompiles
`docs/benchmark.pdf`. It changes no numerical method, dependency, sampling
campaign or benchmark entry. The user requested a prospective literature
discussion and an assessment of where symGCAD and the recent work could help.

## Sources and claims

- [Pak–Smirnov, 1011.4863](https://arxiv.org/abs/1011.4863): geometric region
  identification in the stated non-threshold setting.
- [Jantzen–Smirnov–Smirnov, 1206.0546](https://arxiv.org/abs/1206.0546): asy2
  and exposure of potential/Glauber regions. Local transformations revealing
  hidden regions are established prior work, not claimed as new here.
- [Heinrich et al., 2108.10807](https://arxiv.org/abs/2108.10807): numerical
  expansion by regions with pySecDec and cancellation limitations.
- [Gardi et al., 2407.13738](https://arxiv.org/abs/2407.13738): crown and
  interior pinch dissection.
- [HRF, 2609.29442, §2.4](https://arxiv.org/html/2609.29442#S2.SS4): the
  closest comparator. Credit its theoretical face/scaling-first route and
  distinguish it from the negative audits actually used in the paper.
  The paper's massless, unit-numerator scope motivates the proposed extension.
- [SOFIA, 2503.16601, §5](https://arxiv.org/html/2503.16601v2#S5): candidate
  singularities/alphabets versus unresolved local-exponent and sheet analysis.
  It is not described as a facet-only method-of-regions tool.
- [Fevola–Mizera–Telen, 2311.14669](https://arxiv.org/html/2311.14669):
  first/second-type distinction and the different role of the LP polynomial.
- [EC-CAD, 1903.08999](https://arxiv.org/abs/1903.08999) and
  [variety sub-CAD, 1401.0647](https://arxiv.org/abs/1401.0647): targeted
  equality processing is possible under explicit hypotheses. This capability
  is not attributed to the current generic-open symGCAD API.
- [Jantzen, 1111.2589](https://arxiv.org/abs/1111.2589): coverage, convergent
  expansion assumptions and overlaps; pole cancellation alone is insufficient.

## Native capabilities and limits

The read-only dependency audit is pinned to symGCAD
`36bfd9ebedf617a884518458d51cf1c2adcd30bd`, using its README, final checkpoint,
handoff and reduction ablations. Actual capabilities include exact positivity
and homogeneity reductions, conditional linear/quadratic reductions, selected
positive-root covers, bounded ordering search, exact caches, optional invertible
factorization preconditioning and independent proof replay. Their proposed
benefits for region analysis are hypotheses, not comparative measurements.

The 6.82-fold number is explicitly the dependency's matched generic-CAD
algebra-phase geometric mean. It is not a FastSecDec region-analysis timing,
an end-to-end speedup, or a statement that every case improved. Unresolved
fixed and parametric examples remain acknowledged.

The preceding native scalar/crown probes establish only their stated exact
critical-system results. General equality-section enumeration, resolution of
asymptotic families and integrated remainder certificates remain missing.
The PDF proposes a complete, possibly redundant expansion under stated
hypotheses, not an implemented universal method or a minimal region census.

## Independent review and validation

A separate research/review agent inspected the draft and primary literature,
checked the actual dependency inventory, and supplied the private review at
`target/region-analysis-discussion/research-review.md`. The coordinator adopted
its narrower HRF attribution and added the earlier asy/asy2 references.

Reviewed distinctions include parameter-domain versus exponent-polytope
interior, limiting-family versus fixed-point analysis, leading-face systems
versus the complete unscaled F, and full-density/overlap contributions versus
candidate geometric regions. No numerical cancellation is used as proof of
completeness or exact vanishing.

Validation is document-focused: Typst compilation, extracted-text checks,
visual inspection of the new pages, unchanged opening benchmark tables and
`git diff --check`. No production tests or integration runs are needed for
this documentation-only update.
