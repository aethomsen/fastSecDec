# Timed integration and complete work accounting

`integrate --max-seconds 600` (or `integration.max_seconds = 600`) gives the
CLI an invocation-local deadline. It is supported for serial QMC/Havana and
ordinary discrete MC. The default is absent; unsupported ordinary schedulers
reject the option. Native integration sessions remain caller-driven.

The deadline uses the existing integration elapsed clock. The serial driver
includes resident loading and reloading within that clock. Ordinary execution
receives already restored kernels before starting its clock. Artifact-loading
and process-wall measurements remain separate. This is an operational allowance,
not a new statistical design and not the minimum sector residence time.

At the deadline the serial coordinator stops issuing reservations, continues
admitting complete worker returns, and waits for their completion messages.
Discrete MC finishes its already assigned wave and starts no new wave. Neither
path sets the ordinary cancellation signal merely because time expired. Each
native return still passes the existing lease, epoch, duplicate-return, replay
and covariance checks. No seed, coordinate or proposal is changed by the timer.

The final report records the requested time, when the deadline was observed,
outstanding work and drain duration. `counts_complete` is true only after clean
completion; cancellation, worker failures and statistical failure clear it.
Ctrl-C retains immediate cancellation behavior. Deadline exhaustion reports
`WorkLimit`, even if the additional drained work happens to meet the tolerance.
An early target or explicit allocation limit can stop before the deadline.

Logical sector calls count evaluated coordinates, including reported pilots and
discarded work. Backend precision retries are reported separately. Accepted
production work is a different counter. Invocation operational counts restart
on resume, while the native accepted estimate and its weight diagnostics retain
checkpointed production. Changing the time allowance does not change checkpoint
statistical identity. Historical interrupted runs may omit unpublished worker
tails and remain lower bounds rather than being relabelled as exact totals.

The independent source review covers the coordinator, discrete driver and
deterministic native wave controls. Process tests exercise a deadline during
active resident work, complete late admission, exact native call accounting,
early exhaustion and resume with a changed time allowance. The wave test checks
that a timed latch does not abort native batches, duplicate admission is refused,
and explicit cancellation still interrupts work. Saved-result and presentation
checks retain the complete native vector/covariance and typed weight diagnostics.

The consuming milestone records final build and test receipts separately. A
parallel CLI test run exposed transient journal-lock reacquisition while other
process tests were spawning; the full sequential CLI suite passes. The original
failure is retained rather than omitted from the evidence. This timing change
does not alter the journal or weaken its locking checks.

The consuming checks pass: 140 active CLI tests (eight ignored), eleven final
native diagnostic groups, twelve saved-result groups, three CLI saved-result
groups and two real timed-process tests. Strict workspace/all-target Clippy and
formatting pass. The optimized release is
`cf9eb45032498fb9def5a8f09c074653fa1f38e217fcfb0b0af3aae586d0fcf8`.
Separate two-worker, 0.5-second smoke runs retain 393,216 serial-QMC and 786,944
discrete-MC calls, respectively, with complete counters, native weight/norm
metadata and failure-free `WorkLimit` outcomes. These smoke measurements test
the contract and are excluded from the performance comparison.
