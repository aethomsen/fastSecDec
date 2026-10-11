use super::*;
#[test]
fn detached_template_jobs_restore_same_guard_and_native_runtime() {
    let (family, p) = family();
    let proof = Arc::downgrade(family.owner());
    let guard = Arc::downgrade(family.parameter_chamber().unwrap().guard());
    let continued = family
        .continue_symbolically(&GenerationOptions {
            mode: GenerationMode::Symbolic,
            max_order: 1,
            ..Default::default()
        })
        .unwrap();
    let staging = tempfile::tempdir().unwrap();
    let plan = ThresholdCompilationPlan::prepare_family(
        &continued,
        staging.path(),
        1,
        PrecisionPolicy {
            boundary_threshold: 1.,
            ..Default::default()
        },
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    drop(continued);
    drop(family);
    assert!(proof.upgrade().is_none());
    assert!(guard.upgrade().is_none());
    let mut writer = plan
        .archive_writer(std::io::Cursor::new(Vec::new()))
        .unwrap();
    for index in (0..plan.job_count()).rev() {
        let complete = plan.job(index).unwrap().run().unwrap();
        plan.validate_completion(&complete).unwrap();
        let mut bytes = Vec::new();
        let receipt = complete.write_record(&mut bytes).unwrap();
        writer
            .append_record(ProgramRecipe::ThresholdV1, &mut bytes.as_slice(), receipt)
            .unwrap();
    }
    let (cursor, _) = writer.finish().unwrap();
    let bytes = cursor.into_inner();
    assert!(bytes.starts_with(b"FastSecDec\0indexed\x04"));
    if let Some(path) = std::env::var_os("FASTSECDEC_PROJECTIVE_EXPORT") {
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(std::path::Path::new(&path).join("indexed.bin"), &bytes).unwrap();
    }
    let mut set =
        KernelSet::from_bytes_with_options(&bytes, KernelLoadOptions { validate: true }).unwrap();
    assert!(
        set.threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
    exercise(&mut set, p);
    let mut rebound = KernelSet::from_bytes(&set.to_bytes().unwrap()).unwrap();
    assert!(!rebound.parameters_bound());
    assert_eq!(rebound.content_id(), set.template_content_id());
    rebound
        .bind_parameters(&BTreeMap::from([(p, 16.)]))
        .unwrap();
    assert_eq!(rebound.content_id(), set.content_id());
    assert_eq!(point(&mut rebound), point(&mut set));
    let mut archive = indexed::ProgramArchiveReader::from_reader(
        std::io::Cursor::new(bytes),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert_eq!(archive.catalogue().version, 4);
    let mut exported = Vec::new();
    let directory = tempfile::tempdir().unwrap();
    for index in 0..set.sectors().len() {
        let selected = archive
            .select(ProgramRecipe::ThresholdV1)
            .unwrap()
            .load_sector(index)
            .unwrap();
        assert_eq!(selected.sectors().len(), 1);
        assert!(
            !selected
                .threshold_metadata()
                .unwrap()
                .full_original_scope()
                .unwrap()
        );
        let bytes = selected.to_bytes().unwrap();
        let mut selected = KernelSet::from_bytes(&bytes).unwrap();
        selected
            .bind_parameters(&BTreeMap::from([(p, 16.)]))
            .unwrap();
        let value = point(&mut selected);
        let mut expected = vec![0.; value.len()];
        set.sectors_mut()[index]
            .replay_scaled(&[0.37], &mut expected, 1., 128)
            .unwrap();
        assert_eq!(value, expected);
        let name = format!("selected-{index}.bin");
        std::fs::write(directory.path().join(&name), bytes).unwrap();
        exported.push(serde_json::json!({"file":name,"value":value,"template":selected.template_content_id(),"point":selected.content_id(),"full":false}));
    }
    let mut exact = archive
        .select(ProgramRecipe::ThresholdV1)
        .unwrap()
        .load_exact()
        .unwrap();
    assert!(exact.sectors().is_empty());
    assert!(!exact.parameters_bound());
    assert!(
        !exact
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
    assert!(exact.bind_parameters(&BTreeMap::from([(p, 15.)])).is_err());
    assert!(!exact.parameters_bound());
    exact.bind_parameters(&BTreeMap::from([(p, 16.)])).unwrap();
    assert_eq!(exact.exact_coefficients(), [0.; 6]);
    std::fs::write(
        directory.path().join("exact.bin"),
        exact.to_bytes().unwrap(),
    )
    .unwrap();
    exported.push(serde_json::json!({"file":"exact.bin","value":exact.exact_coefficients(),"template":exact.template_content_id(),"point":exact.content_id(),"full":false}));
    std::fs::write(directory.path().join("full.bin"), set.to_bytes().unwrap()).unwrap();
    exported.push(serde_json::json!({"file":"full.bin","value":point(&mut set),"template":set.template_content_id(),"point":set.content_id(),"full":true}));
    std::fs::write(
        directory.path().join("expected.json"),
        serde_json::to_vec_pretty(&exported).unwrap(),
    )
    .unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "kernel::projective_parameter_tests::artifacts::restore_child",
            "--nocapture",
        ])
        .env("FASTSECDEC_PROJECTIVE_CHILD", directory.path())
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    let (refreshed, _) = archive
        .write_with_native_cache(std::io::Cursor::new(Vec::new()), |_| {
            std::ops::ControlFlow::Continue(())
        })
        .unwrap();
    let mut refreshed = KernelSet::from_bytes(&refreshed.into_inner()).unwrap();
    assert_eq!(refreshed.template_content_id(), set.template_content_id());
    refreshed
        .bind_parameters(&BTreeMap::from([(p, 16.)]))
        .unwrap();
    assert_eq!(point(&mut refreshed), point(&mut set));
    if let Some(path) = std::env::var_os("FASTSECDEC_PROJECTIVE_EXPORT") {
        std::fs::create_dir_all(&path).unwrap();
        for file in std::fs::read_dir(directory.path()).unwrap() {
            let file = file.unwrap();
            std::fs::copy(
                file.path(),
                std::path::Path::new(&path).join(file.file_name()),
            )
            .unwrap();
        }
    }
}
#[test]
#[ignore = "fresh native State import called by detached template test"]
fn restore_child() {
    let path = std::path::PathBuf::from(std::env::var_os("FASTSECDEC_PROJECTIVE_CHILD").unwrap());
    for i in (0..100).rev() {
        let _ = symbol!(format!("parameter_child_scramble::s{i}"));
    }
    let rows: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(path.join("expected.json")).unwrap()).unwrap();
    for row in rows {
        let mut set = KernelSet::from_bytes_with_options(
            &std::fs::read(path.join(row["file"].as_str().unwrap())).unwrap(),
            KernelLoadOptions { validate: true },
        )
        .unwrap();
        assert!(!set.parameters_bound());
        assert_eq!(set.template_content_id(), row["template"].as_str().unwrap());
        assert_eq!(
            set.threshold_metadata()
                .unwrap()
                .full_original_scope()
                .unwrap(),
            row["full"].as_bool().unwrap()
        );
        let p = set.runtime_parameters()[0];
        assert_eq!(p.get_name(), "compiled_projective::s");
        assert!(set.bind_parameters(&BTreeMap::from([(p, 18.)])).is_err());
        assert!(!set.parameters_bound());
        set.bind_parameters(&BTreeMap::from([(p, 16.)])).unwrap();
        assert_eq!(set.content_id(), row["point"].as_str().unwrap());
        let expected: Vec<f64> = serde_json::from_value(row["value"].clone()).unwrap();
        assert_eq!(point(&mut set), expected);
    }
}
