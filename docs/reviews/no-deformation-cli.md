# Threshold preparation through the native CLI

The first supported CLI path uses HEPKit graph input, fixed physical values,
one-dimensional bounded rational threshold cells and native symbolic endpoint
continuation. It does not claim arbitrary algebraic cells, parametric chamber
rebinding, source/cell selection or the general endpoint resolver. Those remain
required delivery work. Unsupported cases return an explicit diagnostic.

The subsequent [regular-family factory](no-deformation-regular-family-factory.md)
extends this same workflow to complete fixed unit-cube families with two or
three regular algebraic cells. Its sextic example passes all four normal/serial
generation–integration combinations. The historical bubble evidence below
remains distinct from that extension and from the unfinished general resolver.

Both generation schedules use the existing caller-owned process supervisor.
One preparer owns global solve/verification/continuation; compilation children
load one detached native work record each. The coordinator receives compact
publication metadata and receipts, sorts records by native contribution index,
and publishes the common indexed archive atomically. Ordinary generation then
restores its resident result through the existing reader. The preparer's global
peak is distinct from compilation-sector residency; the current 1 GiB IPC bound
is not an allocator or aggregate-RSS guarantee.

Interrupted preparation retains raw evidence. Resume starts a fresh verifier,
checks source/build/provenance and reconstructed mathematical identity, and only
then re-admits the old compilation anchor. It cannot reuse a saved verification
receipt as proof. Raw checkpoints additionally bind the scientific run card
and exact issued CLI overrides before any replay or mutation. An independent
review found this association missing initially; the corrected path rejects a
genuine checkpoint imported from a different override context. Existing lease
checks, checksums, immutable records and journal
publication fences remain in effect. Tests deliberately restore the last cached
contribution before newly completed earlier contributions.

The maintained `examples/no_deformation/threshold_bubble.toml` uses the existing
scalar model and bubble graph. Its causal polynomial is `3-16*x*(1-x)`. With the
declared measure the complete coefficient vector is
`[1, 0, 2-3*log(3)/2, pi/2]` at real/imaginary orders `[-1,-1,0,0]`.
The card sets the scalar product to 16 directly: declaring it both runtime and
fixed is correctly rejected by the native input owner.

The private source-matched CLI passed 27 configuration/process/recovery tests,
plus strict Clippy and formatting. Separate root-run executable controls used
both generation modes, then both integration modes for each artifact:

```sh
fastsecdec generate examples/no_deformation/threshold_bubble.toml \
  --workers 2 --output normal.fsd
fastsecdec generate examples/no_deformation/threshold_bubble.toml \
  --serial --workers 2 --output serial.fsd
fastsecdec integrate normal.fsd --method qmc --points 4096 --shifts 8 \
  --workers 2 --target-order 0 --relative-tolerance 0.005 --max-rounds 2
# Repeat with --serial 0.01, and both choices for serial.fsd.
```

All four reached their accuracy target and reproduced the full analytic vector
to floating-point precision. Both generation schedules produced the same native
content identity. The saved results retained `FullIntegral` scope, complete
production evidence and all 16 covariance entries. Ordinary and serial per-sector
MC, and ordinary discrete MC, also reached their requested 5% target and agreed
with the analytic vector within six reported standard errors. These debug runs
are correctness controls, not performance measurements.
The combined `run --threshold-decomposition --contour off --serial 0.01`
command also reaches the analytic target, checking that an explicit contour-off
setting preserves the saved threshold recipe.

An explicit MC work-limit stop remains `WorkLimit` after restoring with a
different worker count and residence time; accepted means and point counts stay
unchanged. A separate process-group SIGINT fault interrupted an active resident
worker after 786,432 accepted points. The saved result reported that worker
failure, rather than successful convergence. Restoring its checkpoint with two
workers and a changed residence time completed 6,291,456 accepted points and
returned `WorkLimit`, with the finite coefficient consistent with the oracle.
This exercises the existing lease/recovery path on threshold kernels; it does
not replace the separate coordinate-stream uniqueness regressions.

Python archive selection now recognizes `threshold` using the existing native
reader. Its binding crate passes a locked native check. This narrow change does
not expose threshold generation or establish Python runtime/WASM support for it;
the native preparation wrapper is separate pending work.

Metadata-only and selected-record deep inspection also run successfully, with
optional record-integrity validation. Inspection now exposes the existing native
threshold directory and record receipts without reading evaluator data. Deep
inspection adds the native lineage, map descriptors, certificate identities,
resident selection and physical result scope. Parent inventory counts are
explicitly distinguished from the resident contribution. Saved map descriptors
use native expression-record references; printing their algebraic expressions is
still a separate presentation extension. Loading saved descriptors is explicitly
not global proof replay.

The six inspection tests pass, including a threshold control which loads one
numerical record, compares full versus selected residency, deletes the binary,
and still inspects metadata successfully. Independent HEPKit/ecosystem review
accepts reuse of the native owners and selective-loading boundary. No new algebra,
lineage schema or artifact format is introduced.

After registration, the complete CLI test suite passes 127 tests with five
existing ignored process/host entries; final all-target CLI Clippy and formatting
also pass. The separate joined native threshold
suite passes 156 tests with one ignored child entry. The Python archive-selection
change passes its locked native binding check; no installed-wheel execution is
claimed by that check.

The native HEPKit feature-union check additionally exposed two ambiguous
collection types when downstream serialization traits are enabled. Explicit
`BTreeSet` annotations in the existing coordinate-role and compilation-inventory
checks preserve their behavior and remove that consumer-specific ambiguity.
