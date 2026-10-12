// Scratch experiments only; production algorithms and benchmark rows unchanged.
#let grad = math.op("grad")
= Generic CAD and critical loci: exact scratch experiments

== A generic decomposition is not a complete stratification

*Full CAD can represent lower-dimensional solution cells. The present generic
CAD interface does not return them as cells.*
#link("https://github.com/magv/gcad#api")[Vitaly's original implementation]
recurses between roots and accepts strict inequalities; its API states that
the original algorithm's separate boundary list is not computed. The Rust
port has the same open-cell contract, with retained exceptional polynomials
and independently checked generic coverage. This is a documented limitation
for equality-locus queries, not evidence of a porting regression.
#link("https://reference.wolfram.com/language/ref/GenericCylindricalDecomposition.html")[Wolfram's documentation]
likewise distinguishes generic decomposition, with hypersurfaces containing
omitted solutions, from full cylindrical decomposition. Knowing a hypersurface
that contains exceptions is not the same as resolving all its solution strata.

A native executable control uses $0<x<1$, $F=(2x-1)^2$ and $F'=8x-4$.
symGCAD returns and verifies two open cells, $(0,1/2)$ and $(1/2,1)$, with
sign vectors $(+,-)$ and $(+,+)$. Nevertheless, exact substitution gives
$F(1/2)=F'(1/2)=0$. The point survives in the root-boundary description;
there is no returned zero-dimensional cell identifying it as critical.
An equality constraint is explicitly rejected by this API. Thus a completed
generic solve and the absence of zero-sign cells cannot certify absence of
stationary zeros. A separate executable run of Vitaly's original code on
$F>0$ returns the same two raw intervals; its optional merge joins them into
$(0,1)$, absorbing the zero seam as its documented semantics allow. An
algebraic zero seam must not be forgotten merely because signs agree on
either side. This original-code probe took 7.8 ms.

For integration, this contract is usable if the partition identity holds in
a common convergent regulated family and its endpoint terms are continued
with the correct causal phases. The zero set need not be a separately sampled
integration cell, but its singular neighbourhood cannot be discarded.
Measure zero alone is insufficient after taking a distributional boundary
value. The prior closed-face and analytic-continuation obligations therefore
remain essential; these probes do not supply a missing general resolver.
For example, splitting the following integral into its two open halves,
initially for $op("Re") epsilon<0$, gives
$
 integral_(-1)^1 (x-i 0)^(-1-epsilon) dif x
 = (1-e^(i pi epsilon))/(-epsilon) arrow.r.long i pi
 quad (epsilon arrow.r 0).
$
Correct continuation retains the imaginary boundary contribution without a
separate point integral. Dropping the phase or replacing the boundary value
with an ordinary divergent real integral would lose it.

== How native algebra and GCAD can cooperate

A targeted critical-locus analysis can solve the equations first, then use
GCAD for the remaining inequalities. Native Symbolica differentiation,
Groebner bases and algebraic solution facilities can reduce the critical
ideal. On a positive projective chart, adjoining
$h product_i x_i-1=0$ excludes coordinate boundaries from this interior
problem. A unit ideal proves absence. A non-unit ideal requires real-domain
analysis; it is not evidence that a physical solution exists.

For an exact parametrized family, substitute into every original equation,
prove the reverse implication under explicit nonzero guards, and certify the
free-parameter domain and $U$ with symGCAD. All denominator-zero and exceptional
kinematic cases remain separate obligations. Where no parametrization is
available, a complete equality/section solver or certified recursion on
exceptional strata is needed. Calling generic CAD again in the same ambient
space does not solve that problem.

Another sufficient absence test is an exact nonvanishing certificate for
$P=F^2+sum_i (partial_i F)^2$ on the real domain. The native domain-certificate
API verifies such a certificate for the control $F=(2x-1)^2+1$ and correctly
finds none for the stationary-zero control above. Failure to obtain this
certificate is inconclusive. Neither a numerical minimum nor a ball enclosing
zero establishes an exact equality.

== Existing scalar points and threshold controls

The following scratch runs reuse native exported graph polynomials and exact
kinematics. They concern the *strict positive projective interior*; boundary
Landau configurations and kinematic chambers were not classified. The last
Feynman parameter is fixed to one. Homogeneity and Euler's identity connect
the chart equations with the full homogeneous gradient on $F=0$.

