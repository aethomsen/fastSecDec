use super::*;
use crate::{kernel::threshold_metadata::ResidentSelection, results::ExactContributionPolicy};
use std::io::Cursor;

fn archive(family: NormalizedFamily) -> Vec<u8> {
    let continued = family.continue_symbolically(&options()).unwrap();
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
    let mut writer = plan.archive_writer(Cursor::new(Vec::new())).unwrap();
    for i in (0..plan.job_count()).rev() {
        let mut bytes = Vec::new();
        let receipt = plan
            .job(i)
            .unwrap()
            .run()
            .unwrap()
            .write_record(&mut bytes)
            .unwrap();
        writer
            .append_record(ProgramRecipe::ThresholdV1, &mut bytes.as_slice(), receipt)
            .unwrap();
    }
    writer.finish().unwrap().0.into_inner()
}

fn exercise(bytes: &[u8]) {
    for validate in [false, true] {
        let mut reader = indexed::ProgramArchiveReader::from_reader(
            Cursor::new(bytes),
            KernelLoadOptions { validate },
        )
        .unwrap();
        let mut selected = reader.select(ProgramRecipe::ThresholdV1).unwrap();
        let catalogue = selected.catalogue().clone();
        let mut exact = selected.load_exact().unwrap();
        let mut full = selected.load_all().unwrap();
        drop(reader); // Association has no reader and cannot load stochastic records.
        assert!(exact.sectors().is_empty());
        let p = exact.runtime_parameters()[0];
        let mut first = None;
        for value in [16., 17., 16.] {
            let values = BTreeMap::from([(p, value)]);
            exact.bind_parameters(&values).unwrap();
            assert!(exact.exact_coefficients().iter().all(|v| *v == 0.));
            full.bind_parameters(&values).unwrap();
            let before = (
                exact.content_id().to_owned(),
                exact.template_content_id().to_owned(),
                exact.exact_coefficients().to_vec(),
                exact.threshold_metadata().unwrap().resident().clone(),
            );
            let id = catalogue
                .bound_threshold_content_id(&exact, &values)
                .unwrap();
            assert_eq!(id, full.content_id());
            assert_ne!(id, exact.content_id());
            assert_ne!(id, catalogue.content_id);
            let vector = point(&mut full);
            if value == 16. {
                if let Some((old_id, old_vector)) = &first {
                    assert_eq!(&id, old_id);
                    assert_eq!(&vector, old_vector);
                } else {
                    first = Some((id, vector));
                }
            } else {
                assert_ne!(&id, &first.as_ref().unwrap().0);
            }
            assert_eq!(
                before,
                (
                    exact.content_id().to_owned(),
                    exact.template_content_id().to_owned(),
                    exact.exact_coefficients().to_vec(),
                    exact.threshold_metadata().unwrap().resident().clone()
                )
            );
        }
    }
}

#[test]
fn complete_point_identity_matches_full_family_and_fresh_restore() {
    let bytes = archive(family().0);
    exercise(&bytes);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("indexed.bin"), &bytes).unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "kernel::projective_parameter_tests::catalogue_point::restored_child",
            "--nocapture",
        ])
        .env("FASTSECDEC_CATALOGUE_POINT", dir.path().join("indexed.bin"))
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
}

#[test]
#[ignore = "fresh process invoked by complete_point_identity_matches_full_family_and_fresh_restore"]
fn restored_child() {
    for i in (0..80).rev() {
        let _ = symbol!(format!("catalogue_point_noise::s{i}"));
    }
    exercise(&std::fs::read(std::env::var_os("FASTSECDEC_CATALOGUE_POINT").unwrap()).unwrap());
}

