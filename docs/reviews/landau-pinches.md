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

## Automatic detection and localized hybrid follow-up

The user's follow-up asks whether the dynamic prescription itself detects
stationary zeros, and whether localized blow-ups could avoid the global GCAD
cell count. Two independent agents reviewed native detection and local theory.
Only the theoretical report and review documentation changed; numerical code,
sampling and the recorded benchmark measurements are unchanged.

The source audit checked `FailureContext`, its terminal-error call sites,
validation policy, stationary-point envelope and existing cancellation controls.
Current detection is a cold, encounter-dependent diagnostic, not a domain
search. Exact ball zeros differ from enclosures merely containing zero. The
polynomial radius remains at its cap when the direction vanishes, and shifting
F by a constant leaves its derivative envelope unchanged. Therefore neither
small strength nor successful root solving detects or excludes a pinch.

The report proposes a separate certified critical-system search at fixed
kinematics, with exact F evaluation at isolated gradient roots, face strata and
explicit unresolved outcomes. It cites
[Landau Discriminants](https://arxiv.org/html/2109.08036) and
[Landau Singularities Revisited](https://arxiv.org/html/2311.14669) for algebraic
approaches and their limitations. No complete real-locus solver is claimed to
exist in FastSecDec.

The localized contour-plus-blow-up derivation is independently checked. For
a real nonsingular Hessian H, positive W and finite positive strength lambda,
M = I - i lambda W H gives Im(F's quadratic pullback) =
-lambda y^T H W H y. This is negative definite even for a saddle Hessian.
Compact radial charts and certified remainder bounds therefore expose a radial
endpoint with a smooth nonzero angular factor. The argument uses ordinary
transpose, not Hermitian transpose. It retains the full contour Jacobian and
strength derivatives, exact centers, analytic continuation and matching contour
maps across partition interfaces.

The initial d*2^d chart count is an explicit local construction, not a general
resolution complexity bound or a prediction for the double box. Degenerate
critical loci, other vanishing factors and the complement remain separate
obligations. The example is our derivation, while
[Gardi, Herzog, Jones and Ma](https://arxiv.org/html/2407.13738) supply a published
precedent for targeted pinch dissection before sector decomposition. Neither
source proves universal efficiency or improved variance for the proposed hybrid.

The independent theoretical review passed for source SHA-256
`9104125542cbc23c5f33da80107b04e86487b48738095ae383de2ac757411a3c`.
The coordinator incorporated its two clarifications: Q at the radial endpoint
is a derivative-defined extension, never numerical 0/0, and sign-aware
smoothing need not be described as universally a small correction. The full
Typst report was recompiled and the new sections visually checked. No scientific
tests were rerun for this documentation-only change.
