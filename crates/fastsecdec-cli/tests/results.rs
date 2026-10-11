#[path = "support/artifact.rs"]
mod artifact_data;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use fastsecdec::{
    integration::{IntegrationProblem, Periodization, QmcSession, QmcSettings, SectorSpec},
    reference::{
        CoefficientKey, ReferenceCoefficient, ReferenceProvenance, ReferenceResult,
        ReferenceUncertainty, ReferenceValidation, encode_reference, read_reference,
    },
    results::{
        ExactContributionPolicy, KernelResultManifest, ResultScope, ResultTimings,
        SavedIntegrationResult, encode_result, read_result,
    },
    status::{CoefficientComponent, StoppingReason},
};

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fastsecdec"));
    command
        .env("SYMBOLICA_HIDE_BANNER", "1")
        .args(["--json", "--plain"]);
    command
}

fn success(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(output: Output) -> serde_json::Value {
    assert!(!output.status.success(), "unexpected success");
    serde_json::from_slice(&output.stdout).unwrap()
}

fn write_reference(path: &Path) -> ReferenceResult {
    let reference = ReferenceResult::new(
        [
            (
                CoefficientComponent::Real,
                2.5,
                ReferenceUncertainty::StandardError(0.2),
            ),
            (
                CoefficientComponent::Imag,
                3.75,
                ReferenceUncertainty::Unknown,
            ),
        ]
        .into_iter()
        .map(|(component, value, uncertainty)| ReferenceCoefficient {
            key: CoefficientKey {
                order: 0,
                component,
            },
            value,
            uncertainty,
        })
        .collect(),
        ReferenceProvenance::new(
            "distinct stored transport fixture",
            "direct unit-cube measure",
        ),
    );
    fs::write(path, encode_reference(&reference).unwrap()).unwrap();
    reference
}

#[test]
fn saved_run_survives_removing_sources_and_exports_the_explicit_reference() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.toml");
    let artifact = dir.path().join("artifact.fsd");
    let checkpoint = dir.path().join("checkpoint.json");
    let result_path = dir.path().join("result.json");
    let reference_path = dir.path().join("reference.json");
    let original_reference = write_reference(&reference_path);
    fs::write(
        &input,
        r#"
[direct]
domain = "unit_cube"
parameters = ["x"]
[[direct.terms]]
prefactor = "2+3𝑖"
monomial_powers = ["1"]
[integration]
points = 1024
shifts = 4
[reference]
path = "reference.json"
normalization_evidence = "Both use the declared unit-cube measure."
kinematics_evidence = "There are no external kinematic inputs."
independence_evidence = "The stored test target was not derived from this run."
"#,
    )
    .unwrap();
    let report = success(
        cli()
            .arg("run")
            .arg(&input)
            .arg("--output")
            .arg(&artifact)
            .arg("--checkpoint")
            .arg(&checkpoint)
            .arg("--save-result")
            .arg(&result_path)
            .args(["--lattice", "hkkn-alpha3"])
            .output()
            .unwrap(),
    );
    let saved = read_result(&fs::read(&result_path).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&saved.weight_diagnostics).unwrap(),
        report["weight_diagnostics"]
    );
    let norm = saved
        .contributions
        .total
        .as_ref()
        .unwrap()
        .complex_norm_uncertainty(0);
    assert_eq!(
        saved.provenance.attributes["complex_norm_uncertainty"]["0"],
        serde_json::to_value(norm).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&saved.contributions.total).unwrap(),
        report["estimate"]
    );
    assert_eq!(
        serde_json::to_value(&saved.qmc_design).unwrap(),
        report["qmc_design"]
    );
    assert_eq!(
        saved
            .contributions
            .total
            .as_ref()
            .unwrap()
            .covariance_of_mean
            .len(),
        4
    );
    assert_eq!(saved.validation, ReferenceValidation::Unverified);
    let stored = saved.stored_reference.as_ref().unwrap();
    assert_eq!(stored.reference, original_reference);
    assert_eq!(
        stored.context.kernel_content_id,
        saved.manifest.kernel_content_id
    );
    assert_ne!(
        saved.manifest.kernel_content_id,
        report["content_id"].as_str().unwrap()
    );
    assert_eq!(
        saved.provenance.attributes["artifact_content_id"],
        report["content_id"]
    );
    for path in [
        input,
        artifact.with_extension("fsd.json"),
        artifact_data::data_path(&artifact),
        checkpoint,
        reference_path,
    ] {
        fs::remove_file(path).unwrap();
    }
    let viewed = success(
        cli()
            .arg("show-result")
            .arg(&result_path)
            .args(["--sort", "magnitude", "--order", "0", "--component", "imag"])
            .output()
            .unwrap(),
    );
    assert_eq!(viewed["result"], serde_json::to_value(&saved).unwrap());
    assert_eq!(
        viewed["sector_order"].as_array().unwrap().len(),
        saved.contributions.sectors.len()
    );
    let exported = dir.path().join("exported.json");
    for source in ["estimate", "stored"] {
        success(
            cli()
                .arg("export-reference")
                .arg(&result_path)
                .args(["--source", source, "--output"])
                .arg(&exported)
                .output()
                .unwrap(),
        );
        let reference = read_reference(&fs::read(&exported).unwrap()).unwrap();
        if source == "stored" {
            assert_eq!(reference, original_reference);
        } else {
            assert_eq!(reference.validation, ReferenceValidation::Unverified);
            assert_eq!(
                reference.kernel_content_id.as_deref(),
                Some(saved.manifest.kernel_content_id.as_str())
            );
            for (row, value) in reference.coefficients.iter().zip([1.0, 1.5]) {
                assert!((row.value - value).abs() < 1e-5);
                assert!(matches!(
                    row.uncertainty,
                    ReferenceUncertainty::StandardError(_)
                ));
            }
        }
    }
}

