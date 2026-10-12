# Feature-branch consolidation — 12 October 2026

`main` contains the completed contour-deformation and no-contour development,
the benchmark campaign and the reorganized report. It is the current delivery
and development branch. This consolidation preserves the documented scientific
limits; it does not declare the general algebraic endpoint resolver complete.

## Preserved branch tips

| Former branch | Annotated archive tag | Preserved commit |
| --- | --- | --- |
| `contour_deformation` | `archive/contour_deformation-2026-10-12` | `cf10c69` — restored evaluator equivalence across generation workers |
| `no_deformation` | `archive/no_deformation-2026-10-12` | `663f813` — six-chapter benchmark report |

The contour tip was already contained in the old `main`. The no-contour tip
contains both, so consolidation is a fast-forward preserving every commit.
The archive tags retain the feature tips before the main-branch documentation
update. Both local and remote feature-branch names are removed only after their
tips are reachable through published `main` and the archive tags.

The remote publication updates main, creates both tags and removes the two
branch refs atomically, with expected-tip leases on the removals. A concurrent
feature-branch update therefore prevents deletion rather than losing work.

To inspect an archived state:

```sh
git fetch origin --tags
git show archive/no_deformation-2026-10-12
git switch --detach archive/no_deformation-2026-10-12
```

Return with `git switch main`. Create a new branch from the tag if new work on
that historical state is required; the archive itself should remain unchanged.

## Preservation and validation

- Independent ancestry review found no unique work outside the integrated
  feature history. Both existing detached reference worktrees also point to
  ancestors of the consolidated main and are left untouched.
- Untracked user example files are neither staged nor removed.
- Dependency revisions, numerical records, report/PDF and implementation are
  unchanged by the consolidation documentation commit.
- Fresh `cargo fmt --all --check` and `git diff --check` pass. The licensed
  native regression runs pass 32 normalized-map, 22 algebraic-evaluator and
  14 contour tests, including their separately invoked subprocess helpers.
  These use `nix-shell`, Rust 1.97.1, locked dependencies and the native
  `threshold-decomposition` feature. An initial cold-child run hit the
  restricted Symbolica thread allowance; supplying the existing license to
  the environment resolved it without an implementation change. The previous
  milestone reviews retain the broader scientific evidence.

Historical plans and reviews keep the original branch names as provenance.
Current entry-point documentation directs new work to `main`.
