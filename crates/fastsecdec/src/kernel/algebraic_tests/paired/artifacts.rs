use super::*;

#[test]
fn detached_three_cell_native_indexed_lineage() {
    let bytes = archived(0, true);
    let k = KernelSet::from_bytes_with_options(
        &bytes,
        fastsecdec::kernel::KernelLoadOptions { validate: true },
    )
    .unwrap();
    let m = k.threshold_metadata().unwrap();
    assert!(m.full_original_scope().unwrap());
    assert_eq!(m.lineage().cells.len(), 3);
    assert_eq!(m.lineage().endpoint_charts.len(), 4);
    assert_eq!(k.sectors().len(), 4);
    assert!(m.require_global_proof().is_err());
    for chart in &m.lineage().endpoint_charts {
        if let fastsecdec::kernel::threshold_metadata::MapDescriptor::RegularSecantV1 {
            sections,
            ..
        } = &chart.map
        {
            assert_eq!(sections.len(), 2);
            assert_ne!(sections[0].root_variable, sections[1].root_variable);
            assert_ne!(
                sections[0].branch_certificate,
                sections[1].branch_certificate
            );
        } else {
            panic!("paired descriptor");
        }
    }
    let mut reader = fastsecdec::kernel::indexed::ProgramArchiveReader::from_reader(
        Cursor::new(bytes),
        Default::default(),
    )
    .unwrap();
    let mut recipe = reader.select(ProgramRecipe::ThresholdV1).unwrap();
    let selected = recipe.load_sector(1).unwrap();
    assert!(
        !selected
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
    let restored = KernelSet::from_bytes_with_options(
        &selected.to_bytes().unwrap(),
        fastsecdec::kernel::KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert_eq!(selected.content_id(), restored.content_id());
    assert!(
        !restored
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
}

#[test]
fn paired_geometry_and_saved_helper_tampering_refused() {
    use fastsecdec::threshold::gcad::NativeDecomposition;
    let (o, u) = source(0);
    let mut raw = o.native_result().clone();
    let middle = raw
        .cells
        .iter_mut()
        .find(|c| {
            c.axes[1]
                .lower
                .as_ref()
                .is_some_and(|b| b.polynomial == c.axes[1].upper.as_ref().unwrap().polynomial)
        })
        .unwrap();
    middle.axes[1].lower.as_mut().unwrap().index += 1;
    assert!(
        o.request()
            .verify(NativeDecomposition::from_parts(
                o.request().identity().clone(),
                raw
            ))
            .is_err()
    );
    let f = NormalizedFamily::admit_pair_affine(o, u, proposals(), limits()).unwrap();
    let saved = f.callback_scope().saved();
    assert_eq!(saved.len(), 2);
    let owners = f
        .callback_scope()
        .owners()
        .map(|(_, p)| p.clone())
        .collect::<Vec<_>>();
    assert_ne!(owners[0].semantic_identity(), owners[1].semantic_identity());
    assert_ne!(owners[0].transport_symbol(), owners[1].transport_symbol());
    let altered = regular::callback::Scope::restore(vec![
        (saved[0].0, saved[1].1.clone()),
        (saved[1].0, saved[0].1.clone()),
    ])
    .unwrap();
    let program = crate::kernel::program::PreparedCoefficientVector {
        runtime_parameters: Vec::new(),
        coordinates: f.coordinates().to_vec(),
        coefficients: f.roots().iter().cloned().map(Into::into).collect(),
        functions: Default::default(),
        endpoint_profiles: vec![],
    }
    .build(CompilationSettings::default())
    .unwrap();
    let error = altered.enter(53, || {
        crate::kernel::SectorKernel::from_program_with_backend(
            program,
            &Default::default(),
            true,
            EvaluatorBackend::Eager,
        )
    });
    assert!(
        error.is_err(),
        "other-branch helper cannot satisfy the original callback contract"
    );
}

#[test]
fn paired_incomplete_archive_and_duplicate_record_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let f = family(0, true);
    let c = f.continue_symbolically(&options(0)).unwrap();
    let weak = f
        .callback_scope()
        .owners()
        .map(|(_, p)| Arc::downgrade(p))
        .collect::<Vec<_>>();
    let plan = ThresholdCompilationPlan::prepare_family(
        &c,
        tmp.path(),
        0,
        Default::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    drop(c);
    drop(f);
    assert!(weak.iter().all(|w| w.upgrade().is_none()));
    let mut writer = plan.archive_writer(Cursor::new(Vec::new())).unwrap();
    let mut duplicated = None;
    for i in 0..plan.job_count() {
        if i == 3 {
            continue;
        }
        let done = plan.job(i).unwrap().run().unwrap();
        let mut bytes = Vec::new();
        let receipt = done.write_record(&mut bytes).unwrap();
        writer
            .append_record(
                ProgramRecipe::ThresholdV1,
                &mut bytes.as_slice(),
                receipt.clone(),
            )
            .unwrap();
        if i == 2 {
            duplicated = Some((bytes, receipt));
        }
    }
    let (bytes, receipt) = duplicated.unwrap();
    // Appends are transport-only; closed inventory admission happens at finish.
    writer
        .append_record(ProgramRecipe::ThresholdV1, &mut bytes.as_slice(), receipt)
        .unwrap();
    assert!(
        writer.finish().is_err(),
        "one missing middle half is incomplete even when all three source cells are represented"
    );
}

fn artifact_observations(bytes: &[u8]) -> serde_json::Value {
    let k = KernelSet::from_bytes_with_options(
        bytes,
        fastsecdec::kernel::KernelLoadOptions { validate: true },
    )
    .unwrap();
    let m = k.threshold_metadata().unwrap();
    assert_eq!(m.lineage().cells.len(), 3);
    assert_eq!(m.lineage().endpoint_charts.len(), 4);
    let roots = m
        .lineage()
        .endpoint_charts
        .iter()
        .map(|c| {
            let fastsecdec::kernel::threshold_metadata::MapDescriptor::RegularSecantV1 {
                sections,
                ..
            } = &c.map
            else {
                panic!("paired map")
            };
            assert_eq!(sections.len(), 2);
            assert_ne!(sections[0].root_variable, sections[1].root_variable);
            sections
                .iter()
                .map(|s| {
                    (
                        m.native_symbol(s.root_variable)
                            .unwrap()
                            .get_name()
                            .to_string(),
                        s.branch_certificate.0.clone(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let vectors = (0..k.sectors().len())
        .map(|i| {
            let mut out = vec![0.; k.components().len()];
            k.evaluation_context(i, Default::default())
                .unwrap()
                .evaluate_weighted(&[0.23, 0.37], 1., &mut out)
                .unwrap();
            out
        })
        .collect::<Vec<_>>();
    serde_json::json!({"content_id":k.content_id(),"roots":roots,"vectors":vectors})
}
#[test]
#[ignore = "fresh-process helper invoked by paired_fresh_native_restore"]
fn paired_restore_child() {
    for i in 0..19 {
        Symbol::parse(format!("paired_fresh::shift_{i}"), "").unwrap();
    }
    let directory = std::env::var_os("FASTSECDEC_PAIRED_ARTIFACT").unwrap();
    let directory = std::path::Path::new(&directory);
    let bytes = std::fs::read(directory.join("paired.bin")).unwrap();
    let actual = artifact_observations(&bytes);
    let expected: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("expected.json")).unwrap()).unwrap();
    assert_eq!(actual, expected);
    let mut reader = fastsecdec::kernel::indexed::ProgramArchiveReader::from_reader(
        Cursor::new(bytes),
        Default::default(),
    )
    .unwrap();
    let selected = reader
        .select(ProgramRecipe::ThresholdV1)
        .unwrap()
        .load_sector(2)
        .unwrap();
    let resaved = selected.to_bytes().unwrap();
    let restored = KernelSet::from_bytes_with_options(
        &resaved,
        fastsecdec::kernel::KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert_eq!(restored.content_id(), selected.content_id());
    assert!(
        !restored
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
}
#[test]
fn paired_fresh_native_restore() {
    let directory = tempfile::tempdir().unwrap();
    let bytes = archived(0, true);
    let observation = artifact_observations(&bytes);
    std::fs::write(directory.path().join("paired.bin"), bytes).unwrap();
    std::fs::write(
        directory.path().join("expected.json"),
        serde_json::to_vec(&observation).unwrap(),
    )
    .unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "kernel::algebraic_tests::paired::artifacts::paired_restore_child",
            "--ignored",
            "--test-threads=1",
        ])
        .env("FASTSECDEC_PAIRED_ARTIFACT", directory.path())
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
#[ignore = "invoked by the paired fresh-record test"]
fn paired_zero_record_child() {
    for i in 0..31 {
        let _ = Symbol::parse(format!("paired_record_child::unrelated{i}"), "").unwrap();
    }
    let path = std::env::var("FSD_PAIRED_ZERO_WORK").unwrap();
    let work: crate::kernel::ThresholdCompilationWork =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let root = std::path::Path::new(&path).parent().unwrap();
    let mut output = std::fs::File::create(root.join("paired-worker.fsd")).unwrap();
    let receipt = work
        .compile_record(root, 16 * 1024 * 1024, &mut output)
        .unwrap();
    std::fs::write(
        root.join("paired-worker.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
}

#[test]
fn paired_local_zero_survives_fresh_worker_and_artifact() {
    let tmp = tempfile::tempdir().unwrap();
    let f = family(2, true);
    let c = f.continue_symbolically(&options(2)).unwrap();
    let plan = ThresholdCompilationPlan::prepare_family(
        &c,
        tmp.path(),
        2,
        Default::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    let weak = Arc::downgrade(f.owner());
    drop(c);
    drop(f);
    assert!(weak.upgrade().is_none());
    let mut writer = plan.archive_writer(Cursor::new(Vec::new())).unwrap();
    for index in (0..plan.job_count()).rev() {
        let work = plan.work(index).unwrap();
        if index == 1 {
            let path = tmp.path().join("paired-work.json");
            std::fs::write(&path, serde_json::to_vec(&work).unwrap()).unwrap();
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "kernel::algebraic_tests::paired::artifacts::paired_zero_record_child",
                    "--ignored",
                    "--test-threads=1",
                ])
                .env("FSD_PAIRED_ZERO_WORK", &path)
                .status()
                .unwrap();
            assert!(status.success());
            let receipt = serde_json::from_slice(
                &std::fs::read(tmp.path().join("paired-worker.json")).unwrap(),
            )
            .unwrap();
            plan.validate_work_receipt(&receipt).unwrap();
            plan.append_work_record(
                &mut writer,
                &mut std::fs::File::open(tmp.path().join("paired-worker.fsd")).unwrap(),
                receipt,
            )
            .unwrap();
        } else {
            let mut bytes = Vec::new();
            let receipt = work
                .compile_record(tmp.path(), 16 * 1024 * 1024, &mut bytes)
                .unwrap();
            plan.append_work_record(&mut writer, &mut bytes.as_slice(), receipt)
                .unwrap();
        }
    }
    let (bytes, catalogue) = writer.finish().unwrap();
    let k = KernelSet::from_bytes_with_options(
        &bytes.into_inner(),
        crate::kernel::KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert_eq!(
        k.content_id(),
        catalogue
            .recipe(ProgramRecipe::ThresholdV1)
            .unwrap()
            .content_id
    );
    let restored = KernelSet::from_bytes_with_options(
        &k.to_bytes().unwrap(),
        crate::kernel::KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert_eq!(k.content_id(), restored.content_id());
    let mut context = restored.evaluation_context(0, Default::default()).unwrap();
    let mut outputs = vec![0.; 6];
    context
        .evaluate_weighted_batch(&[0.37, 0.42], &[1.], &mut outputs)
        .unwrap();
    assert_eq!(&outputs[..2], &[0., 0.]);
}
