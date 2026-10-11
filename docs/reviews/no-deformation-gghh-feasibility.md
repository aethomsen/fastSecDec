# Independent bounded outcome audit

All three attempts stopped at the external wall limit before returning a raw SolveResult. No native completion status, cell inventory or independent verification was produced. The result is unresolved within these bounds, not an infeasibility proof, an empty domain, a zero integral or a general-resolver failure theorem.

| Attempt | Child wall seconds | wait4 peak RSS MiB | Last completed native projection checkpoint |
| --- | ---: | ---: | --- |
| Two-parameter s,t chamber | 60.036883 | 179.628906 | After x5: 15 polynomials, 406 terms, maximum per-axis degree 6 |
| Exact rational point, original order | 180.043351 | 198.835938 | After x4: 22 polynomials, 1393 terms, maximum per-axis degree 14 |
| Exact rational point, one native alternate order | 180.037346 | 169.480469 | After x2: 25 polynomials, 1643 terms, maximum per-axis degree 14 |

Each receipt records child exit -9 and `wall_cap`; the orchestration outcome is `solve_outer_wall_timeout` with native_status=null and independently_verified=false. Supervisor exit 0 means that observation succeeded, not that its child solve completed. Stage totals include wrapper overhead and are about 60.071, 180.080 and 180.075 seconds. The small overrun reflects polling/termination/reaping, not a further solve allowance. No verifier got a new independent deadline. There are no raw-result or verification files to mistake for completed evidence.

Native checkpoints are partial projection telemetry. Their polynomial counts, coefficient bits and elapsed fields do not certify completed projection or cells and do not give the duration of the interrupted operation. `max_degree` is the maximum degree in any one variable. The native optimized cache retains the original reduced variable map: in the alternate run index 1 denotes x2, even though its lifting-order position differs. This audit checked that distinction against `prepare_optimized`.

Resource semantics need one clarification beyond the pre-run review: in this direct optimized-library path, `limits.memory_mib=8192` is not an enforced process RSS/AS cap. The native eager projection uses a separately configured cache budget, while the native 45/150-second time check appears in lifting after eager projection. The enforced study bounds were the external 10 GiB address-space limit and pooled 60/180-second process deadline. Thus the actual runs remain within the user's memory maximum; none hit the initial memory cap. Parent-authorized conditional memory escalation was not triggered. Neither a bigger RAM allowance alone nor a longer timeout is claimed to solve the observed problem.

The same original prepared problem is retained in both fixed-point runs. Independently comparing their JSON objects confirms that only `solver.order` changes to `[x0,x5,x4,x3,x2,x1,x6]`; constraints, split polynomial, exact parameters, variable groups, homogeneous anchor, positivity settings and resource configuration are identical. That order appears in the native `cheap_seed_orders` output for the unchanged x0 anchor. The proposal pass takes 0.008602 seconds, performs native normalization plus input-feature ranking and no projections. Seven anchor proposals were inspected cheaply, but only one alternate order was solved. No selection of an easier source region occurred.

The r4 helper changes only the verifier's expected full order to `effective_order`, additionally retaining the exact original nonblock prefix. The native positive-scale reconstruction still establishes anchor/order and original-source coverage if it returns a complete result; no such result returned here. All explicit fields of the three reproducible TOML cards were compared to the actual prepared JSON. The existing CLI commands are algorithm/input equivalents, with their own resource wrapper, not claimed bit-identical timing reproductions.

The helper build uses opt-level 0/thin-LTO/codegen-units 16 with the pinned existing release a113/ae0 dependency artifacts. It is not presented as a fresh optimized CLI benchmark. Setup/link failures and setup-only timeout remain separate from these solve attempts. No speed ratio, root-only cost or asymptotic feasibility claim is inferred.

Source/API review and outcome-accounting PASS at this bounded feasibility scope. No duplicate native runs or shared-source edits were performed. This leaves generic decomposition, exceptional strata, endpoint resolution, full tensor density and numerical integration unresolved for D05. An actual issued unsubtracted source-chart representation may be a separate future experiment, with its partial-source scope retained; it is not supplied by these results.

Independent publication review also passes for the public JSON, reproduction
cards, Typst tables and rendered PDF page 15. The final review receipt is
`bd1bd7d8815e7ad5e62f0d1f22f94ec3c424fccedfe58d7d861486e1f8eb24b2`.
The negative-t inequality was corrected after visual inspection. All measured
rows describe these three initial scouts; the separate extended one-hour,
100-GB rational-point trial is explicitly excluded from their results.
