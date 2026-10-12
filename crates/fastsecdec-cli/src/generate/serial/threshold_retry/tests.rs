use super::*;
use crate::config::GenerationOverrides;
fn card(path: &Path) {
    fs::write(path,"[direct]\ndomain='unit_cube'\nparameters=['x']\n[[direct.terms]]\nprefactor='1/eps'\nmonomial_powers=['0']\n[[direct.terms.factors]]\npolynomial='x-1/3'\nexponent='-eps'\nsemantics='causal'\n[generation]\nthreshold_decomposition=true\norder=0\n").unwrap();
}
fn retained(input: &Path, output: &Path) -> PathBuf {
    let mut journal = Journal::open(input, output, false, "original").unwrap();
    let request = Request::PrepareThreshold {
        input: input.into(),
        workers: 1,
        overrides: Default::default(),
        attempt: "original".into(),
        prior: None,
    };
    journal
        .request("prepare-original", request.clone())
        .unwrap();
    jobs::execute(&journal.job_path("prepare-original"), &mut |_| Ok(())).unwrap();
    journal.accept("prepare-original", &request).unwrap();
    journal
        .anchor_threshold_preparation("prepare-original")
        .unwrap();
    journal.root.clone()
}
#[test]
fn explicit_retry_preserves_original_journal_and_publishes_history() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.toml");
    card(&input);
    let old = dir.path().join("old.fsd");
    let old_root = retained(&input, &old);
    let journal_path = journal_directory(&old).unwrap().join("journal.json");
    let before = fs::read(&journal_path).unwrap();
    let checkpoint = fs::read(old_root.join("threshold-checkpoint.json")).unwrap();
    let overlay = dir.path().join("resources.json");
    fs::write(&overlay, r#"{"max_charts":700}"#).unwrap();
    let output = dir.path().join("retry.fsd");
    let artifact = super::super::retry_endpoint(
        GenerationRun {
            path: &input,
            output: &output,
            reference: None,
            workers: 2,
            resume: false,
            overrides: Default::default(),
        },
        &old,
        &overlay,
        &mut Dashboard::new(false, false).unwrap(),
    )
    .unwrap();
    assert_eq!(fs::read(&journal_path).unwrap(), before);
    assert_eq!(
        fs::read(old_root.join("threshold-checkpoint.json")).unwrap(),
        checkpoint
    );
    let saved = artifact
        .generation
        .as_ref()
        .unwrap()
        .endpoint_retry
        .as_ref()
        .unwrap();
    assert_eq!(saved.resources.max_charts, Some(700));
    let original: serde_json::Value = serde_json::from_slice(&checkpoint).unwrap();
    assert_eq!(
        saved.original.evidence.request.source_identity,
        original["native"]["evidence"]["request"]["source_identity"]
            .as_str()
            .unwrap()
    );
    assert!(Path::new(&saved.attempt_directory).is_absolute());
    assert_ne!(Path::new(&saved.attempt_directory), old_root.as_path());
    let mut without_history = Artifact::load_metadata(&output).unwrap();
    without_history.generation.as_mut().unwrap().endpoint_retry = None;
    let stripped = dir.path().join("without-history.fsd");
    fs::write(
        crate::artifact::paths(&stripped).unwrap().0,
        serde_json::to_vec(&without_history).unwrap(),
    )
    .unwrap();
    let stripped = Artifact::load_metadata_with_options(
        &stripped,
        fastsecdec::kernel::KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert_eq!(stripped.content_id, artifact.content_id);
    super::super::threshold_tests::check(&output);
    // Successful ordinary cleanup retains outer journal/history, not native evidence.
    let refused = open(
        &input,
        &dir.path().join("third.fsd"),
        &output,
        &overlay,
        Default::default(),
        false,
    )
    .err()
    .unwrap()
    .to_string();
    assert!(refused.contains("evidence unavailable"), "{refused}");
}
#[test]
fn retry_refuses_changed_card_overrides_existing_paths_and_lost_records() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.toml");
    card(&input);
    let old = dir.path().join("old.fsd");
    let root = retained(&input, &old);
    let overlay = dir.path().join("resources.json");
    fs::write(&overlay, "{}").unwrap();
    let output = dir.path().join("new.fsd");
    assert!(open(&input, &output, &old, &overlay, Default::default(), true).is_err());
    assert!(open(&input, &old, &old, &overlay, Default::default(), false).is_err());
    let changed = GenerationOverrides {
        contour: true,
        ..Default::default()
    };
    assert!(open(&input, &output, &old, &overlay, changed, false).is_err());
    let bytes = fs::read(&input).unwrap();
    fs::write(
        &input,
        String::from_utf8(bytes.clone())
            .unwrap()
            .replace("order=0", "order=1"),
    )
    .unwrap();
    assert!(open(&input, &output, &old, &overlay, Default::default(), false).is_err());
    fs::write(&input, bytes).unwrap();
    let pair = crate::artifact::paths(&output).unwrap();
    fs::write(&pair.0, "existing").unwrap();
    assert!(open(&input, &output, &old, &overlay, Default::default(), false).is_err());
    fs::remove_file(&pair.0).unwrap();
    fs::remove_dir_all(root).unwrap();
    assert!(
        open(&input, &output, &old, &overlay, Default::default(), false)
            .err()
            .unwrap()
            .to_string()
            .contains("evidence unavailable")
    );
}
#[test]
fn old_generation_metadata_omits_retry_and_roundtrips_unchanged() {
    let value = serde_json::json!({"workers":1,"requested_coefficient_expansion":"direct"});
    // Use the ordinary default enum's serde spelling, independently of retry.
    let mut value = value;
    value["requested_coefficient_expansion"] =
        serde_json::to_value(fastsecdec::generation::CoefficientExpansionMethod::default())
            .unwrap();
    let record: crate::artifact::GenerationRecord = serde_json::from_value(value.clone()).unwrap();
    assert!(record.endpoint_retry.is_none());
    assert_eq!(serde_json::to_value(record).unwrap(), value);
}
#[test]
fn real_triangle_graph_retry_compiles_restores_and_matches_oneloop() {
    use fastsecdec::integration::{Periodization, QmcSession, QmcSettings, RuleSource};
    let dir = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/no_deformation/benchmarks/overview/triangle.toml");
    let mut source: toml::Value = toml::from_str(&fs::read_to_string(&fixture).unwrap()).unwrap();
    for key in ["graph", "model"] {
        source["input"][key] = toml::Value::String(
            fixture
                .parent()
                .unwrap()
                .join(source["input"][key].as_str().unwrap())
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
        );
    }
    source["generation"]
        .as_table_mut()
        .unwrap()
        .insert("threshold_decomposition".into(), toml::Value::Boolean(true));
    source["generation"]["evaluator"]["backend"] = toml::Value::String("eager".into());
    source["generation"]["evaluator"]["cpe_rounds"] = toml::Value::Integer(0);
    source["generation"]["evaluator"]
        .as_table_mut()
        .unwrap()
        .insert("horner_iterations".into(), toml::Value::Integer(0));
    let input = dir.path().join("triangle.toml");
    fs::write(&input, toml::to_string(&source).unwrap()).unwrap();
    let old = dir.path().join("original.fsd");
    retained(&input, &old);
    let overlay = dir.path().join("resources.json");
    fs::write(
        &overlay,
        r#"{"geometry":{"wall_time_secs":90},"discovery":{"wall_time_secs":90}}"#,
    )
    .unwrap();
    let output = dir.path().join("retry.fsd");
    super::super::retry_endpoint(
        GenerationRun {
            path: &input,
            output: &output,
            reference: None,
            workers: 2,
            resume: false,
            overrides: Default::default(),
        },
        &old,
        &overlay,
        &mut Dashboard::new(false, false).unwrap(),
    )
    .unwrap();
    let (artifact, mut kernels) = Artifact::load_with_options(
        &output,
        fastsecdec::kernel::KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert!(
        artifact
            .generation
            .as_ref()
            .unwrap()
            .endpoint_retry
            .is_some()
    );
    let metadata = kernels.threshold_metadata().unwrap();
    assert!(metadata.full_original_scope().unwrap());
    assert_eq!(metadata.lineage().cells.len(), 4);
    assert_eq!(metadata.lineage().endpoint_charts.len(), 18);
    let problem = fastsecdec::results::KernelResultManifest::integration_problem_from_kernels(
        &kernels,
        &fastsecdec::results::ResultScope::FullIntegral,
        "triangle-retry-reference",
    )
    .unwrap();
    let mut session = QmcSession::democratic(
        problem,
        QmcSettings {
            points: 4096,
            shifts: 8,
            seed: 202610128141,
            package_points: 1024,
            periodization: Periodization::Korobov3,
            rule: RuleSource::Kuo,
        },
    )
    .unwrap();
    while let Some(task) = session.next_work().unwrap() {
        let id = task.sector_id();
        let result = session
            .worker_context(id)
            .unwrap()
            .evaluate(task, |p, o| {
                kernels.sectors_mut()[id as usize].evaluate(p, o)
            })
            .unwrap();
        session.submit(result).unwrap();
    }
    let estimate = session.estimate().unwrap();
    assert_eq!(
        estimate.covariance_of_mean.len(),
        estimate.mean.len().pow(2)
    );
    assert!(kernels.orders().contains(&0));
    for (index, (order, component)) in kernels
        .orders()
        .iter()
        .zip(kernels.components())
        .enumerate()
    {
        let expected = match (order, component) {
            (0, fastsecdec::status::CoefficientComponent::Real) => -0.8943345118780581,
            (0, fastsecdec::status::CoefficientComponent::Imag) => -0.6047086137711147,
            (order, _) if *order < 0 => 0.,
            _ => panic!("unexpected Laurent order {order}"),
        };
        let error = estimate.covariance_of_mean[index * estimate.mean.len() + index].sqrt();
        assert!(
            (estimate.mean[index] - expected).abs() < 8. * error + 2e-9,
            "{:?}, {error}, {expected}",
            estimate.mean
        );
    }
}

#[test]
fn interrupted_retry_recovers_recorded_overlay_and_native_checkpoint_in_new_workers() {
    for remove_frontend_checkpoint in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.toml");
        card(&input);
        let old = dir.path().join("old.fsd");
        let old_root = retained(&input, &old);
        let old_journal = journal_directory(&old).unwrap().join("journal.json");
        let old_bytes = fs::read(&old_journal).unwrap();
        let old_checkpoint = fs::read(old_root.join("threshold-checkpoint.json")).unwrap();
        let overlay = dir.path().join("resources.json");
        fs::write(&overlay, r#"{"max_charts":700}"#).unwrap();
        let output = dir.path().join("resumed.fsd");
        let seed = open(&input, &output, &old, &overlay, Default::default(), false).unwrap();
        let mut journal = Journal::open_with_family(
            &input,
            &output,
            false,
            "interrupted-retry",
            Default::default(),
            RecipeFamily::single(ProgramRecipe::ThresholdV1),
        )
        .unwrap();
        let retry = journal
            .endpoint_retry(Some(seed.attempt.clone()))
            .unwrap()
            .unwrap();
        let request = Request::RetryThreshold {
            input: input.clone(),
            workers: 1,
            overrides: Default::default(),
            attempt: "interrupted-retry".into(),
            prior: None,
            retry: Box::new(retry),
        };
        journal.request("retry-interrupted", request).unwrap();
        assert!(
            jobs::execute(&journal.job_path("retry-interrupted"), &mut |p| {
                if p.detail.contains("Threshold Regularize") {
                    Err(std::io::Error::other(
                        "test interruption after durable retry checkpoint",
                    ))
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        let native = journal
            .root
            .join("threshold-retry/endpoint-retry-checkpoint.json");
        assert!(native.is_file());
        assert!(journal.threshold_preparation().unwrap().is_none());
        let checkpoint = journal.root.join("threshold-checkpoint.json");
        assert!(checkpoint.is_file());
        if remove_frontend_checkpoint {
            fs::remove_file(checkpoint).unwrap();
        }
        drop(journal);
        drop(seed);
        // Resume consumes the recorded overlay, never the mutable original file.
        fs::write(&overlay, r#"{"max_charts":0}"#).unwrap();
        let artifact = super::super::generate_with_overrides(
            &input,
            &output,
            &mut Dashboard::new(false, false).unwrap(),
            None,
            2,
            true,
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            artifact
                .generation
                .as_ref()
                .unwrap()
                .endpoint_retry
                .as_ref()
                .unwrap()
                .resources
                .max_charts,
            Some(700)
        );
        super::super::threshold_tests::check(&output);
        assert_eq!(fs::read(old_journal).unwrap(), old_bytes);
        assert_eq!(
            fs::read(old_root.join("threshold-checkpoint.json")).unwrap(),
            old_checkpoint
        );
    }
}