#block(breakable: false)[
#set text(size: 8.5pt)
#table(
  columns: (1.35fr, 1.35fr, 2.4fr), inset: 5pt,
  stroke: 0.35pt + rgb("cad5de"),
  fill: (x, y) => if y == 0 { rgb("e7eff5") } else if calc.even(y) { rgb("f5f8fa") },
  table.header([*Case*], [*Exact point*], [*Interior critical system*]),
  [Massive bubble], [$m^2=3, s=16$], [Absent: saturated ideal is $(1)$],
  [Massive triangle], [$m^2=1, s=5$], [Absent: saturated ideal is $(1)$],
  [Massive box], [$m^2=1, s=5, t=-1$], [Absent: saturated ideal is $(1)$],
  [Massless sunrise], [$s=1$], [Absent: saturated ideal is $(1)$],
  [Massive sunrise], [$m^2=1, s=16$], [Absent: saturated ideal is $(1)$],
  [Massive kite], [$m^2=1, s=16$], [Absent: ideal contains a strictly positive polynomial],
  [Bubble threshold control], [$m^2=3, s=12$], [Found: $(x_0,x_1)=(1,1)$, $U=2$],
  [Sunrise threshold control], [$m^2=1, s=9$], [Found: $(x_0,x_1,x_2)=(1,1,1)$, $U=3$],
  [Three-loop crown], [$s=1, t=-1/5$], [Found: four-dimensional projective family],
)
]

The triangle has two massless external legs; the box has massless external
legs and $u=-4$. The kite has five equal massive propagators. For the kite,
native grevlex reduction gives an ideal member proportional to
$1+x_0+x_1+x_2+x_3+16x_0 x_3$, strictly positive in this domain. Its complex
critical variety need not be empty. A second native calculation verifies an
explicit linear-polynomial combination of the five homogeneous derivatives
equal to this positive polynomial, independently checking the obstruction.
The final grevlex reduction took 4.22 ms and this identity took 8.23 ms;
an earlier lex-order attempt was stopped after 2.50 minutes. The other scalar native
calculations took less than 9 ms each. These are small diagnostic calculations,
not timings for integral generation or a generic critical-locus solver.

The threshold controls also show why “only three loops can have a pinch” is
too broad. It holds neither for arbitrary masses nor for exceptional external
kinematics. Among these tested benchmark points the crown is the positive
case; the claim does not cover the entire symGCAD corpus or the double box.

== The crown: a critical manifold with three normal directions

The massless crown has *three loops and eight propagators*. No crown fixture
was identified in the searched symGCAD benchmark inventory, so the scratch
input transcribes the polynomial in
#link("https://arxiv.org/html/2407.13738#S3.SS3")[Gardi, Herzog, Jones and Ma, §3.3].
It is also the three-loop non-planar example in
#link("https://arxiv.org/html/2506.24073v1#S4.SS1.SSS6")[Olsson and Stone, §4.1.6].
With zero-based parameter labels, set $x_1=1$ and introduce the invertible
polynomial chart
$
 a=x_0, quad b=x_3, quad c=x_5, quad d=x_7,
 quad x_2=a b+u, quad x_4=a c+v, quad x_6=a d+w.
$
Native substitution and matrix arithmetic verify exactly
$
 F=(s+t)d u v-t c u w-s b v w,
 quad det H_perp=2s t(s+t)b c d.
$
For $s t(s+t)!=0$ and $b,c,d>0$, the normal-gradient equations therefore
force $u=v=w=0$. Substitution makes $F$ and all eight original derivatives
zero. This proves conditional completeness of the positive interior family,
not merely the existence of a candidate point. It has four free positive
projective coordinates, and
$ U|_(u=v=w=0)=(a+1)^3(b c+b d+c d+b c d)>0. $
GCAD then verifies the reduced four-parameter domain as one positive-$U$
cell; changing its claimed sign makes verification fail. This call certifies
the reduced inequalities, while the exact algebra locates the critical locus.

At $s=1,t=-1/5$ and all $x_i=1$, the native calculation gives $U=32$ and
Hessian rank three. The full seven-dimensional projective Hessian is
degenerate; the previous isolated-point construction does not apply directly.
The natural extension is a transverse, Morse--Bott-type treatment: retain the
four tangent parameters and resolve the three normal directions, checking
uniform bounds and the boundary/exceptional strata. For a regular local
normal patch the earlier orthant/wedge count would start at $3 times 2^3=24$;
this is not a global sector-count prediction or the paper's distinct
24-ordering dissection. No crown integrand was generated or integrated here.

The entire crown scratch process took 0.0432 s and 12.7 MiB peak RSS; reduced
GCAD solving and verification took 7.36 ms and 2.31 ms. It used existing warm
native libraries, not a fresh latest-revision build. Scalar algebra uses
Symbolica `ae0c82b`; the API control uses symGCAD `3baffa7` and the crown uses
`bef2a9e`. The local `36bfd9e` API/source contract was separately audited.
Exact revisions, source hashes, commands, independent review and limitations
are recorded in `docs/reviews/gcad-critical-loci.md`. No production algorithm,
dependency revision or numerical benchmark row changed.
