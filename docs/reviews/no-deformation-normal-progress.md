# Normal endpoint preparation: sparse native progress

The physical source-bound endpoint probe previously exposed only entry to
`NormalNumeratorJets::prepare`. That did not distinguish root-program setup,
face-denominator certification, native series, or source-definition collection.
The new `prepare_with_progress` reports typed started/completed stages and
inventory indices around those operations. The ordinary entry point remains a
compatible no-observer adapter.

The observer is separate from the existing cancellation callback. It is not
invoked for every expression node. Native calls remain cooperatively bounded,
not asynchronously interruptible. Observer time belongs to the caller's
existing budget; errors and cancellation do not invent completion events.
Snapshots contain no expressions, threads, clocks, logging or proof copies.

The independent source review found no changes to root selectors, source/chart
identity, rational reconstruction, denominator certificates, native Symbolica
derivatives or series, branch admission, and retained coefficients. The shared
mixed path retains its existing signatures through private no-observer adapters.
This reuses the existing native `Work` budget/cancellation owner and typed
progress convention; it introduces no algebra or AD implementation.

Validation: 23 focused numerator tests passed, including exact coefficient and
definition equality, compiled native Float evaluation equality, nested sparse
stages, legacy/observer cancellation, and an unfinished denominator refusal.
Strict workspace/all-target Clippy passed for the reviewed boundary. Later
integration-diagnostic work is separate and needs its own gates.

Evidence under the untracked `target/no-deformation-normal-preparation-progress/`:
the six-file review inventory has SHA256
`9c462b846497a3ffba104b43f693fe1a1ed3dd3c7b1485cee9ae437d00e3ef3e`;
the final handoff is
`744f517028a49411e2d8c6db47ab5748f63ca1814af74934c01a689909d4636d`.
Independent source review SHA256:
`1352932632a5fcc52c7a023c6616bd00c1366db280b6828879eb2dc6a8339d6b`.

This milestone provides attribution, not a successful physical endpoint result.
The frozen long-running physical probes had not returned their normal
coefficients at review time. A subsequent instrumented probe must preserve its
own executable/source identities and report that outcome separately.