fn numerical_fixture(selected: bool) -> SavedIntegrationResult {
    let sector = SectorSpec {
        id: 7,
        dimension: 1,
    };
    let problem = IntegrationProblem::new(
        "parent-kernel".into(),
        vec![0],
        vec![sector.clone()],
        vec![2.0],
    )
    .unwrap();
    let mut session = QmcSession::democratic(
        problem,
        QmcSettings {
            points: 1024,
            shifts: 2,
            periodization: Periodization::None,
            ..Default::default()
        },
    )
    .unwrap();
    let mut worker = session.worker_context(7).unwrap();
    while let Some(task) = session.next_work().unwrap() {
        let returned = worker
            .evaluate(task, |_, output| {
                output[0] = 3.0;
                Ok::<_, String>(())
            })
            .unwrap();
        session.submit(returned).unwrap();
    }
    SavedIntegrationResult {
        weight_diagnostics: None,
        manifest: KernelResultManifest {
            threshold_scope: None,
            source_selection: None,
            kernel_content_id: "parent-kernel".into(),
            orders: vec![0],
            components: vec![CoefficientComponent::Real],
            sectors: if selected {
                vec![
                    SectorSpec {
                        id: 2,
                        dimension: 1,
                    },
                    sector,
                ]
            } else {
                vec![sector]
            },
            exact_coefficients: vec![2.0],
        },
        scope: if selected {
            ResultScope::SelectedSectors {
                sector_ids: vec![7],
                exact_policy: ExactContributionPolicy::IncludeAll,
            }
        } else {
            ResultScope::FullIntegral
        },
        contributions: session.contributions().unwrap(),
        stopping_reason: StoppingReason::WorkLimit,
        requested_tolerance: None,
        requested_accuracy_target: Default::default(),
        evaluation_diagnostics: None,
        qmc_design: Some(session.design()),
        provenance: ReferenceProvenance::new("native constant fixture", "unit-cube measure"),
        validation: ReferenceValidation::Unverified,
        stored_reference: None,
        timings: ResultTimings::default(),
    }
}

