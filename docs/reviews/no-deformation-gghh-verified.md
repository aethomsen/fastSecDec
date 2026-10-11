# Verified fixed-point D05 generic geometry

The native symGCAD solver returned `CompleteGeneric` for the full signed F
decomposition of the D05 double box on the positive orthant. A fresh process
independently verified all **644,699 original generic-open cells**. This closes
the fixed-point generic geometry feasibility gate; it does not produce a
threshold artifact, endpoint subtraction or an SD integration benchmark.

The exact point is s=1,000,000, t=-97,077, u=-871,673, mH²=15,625 and mt=345/2
(mt²=119025/4). It is the documented nearby rationalized physical point, distinct
from the original angle with t=-484375+100000√15. The exported native graph,
denominator order, masses and full F sign split are retained. Geometry is
independent of the numerator; this result does not evaluate the contracted ggHH
amplitude or transfer its old polarization values to the new point.

The successful recipe uses homogeneous anchor x0, the complete block x0…x6,
native `monotone` engine, given order `[x0,x1,x3,x4,x5,x6,x2]`, and the default
optimized base with `negative_target=false`. Inspection of all six possible
first ratio axes admitted a conditional quadratic peel only for x2. Its native
leading-coefficient and conditional-linear certificates hold on the entire
positive orthant, so both signs of F remain covered. The solver issued its own
evidence from the original Problem; no inspection result was substituted as a
certificate. The source, domain and masses were unchanged by this engine/order
choice.

| Measured stage or resource | Result |
| --- | ---: |
| Native solver statistics: total | 36.356 s |
| Native projection / lifting | 0.446 s / 23.186 s |
| Solve helper, including raw JSON serialization/write | 152.310 s |
| Whole solve process | 156.526 s |
| Fresh verification process, including decode | 142.072 s |
| Pooled solve and verification | 298.601 s |
| Maximum child `wait4` peak RSS | 10.688 GiB |
| Maximum sampled concurrent owned-cohort RSS | 11.875 GB decimal |
| Raw native JSON | 5.284 GB decimal |

The cohort measurement includes the concurrently running original optimized
trial and its supervisor; it is a sampled aggregate, distinct from a child's
kernel-reported RSS high-water mark. The native solver reports 912,150 visited
stacks; the verifier reports 447,886 stacks and 807,325 rejected branches. These
are differently scoped counters. The nested base's 464,265 cells are not the
original reconstructed cell count.

Raw `SolveResult` was saved before fresh verification. The helper required exact
prepared-Problem equality, the complete declared order and unchanged prefix,
`CompleteGeneric`, and then the native verifier's positive result. Both processes
exited successfully. The bound was a pooled 3,600 seconds and 100 GB decimal of
process address space plus monitored aggregate RSS, including the concurrent
trial. Native configuration was 65,536 MiB, one worker and no cell cap. Native
configured limits alone do not enforce the external bound during every eager CAS
operation. The Rust helper used opt-level zero orchestration over opt-level three
release dependencies; a live stack audit located the expensive CAS in the native
library. This is not a matched completed-time ratio against the original order.

The pinned graph is symGCAD `a1132d4f4545c7784b3ec61239a05485e544b403` with
Symbolica/Numerica `ae0c82b7bda670fdb9f6229226bd47b13bb49a20`. The
[complete native card](../../examples/no_deformation/gghh_feasibility/fixed-monotone-prepared.toml)
records the exact source and recipe. Equivalent public operations are
`symgcad solve prepared.toml --output raw-result.json` followed by
`symgcad verify raw-result.json`, with the pooled external bound applied to both.
Use a fresh private checkpoint directory for a cold reproduction; changing the
owner dependency graph or CLI resource wrapper changes the measured setup.

Native checkpoints are trusted local computation caches with key, schema,
backend, domain and payload checks. Factor restoration replays its product;
cached resultant/discriminant restoration does not recompute the entire
operation. Independent final verification remains essential. Native aggressive
merging provides compatibility cells while retaining raw seams and coverage;
FastSecDec currently consumes the certified raw inventory. A smaller merged
count cannot replace it without a checked density/branch consumer.

The sealed successful handoff is `db8a646630616baed7bf561f19e0a4ec36a6b1af831fc60c517eea3d2a98974e`;
the independent outcome audit is `dbdde88277b57b93319c1da5dbe782f1a555ca18fc6d3925d70f6aeb1507b3f0`.
Raw proof SHA-256 is `6f0df3b9fa950b70a1c6bfeab150db5fac0b45a0a6789ecf5bef79adeef3fa7f`.
Large raw proofs, caches and build products remain untracked. The unchanged
original-order optimized baseline separately reached its external wall cap at
3,600.598 seconds, with no raw result or verification. Its child peak RSS was
897.449 MiB; sampled original-cohort peak RSS was 957.395 MB decimal. Its last
completed projection prefix remained 22 polynomials and 1,393 terms, but native
operation-cache records show intermediate work beyond that prefix. This is an
incomplete attempt, not an infeasibility claim or a completed-time speed ratio.
Projective density transport,
closed faces and infinity, endpoint resolution, continuation and numerical
integration remain separate required gates.
