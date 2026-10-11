# Causal validation is opt-in

The user's latest runtime policy overrides the original Phase B default.
New native settings, omitted JSON/TOML policy fields, the CLI, and the thin
HEPKit Python constructors now default to `off`. Explicit persisted `always`
and `pilot` policies remain effective, and CLI overrides still take precedence.
This changes optional checking, not the deformation prescription or production
sampling streams. Solver termination, nonfinite failures, numerical stability
checks and precision rescue remain active.

An independent source/API audit verified forwarding and persisted-policy
behavior. Two native contour settings tests and four explicit validation tests
pass, along with four CLI policy tests and strict workspace/all-target Clippy.
The locked native Python leaf build passes. The Python constructor test source
covers omitted and explicit policies; a rebuilt community host wheel was not
executed for this change, so that runtime gate is not claimed.

Every new Overview profile and integration command explicitly requests `off`,
including campaigns using the previously frozen executable. Native receipts
must report no causal pilot and zero causal-check counts. The independent
profile review rehashed and recomputed all 17 currently selected measurements
and rejected mutated always-policy, pilot-present and nonzero-check receipts.
Earlier checked-policy measurements are retained as separate diagnostics.

The distinction matters for profiling: optional dynamic validation can execute
inside a native f64 callback, while fixed checks occur outside that timer.
Only the new common off-policy measurements enter the Overview's f64 columns.
The checked-wall column continues to include mandatory stability and rescue
work, with causal validation excluded.

Independent API review identity:
`a3eb43b311364f20df1045c0d8349bbd9acd42153e81677721412fe50c08e748`.
Detailed commands and raw local receipts are untracked; the public report keeps
the compact measurements and their provenance.
