# Documentation-only Landau-pinch discussion

The delegated mathematical review and coordinator review produced
[`landau-pinches.typ`](../landau-pinches.typ). No new pinch solver, cell splitting,
subtraction algorithm or benchmark was implemented.

The central claims are deliberately separate:

- The existing finite, smooth weighted-gradient contour fixes an interior
  stationary zero. Its Jacobian need not be one there.
- A failing gradient direction is not by itself a complete physical Landau
  classification. A physical pinch, singular dependence on kinematics and a
  dimensional-regulator pole are distinct.
- A positive-definite nondegenerate local model gives the stated threshold
  power through a Schwinger/Gaussian calculation. Its logarithmic marginal
  radial integral can expose an epsilon pole, subject to coefficients and
  cancellations. Indefinite Hessians and degenerate critical loci require
  additional analysis.
- Cell splitting and resolution could expose an endpoint normal form, but
  require complete density/branch/coverage certificates and a common regulated
  convergence domain. The present dynamic prescription does not implement
  general automatic pinch resolution or epsilon-pole extraction.

The review checked the primary sources, with the following limits:

- [Collins, Landau-condition proof](https://arxiv.org/html/2007.04085v1),
  section II.1.4: pinch versus divergence and numerator/physical-sheet
  qualifications. No unrestricted polynomial-integral theorem is inferred.
- [Borowka, Carter and Heinrich, SecDec 2.0](https://arxiv.org/html/1204.4152v2),
  sections 2.1–2.2: scalar F exponent and causal weighted-gradient deformation.
- [Bogner and Weinzierl, resolution for multiloop integrals](https://arxiv.org/html/0709.4092v2),
  section 3: the stated sector-decomposition class excludes interior zeros.
- [pySecDec](https://arxiv.org/html/1703.09692v3), section 4.8:
  multiple-regulator support, without a claim that every partition has a
  common convergence chamber.

The local Gaussian identity, marginal radial pole and Taylor subtraction
formula are explicit elementary derivations in the report. Source inspection
checked native envelope/Jacobian construction, retained-context diagnostics,
stationary-zero and cancelled-density controls, and existing family continuation.
No new CAS or AD was used. The delegated evidence handoff has SHA-256
`d9495bec66cc6eec038588b27ca5b963e8975149cb75a5cdd51dcf5a9aa50b9b`.

The coordinator compiled the complete report and visually inspected both new
theory pages. Local Typst operator definitions and explicit multiplication
spacing prevent accidental subscript capture in the rendered gradient formulas.
