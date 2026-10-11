//! Real caller-owned process pipeline, resumed native work, and recipe scope.
use super::*;
use fastsecdec::{
    integration::{Periodization, QmcSession, QmcSettings, RuleSource},
    kernel::KernelLoadOptions,
};

fn card(path: &Path) {
    fs::write(path,"[direct]\ndomain='unit_cube'\nparameters=['x']\n[[direct.terms]]\nprefactor='1/eps'\nmonomial_powers=['0']\n[[direct.terms.factors]]\npolynomial='x-1/3'\nexponent='-eps'\nsemantics='causal'\n[generation]\nthreshold_decomposition=true\norder=0\n").unwrap();
}
fn run(path: &Path, output: &Path, resume: bool) -> Artifact {
    generate_with_overrides(
        path,
        output,
        &mut Dashboard::new(false, false).unwrap(),
        None,
        2,
        resume,
        Default::default(),
    )
    .unwrap()
}
fn check(output: &Path) {
    let expected = [
        1.,
        0.,
        1. - (1f64 / 3.).ln() / 3. - 2. * (2f64 / 3.).ln() / 3.,
        std::f64::consts::PI / 3.,
    ];
    check_values(output, expected);
}
fn check_values(output: &Path, expected: [f64; 4]) {
    let (artifact, mut kernels) =
        Artifact::load_with_options(output, KernelLoadOptions { validate: true }).unwrap();
    assert_eq!(
        artifact.programs.as_ref().unwrap().default_recipe,
        ProgramRecipe::ThresholdV1
    );
    assert!(
        kernels
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
    let problem = fastsecdec::results::KernelResultManifest::integration_problem_from_kernels(
        &kernels,
        &fastsecdec::results::ResultScope::FullIntegral,
        "cli-threshold-scope",
    )
    .unwrap();
    let mut session = QmcSession::democratic(
        problem,
        QmcSettings {
            points: 4096,
            shifts: 8,
            seed: 202610107001,
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
    assert_eq!(kernels.orders(), [-1, -1, 0, 0]);
    for (i, value) in expected.iter().enumerate() {
        assert!(
            (estimate.mean[i] - value).abs()
                < 6. * estimate.covariance_of_mean[i * 4 + i].sqrt() + 1e-8,
            "{:?} != {:?}",
            estimate.mean,
            expected
        );
    }
}
#[test]
fn threshold_serial_and_normal_paths_share_native_identity_and_full_scope() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.toml");
    card(&input);
    let serial = dir.path().join("serial.fsd");
    let normal = dir.path().join("normal.fsd");
    run(&input, &serial, false);
    check(&serial);
    let (_, kernels) = crate::generate::generate_with_overrides(
        &input,
        &normal,
        &mut Dashboard::new(false, false).unwrap(),
        None,
        2,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        kernels.content_id(),
        Artifact::load(&serial).unwrap().1.content_id()
    );
    check(&normal);
}
#[test]
fn threshold_resume_reverifies_and_keeps_immutable_successful_worker_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.toml");
    card(&input);
    let output = dir.path().join("resumed.fsd");
    let mut journal = Journal::open(&input, &output, false, "initial").unwrap();
    let request = Request::PrepareThreshold {
        input: input.clone(),
        workers: 1,
        overrides: Default::default(),
        attempt: "initial".into(),
        prior: None,
    };
    journal
        .request("threshold-prepare-initial", request.clone())
        .unwrap();
    jobs::execute(&journal.job_path("threshold-prepare-initial"), &mut |_| {
        Ok(())
    })
    .unwrap();
    let Response::PreparedThreshold(prepared) = journal
        .accept("threshold-prepare-initial", &request)
        .unwrap()
    else {
        panic!("bad prepare");
    };
    journal
        .anchor_threshold_preparation("threshold-prepare-initial")
        .unwrap();
    let plan = jobs::threshold::publication(&journal.root, &prepared).unwrap();
    // Cache the final contribution first: publication must sort it after fresh
    // earlier workers, independently of cache/completion scheduling.
    let index = plan.job_count() - 1;
    let key = format!("threshold-compile-{index}");
    let record = journal.root.join(format!("threshold-record-{index}.fsd"));
    let request = Request::CompileThreshold {
        directory: Path::new(&prepared.directory)
            .join(&prepared.native.work_directory)
            .to_str()
            .unwrap()
            .into(),
        work: plan.work(index).unwrap(),
        output: record.clone(),
    };
    journal.request(&key, request.clone()).unwrap();
    jobs::execute(&journal.job_path(&key), &mut |_| Ok(())).unwrap();
    journal.accept(&key, &request).unwrap();
    // A cached success must be accepted under the freshly reverified anchor.
    // Its native work directory need not be regenerated byte-identically.
    let before = fs::read(journal.response_path(&key)).unwrap();
    assert!(!before.is_empty());
    let mut permissions = fs::metadata(&record).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&record, permissions).unwrap();
    drop(journal);
    run(&input, &output, true);
    check(&output);
}
#[test]
fn interrupted_raw_preparation_recovers_in_a_new_process() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.toml");
    card(&input);
    let output = dir.path().join("recovered.fsd");
    let mut journal = Journal::open(&input, &output, false, "interrupted").unwrap();
    let request = Request::PrepareThreshold {
        input: input.clone(),
        workers: 1,
        overrides: Default::default(),
        attempt: "interrupted".into(),
        prior: None,
    };
    journal
        .request("threshold-prepare-interrupted", request)
        .unwrap();
    assert!(
        jobs::execute(
            &journal.job_path("threshold-prepare-interrupted"),
            &mut |p| if p.detail.contains("Threshold Regularize") {
                Err(std::io::Error::other(
                    "test cancellation after raw evidence",
                ))
            } else {
                Ok(())
            }
        )
        .is_err()
    );
    assert!(journal.root.join("threshold-checkpoint.json").exists());
    assert!(journal.threshold_preparation().unwrap().is_none());
    drop(journal);
    run(&input, &output, true);
    check(&output);
}
#[test]
fn native_graph_bubble_card_uses_threshold_recipe_after_restore() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bubble.toml");
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let card = fs::read_to_string(workspace.join("examples/no_deformation/threshold_bubble.toml"))
        .unwrap()
        .replace(
            "../graphs/bubble.dot",
            workspace
                .join("examples/graphs/bubble.dot")
                .to_str()
                .unwrap(),
        )
        .replace(
            "../models/scalar.json",
            workspace
                .join("examples/models/scalar.json")
                .to_str()
                .unwrap(),
        );
    fs::write(&input, card).unwrap();
    let output = dir.path().join("bubble.fsd");
    run(&input, &output, false);
    let (mut artifact, mut kernels) = Artifact::load(&output).unwrap();
    crate::contour_cli::select_program(&mut artifact, &Default::default()).unwrap();
    assert_eq!(kernels.program_recipe(), ProgramRecipe::ThresholdV1);
    kernels.bind_parameters(&BTreeMap::new()).unwrap();
    assert!(
        kernels
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
    assert_eq!(kernels.orders(), [-1, -1, 0, 0]);
    check_values(
        &output,
        [1., 0., 2. - 1.5 * 3f64.ln(), std::f64::consts::PI / 2.],
    );
}

