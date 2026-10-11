use super::*;
fn write_artifacts(directory: &std::path::Path) {
    std::fs::create_dir_all(directory).unwrap();
    for shifted in [false, true] {
        let family = family(shifted);
        let continued = family
            .continue_symbolically(&generation::GenerationOptions {
                mode: generation::GenerationMode::Symbolic,
                ..Default::default()
            })
            .unwrap();
        let orders = if shifted { vec![0] } else { vec![-1, 0, 1] };
        for backend in [EvaluatorBackend::Eager, EvaluatorBackend::Symjit] {
            let settings = CompilationSettings {
                backend,
                ..Default::default()
            };
            let programs = continued
                .charts()
                .iter()
                .zip(continued.profiles())
                .map(|(expression, profiles)| {
                    let coefficients = generation::threshold_expand_vector(
                        expression,
                        family.coordinates(),
                        family.regulators()[0],
                        *orders.last().unwrap(),
                    )
                    .unwrap();
                    PreparedCoefficientVector {
                        coordinates: family.coordinates().to_vec(),
                        coefficients: orders
                            .iter()
                            .map(|o| {
                                coefficients
                                    .get(o)
                                    .cloned()
                                    .unwrap_or_else(|| AliasedAtom::from(Atom::zero()))
                            })
                            .collect(),
                        functions: Arc::new(continued.functions().clone()),
                        endpoint_profiles: profiles.clone(),
                    }
                    .build(settings)
                    .unwrap()
                })
                .collect();
            let set = family
                .callback_scope()
                .enter(53, || {
                    crate::kernel::KernelSet::from_programs_for_load_with_progress(
                        orders.clone(),
                        programs,
                        vec![Atom::zero(); orders.len()],
                        PrecisionPolicy::default(),
                        None,
                        true,
                        vec![],
                        settings,
                        None,
                        None,
                        false,
                        &mut |_| std::ops::ControlFlow::Continue(()),
                    )
                })
                .unwrap();
            assert!(set.requires_validated_algebraic_callbacks());
            assert_eq!(
                set.stability_settings().mode,
                crate::kernel::StabilityMode::Validated
            );
            let standalone = crate::kernel::artifact::test_standalone(&set);
            let stem = format!(
                "{}-{}",
                if shifted { "close" } else { "normal" },
                if backend == EvaluatorBackend::Eager {
                    "eager"
                } else {
                    "symjit"
                }
            );
            std::fs::write(directory.join(format!("{stem}-standalone.bin")), standalone).unwrap();
            assert!(
                set.sectors()
                    .iter()
                    .all(|sector| sector.contour_validation_report().is_none())
            );
            let bytes = set.to_bytes().unwrap();
            let mut selective = crate::kernel::indexed::IndexedReader::from_reader(
                std::io::Cursor::new(bytes.clone()),
                crate::kernel::KernelLoadOptions::default(),
            )
            .unwrap();
            let selected = selective.load_sector(1).unwrap();
            assert_eq!(selected.sectors().len(), 1);
            assert!(selected.requires_validated_algebraic_callbacks());
            let (refreshed, _) = selective
                .write_with_native_cache(std::io::Cursor::new(Vec::new()), |_| {
                    std::ops::ControlFlow::Continue(())
                })
                .unwrap();
            let refreshed = crate::kernel::KernelSet::from_bytes(&refreshed.into_inner()).unwrap();
            assert!(refreshed.requires_validated_algebraic_callbacks());
            let mut decoded = crate::kernel::KernelSet::from_bytes(&bytes).unwrap();
            assert!(decoded.requires_validated_algebraic_callbacks());
            assert_eq!(
                decoded.stability_settings().mode,
                crate::kernel::StabilityMode::Validated
            );
            assert!(
                decoded
                    .sectors()
                    .iter()
                    .all(|sector| sector.contour_validation_report().is_none())
            );
            assert!(
                decoded
                    .set_stability_settings(&StabilitySettings::default())
                    .is_err()
            );
            let mut cutoff = StabilitySettings::validated();
            cutoff.unstable_cutoff = Some(1e-12);
            assert!(decoded.set_stability_settings(&cutoff).is_err());
            decoded
                .set_stability_settings(&StabilitySettings::validated())
                .unwrap();
            // Native indexed catalogues order records by their issued identity,
            // not the original resident vector position. Match exact native IR
            // before comparing values, retaining a one-to-one inventory.
            let mut unmatched = (0..decoded.sectors().len()).collect::<Vec<_>>();
            for old in set.sectors() {
                let original = crate::kernel::program::encode(old.exact_program()).unwrap();
                let position = unmatched
                    .iter()
                    .position(|index| {
                        crate::kernel::program::encode(decoded.sectors()[*index].exact_program())
                            .unwrap()
                            == original
                    })
                    .expect("restored native program association");
                let index = unmatched.remove(position);
                let new = &mut decoded.sectors_mut()[index];
                let mut old = old.try_clone().unwrap();
                let mut a = vec![0.; old.output_count()];
                let mut b = a.clone();
                old.evaluate(&[1., 0.37], &mut a).unwrap();
                new.evaluate(&[1., 0.37], &mut b).unwrap();
                assert_eq!(a, b);
            }
            assert!(unmatched.is_empty());
            let name = format!(
                "{}-{}.bin",
                if shifted { "close" } else { "normal" },
                if backend == EvaluatorBackend::Eager {
                    "eager"
                } else {
                    "symjit"
                }
            );
            std::fs::write(directory.join(name), bytes).unwrap();
        }
    }
}

