# Benchmark report reorganization — 12 October 2026

## Scope

Documentation only, requested after the report grew through multiple campaigns
and research follow-ups. Preserve the opening benchmark tables unchanged and
first; replace the chronological accumulation with a coherent chaptered account.
No integral generation, sampling, dependency or production code changed.

## Independent audits and decisions

Three separate agents audited structure, results/provenance, and theory/literature.
They then owned disjoint chapter sources. The coordinator assembled the report,
preserved the contour formulas, consolidated the implementation discussion and
reviewed the resulting PDF. Private working audits are in
`target/report-reorganization/` and are not published as raw output.

The audits identified:

- Current results appeared before their method definitions, with future
  research between tables and numerical validation, followed by a second title.
- Five-minute/50-core, one-worker and Horner-zero comparisons competed with the
  newer near-ten-minute, 25-worker comparison and sometimes different inputs.
- Historical prose still called completed triangle/box/sunrise comparisons
  pending and described the stopped no-contour double box as active delivery.
- Geometry completion, local chart admission and compiled integral completion
  needed a single consistent account.
- The prose definition of C incorrectly said absolute values of sector
  integrals. It now states integrals of absolute sector weights, retaining
  continuous importance corrections and the existing exact-offset convention.
  This corrects the explanation, not the numerical data.

Six chapters now progress from campaign interpretation to methods, measured
findings, implementation, actual support boundaries, and future critical-locus
and region analysis. The cover/reading guide and linked contents come after
the five landscape pages. Chapter and page references resolve through Typst
labels. The source/retention map is in `docs/BENCHMARK_REPORT.md`.

Superseded benchmark tables and initial failed attempts leave the rendered PDF;
their source and compact records remain available. The matched Jacobian study,
loader ownership measurements and a large-weight cap diagnostic remain because
they explain current implementation choices. Their historical settings and
limited scope are explicit.

## Scientific retention checks

- All 66 measured overview rows and the two unavailable kite slots retain the
  same source, values, order, layout and labels.
- Finite-coefficient norm uncertainty, full covariance, MC-equivalent display
  scaling, timed-call accounting, rescue, rounding and missing-reference caveats
  remain explicit. C and maximum-weight definitions are separated correctly.
- Current tuning and independent work controls remain distinct from calibration,
  original baselines and incomplete MC work allocations.
- Fixed/polynomial/sign-aware equations, implicit-strength derivatives,
  deform-before-subtraction ordering, factorwise branches and optional checking
  are preserved. The alternative sign-aware envelope is not claimed to dominate
  the polynomial bound pointwise.
- Symbolic endpoints remain independent of contour-only dual Jacobians. Native
  owner serialization is not described as saved machine code.
- No-contour composition, phases and regular root derivatives are connected to
  closed-face admission and the incomplete general resolver.
- The 644,699-cell double-box atlas, local 307-chart admission, zero compiled
  kernels, 9,840-cell kite and unfinished full intervals retain their scope.
- Exact scalar/crown experiments, generic-open CAD limitations, conditional
  local blow-ups and region-analysis proof obligations remain explicitly
  separate from implemented production support. Literature and native-reuse
  provenance survive consolidation.

## Validation

The Typst 0.15.1 build with Libertinus 7.051 succeeds and reduces the report
from 43 to 24 pages. All five opening pages have identical extracted text and
byte-identical PNG rasters at 1300-pixel resolution. The overview Typst source
and every tracked `docs/*.json` are unchanged. Chapter starts are pages 7, 10,
14, 17, 19 and 21; the linked contents follow the tables on page 6.

Extracted-text checks find no superseded pending/delivery assertions. Visual
inspection covers the chapter pages, formulas, table layout and contents;
`git diff --check` passes. Independent agents re-reviewed the assembled sources
and affected chapter layouts without blocking findings. Production tests are
unnecessary for this documentation-only change; no scientific campaign is
restarted.
