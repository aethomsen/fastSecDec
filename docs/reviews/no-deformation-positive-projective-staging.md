# Recovery of original positive-projective requests

`threshold::gcad::staging::StagedPositiveRequest` persists the original
homogeneous request, its complete source density and raw native solve output.
It complements the existing affine/cube staging API. Both use the existing
content-addressed `RecordRef`, native Symbolica Atom codec and source `Input`
record; no graph parser or alternative algebra serialization is introduced.

The graph route records its represented-value policy and source witness.
Restoration requires the caller's original HEPKit `GraphPoint`, repeats bounded
conversion and native parameterization, and checks the resulting exact density,
source identity and complete native problem. Decoding a parametric density
cannot replace that graph ownership. Exact sources retain their separate native
constructor. Caller byte and conversion limits apply before expensive import.

Raw evidence is serialized by reference without cloning the complete solve.
Its request, status and quotient-cell count are checked on read. Incomplete and
invalid proof results remain available for diagnosis. Every verification attempt
runs the native homogeneous verifier again; saved successful observations never
issue mathematical authority. The positive-projective request kind is distinct
from the existing affine request kind, whose wire representation is unchanged.
Historical problems with different settings cannot be relabelled as a new
request, even when an independently checked mathematical geometry is equivalent.

The library owns no process, thread or scheduling loop. Record-byte caps bound
transport, not peak decoded or native-verifier memory. Hard time and RSS limits
remain the caller's responsibility. Original source and global proof owners can
be released between attempts; a test verifies their actual `Weak` expiration.

## Acceptance evidence

Independent source/API and HEPKit reuse review passed. The threshold GCAD
module passes 36 tests, including two cold-process parent controls that invoke
their otherwise ignored child entries. New controls cover:

- Exact full-density round trips after releasing the original request owner.
- The actual HEPKit bubble point, represented floating-point inputs and measure,
  including fresh-process reconstruction and native verification.
- Rejection of a changed original graph, insufficient conversion/transport caps,
  mismatched request settings, altered receipt counts and truncated records.
- Durable incomplete evidence and a tampered complete proof that the current
  verifier rejects, without deleting its raw record.

Strict threshold-enabled core Clippy across all targets and workspace formatting
also pass. This additive library milestone does not replace the installed release
executable or relabel any recorded benchmark binary.

The first test invocation omitted the provided Symbolica license from child
process environments; all three old/new cold-process controls failed. The same
new parent/child binary passed with the license exported, followed by the full
licensed 36-test pass. This was a test-environment correction, not a relaxation
of source association or verification.

This milestone provides native evidence recovery. It does not yet connect the
new positive-projective endpoint family to automatic CLI generation, nor claim
completion of the general algebraic resolver or the physical double box.
