# Explicit original-cell selection scope

The additive `SelectedOriginalCellsV1` extent distinguishes selected original
GCAD cells from ordinary source-sector selection and from selecting resident
kernels in a complete archive. It retains separate original and quotient
request/evidence identities, the source and quotient-map association, original
cell count, sorted selected ordinals and exact replacement-coverage identity.

Structural admission checks the single GCAD-first source patch and exact native
cell inventory. These data remain arithmetic provenance: reading them does not
reconstruct a geometry certificate. Original evidence digests are retained as
such; their mathematical association must be established by the native producer.
The old full/ordinary-selected variants keep their serialization and semantics.
Older readers reject the new explicitly versioned tag.

Selecting all original cells explicitly still produces a partial-qualified
result. Existing native result canonicalization propagates that qualification,
including through the HEPKit execution helper and archive catalogue. CLI
catalogue inspection labels the new extent as selected original cells. The
forthcoming compiler producer must preserve this scope through an actual cold
archive test; metadata-only tests are not that integration gate.

Independent source review checked all eight modified/new files. Nineteen native
metadata tests passed, including seventeen owner/coverage/inventory tamper
mutations, strict wire handling, legacy wire compatibility, codec round trips
and selected-all result qualification. Strict all-target core and CLI Clippy
passed. The portable-feature core check also passed; an initial no-backend
invocation was rejected as expected by Numerica and was corrected to select
the existing portable numeric feature.

The ecosystem audit found one presentation gap: Python preserves native scope
but `Kernels` has no direct read-only threshold scope/lineage getter. A separate
thin binding slice is assigned before exposing selected-cell compiler results
there. No alternate graph, algebra, proof or execution implementation is added.
