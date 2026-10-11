//! CLI transport and full-point identity over an actual native indexed template.
use super::*;
use fastsecdec::{
    integration::{QmcSession, QmcSettings},
    kernel::{
        StabilitySettings,
        indexed::{ProgramArchiveReader, ProgramRecipe},
    },
    results::ResultScope,
};
use std::{collections::BTreeMap, fs};

fn publish(bytes: &Path, output: &Path) -> Artifact {
    let reader = ProgramArchiveReader::from_reader(
        File::open(bytes).unwrap(),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    let catalogue = reader.catalogue().clone();
    assert_eq!(catalogue.version, 4);
    let provenance = crate::artifact::Provenance {
        name: "saved native uniform template transport control".into(),
        sources: vec![],
        dependencies: crate::artifact::dependencies(),
        domain: "ProjectiveSimplex".into(),
        assume_no_threshold: false,
        dimension: "4-2*eps".into(),
        regulator: "eps".into(),
        measure: "native".into(),
        measure_multiplier: "1".into(),
        max_order: 1,
        integration: serde_json::json!({}),
        family_preparation: None,
        model_parameter_defaults: BTreeMap::new(),
    };
    let artifact = Artifact::from_program_archive(
        bytes,
        catalogue.clone(),
        ProgramRecipe::ThresholdV1,
        provenance.clone(),
    )
    .unwrap();
    artifact.save_staged(output).unwrap();
    let loaded = Artifact::load_metadata(output).unwrap();
    assert_eq!(artifact.content_id, loaded.content_id);
    let mut future = catalogue;
    future.version = 5;
    assert!(
        Artifact::from_program_archive(
            bytes,
            future.clone(),
            ProgramRecipe::ThresholdV1,
            provenance,
        )
        .is_err()
    );
    let mut storage = loaded.programs.as_ref().unwrap().clone();
    storage.catalogue = future;
    // This reaches the same metadata validation used during every CLI load.
    let mut bad = Artifact::load_metadata(output).unwrap();
    bad.programs = Some(storage);
    assert!(
        bad.open_program_archive(output, KernelLoadOptions { validate: true })
            .is_err()
    );
    loaded
}

fn exercise(path: &Path, name: &str, points: [f64; 2]) {
    let mut problems = Vec::new();
    for validate in [false, true] {
        let (artifact, mut full) =
            Artifact::load_with_options(path, KernelLoadOptions { validate }).unwrap();
        let template = full.template_content_id().to_owned();
        let mut ids = Vec::new();
        for value in [points[0], points[1], points[0]] {
            let settings = IntegrationInput {
                parameters: BTreeMap::from([(name.into(), value)]),
                stability: StabilitySettings::validated(),
                ..Default::default()
            };
            crate::bind_parameters(&mut full, &settings).unwrap();
            let staging = tempfile::tempdir().unwrap();
            let request = staging.path().join("request.json");
            let result = staging.path().join("result.json");
            fs::write(
                &request,
                serde_json::to_vec(&Request::Exact {
                    artifact: path.into(),
                    expected: artifact.kernel_content_id.clone(),
                    settings: Box::new(settings.clone()),
                    validate,
                    result: result.clone(),
                })
                .unwrap(),
            )
            .unwrap();
            execute(request, &mut |_| Ok(())).unwrap();
            let setup: ExactSetup = serde_json::from_reader(File::open(result).unwrap()).unwrap();
            let resident = KernelResultManifest::from_kernels(&full);
            assert_eq!(
                serde_json::to_value(&setup.manifest).unwrap(),
                serde_json::to_value(&resident).unwrap()
            );
            assert_eq!(full.template_content_id(), template);
            ids.push(setup.manifest.kernel_content_id.clone());
            problems.push(
                setup
                    .manifest
                    .integration_problem(
                        &ResultScope::FullIntegral,
                        format!(
                            "{}:{}:{}",
                            artifact.content_id, template, setup.manifest.kernel_content_id
                        ),
                    )
                    .unwrap(),
            );
        }
        assert_ne!(ids[0], ids[1]);
        assert_eq!(ids[0], ids[2]);
    }
    // No CLI settings envelope participates in this native wrong-point check.
    let session = QmcSession::democratic(problems[0].clone(), QmcSettings::default()).unwrap();
    let saved = session.checkpoint().unwrap();
    QmcSession::restore(&saved, &problems[2]).unwrap();
    assert!(QmcSession::restore(&saved, &problems[1]).is_err());
    use fastsecdec::integration::{
        AccuracyTarget, Tolerance,
        serial::{SerialMethod, SerialSession, SerialSettings},
    };
    let serial = SerialSession::new(
        problems[0].clone(),
        SerialSettings {
            method: SerialMethod::Qmc(QmcSettings::default()),
            double_points: false,
            max_rounds: Some(1),
            max_in_flight: 2,
            target: AccuracyTarget::default(),
            tolerance: Tolerance::new(0., 0.).unwrap(),
            pilot_iterations: 1,
            learning_rate: 1.,
        },
    )
    .unwrap();
    let saved = serial.checkpoint().unwrap();
    SerialSession::restore(&saved, &problems[2]).unwrap();
    assert!(SerialSession::restore(&saved, &problems[1]).is_err());
}

#[test]
#[ignore = "explicit historical native V4 fixture, supplied by the acceptance campaign"]
fn saved_projective_v4_uses_full_bound_identity_in_exact_setup() {
    let source = PathBuf::from(std::env::var_os("FASTSECDEC_CLI_PARAMETER_ARCHIVE").unwrap());
    let dir = tempfile::tempdir().unwrap();
    // save_staged owns/moves its input; the historical evidence stays immutable.
    let staged = dir.path().join("staged.bin");
    fs::copy(source, &staged).unwrap();
    let output = dir.path().join("uniform.fsd");
    publish(&staged, &output);
    exercise(&output, "compiled_projective::s", [16., 17.]);
}

#[test]
fn declared_direct_runtime_template_reaches_exact_setup() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("direct.toml");
    fs::write(
        &input,
        include_str!("../../../../examples/no_deformation/parameter_templates/quintic.toml"),
    )
    .unwrap();
    let output = dir.path().join("direct.fsd");
    // Compile through the public native issuer; the separate CLI process matrix
    // exercises the wrapper's preparation worker and checkpoint forwarding.
    let loaded = crate::input::load(&input).unwrap();
    let mut options = fastsecdec::threshold::generation::PreparationOptions::new(
        crate::input::symbol("cli_uniform_test::u").unwrap(),
    );
    options.threshold = loaded
        .card
        .threshold_decomposition
        .native(&loaded.runtime_parameters)
        .unwrap();
    options.regular_family = Some(Default::default());
    options.generation.max_order = 1;
    let stage = dir.path().join("native");
    fs::create_dir(&stage).unwrap();
    let receipt = fastsecdec::threshold::generation::prepare(
        std::sync::Arc::new(loaded.integrand),
        options,
        &stage,
        |_| std::ops::ControlFlow::Continue(()),
    )
    .unwrap()
    .receipt;
    let work_root = stage.join(&receipt.work_directory);
    let plan = fastsecdec::kernel::ThresholdPublicationPlan::from_trusted_preparer(
        &work_root,
        receipt.publication.clone(),
        &receipt.publication.source_identity,
        &receipt.publication.prepared_identity,
        16 * 1024 * 1024,
    )
    .unwrap();
    let staged = dir.path().join("native.bin");
    let mut writer = plan.archive_writer(File::create(&staged).unwrap()).unwrap();
    for i in 0..plan.job_count() {
        let mut record = Vec::new();
        let completed = plan
            .work(i)
            .unwrap()
            .compile_record(&work_root, 16 * 1024 * 1024, &mut record)
            .unwrap();
        plan.append_work_record(&mut writer, &mut record.as_slice(), completed)
            .unwrap();
    }
    writer.finish().unwrap();
    publish(&staged, &output);
    exercise(&output, "cli_uniform::p", [2.3125, 2.4375]);
    if let Some(actual_cli_archive) = std::env::var_os("FASTSECDEC_CLI_DIRECT_ARCHIVE") {
        let (_, mut native) = Artifact::load(&output).unwrap();
        let mut reader = ProgramArchiveReader::from_reader(
            File::open(actual_cli_archive).unwrap(),
            KernelLoadOptions { validate: true },
        )
        .unwrap();
        let mut cli = reader
            .select(ProgramRecipe::ThresholdV1)
            .unwrap()
            .load_all()
            .unwrap();
        assert_eq!(native.orders(), cli.orders());
        assert_eq!(native.components(), cli.components());
        for p in [2.3125, 2.4375] {
            let settings = IntegrationInput {
                parameters: BTreeMap::from([("cli_uniform::p".into(), p)]),
                stability: StabilitySettings::validated(),
                ..Default::default()
            };
            let mut vectors = Vec::new();
            for kernels in [&mut native, &mut cli] {
                crate::bind_parameters(kernels, &settings).unwrap();
                let mut total = kernels.exact_coefficients().to_vec();
                for sector in kernels.sectors_mut() {
                    let mut row = vec![0.; total.len()];
                    sector.evaluate(&[0.37], &mut row).unwrap();
                    for (sum, value) in total.iter_mut().zip(row) {
                        *sum += value;
                    }
                }
                vectors.push(total);
            }
            for (native, cli) in vectors[0].iter().zip(&vectors[1]) {
                assert!(
                    (native - cli).abs() <= 2e-10 * (1. + native.abs()),
                    "{native} != {cli}"
                );
            }
        }
    }
}
