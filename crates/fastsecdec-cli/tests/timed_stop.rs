//! Actual CLI resident workers drain before a timed invocation claims exact work.
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fastsecdec"));
    command
        .env("SYMBOLICA_HIDE_BANNER", "1")
        .args(["--plain", "--json"]);
    command
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn generate(directory: &Path) -> std::path::PathBuf {
    let card = directory.join("input.toml");
    fs::write(
        &card,
        r#"
[direct]
domain="unit_cube"
parameters=["x"]
[[direct.terms]]
monomial_powers=["1"]
[generation]
order=0
[generation.evaluator]
backend="eager"
[integration]
points=1024
shifts=4
workers=2
max_rounds=1
absolute_tolerance=0.0
relative_tolerance=0.0
"#,
    )
    .unwrap();
    let base = directory.join("integral.fsd");
    success(
        cli()
            .arg("generate")
            .arg(card)
            .arg("--output")
            .arg(&base)
            .output()
            .unwrap(),
    );
    base
}
fn verify_counts(result: &Value) -> u64 {
    assert_eq!(result["timed_stop"]["counts_complete"], true);
    assert_eq!(result["operational"]["diagnostics"]["failures"], 0);
    let rows = result["operational"]["diagnostics"]["f64_timing"]["calls"]
        .as_u64()
        .unwrap();
    let classified = result["operational"]["evaluations"].as_u64().unwrap();
    assert_eq!(
        rows, classified,
        "this successful no-rescue fixture has one native primary per point"
    );
    assert_eq!(
        rows,
        result["snapshot"]["completed_points"].as_u64().unwrap()
    );
    assert!(rows > 0);
    rows
}
#[test]
fn timed_serial_deadline_drains_full_reservations_and_checkpoints_exact_counts() {
    let dir = tempfile::tempdir().unwrap();
    let base = generate(dir.path());
    let checkpoint = dir.path().join("timed.checkpoint");
    let result = success(
        cli()
            .arg("integrate")
            .arg(&base)
            .args([
                "--serial",
                "60",
                "--method",
                "qmc",
                "--points",
                "65536",
                "--shifts",
                "1024",
                "--evaluation-batch-size",
                "1",
                "--max-seconds",
                "0.2",
                "--checkpoint",
            ])
            .arg(&checkpoint)
            .output()
            .unwrap(),
    );
    assert_eq!(result["stopping_reason"], "work limit");
    assert_eq!(result["converged"], false);
    let stop = &result["timed_stop"];
    assert!(stop["deadline_observed_seconds"].as_f64().unwrap() >= 0.2);
    assert!(stop["in_flight_at_deadline"].as_u64().unwrap() > 0);
    assert!(stop["drain_seconds"].as_f64().unwrap() > 0.);
    let calls = verify_counts(&result);
    assert_eq!(
        calls % 65536,
        0,
        "no partial native replica may enter the estimate"
    );
    assert!(
        calls < 1024 * 65536,
        "no new reservations after the deadline"
    );
    let saved: Value = serde_json::from_slice(&fs::read(checkpoint).unwrap()).unwrap();
    assert_eq!(
        saved["checkpoint"]["operational"]["diagnostics"]["f64_timing"]["calls"],
        calls
    );
    assert_eq!(saved["checkpoint"]["diagnostics"]["evaluations"], calls);
}
#[test]
fn timed_early_work_limit_has_complete_counts_without_claiming_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let base = generate(dir.path());
    let checkpoint = dir.path().join("early.checkpoint");
    let result = success(
        cli()
            .arg("integrate")
            .arg(&base)
            .args([
                "--serial",
                "0.01",
                "--method",
                "qmc",
                "--max-seconds",
                "60",
                "--checkpoint",
            ])
            .arg(&checkpoint)
            .output()
            .unwrap(),
    );
    assert_eq!(result["stopping_reason"], "work limit");
    assert!(result["timed_stop"]["deadline_observed_seconds"].is_null());
    assert_eq!(verify_counts(&result), 4 * 1024);
    // A later invocation may change its operational time allowance; native
    // checkpoint stream identity remains fixed and completed work is reused.
    let resumed = success(
        cli()
            .arg("integrate")
            .arg(&base)
            .args([
                "--serial",
                "0.01",
                "--method",
                "qmc",
                "--max-seconds",
                "30",
                "--checkpoint",
            ])
            .arg(&checkpoint)
            .arg("--resume")
            .output()
            .unwrap(),
    );
    assert_eq!(resumed["estimate"], result["estimate"]);
    assert_eq!(resumed["operational"]["evaluations"], 0);
    assert_eq!(resumed["timed_stop"]["counts_complete"], true);
    assert!(resumed["timed_stop"]["deadline_observed_seconds"].is_null());
}
