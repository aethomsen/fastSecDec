# Physical D05: bounded GCAD feasibility

This input reproduces the **geometry-only** double-box trials recorded in
[`docs/no-contour-gghh-feasibility.json`](../../../docs/no-contour-gghh-feasibility.json)
and [`docs/benchmark.pdf`](../../../docs/benchmark.pdf). None returned a complete
GCAD result within the tested deadlines. These are the three initial short scouts;
the separately budgeted extended rational-point trial is not included here.
They do not produce an integrable
artifact or a helicity amplitude. The larger no-contour implementation goal
remains open.

The original graph/model/numerator remain the maintained
[`gghh_double_box`](../../gghh_double_box/) assets. `parametric.toml` is an export
card preserving the source's generation fields; only `export-symanzik` is used
here. It retains numerator-only scalar products as unbound symbols. Do not reuse
the old numerical polarization Gram entries at the new scattering angle.

The first solve keeps `s,t` symbolic and fixes `mH=125`, `mt=345/2` exactly in a
small above-threshold physical chamber. The fallback fixes all geometry data:
`s=1000000`, `t=-97077`, `u=-871673`, `mH²=15625`, `mt²=119025/4`.
This is a nearby distinct point from `t=-484375+100000*sqrt(15)` at `cosθ=4/5`.
Native exact on-shell/Gram checks pass; the new `cosθ` is approximately
`0.799999308810368`. All future contour/amplitude comparisons must bind the same
new point, including fresh external wavefunctions. Fully parametric masses were
not tested.

The three `*-prepared.toml` files are exact symGCAD problem inputs, including
source polynomials, bindings, chamber constraints, given order, native positive
homogeneous block/anchor and options. They use the whole positive orthant, not a
unit cube substituted for the projective domain. The native homogeneous recipe
is responsible for reduction/reconstruction. These records remain geometry
inputs rather than endpoint or integration certificates.

Reproduce source export from the FastSecDec repository root:

```bash
out_dir=target/gghh-gcad-reproduction
mkdir -p "$out_dir"
for mode in two fixed; do
  point_file=masses.toml
  if [[ "$mode" == fixed ]]; then point_file=fixed-point.toml; fi
  ./target/release/fastsecdec --plain --json export-symanzik \
    examples/no_deformation/gghh_feasibility/parametric.toml \
    --point "examples/no_deformation/gghh_feasibility/$point_file" \
    --kinematic-constraint 'k1>999000' --kinematic-constraint 'k1<1001000' \
    --kinematic-constraint 'k2>-97100' --kinematic-constraint 'k2<-97050' \
    --kinematic-constraint 'k1>4*k0' --kinematic-constraint 'k1>4*k3^2' \
    --kinematic-constraint 'k2*(2*k0-k1-k2)>k0^2' \
    --output "$out_dir/$mode-export"
done
```

Aliases are `k0=mH²`, `k1=s`, `k2=t`, `k3=mt`, with native edge4…10 mapped to
`x0…x6`. This association is checked in the source export, not guessed from a
new graph ordering. The prepared cards then add native solver options explicitly.

Use symGCAD `a1132d4f4545c7784b3ec61239a05485e544b403`, with the same public
Symbolica/Numerica revision `ae0c82b7bda670fdb9f6229226bd47b13bb49a20` as FastSecDec.
Do not substitute an unrelated prebuilt binary. Public CLI equivalents are:

```sh
symgcad import-fastsecdec target/gghh-gcad-reproduction/two-export/symanzik.json \
  --problem target/gghh-gcad-reproduction/two-export/problem.toml --output target/gghh-gcad-reproduction/two-import.json
symgcad solve examples/no_deformation/gghh_feasibility/two-prepared.toml --output target/gghh-gcad-reproduction/two-result.json
# Only after a saved CompleteGeneric result:
symgcad verify target/gghh-gcad-reproduction/two-result.json

symgcad import-fastsecdec target/gghh-gcad-reproduction/fixed-export/symanzik.json \
  --problem target/gghh-gcad-reproduction/fixed-export/problem.toml --output target/gghh-gcad-reproduction/fixed-import.json
symgcad solve examples/no_deformation/gghh_feasibility/fixed-prepared.toml --output target/gghh-gcad-reproduction/fixed-result.json
symgcad verify target/gghh-gcad-reproduction/fixed-result.json
symgcad solve examples/no_deformation/gghh_feasibility/fixed-alternate-prepared.toml --output target/gghh-gcad-reproduction/alternate-result.json
symgcad verify target/gghh-gcad-reproduction/alternate-result.json
```

The actual trials called the same library directly in a standalone Rust driver,
compiled with opt-level 0/thin-LTO using pinned release dependency IR. This is
separate from the release CLI performance campaign. Its supervisor enforced a
single pooled solve-plus-verify deadline of 60 seconds (parametric) or 180 seconds
(fixed), 10 GiB address space, one Rayon thread and CPU affinity 16–17. The raw
native result would have been saved before verification in a fresh process;
none was returned before the external wall stops. Verification never started.

The cards configure native 45/150-second and 8192-MiB settings, but those do not
enforce a process bound around eager projection in this direct library path.
The external limits are the authoritative enforced bounds. Reproducing the
same deadline requires an external process supervisor with a shared deadline;
separate `timeout` invocations would grant solve and verify separate budgets.
The public symGCAD CLI owns a different worker/resource wrapper and its own
verification limit, so the commands reproduce the mathematical input and
algorithm/options, not identical timing or interruption serialization.

No trial reached a cell or memory cap. Last completed projection checkpoints
are diagnostic only. The fully rational point reached more projection stages
than the two-parameter family, but this does not establish either eventual
completion or impossibility. A different gauge/order or improved projection may
change the result; only CompleteGeneric plus successful independent verification
can establish the required generic-cell completion.
