# Exact-only carriers and full threshold point identity

`ProgramRecipeCatalogue::bound_threshold_content_id` associates an already-bound
exact-only carrier with its full catalogue. This supplies the numerical identity
needed when a serial coordinator retains exact contributions while workers load
stochastic records separately.

The operation validates the native parent and guard, complete catalogue, selected
scope, output/runtime layout, both template identities and the carrier's committed
point. Only then does it return the full catalogue's identity at that point. It
reads no records, runs no evaluator, changes no binding and grants no additional
resident or geometric authority. Zero-sector carriers with all-zero exact offsets
still require their actual committed point to match.

The existing ordered-parameter admission and exact chamber guard are reused.
The point-hash routine is extracted without changing its bytes; empty parameter
lists retain historical template IDs and signed zero retains its previous
distinction. No wire format, alternate CAS, stored binding vector or new geometry
proof is introduced.

Independent review accepts the seven source files and the final test-only
correction in handoff
`47df914a83c10fd4f524682011504271b7225211f87c4dcfeaaae46da1fbb927`.
The coordinator verified all source preimages/postimages and 66 evidence files.
Maintained controls cover missing, nonfinite, unknown, unbound, wrong and
out-of-chamber points, altered owners/layouts, immutable refusal and old point
hashes. Fresh default-native consumers restore actual V4 projective and historical
V3 fixed archives under both validation settings and changed symbol-registration
order. Full IDs, layouts and vectors match the previous reader byte-for-byte at
s=16,17,16; default-native restoration does not enable GCAD.

The original misplaced zero-offset test assertion is preserved in private
evidence: unbound native offsets are intentionally NaN. The final assertion runs
after binding and retains all negative controls. Source/API and final test review,
focused native/default-consumer tests, strict library/test lint and formatting
pass. These are correctness checks, not performance measurements.

CLI parameter forwarding and cross-point checkpoint isolation are separate
integration gates. This native helper does not authenticate hostile artifact
files, replay global geometry or complete the general endpoint resolver.

The consuming workspace passes the point-hash regression, three catalogue groups
and the inherited ordinary archive-binding test, together with 19 resolver groups.
Strict workspace/all-target Clippy, formatting and diff checks pass. The native
point operation remains independent of the concurrently measured frozen GCAD
executable and does not change those campaign identities or results.