/// Existing public plan/record/indexed pipeline, with no retained proof owners.
pub(super) fn factory(path: &std::path::Path) -> serde_json::Value {
    factory_recipe(path, false)
}
pub(super) fn factory_recipe(path: &std::path::Path, affine: bool) -> serde_json::Value {
    use crate::kernel::{KernelSet, ThresholdCompilationPlan};
    std::fs::create_dir_all(path).unwrap();
    let mut results = Vec::new();
    for backend in [EvaluatorBackend::Eager, EvaluatorBackend::Symjit] {
        let family = family_recipe(false, 20, affine);
        let proof = Arc::downgrade(family.owner());
        let root = family
            .callback_scope()
            .owners()
            .next()
            .map(|(_, r)| Arc::downgrade(r))
            .unwrap();
        let continued = family
            .continue_symbolically(&generation::GenerationOptions {
                mode: generation::GenerationMode::Symbolic,
                max_order: 1,
                ..Default::default()
            })
            .unwrap();
        let staging = path.join(format!("{backend:?}"));
        std::fs::create_dir_all(&staging).unwrap();
        let plan = ThresholdCompilationPlan::prepare_family(
            &continued,
            &staging,
            1,
            Default::default(),
            CompilationSettings {
                backend,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(plan.job_count(), 3);
        drop(continued);
        drop(family);
        assert!(
            proof.upgrade().is_none(),
            "detached plan retained global proof"
        );
        assert!(
            root.upgrade().is_none(),
            "detached plan retained callback preparation"
        );
        let mut writer = plan
            .archive_writer(std::io::Cursor::new(Vec::new()))
            .unwrap();
        for index in (0..plan.job_count()).rev() {
            let complete = plan.job(index).unwrap().run().unwrap();
            plan.validate_completion(&complete).unwrap();
            let mut bytes = Vec::new();
            let receipt = complete.write_record(&mut bytes).unwrap();
            writer
                .append_record(
                    crate::kernel::ProgramRecipe::ThresholdV1,
                    &mut bytes.as_slice(),
                    receipt,
                )
                .unwrap();
        }
        let (cursor, _) = writer.finish().unwrap();
        let bytes = cursor.into_inner();
        std::fs::write(path.join(format!("family-{backend:?}.bin")), &bytes).unwrap();
        let mut kernels = KernelSet::from_bytes_with_options(
            &bytes,
            crate::kernel::KernelLoadOptions { validate: true },
        )
        .unwrap();
        assert_eq!(kernels.sectors().len(), 2);
        assert_eq!(
            kernels.algebraic_callback_recipe(),
            Some(if affine {
                crate::kernel::AlgebraicCallbackRecipe::AffineRationalBracketV2
            } else {
                crate::kernel::AlgebraicCallbackRecipe::StaticExactF64BracketV1
            })
        );
        assert!(
            kernels
                .threshold_metadata()
                .unwrap()
                .full_original_scope()
                .unwrap()
        );
        assert_eq!(
            kernels
                .threshold_metadata()
                .unwrap()
                .lineage()
                .endpoint_charts
                .len(),
            2
        );
        assert!(
            kernels
                .threshold_metadata()
                .unwrap()
                .require_global_proof()
                .is_err()
        );
        let mut total = vec![0.; kernels.orders().len()];
        for sector in kernels.sectors_mut() {
            let mut out = vec![0.; total.len()];
            sector.evaluate(&[0.5, 0.37], &mut out).unwrap();
            for (a, b) in total.iter_mut().zip(out) {
                *a += b
            }
        }
        assert_eq!(total[0], 0.);
        assert_eq!(total[1], 0.);
        assert!((total[3] - 2.501397079832065).abs() < 1e-12);
        let restored = KernelSet::from_bytes(&kernels.to_bytes().unwrap()).unwrap();
        assert_eq!(restored.content_id(), kernels.content_id());
        results.push(serde_json::json!({"backend":format!("{backend:?}"),"bytes":bytes.len(),"id":kernels.content_id(),"vector":total,"proof_released_before_jobs":true}));
    }
    serde_json::json!({"factory":results})
}

#[test]
fn detached_records_native_restore_and_fresh_process() {
    let directory = tempfile::tempdir().unwrap();
    factory(directory.path());
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "kernel::algebraic_tests::artifacts::restore_child",
            "--nocapture",
        ])
        .env("FASTSECDEC_ALGEBRAIC_CHILD", directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    restore(directory.path());
    write_artifacts(&directory.path().join("plain"));
}
#[test]
#[ignore = "called by detached_records_native_restore_and_fresh_process"]
fn restore_child() {
    let path = std::env::var_os("FASTSECDEC_ALGEBRAIC_CHILD").expect("parent path");
    restore(std::path::Path::new(&path));
}
use crate::kernel::{
    KernelLoadOptions, KernelSet, ProgramRecipe, StabilityMode, indexed::ProgramArchiveReader,
};
use std::io::Cursor;
fn evaluate(set: &mut KernelSet, full: bool) {
    assert!(set.requires_validated_algebraic_callbacks());
    assert_eq!(set.stability_settings().mode, StabilityMode::Validated);
    assert!(set.contour_validation_report().is_none());
    let metadata = set.threshold_metadata().unwrap();
    assert_eq!(metadata.full_original_scope().unwrap(), full);
    assert!(metadata.require_global_proof().is_err());
    let mut sum = vec![0.; set.orders().len()];
    for kernel in set.sectors_mut() {
        let mut value = vec![0.; sum.len()];
        kernel.evaluate(&[0.5, 0.37], &mut value).unwrap();
        assert!(value.iter().all(|v| v.is_finite()));
        for (a, b) in sum.iter_mut().zip(value) {
            *a += b;
        }
    }
    if full {
        let expected = [
            0.,
            0.,
            -0.6026827034130204,
            2.501397079832065,
            -5.52651717135341,
            0.4992567263475912,
        ];
        for (a, b) in sum.iter().zip(expected) {
            assert!((a - b).abs() < 3e-12 * (1. + b.abs()));
        }
    }
}
pub(super) fn restore(path: &std::path::Path) {
    let path = path.display().to_string();
    for backend in ["Eager", "Symjit"] {
        let bytes = std::fs::read(format!("{path}/family-{backend}.bin")).unwrap();
        for validate in [false, true] {
            let mut all =
                KernelSet::from_bytes_with_options(&bytes, KernelLoadOptions { validate }).unwrap();
            evaluate(&mut all, true);
            let resaved = all.to_bytes().unwrap();
            let mut saved = KernelSet::from_bytes(&resaved).unwrap();
            assert_eq!(all.content_id(), saved.content_id());
            evaluate(&mut saved, true);
        }
        let mut archive = ProgramArchiveReader::from_reader(
            Cursor::new(bytes),
            KernelLoadOptions { validate: true },
        )
        .unwrap();
        let original_catalogue = archive.catalogue().clone();
        {
            let mut selected = archive.select(ProgramRecipe::ThresholdV1).unwrap();
            for i in 0..2 {
                let mut kernel = selected.load_sector(i).unwrap();
                assert_eq!(kernel.sectors().len(), 1);
                evaluate(&mut kernel, false);
                let bytes = kernel.to_bytes().unwrap();
                let mut restored = KernelSet::from_bytes(&bytes).unwrap();
                evaluate(&mut restored, false);
                assert_eq!(kernel.content_id(), restored.content_id());
            }
        }
        let (cache, catalogue) = archive
            .write_with_native_cache(Cursor::new(Vec::new()), |_| {
                std::ops::ControlFlow::Continue(())
            })
            .unwrap();
        assert_eq!(
            catalogue.source_identity,
            original_catalogue.source_identity
        );
        let mut restored = KernelSet::from_bytes(&cache.into_inner()).unwrap();
        evaluate(&mut restored, true);
        println!(
            "PASS {backend}: fresh native process, full-vector and selected scopes, resave, cache refresh, no contour report"
        );
    }
}
