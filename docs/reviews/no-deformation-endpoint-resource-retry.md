# Explicit endpoint resource retry

This slice creates a new endpoint preparation attempt from retained native
configuration, request and raw GCAD evidence. It preserves the exact source
density, domain and kinematics. Only the explicitly issued resource overlay may
change; an existing publication is neither overwritten nor relabelled. Ordinary
resume and completed-preparation identity checks retain their previous meaning.

## Native ownership and reuse

`retry_endpoint_evidence`, `retry_graph_endpoint_evidence` and
`retry_family_endpoint_evidence` reuse the existing request/source restoration,
native GCAD verifier and endpoint continuation. Their `resume_*_endpoint_retry`
counterparts read the persisted effective configuration instead of applying a
second overlay. No new solver, geometry, algebra or numerical primitives are
introduced, and this path never invokes GCAD solve.

Every original RecordRef is length/digest authenticated. The same three-record
closure is copied with bounded 64 KiB I/O into an exclusive, canonical, distinct
directory; symlink records, existing destinations and ancestor/descendant overlap
are refused. Original records are immutable. Effective configuration, readable
lineage and the atomic native retry checkpoint are persisted before fresh proof
verification or endpoint work. Cancellation before this checkpoint leaves a
partial new directory and requires another distinct attempt. After the
checkpoint, the typed native retry resumes without relying on the old directory.

Legacy fixed-fiber routes retain their saved `regular_family=None` and accept only
resource fields supported by that route. They refuse meaningful unsupported
overrides rather than upgrading the mathematical route. Other routes reuse the
same native resource overlay used by fresh preparation. Discovery-owned private
certificate limits still receive their existing native admission checks.

The configuration codec is unchanged. Cold-process testing demonstrated that
re-encoding an equal native State can produce different exported record bytes.
The resource-only comparison therefore compares the existing decoded Stored DTO,
exact Atom values and ordered Symbol roles in one native State. It never replaces
the mandatory checks of the original saved bytes. Tests mutate each of these
three components and reject the freshly authenticated forged configuration.

## CLI and historical metadata

```
fastsecdec generate ORIGINAL.toml --retry-endpoint-from OLD.fsd \
  --endpoint-resource-overlay limits.json --output NEW.fsd
```

For example, `limits.json` may contain
`{"geometry":{"wall_time_secs":17700},"discovery":{"wall_time_secs":17700}}`.
Endpoint clocks remain independent of the GCAD solver budget and of integration
time. The original card, CLI overrides, source files, dependency identities and
build identity remain checked; the overlay is a separate persisted request.

The CLI opens and locks the existing old journal without recreating removed
directories. Successful ordinary publication removes its run directory while
leaving the outer journal. A cleaned `.fsd` alone therefore reports preparation
evidence unavailable; there is no silent solver rerun or new retention framework.
Native completed checkpoints remain usable when their record closure is retained.

Ordinary `--resume` of NEW uses its saved retry intent and effective configuration.
It can recover either its typed frontend checkpoint or the native sidecar written
just before that checkpoint. The original overlay file is not read again. The
existing completed anchor/readmission guards remain in force. New retry flags
must be paired and cannot be combined with `--resume`.

The optional `GenerationRecord.endpoint_retry` is observed history, outside the
artifact's mathematical identity. Its resource settings remain readable after
cleanup; its RecordRefs are historical identities, not promises of retained proof
locations or new retry authority. Omission preserves old artifact JSON. This
slice adds no Python retry API.

## Acceptance and limitations

Final executable and strict receipts are collected in the private handoff. Tests
exercise retained incomplete/completed native seeds, cold-process recovery,
source/configuration/role/Atom tampering, path refusals, original immutability,
CLI recovery across both checkpoint boundaries, unchanged ordinary resume and
artifact formats, and a real massive-triangle graph retry. The latter compiles
all four cells/eighteen charts, restores the archive, retains the entire Laurent
covariance, and checks poles plus epsilon-zero against the accepted OneLOop values
using eight independent 4096-point QMC replicas and the predeclared 8 SE + 2e-9
allowance. This proves the scoped triangle route, not a complete double-box atlas.

Independent full source/API review and the parent source audit are retained with
the handoff. The parent imported the exact reviewed source after checking all
24 preimages and 79 evidence records. Shared consuming checks cover its
combination with the subsequently published affine-prefix implementation.