#[test]
fn result_only_commands_preserve_scope_zero_error_and_reject_invalid_views() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("selected.json");
    fs::write(&path, encode_result(&numerical_fixture(true)).unwrap()).unwrap();
    let view = success(
        cli()
            .arg("show-result")
            .arg(&path)
            .args(["--sort", "error", "--order", "0"])
            .output()
            .unwrap(),
    );
    assert_eq!(view["sector_order"], serde_json::json!([7]));
    assert_eq!(
        view["result"]["contributions"]["exact_coefficients"],
        serde_json::json!([2.0])
    );
    let plain = Command::new(env!("CARGO_BIN_EXE_fastsecdec"))
        .env("SYMBOLICA_HIDE_BANNER", "1")
        .args(["--plain", "show-result"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(plain.status.success());
    let text = String::from_utf8(plain.stdout).unwrap();
    assert!(text.contains("selected") && text.contains("IncludeAll") && text.contains("Exact"));
    let output = dir.path().join("reference.json");
    let rejected = failure(
        cli()
            .arg("export-reference")
            .arg(&path)
            .args(["--source", "estimate", "--output"])
            .arg(&output)
            .output()
            .unwrap(),
    );
    assert!(
        rejected["error"]["message"]
            .as_str()
            .unwrap()
            .contains("selected")
    );
    assert!(!output.exists());
    let bad_key = failure(
        cli()
            .arg("show-result")
            .arg(&path)
            .args(["--sort", "error", "--order", "-17"])
            .output()
            .unwrap(),
    );
    assert!(
        bad_key["error"]["message"]
            .as_str()
            .unwrap()
            .contains("coefficient")
    );
    fs::write(&path, encode_result(&numerical_fixture(false)).unwrap()).unwrap();
    let missing = failure(
        cli()
            .arg("export-reference")
            .arg(&path)
            .args(["--source", "stored", "--output"])
            .arg(&output)
            .output()
            .unwrap(),
    );
    assert!(
        missing["error"]["message"]
            .as_str()
            .unwrap()
            .contains("absent")
    );
    success(
        cli()
            .arg("export-reference")
            .arg(&path)
            .args(["--source", "estimate", "--output"])
            .arg(&output)
            .output()
            .unwrap(),
    );
    let reference = read_reference(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(reference.coefficients[0].value, 5.0);
    assert_eq!(
        reference.coefficients[0].uncertainty,
        ReferenceUncertainty::StandardError(0.0)
    );
    let mut malformed: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    malformed["version"] = 99.into();
    fs::write(&path, serde_json::to_vec(&malformed).unwrap()).unwrap();
    assert!(
        failure(cli().arg("show-result").arg(&path).output().unwrap())
            .get("error")
            .is_some()
    );
}

#[test]
fn numerical_failure_saves_accepted_coverage_and_exits_unsuccessfully() {
    for (name, term) in [
        (
            "worker",
            "prefactor = \"10^308\"\nmonomial_powers = [\"-1/2\"]",
        ),
        (
            "statistics",
            "prefactor = \"10^200\"\nmonomial_powers = [\"0\"]\n[[direct.terms.factors]]\npolynomial = \"x-1/2\"\nexponent = \"1\"\nrole = \"polynomial\"",
        ),
    ] {
        for interval in ["0", "60000"] {
            let dir = tempfile::tempdir().unwrap();
            let input = dir.path().join("input.toml");
            let result_path = dir.path().join("failed.json");
            let checkpoint = dir.path().join("checkpoint.json");
            fs::write(
                &input,
                format!(
                    r#"
[direct]
domain = "unit_cube"
parameters = ["x"]
[[direct.terms]]
{term}
[integration]
points = 1024
shifts = 4
periodization = "none"
"#
                ),
            )
            .unwrap();
            let output = cli()
                .args(["--status-json", "--status-interval-ms", interval])
                .arg("run")
                .arg(&input)
                .arg("--output")
                .arg(dir.path().join("artifact.fsd"))
                .arg("--save-result")
                .arg(&result_path)
                .arg("--checkpoint")
                .arg(&checkpoint)
                .output()
                .unwrap();
            let statuses = String::from_utf8_lossy(&output.stderr)
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|status| status.get("method").is_some())
                .collect::<Vec<_>>();
            assert!(
                statuses.last().unwrap()["stop_reason"]
                    .get("NumericalFailure")
                    .is_some()
            );
            let report = failure(output);
            assert!(
                report["snapshot"]["stop_reason"]
                    .get("NumericalFailure")
                    .is_some(),
                "{report}"
            );
            assert!(!report["converged"].as_bool().unwrap());
            let saved = read_result(&fs::read(&result_path).unwrap()).unwrap();
            assert!(matches!(
                saved.stopping_reason,
                StoppingReason::NumericalFailure(_)
            ));
            if name == "worker" {
                assert!(saved.evaluation_diagnostics.as_ref().unwrap().failures > 0);
                assert_eq!(saved.contributions.sectors[0].progress.completed_points, 0);
            } else {
                assert_eq!(saved.evaluation_diagnostics.as_ref().unwrap().failures, 0);
                assert!(saved.contributions.sectors[0].progress.completed_points >= 2048);
                assert!(matches!(
                    saved.contributions.uncertainty,
                    fastsecdec::status::UncertaintyStatus::StatisticalFailure { .. }
                ));
            }
            assert!(checkpoint.exists());
            success(cli().arg("show-result").arg(&result_path).output().unwrap());
            let rejected = failure(
                cli()
                    .arg("export-reference")
                    .arg(&result_path)
                    .args(["--source", "estimate", "--output"])
                    .arg(dir.path().join("rejected.json"))
                    .output()
                    .unwrap(),
            );
            assert!(
                rejected["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("ineligible")
            );
        }
    }
}
