# Reusing native source-density preparation across cells

The coordinator independently reviewed the source and tests against `f42796e`.
This change separates immutable source-density preparation from cell-map
composition. It adds no CAS, evaluator codec, numerical sampler or global cache.

`CompiledDensityBody` retains the actual projective quotient/preparation owner,
the complete ordered native sign vector, source-coordinate order, regulator and
coefficient roles, caller FunctionMap and exact native evaluator. Reusing it
requires that same source and sign authority. Every new map repeats role checks,
including its later root and normalized-chart variables. The original source
terms, their exponents and protected causal phases remain unchanged; the map's
positive Jacobian enters exactly once. Caller aliases cannot replace constants
inside the causal-phase program.

Symbolica's public evaluator builder, FunctionMap and EvaluatorComposer provide
all compilation and composition. Source inspection and the focused native probe
confirm that Composer copies/inlines instructions and optimizes the resulting
program. Consequently sharing saves source-body preparation, but does not promise
shared final machine code, removal of final optimization, or bounded memory for
the complete cell catalogue. Body and map lifetimes remain caller owned.

The dependency audit also checked symGCAD compatibility merges and cylindrical
caches. Neither is an independently verified replacement for the retained raw
cell catalogue. This change does not merge cells or weaken root-selector
identity. Closed-face certificates, symbolic subtraction and integration remain
separate work.

Validation on the exact imported source:

- Nine density tests, including three new groups, pass. Distinct maps share
  positive/negative-sign bodies with complete output and native codec parity at
  128 and 256 bits. Complex coefficients, regulator changes, phase isolation,
  foreign owners/signs and later role collisions are covered.
- Seven existing normalized-map tests and strict native library/test Clippy
  pass. The coordinator checked all four source pairs and 35 evidence files.
- A review correction makes the body-settings getter report its effective
  CPE-disabled settings; final composition settings remain independently visible.

A small matched fixture measured body preparation at 0.533/0.756 ms, attachment
including `Composer.finish` at 0.374–0.417 ms, and full preparation plus attachment
at 0.766–1.076 ms. These are descriptive single-process observations, not a
physical-catalogue speedup claim. No large proof was reread for this change.

The local handoff is
`target/no-deformation-density-body-sharing/handoff.json`, SHA-256
`211fd23f993a4aa4c476713abd0b8b070b7c91392687d28266ad7e105e9c3497`.
Raw build logs and measurements remain untracked.
