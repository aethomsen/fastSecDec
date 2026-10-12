# Scratch critical-locus investigation — 12 October 2026

This documentation-only investigation adds no FastSecDec algorithm, dependency
change or integration result. Private probes and raw receipts remain under
`target/no-deformation-critical-loci/`. The report is
`docs/gcad-critical-loci.typ`, included in `docs/benchmark.pdf`.

## Generic and full decomposition

The inspected symGCAD source `36bfd9ebedf617a884518458d51cf1c2adcd30bd`
documents `CompleteGeneric` as open-cell coverage outside recorded exceptions.
`problem::normalize` rejects equalities. `Cell`/`Axis` stores intervals and root
bounds; the verifier replays generic coverage, not a decomposition into
lower-dimensional equality sections.

Vitaly's original code `fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e` was inspected
and executed using its existing reference installation. Its
[public API](https://github.com/magv/gcad#api) states that the original
algorithm's boundary list is not computed. `gcad/gcad.py:365` and
`gcad_ext/gcad.cpp:278` recurse between roots; `relations_to_positives` accepts
strict inequalities. The documented `merge` may absorb boundaries where those
inequalities fail. This agrees with the
[generic/full CAD distinction](https://reference.wolfram.com/language/ref/GenericCylindricalDecomposition.html).

For `0<x<1`, `F=(2*x-1)^2`, the native symGCAD probe returns two verified
open cells, retaining `x=1/2` as a root boundary. Exact substitution gives
`F=F'=0`. Original GCAD on `F>0` returns the same raw intervals; merging
returns `(0,1)`, and equality input is rejected. Thus this is not a Rust port
regression. Neither an exceptional hypersurface nor a list of root bounds
alone is a complete critical-locus stratification.

The native nonvanishing API cannot certify `F^2+(F')^2` for this example.
For `F=(2*x-1)^2+1`, a Bernstein certificate succeeds and independently
replays. Such a certificate proves absence; failure is inconclusive.

For integration, the partition identity must hold in a common convergent
regulated family, then continue with the causal phases and endpoint terms.
The PDF's explicit two-half integral retains `+i*pi` in the limit without a
separate point integral. Measure-zero omission alone would not justify
discarding a singular boundary contribution after taking the causal limit.

## Scalar experiments and scope

Native graph exports supply homogeneous `U,F`. Symbolica imposes the exact
kinematics, fixes the last positive projective parameter to one, differentiates
and adjoins `sat*product(x_i)-1`. Euler's identity supplies the omitted
homogeneous derivative on `F=0`. Coordinate boundaries are outside this test.

At the published scalar benchmark points, bubble, triangle, box, massless
sunrise and massive sunrise reduce to the unit ideal. Kite's native grevlex
basis contains a positive multiple of `1+x0+x1+x2+x3+16*x0*x3`, excluding
positive real solutions despite a non-unit complex ideal. An earlier lex
attempt was stopped after 149.695 seconds and supplies no conclusion.

The positive controls are the equal-mass bubble at `m^2=3,s=12` and sunrise
at `m^2=1,s=9`. Native solving finds the all-one projective coordinates.
Direct homogeneous-gradient substitution gives zero and `U` is two and three.
These are diagnostic threshold points, not replacements for benchmark rows.

Native exact Groebner computation establishes ideal membership. Tests that
the result is a Groebner basis and inputs reduce to zero are useful checks;
alone they would not verify reverse ideal containment for a forged basis.
We claim independently replayed native calculations, not an exported formal
proof certificate for each Groebner computation.

For kite, a further native exact matrix solve constructs linear rational
multipliers of all five original homogeneous derivatives, restricted to
`x4=1`, that reconstruct the positive polynomial exactly. The published
compact receipt includes all multipliers and derivatives. This independently
checkable obstruction does not depend on Groebner ideal membership or
saturation. Euler's identity connects the fifth derivative with the chart
critical equations. The final native grevlex calculation took 4.22 ms and
the explicit identity computation took 8.23 ms.

## Crown experiment

The polynomial is transcribed from
[arXiv:2407.13738, §3.3, eq. 41](https://arxiv.org/html/2407.13738#S3.SS3),
not an identified pre-existing symGCAD fixture. The native exact identities are

```
x1=1; x0=a, x3=b, x5=c, x7=d
x2=ab+u, x4=ac+v, x6=ad+w
F=(s+t)*d*u*v-t*c*u*w-s*b*v*w
det(H_normal)=2*s*t*(s+t)*b*c*d
U|_(u=v=w=0)=(a+1)^3*(bc+bd+cd+bcd)
```

This invertible unit-Jacobian chart covers the strict positive projective
interior. For `s*t*(s+t)!=0` and positive `b,c,d`, the normal-gradient linear
system forces `u=v=w=0`. Conversely, all original derivatives vanish there.
The complete interior family under those guards has four projective tangent
coordinates and three normal directions. Exceptional kinematics and boundaries
remain unclassified. The full seven-dimensional isolated-point construction
cannot be applied directly.

GCAD certifies the reduced positive four-coordinate domain and `U>0`, not
the original equality system. One cell verifies, and a changed sign is
rejected. At `s=1,t=-1/5,x_i=1`, exact Hessian rank is three and `U=32`.
The prospective 24 normal orthant/wedge charts are not a global final-sector
count or the paper's distinct 24 ordered-ratio dissection pieces.

## Reuse, pins and reproduction

Public APIs, source/tests and executable probes cover native polynomial
conversion, differentiation, substitution, Groebner construction/reduction/
solving, Numerica matrix determinant/rank, and symGCAD solve/verification and
domain certificates. Relevant sources are Symbolica `poly/groebner.rs` and
`poly/polynomial.rs`, Numerica `tensors/matrix.rs`, and symGCAD `algebra.rs`,
`problem.rs`, `output.rs`, `verify.rs`, `domain.rs` and solver tests.
Native algebraic solving supports zero-dimensional ideals; the existing
generic-open solver does not supply general equality-stratum orchestration.

Runtime revisions are kept distinct:

- Symbolica/native numeric owner: `ae0c82b7bda670fdb9f6229226bd47b13bb49a20`.
- API probe symGCAD: `3baffa7134334c62b45a6b8df9198004060be715`.
- Crown prebuilt symGCAD: `bef2a9e6a7441aee4c0ba69acb80e68959e78928`.
- Separately audited local source: `36bfd9ebedf617a884518458d51cf1c2adcd30bd`.
- Original GCAD: `fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e`.

The private build scripts record native commands and linked rlib hashes.
`crown/README.md` and `scalars/README.md` give reproduction details. With the
recorded warm artifacts and configured Symbolica license:

```bash
target/no-deformation-critical-loci/crown/build-r1/probe /tmp/crown-replay
target/no-deformation-critical-loci/scalars/probe-grev kite \
  target/no-deformation-critical-loci/scalars/kite-export/symanzik.json
```

All new mathematical calculations are native Rust. Python only orchestrates
processes/receipts and invokes Vitaly's original implementation for comparison.
Compact outcomes and source/receipt hashes are in
`docs/gcad-critical-loci-results.json`; raw outputs and build products stay
untracked. Diagnostic milliseconds are not end-to-end performance claims.

## Independent review and acceptance

The API/scientific reviewer had separate scratch ownership from the crown and
scalar agents. It checked the paper transcription, exact converse argument,
projective/saturation logic, positive-domain guards and dimensions; fresh
processes replayed the crown and all eight successful scalar cases. The
Typst discussion passed that review. The coordinator compiled the PDF and
visually checked the two added pages.

No global critical search, new endpoint resolver, crown integration, exhaustive
corpus search result or double-box classification is claimed. Production
sources, dependencies and numerical benchmark overview remain unchanged.
