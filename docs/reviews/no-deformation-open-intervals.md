# Source-bound maps for finite and half-infinite cell intervals

The native positive-projective mapper now supplies a symbolic map for one
actual cell axis with linear finite bounds. It retains the original source,
verified decomposition, cell ordinal, axis and root selectors. Earlier
nonlinear axes can remain unresolved; issuing a later axis does not claim that
the complete cell is executable.

For `0 < u < 1`, the supported maps and positive diagonal Jacobians are:

| Interval | Map | Diagonal Jacobian |
|---|---|---|
| `(a,b)` | `a + (b-a)u` | `b-a` |
| `(a,infinity)` | `a + u/(1-u)` | `(1-u)^(-2)` |
| `(-infinity,b)` | `b - (1-u)/u` | `u^(-2)` |

Bounds remain expressions in preceding source coordinates. The triangular
determinant uses each diagonal once. A doubly infinite interval remains an
explicit unsupported case. The existing finite-only lowering preserves its
admission and refusal behavior.

Both unit faces record their finite or signed-infinity origin. The original
linear-root leading coefficient is retained even if expression simplification
cancels it. An infinity face records the interval Jacobian's power of `-2`;
other source, root and width valuations remain separate. This information is
an input to endpoint resolution, not a claim that the pulled-back density is
regular on the closed cube.

The implementation reuses Symbolica expressions, differentiation, substitution
and matrix facilities, and the existing native finite-root extraction.
Numerica and HEPKit API/source searches and a focused native two-cell probe
found no existing owner for this interval operation. No root solver, graph
representation, serializer or scheduling layer is introduced.

Independent source review `662b572d…` checks all five source files, actual-owner
association, inverse orientation, Jacobian factors, retained divisors and
ecosystem reuse. The consuming checkout passes all 21 map tests, including four
new controls, strict workspace/all-target Clippy, formatting and diff checks.

A separate native correctness run freshly verifies the complete 644,699-cell
proof and binds the actual 156-term physical source. It then issues the first
cell's rational first axis and three positive rays, and the last cell's final
two rays, with exact inverse and diagonal checks. The preceding nonlinear axes
retain explicit refusals, as does the original finite-only whole-map request.
Independent actual-run review `99c7833f…` passes; the frozen evidence receipt is
`ab0ac4cc…`. The source handoff is `ca86871b…`. The consuming coordinator checks
all five source postimages, 37 closed source/gate records and 16 actual-run
evidence files before publication, without rehashing or solving the raw proof.

That run completes in 247.062 seconds under a 600-second/100-GB allowance.
Source preparation takes 3.500 seconds, decoding 127.011 seconds, verification
and binding 111.542 seconds, and map checks 0.147 seconds. Sampled aggregate
peak RSS is 11.522 GB. These are correctness-build admission timings, separate
from the published GCAD generation trial; they are neither sampling costs nor
additional stages to sum into its measurement.

This operation does not yet resolve the physical double box's nonlinear
prefixes, closed faces or causal zero seams. It does not issue an integration
artifact or supply an integration timing or standard deviation.
