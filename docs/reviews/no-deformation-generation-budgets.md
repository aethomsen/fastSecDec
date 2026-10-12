# Explicit long-generation resource budgets

The user's five-hour double-box allowance exposed an upstream hard ceiling:
symGCAD previously rejected caller-selected solve budgets above 7,200 seconds.
[symGCAD PR 4](https://github.com/alphal00p/symGCAD/pull/4) raises that explicit
ceiling to 18,000 seconds. The default solve budget is unchanged. Invalid,
nonfinite and out-of-range values remain errors.

A separate actual double-box residual needs 793,152 Bernstein coefficients,
above the previous explicit ceiling of 262,144. Its exact positive-margin
certificate and independent native replay both succeed with the larger allowance.
[symGCAD PR 3](https://github.com/alphal00p/symGCAD/pull/3) permits at most
1,048,576 coefficients and sixteen billion arithmetic work units. Defaults remain
4,096 coefficients and one million work units. Checked tensor products, work
counters, certificate budget identity and replay remain enforced. Raising a
ceiling neither proves positivity nor makes incomplete generation successful.

Public revision `bef2a9e6a7441aee4c0ba69acb80e68959e78928` carries exactly these
two reviewed patches on the previously adopted `644a560e` base. An independent
57-source-file comparison checks that union; the owner's manifests and lockfile
are unchanged. FastSecDec continues using its existing Symbolica/Numerica owner
`ae0c82b7`. There is no local source-rewriting or installation-preparation step.

The isolated consuming build uses that same Symbolica revision with serde and
bincode enabled. All 41 selected native tests pass, including the five-hour
solve/verification control, native certificate restoration, large tensor,
overflow and exhaustion controls. Owner Clippy passes only with its documented
inherited lint allowances; this is not a claim that the upstream crate is
globally strict-Clippy clean. Independent source/API and evidence reviews pass.

The consuming FastSecDec CLI also generates the maintained above-threshold
bubble with an explicit 17,700-second GCAD allowance, publishing all six kernels
through serial generation. This small control checks public settings admission
and publication; it does not require waiting five hours or replace the existing
scientific bubble comparisons. The locked native HEPKit binding build passes
with the same public dependency selection.

The consuming native regression gates pass all 48 selected GCAD, options,
certificate, projective-map and generation/restoration tests. Two generation
child-process entry tests remain intentionally ignored by the parent runner.
Strict threshold-enabled core/CLI all-target Clippy, the default library check,
formatting and whitespace checks pass. The source-matched release build is
installed locally; its SHA-256 is
`cae118881efe21871f3e28b2a8136e9afe7163abb79cf83a746d4a555b7c4ad0`.
These checks admit the larger explicit budgets without claiming completion of
the ongoing double-box generation.

The long experimental double-box jobs reserve 17,700 seconds of native work
inside an 18,000-second supervised process allowance, with a shared 100 GB RSS
limit. Incremental verified work is retained. Their frozen executables and
dependency identities remain distinct from the later consuming dependency
update; historical timings are not relabelled.

The CLI exposes the solver allowance through the existing input table:

```toml
[threshold_decomposition.gcad_limits]
wall_time_secs = 17700.0
```

This setting governs GCAD. Subsequent endpoint-preparation budgets and outer
process supervision are independent; it does not silently increase them.
Native callers retain their separate preparation/certificate controls. A longer
budget does not address missing moving-root endpoint resolution, and does not
extend the ten-minute integration benchmark contract.
