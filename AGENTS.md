# FastSecDec implementation guidance

Read `FIRST_PHASE_PLAN.md` before changing scope or architecture. Its original
prompt, subsequent user requirements, and acceptance gates remain authoritative.

- Implement the first phase in Rust. Use HEPKit's native graphs and Linnet
  primitives; reuse Symbolica/Numerica algebra, numeric types, and evaluators.
- Before new algebra or graph helpers, check public APIs, source/tests, and a
  focused Rust probe. Record the reuse evidence and actual missing operation.
- At each major subsystem milestone, assign an independent HEPKit integration
  and ecosystem-reuse audit. Review native objects, public APIs, status/error
  boundaries, caller-owned execution, and existing numerical reference APIs,
  including one-loop masters and reduction. Fix avoidable duplication before
  accepting the milestone; record findings and unresolved limitations in
  `docs/reviews/` and update `docs/REUSE_AUDIT.md`.
- The root agent primarily coordinates. Delegate implementation, research,
  independent review, debugging, and performance tasks to subagents with explicit
  file ownership. Do not edit another active agent's files without coordination.
- Keep FastSecDec on local `main` and commit validated milestones. The user's
  2026-10-06 clarification moves QMC into FastSecDec for now; retain the existing
  Numerica branch and PR as history, while keeping ordinary Havana MC in
  Numerica. The user authorized pushing
  validated FastSecDec milestones to `https://github.com/alphal00p/fastSecDec`.
  Push `main` there after milestone checks. The user also authorized publishing
  the finished Numerica QMC feature branch as a PR against its main branch;
  run its readiness checks first. The 2026-10-11 instruction removes the
  requirement to request or tag Ben Ruijl on future PRs. Other
  reference repositories remain unpublished except for the subsequently
  authorized, tested changes on GammaLoop's `feynkit` branch, the HEPKit bridge
  PR in symbolica-community, and the 2026-10-06 authorization to update HEPKit
  and OneLOop sources in `/common/dev/`. Use authenticated
  `ValentinHirschi` for all subsequent publishing and commit as
  `ValentinHirschi <valentin.hirschi@gmail.com>`. Ask the user to authenticate if
  the active GitHub account differs; never silently publish under another user.
- Never track `DO_NOT_PUSH_FOR_REFERENCE_ONLY`, build artifacts, caches, or raw
  benchmark output. Keep dependency fixes small and evidence-based; preserve
  existing worktrees and unrelated host workloads.
- Keep libraries caller-driven, and CLI/status presentation independent of
  numerical code. No library-owned QMC worker pool or integration loop.
- Keep substantive HEPKit bindings in FastSecDec's isolated `bindings/python`
  Rust crate, with demo helpers/assets/tests in this repository. Community only
  links/registers the module and supplies necessary reexports/stubs. The native
  core and default CLI must remain free of Python/PyO3 dependencies.
- Notebook generation and integration require separate explicit actions. Retain
  native generated objects for lazy sector exploration; automatic presentation
  cells must not generate, compile, contract numerators or start sampling.
- Scientific equivalence is required; legacy options, sector numbering and cache
  formats are not. Preserve complete Laurent-vector cancellations and covariance.
- Prefer small modules with clear responsibilities. No alternate CAS, DOT parser,
  physical graph type, or general-purpose graph canonicalizer.
- Run meaningful scientific tests and independent reviews at milestone boundaries.
  Record unimplemented features and unverified performance honestly. Never turn
  a numerical failure or inconclusive scalelessness check into a zero result.

Use `nix-shell` for a Rust >=1.96 environment with native dependency build tools.
See `docs/DEVELOPMENT.md` for local dependency identities and test commands.