#[test]
fn wrong_point_scope_layout_and_parent_refuse_without_mutation() {
    let bytes = archive(family().0);
    let mut reader = indexed::ProgramArchiveReader::from_reader(
        Cursor::new(bytes),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    let mut selected = reader.select(ProgramRecipe::ThresholdV1).unwrap();
    let catalogue = selected.catalogue().clone();
    let mut exact = selected.load_exact().unwrap();
    let p = exact.runtime_parameters()[0];
    let valid = BTreeMap::from([(p, 16.)]);
    assert!(
        catalogue
            .bound_threshold_content_id(&exact, &valid)
            .unwrap_err()
            .to_string()
            .contains("not bound")
    );
    exact.bind_parameters(&valid).unwrap();
    let baseline = (
        exact.content_id().to_owned(),
        exact.template_content_id().to_owned(),
        exact.exact_coefficients().to_vec(),
        exact.to_bytes().unwrap(),
    );
    for values in [
        BTreeMap::new(),
        BTreeMap::from([(p, 17.)]),
        BTreeMap::from([(p, 15.)]),
        BTreeMap::from([(p, 18.)]),
        BTreeMap::from([(p, f64::NAN)]),
        BTreeMap::from([(p, f64::INFINITY)]),
        BTreeMap::from([(symbol!("catalogue_point_wrong::p"), 16.)]),
    ] {
        assert!(
            catalogue
                .bound_threshold_content_id(&exact, &values)
                .is_err()
        );
        assert_eq!(
            baseline,
            (
                exact.content_id().to_owned(),
                exact.template_content_id().to_owned(),
                exact.exact_coefficients().to_vec(),
                exact.to_bytes().unwrap()
            )
        );
    }
    for mutation in 0..11 {
        let mut wrong = catalogue.clone();
        match mutation {
            0 => wrong.content_id = "0".repeat(64),
            1 => wrong.orders[0] += 1,
            2 => wrong.components.swap(0, 1),
            3 => wrong.runtime_parameters[0] = "wrong::s".into(),
            4 => wrong.threshold.as_mut().unwrap().parent.0 = "0".repeat(64),
            5 => wrong.threshold.as_mut().unwrap().source_identity = "0".repeat(64),
            6 => {
                wrong
                    .threshold
                    .as_mut()
                    .unwrap()
                    .runtime_guard
                    .as_mut()
                    .unwrap()
                    .semantic
                    .0 = "0".repeat(64)
            }
            7 => wrong
                .threshold
                .as_mut()
                .unwrap()
                .contributions
                .pop()
                .map(|_| ())
                .unwrap(),
            8 => wrong.threshold.as_mut().unwrap().complete_content_id = "0".repeat(64),
            9 => wrong.threshold = None,
            _ => wrong.recipe = ProgramRecipe::UndeformedV1,
        }
        assert!(
            wrong.bound_threshold_content_id(&exact, &valid).is_err(),
            "mutation {mutation}"
        );
    }
    let mut full = selected.load_all().unwrap();
    full.bind_parameters(&valid).unwrap();
    assert!(catalogue.bound_threshold_content_id(&full, &valid).is_err());
    let mut sector = selected.load_sector(0).unwrap();
    sector.bind_parameters(&valid).unwrap();
    assert!(
        catalogue
            .bound_threshold_content_id(&sector, &valid)
            .is_err()
    );
    let excluded = exact
        .threshold_metadata()
        .unwrap()
        .selected(ResidentSelection::Selected {
            stochastic_contributions: Vec::new(),
            exact_policy: ExactContributionPolicy::ExcludeAll,
        })
        .unwrap();
    let mut wrong = exact.try_clone().unwrap();
    wrong.threshold = Some(Arc::new(excluded));
    assert!(
        catalogue
            .bound_threshold_content_id(&wrong, &valid)
            .is_err()
    );
    let mut wrong = exact.try_clone().unwrap();
    wrong.content_id = "0".repeat(64);
    assert!(
        catalogue
            .bound_threshold_content_id(&wrong, &valid)
            .is_err()
    );
    let mut wrong = exact.try_clone().unwrap();
    wrong.template_content_id = Some("0".repeat(64));
    assert!(
        catalogue
            .bound_threshold_content_id(&wrong, &valid)
            .is_err()
    );
    assert_eq!(
        baseline,
        (
            exact.content_id().to_owned(),
            exact.template_content_id().to_owned(),
            exact.exact_coefficients().to_vec(),
            exact.to_bytes().unwrap()
        )
    );
}

#[test]
fn fixed_parameterless_catalogue_keeps_its_existing_identity() {
    let (family, _) = crate::kernel::parameter_template_tests::family_kind(false, Some(2.375));
    let bytes = archive(family);
    let mut reader = indexed::ProgramArchiveReader::from_reader(
        Cursor::new(bytes),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    let mut selected = reader.select(ProgramRecipe::ThresholdV1).unwrap();
    let catalogue = selected.catalogue().clone();
    let exact = selected.load_exact().unwrap();
    assert!(exact.runtime_parameters().is_empty());
    assert!(exact.parameters_bound());
    assert_eq!(
        catalogue
            .bound_threshold_content_id(&exact, &BTreeMap::new())
            .unwrap(),
        catalogue.content_id
    );
    assert!(
        catalogue
            .bound_threshold_content_id(
                &exact,
                &BTreeMap::from([(symbol!("foreign_fixed::p"), 1.)])
            )
            .is_err()
    );
}
