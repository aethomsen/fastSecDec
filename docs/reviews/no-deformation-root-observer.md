# Native prepared-root observation

The regular algebraic boundary path uses a defining polynomial and certified
selector at any admitted degree. A diagnostic on the degree-six paired family
found that some high-precision native refinements spend many iterations closing
their numerical bracket after reaching an accurate candidate. This observation
does not prove a solver defect or a production performance improvement.

API, source and focused executable checks found no existing way to inspect a
finite native proposal and stop while retaining its tracked value. The narrow
owner addition is [Symbolica PR #71](https://github.com/symbolica-dev/symbolica/pull/71),
authored and committed as ValentinHirschi. It depends on the prepared-root
[PR #56](https://github.com/symbolica-dev/symbolica/pull/56); the observer-only
commit is `1690f1bad353a0b9f2b7ba6aba431ce11abc5850`. Its draft diff also contains
that unmerged prerequisite. Neither the prerequisite branch nor FastSecDec's
dependency pins were changed.

`nsolve_bracketed_observed` borrows the finite proposal, corrected coordinate,
bracket, tolerance and counters. A standard `ControlFlow` callback can return
`CallerStopped`, preserving the actual native values and an owned caller payload.
It does not assert convergence or validate that payload. A Newton proposal may
lie outside the numerical bracket. The caller must establish the property it
needs for the actual returned coordinate.

The old API supplies an always-continue observer to the same implementation.
Arithmetic, callback order, original termination and reached errors are retained.
Tracked input error is neither replaced nor reset. The observer introduces no
solver, interval arithmetic, evaluator, cache or persistent worker state.

## Evidence and adoption boundary

Actual owner Cargo tests pass all five new observer controls and ten inherited
prepared-root controls, including Eager/JIT evaluation. Separate exact-source
native tests pass 17 groups, with a frozen-original comparison across five
number domains and both stopping policies. This checks callback coordinates,
returned fields, uncertainty, counters and failures. The portable Malachite/Astro
numeric harness passes all 15 groups. Strict changed-source/test lint and
formatting pass. An additional owner Cargo Clippy attempt stops on 131 existing
diagnostics in the unchanged Numerica member; whole-workspace lint is not claimed.

The independently reviewed handoff has SHA-256
`04219b58484a778b20137e4d6c10a10c47b34aff4df96c1b557ae1fcedec706c`.
The coordinator verified all 47 evidence pins and the three source preimages and
postimages before committing and publishing. GitHub rejected the formal reviewer
assignment for lack of repository permission; review was requested from
`benruijl` in the [PR comment](https://github.com/symbolica-dev/symbolica/pull/71#issuecomment-6107147204).

A separate probe uses native ball arithmetic to certify a root of the exact
polynomial formed from the actual rounded coefficient centers. It checks strict
endpoint signs and derivative bounds independently, and includes the actual
mapped return coordinate in the enclosure-distance test. Independently rounded
endpoint controls cannot supply this proof. This certifies nominal termination,
not the original physical polynomial, incoming uncertainty or causal geometry.

Production adoption remains separate work. It must retain real and imaginary
input uncertainty and existing whole-vector acceptance, and measure complete
callback cost on inexpensive roots as well as long bracket tails. FastSecDec's
current callbacks and precision policy remain unchanged; no sampling or variance
improvement is claimed by this API publication.
