# First no-contour profiling campaign

These four cards reproduce the small complete-input profiles in
[`docs/benchmark.pdf`](../../../docs/benchmark.pdf), with full native vectors,
covariance and measurements in
[`docs/no-contour-results.json`](../../../docs/no-contour-results.json).
Scientific source: `20bb812e31bc09a8a28a122d914a1de75e92cae7`.
No physical double-box no-contour result is claimed.

Build the frozen revision with the public `Cargo.lock` identities:

```sh
nix-shell --run 'cargo build --release --locked -j4 -p fastsecdec-cli --features fastsecdec/threshold-decomposition'
```

The measured machine used eight physical AMD EPYC 9754 cores, CPU IDs 0–7
(one hardware thread each). Select eight physical cores appropriate to your host.
Run from the repository root, with a valid Symbolica license in the environment.
Output paths must be fresh. Each card retains the measured settings; only
absolute graph/model paths have been changed to card-relative paths.
The explicit CLI flags below override card defaults exactly as in the campaign.

```bash
case_name=bubble-threshold # also bubble-fixed, degree7-threshold, sextic-threshold
out_dir="target/no-contour-reproduction/$case_name"
mkdir -p "$out_dir"
taskset -c 0-7 ./target/release/fastsecdec --json --status-json --status-interval-ms 200 \
  generate "examples/no_deformation/benchmarks/$case_name.toml" \
  --workers 8 -o "$out_dir/integral.fsd" > "$out_dir/generation.json" 2> "$out_dir/generation-status.jsonl"
./target/release/fastsecdec --json inspect "$out_dir/integral.fsd" \
  --validate-artifact --deep > "$out_dir/inspection.json"

contour_flags=()
order=0
if [[ "$case_name" == bubble-* ]]; then order=1; fi
if [[ "$case_name" == bubble-fixed ]]; then
  contour_flags=(--contour fixed --lambda 0.1 --contour-validation pilot --contour-pilot-points 16)
  taskset -c 0-7 ./target/release/fastsecdec --json benchmark "$out_dir/integral.fsd" \
    --points 1 --repetitions 1 "${contour_flags[@]}" > "$out_dir/pilot-admission.json"
fi
for level in 0 1 2; do
  points=$((1024 << level))
  for family in 1 2; do
    seed=$((202610111700 + family + 1000 * level))
    run_path="$out_dir/n${points}-seed${seed}"
    taskset -c 0-7 ./target/release/fastsecdec --json --status-json --status-interval-ms 200 \
      integrate "$out_dir/integral.fsd" --workers 8 --points "$points" --shifts 8 \
      --seed "$seed" --target-order "$order" --max-rounds 1 \
      --absolute-tolerance 1e-12 --relative-tolerance 1e-8 \
      --checkpoint "$run_path.checkpoint.json" --save-result "$run_path.result.json" \
      "${contour_flags[@]}" > "$run_path.stdout.json" 2> "$run_path.status.jsonl"
  done
done
taskset -c 0-7 ./target/release/fastsecdec --json benchmark "$out_dir/integral.fsd" \
  --points 256 --repetitions 3 "${contour_flags[@]}" > "$out_dir/cost.json"
```

The original campaign imposed 180 seconds per generation/integration action,
30 seconds drain allowance, 30 minutes overall and 16 GiB aggregate owned-process
RSS. No limit was reached. It used OS observations of owned processes, not a
new integrator. A 5 ms target RSS observer and a separate child-exit wait thread
were used for profiling repeats because the original global `/proc` scan missed
brief peaks and delayed the externally observed clock. All generation repeats
retain the native artifact content identity; all repeated integrations retain
bit-identical full-vector means and covariance. The PDF identifies repeat data
separately. RSS sums shared pages per process and is an observation, not a strict
OS high-water mark. Process wall includes load, pilot, integration and writing;
native integration elapsed excludes artifact load and pilot.

SD in the report is the native standard error of the estimated mean from eight
complete random shifts; the joint real/imaginary SD is the square root of the
trace of that coefficient's covariance. Full covariance is retained across
sectors and Laurent outputs. Seeds at different lattice sizes are independent;
no estimates are pooled. Equal lattice sizes per sector do **not** imply equal
total work between the one-sector fixed bubble and six-sector threshold bubble.
No difference significance is inferred from their shared seeds without
cross-arm covariance. Near-roundoff uncertainty is not a certified error bound.

The degree-seven and sextic cases exercise complete regular-branch inputs,
not the still-unfinished singular-boundary resolver. The graph bubble has an
independent finite/pole OneLOop control, while the polynomial cases have
independent imaginary-part root-gap identities. Their real finite parts and
the bubble's epsilon-one coefficient lack an independent analytic oracle here.
