# Explicit certificate capacity and retained double-box evidence

The compatible symGCAD revision
`3baffa7134334c62b45a6b8df9198004060be715` carries the three-file change from
[PR #5](https://github.com/alphal00p/symGCAD/pull/5) onto the previously adopted
`bef2a9e6a7441aee4c0ba69acb80e68959e78928` revision. It raises only the maximum
explicitly requested Bernstein coefficient allowance to `2^21` and checked
certificate-work allowance to 64 billion. Defaults remain 4,096 coefficients
and one million work units. The algorithms, certificate schema, counters and
public dependency declarations are unchanged.

This is necessary for replaying the retained double-box certificates in their
original frame. Four actual tensor grids contain 1,115,370 coefficients; their
required work exceeds the old ceiling. Increasing time or memory alone cannot
make a request outside a native admission limit executable. A caller must still
select the larger allowances explicitly. Certificate records retain their
original settings; replay never rewrites a saved allowance to make it pass.

The isolated consumer uses the adopted Symbolica/Numerica `ae0c82b7` identity.
All 39 native controls pass: 31 domain tests, six budget/replay tests and two
checked-overflow controls. Independent source and result reviews verify that
the published revision is exactly the previous owner plus the reviewed change.
Owner Clippy passes with ten documented inherited lint categories allowed;
this is not an unqualified strict-clean claim. FastSecDec's own strict checks
remain separate. The numerical workspace and native Python consumer lockfile
select the same public revision, with no unrelated package updates.

The consuming core/CLI all-target Clippy check passes with `-D warnings`, and
workspace formatting passes. The broader native threshold test run records 315
distinct successful controls, including the closed-scalar and retained-margin
tests, native geometry verification, generation, recovery and selective artifact
restoration. It was stopped while the higher-degree component-factor control
remained active; a separate exact invocation also exceeded a 120-second probe
limit. The full suite therefore remains **incomplete**, not green. No assertion
failure was observed. Earlier timings used different source/dependency builds;
they do not establish a regression caused by this capacity change. The slow
native resolution path is being isolated separately, without weakening its
proof checks. Logs and the explicit stop record are retained under
`target/no-deformation-symgcad-tensor2m-carry`.

The completed certificate campaign has 850 exact targets with positive native
replay receipts, covering 1,794 factor references in 299 replacement rows.
Its disjoint groups contain 759 retained results, 87 continuation results and
four larger-grid results. Every worker exited, and the original work deadline
had 625.94 seconds remaining. Overlapping worker times are not added together
as generation time. An independent audit checked all targets, constraints,
receipts and the join without repeating the native work.

Eight previously accepted leaves are separate. These results do not yet issue
the complete 307-chart source-bound scalar owner, cover the full 644,699-cell
catalogue, continue the physical numerator or supply a double-box integral.
The next step binds the evidence to that genuine scalar source and inventories
exactly reusable prefixes and boundary structures while the verified geometry
is resident. No new whole-atlas solve is implied by this capacity change.

The alternative source-sector-first trial reached its original five-hour outer
limit without a completed solve or fresh verification. Its retained projection
and cache evidence is diagnostic, not a completed decomposition. The already
verified original projective geometry remains the basis of the endpoint work.

Subsequent resolution milestone: the source-owned native quotient-basis
optimization resolves the degree-five stall without increasing its limits.
The complete consuming threshold suite now passes 424 controls, with zero
failures and six existing diagnostic tests ignored. The earlier incomplete
run above remains its historical result; see the
[native quotient-basis review](no-deformation-native-quotient-basis.md).
