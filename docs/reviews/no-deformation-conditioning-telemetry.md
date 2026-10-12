# Conditioning timing in integration diagnostics

The status layer previously omitted the native evaluator's existing
`EvaluationTimings.conditioning` bucket. This change forwards it through ordinary
evaluations, weighted replay reports, failed attempts and worker merges. It adds
no timer, numerical evaluation, precision-policy change or sampling work.

This bucket includes initial tracked f64 conditioning and tracked
arbitrary-precision evaluations used to validate algebraic precision rescue.
Raw arbitrary-precision evaluations remain in the existing `arbitrary` bucket.
The combined conditioning bucket cannot distinguish those two tracked portions.
Point-level rescue counts, additional weighted replays and evaluator-call counts
have different meanings. The timings exclude setup and other caller overhead;
their sum is not a complete wall-time breakdown. An absent or default bucket
means unrecorded or disabled work, rather than a measured zero cost.

`WeightedEvaluationContext` already reports the cumulative native timing delta
across primary evaluation and weighted replay. Status records that delta once.
The CLI and thin HEPKit batch adapters replace all four timing buckets with the
complete native batch delta, including failed or speculative attempts. The
serial coordinator includes conditioning in its evaluator-time diagnostic;
sampling residence and worker clocks already use their separate elapsed clocks.
HEPKit exposes the native bucket through the existing typed timing wrapper.

The existing checked merge operation rejects counter overflow without changing
the aggregate. Native worker-local counters keep their existing saturating
semantics. In-tree checkpoints, IPC and saved results use JSON. Historical serial
checkpoints authenticate the decoded, re-encoded JSON, so the new field both
defaults on read and skips serialization when empty. This preserves historical
payload bytes. Nonzero observational timing changes the payload checksum but
not mathematical source, kernel or sampling identities. This is not a promise
of compatibility for external positional bincode consumers of the public type.

Four new controls exercise worker/failure aggregation, every conditioning-counter
overflow, historical JSON bytes and checksum, and two genuine native weighted
evaluations of a logarithmic Taylor-cancellation fixture. The last control
compares recorded reports with native cumulative metrics and checks the rescued
coefficient. The consuming gates pass: 14 status controls, 12 saved-result
controls, 12 CLI execution controls and the serial checkpoint control. Independent
native-reuse, persistence and consumer reviews pass.

The native Python crate typechecks with its locked dependencies. That feature
combination also required an explicit `BTreeSet` collection type in retained
certificate role comparison: the additional archive implementations make the
previous inferred type ambiguous. This annotation changes no proof semantics.
The Python presentation test parses successfully; a live Python presentation
test run is not claimed. Core/CLI strict all-target Clippy and both workspace
formatting checks pass.

Evidence is retained under `target/no-deformation-conditioning-telemetry`, with
immutable source reviews and consuming gates in `import-r1`. This milestone
adds diagnostic coverage, not a new sampling-performance measurement.
