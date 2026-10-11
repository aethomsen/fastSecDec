use super::*;
#[test]
fn actual_fresh_compiler_worker_restores_guard_and_preserves_complete_template() {
    let (family, p) = family();
    let continued = family
        .continue_symbolically(&GenerationOptions {
            mode: GenerationMode::Symbolic,
            max_order: 1,
            ..Default::default()
        })
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let plan = ThresholdCompilationPlan::prepare_family(
        &continued,
        dir.path(),
        1,
        PrecisionPolicy::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    let proof = Arc::downgrade(family.owner());
    drop(continued);
    drop(family);
    assert!(proof.upgrade().is_none());
    let work = plan.work(1).unwrap();
    std::fs::write(
        dir.path().join("work.json"),
        serde_json::to_vec(&work).unwrap(),
    )
    .unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "kernel::parameter_template_tests::transport::compiler_child",
            "--nocapture",
        ])
        .env("FASTSECDEC_PARAMETER_WORKER", dir.path())
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    let receipt: ThresholdWorkReceipt =
        serde_json::from_slice(&std::fs::read(dir.path().join("receipt.json")).unwrap()).unwrap();
    plan.validate_work_receipt(&receipt).unwrap();
    let mut writer = plan
        .archive_writer(std::io::Cursor::new(Vec::new()))
        .unwrap();
    for index in (0..plan.job_count()).rev() {
        let (mut bytes, receipt) = if index == 1 {
            (
                std::fs::read(dir.path().join("record.bin")).unwrap(),
                receipt.clone(),
            )
        } else {
            let mut bytes = Vec::new();
            let receipt = plan
                .work(index)
                .unwrap()
                .compile_record(dir.path(), 16 * 1024 * 1024, &mut bytes)
                .unwrap();
            (bytes, receipt)
        };
        plan.append_work_record(&mut writer, &mut bytes.as_slice(), receipt)
            .unwrap();
        bytes.clear();
    }
    let (bytes, _) = writer.finish().unwrap();
    let mut kernels = KernelSet::from_bytes(&bytes.into_inner()).unwrap();
    assert!(
        kernels
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
    exercise(&mut kernels, p);
}
#[test]
#[ignore = "fresh compiler process called by actual_fresh_compiler_worker"]
fn compiler_child() {
    let root = std::path::PathBuf::from(std::env::var_os("FASTSECDEC_PARAMETER_WORKER").unwrap());
    for i in (0..100).rev() {
        let _ = symbol!(format!("parameter_compiler_scramble::s{i}"));
    }
    let work: ThresholdCompilationWork =
        serde_json::from_slice(&std::fs::read(root.join("work.json")).unwrap()).unwrap();
    let mut bytes = Vec::new();
    let receipt = work
        .compile_record(&root, 16 * 1024 * 1024, &mut bytes)
        .unwrap();
    std::fs::write(root.join("record.bin"), bytes).unwrap();
    std::fs::write(
        root.join("receipt.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
}