#[test]
fn raw_checkpoint_from_different_issued_overrides_is_refused() {
    fn copy_tree(source: &Path, target: &Path) {
        fs::create_dir_all(target).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let destination = target.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &destination);
            } else {
                fs::copy(entry.path(), destination).unwrap();
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.toml");
    card(&input);
    let mut old = Journal::open(&input, &dir.path().join("old.fsd"), false, "old").unwrap();
    old.request(
        "prepare-old",
        Request::PrepareThreshold {
            input: input.clone(),
            workers: 1,
            overrides: Default::default(),
            attempt: "old".into(),
            prior: None,
        },
    )
    .unwrap();
    assert!(
        jobs::execute(&old.job_path("prepare-old"), &mut |p| {
            if p.detail.contains("Threshold Regularize") {
                Err(std::io::Error::other("stop after durable raw evidence"))
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    let checkpoint = fs::read(old.root.join("threshold-checkpoint.json")).unwrap();
    let overrides = crate::config::GenerationOverrides {
        // A valid explicit choice, unlike the old omitted/default choice.
        // Physical source and card bytes are identical; issued context differs.
        contour_jacobian: Some(fastsecdec::contour::ContourJacobian::Symbolic),
        ..Default::default()
    };
    let mut current = Journal::open_with_overrides(
        &input,
        &dir.path().join("current.fsd"),
        false,
        "current",
        overrides,
    )
    .unwrap();
    copy_tree(
        &old.root.join("threshold-old"),
        &current.root.join("threshold-old"),
    );
    fs::write(current.root.join("threshold-checkpoint.json"), &checkpoint).unwrap();
    current
        .request(
            "prepare-current",
            Request::PrepareThreshold {
                input,
                workers: 2,
                overrides,
                attempt: "current".into(),
                prior: None,
            },
        )
        .unwrap();
    let error = jobs::execute(&current.job_path("prepare-current"), &mut |_| Ok(())).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("checkpoint issued settings changed"),
        "{error}"
    );
    assert!(!current.response_path("prepare-current").exists());
    assert_eq!(
        fs::read(current.root.join("threshold-checkpoint.json")).unwrap(),
        checkpoint
    );
}

#[test]
fn threshold_prepared_shape_is_bound_to_the_native_publication() {
    for regular in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.toml");
        if regular {
            fs::write(
                &input,
                include_str!("../../../../../examples/no_deformation/regular_sextic.toml"),
            )
            .unwrap();
        } else {
            card(&input);
        }
        let output = dir.path().join("shape.fsd");
        let mut journal = Journal::open(&input, &output, false, "shape").unwrap();
        let request = Request::PrepareThreshold {
            input: input.clone(),
            workers: 1,
            overrides: Default::default(),
            attempt: "shape".into(),
            prior: None,
        };
        journal
            .request("threshold-prepare-shape", request.clone())
            .unwrap();
        jobs::execute(
            &journal.job_path("threshold-prepare-shape"),
            &mut |_| Ok(()),
        )
        .unwrap();
        let Response::PreparedThreshold(prepared) =
            journal.accept("threshold-prepare-shape", &request).unwrap()
        else {
            panic!("bad prepare")
        };
        jobs::threshold::validate_prepared(&journal.root, &prepared).unwrap();
        assert_eq!(
            prepared.native.integration_dimensions,
            if regular { 2 } else { 1 }
        );
        for field in 0..4 {
            let mut forged = prepared.clone();
            match field {
                0 => forged.native.original_dimensions += 1,
                1 => forged.native.integration_dimensions += 1,
                2 => forged.native.cells += 1,
                3 => forged.native.endpoint_charts += 1,
                _ => unreachable!(),
            }
            assert!(jobs::threshold::validate_prepared(&journal.root, &forged).is_err());
        }
    }
}
