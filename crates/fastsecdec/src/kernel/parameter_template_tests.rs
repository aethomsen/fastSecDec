//! Actual compiled native family: geometry is discarded before caller-run jobs.
use super::*;
use crate::{
    generation::{GenerationMode, GenerationOptions},
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{GcadKinematics, GcadRequest, SolverOptions},
        regularization::secant::{Limits, NormalizedFamily},
    },
};
use std::{collections::BTreeMap, sync::Arc};
use symbolica::{atom::Atom, domains::rational::Rational, symbol};

fn family() -> (NormalizedFamily, Symbol) {
    family_kind(false, None)
}
pub(crate) fn family_kind(numerator: bool, fixed: Option<f64>) -> (NormalizedFamily, Symbol) {
    let (p, y, eps, u) = symbol!(
        "compiled_uniform::p",
        "compiled_uniform::y",
        "compiled_uniform::eps",
        "compiled_uniform::u"
    );
    let mut factors = vec![
        PolynomialFactor::new(
            Atom::var(y).pow(5) + Atom::var(y) - Atom::var(p) + Atom::num(2),
            -Atom::one() - Atom::var(eps),
            FactorRole::Singularity,
        )
        .with_semantics(FactorSemantics::Causal),
    ];
    if numerator {
        factors.push(PolynomialFactor::new(
            Atom::one() + Atom::var(p) * Atom::var(y),
            Atom::one(),
            FactorRole::Polynomial,
        ));
    }
    let input = ParametricIntegrand::new(
        vec![y],
        eps,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            if numerator {
                Atom::one() + Atom::var(p)
            } else {
                Atom::one()
            },
            vec![Atom::zero()],
            factors,
        )],
    )
    .unwrap();
    let kinematics = if let Some(value) = fixed {
        GcadKinematics {
            exact_values: BTreeMap::from([(p, Rational::try_from(value).unwrap())]),
            ..Default::default()
        }
    } else {
        GcadKinematics {
            runtime_parameters: vec![p],
            strict_positive: vec![
                Atom::var(p) - Atom::num(Rational::from((9, 4))),
                Atom::num(Rational::from((5, 2))) - Atom::var(p),
            ],
            ..Default::default()
        }
    };
    let request = GcadRequest::unit_cube(
        &input,
        kinematics,
        SolverOptions {
            order: if fixed.is_some() {
                vec!["v0".into()]
            } else {
                vec!["v0".into(), "v1".into()]
            },
            ..Default::default()
        },
        GcadRequest::default_limits(),
    )
    .unwrap();
    let owner = Arc::new(request.solve_verified().unwrap());
    let limits = Limits {
        unit_margin: Rational::from((1, 8)),
        ..Default::default()
    };
    let family = if fixed.is_some() {
        NormalizedFamily::admit_affine_shifted(owner, vec![u], limits)
    } else {
        NormalizedFamily::admit_parameter_affine_shifted(owner, vec![u], limits)
    }
    .unwrap();
    (family, p)
}
fn point(set: &mut KernelSet) -> Vec<f64> {
    let mut total = set.exact_coefficients().to_vec();
    for sector in set.sectors_mut() {
        let mut out = vec![0.; total.len()];
        let report = sector.replay_scaled(&[0.37], &mut out, 1., 128).unwrap();
        assert!(report.checked && report.rescued && report.bits >= 256);
        for (a, b) in total.iter_mut().zip(out) {
            *a += b;
        }
    }
    assert!(total.iter().all(|v| v.is_finite()));
    total
}
fn programs(set: &KernelSet) -> Vec<Vec<u8>> {
    set.sectors()
        .iter()
        .map(|s| program::encode(s.exact_program()).unwrap())
        .collect()
}
fn exercise(set: &mut KernelSet, p: Symbol) {
    assert_eq!(set.runtime_parameters(), [p]);
    assert!(!set.parameters_bound());
    let template = set.template_content_id().to_owned();
    let ir = programs(set);
    let mut first = None;
    let mut previous = None;
    for value in [2.375, 2.4, 2.3125, 2.375] {
        set.bind_parameters(&BTreeMap::from([(p, value)])).unwrap();
        assert!(set.parameters_bound());
        assert_eq!(set.template_content_id(), template);
        assert_eq!(programs(set), ir);
        let vector = point(set);
        if let Some(old) = previous.replace(vector.clone()) {
            assert_ne!(vector, old);
        }
        let algebra = symgcad::algebra::Algebra::new(&["y".into()]).unwrap();
        let polynomial = algebra
            .parse(&format!(
                "y^5+y-{}",
                Rational::try_from(value - 2.).unwrap()
            ))
            .unwrap();
        let fiber = algebra.specialize_univariate(&polynomial, 0, &[]).unwrap();
        let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
        assert_eq!(roots.len(), 1);
        for _ in 0..128 {
            symgcad::roots::refine_once(&mut roots[0]);
        }
        let root = (&roots[0].interval.0 + &roots[0].interval.1) / Rational::from(2);
        let oracle =
            std::f64::consts::PI / (Rational::one() + Rational::from(5) * root.pow(4)).to_f64();
        assert!((vector[3] - oracle).abs() < 1e-11, "{vector:?} vs {oracle}");
        if value == 2.375 {
            if let Some((id, old)) = &first {
                assert_eq!(set.content_id(), id);
                assert_eq!(&vector, old);
            } else {
                first = Some((set.content_id().to_owned(), vector.clone()));
            }
        }
        let id = set.content_id().to_owned();
        for invalid in [
            BTreeMap::new(),
            BTreeMap::from([(p, 2.25)]),
            BTreeMap::from([(p, 2.5)]),
            BTreeMap::from([(p, 0.)]),
            BTreeMap::from([(p, f64::NAN)]),
        ] {
            assert!(set.bind_parameters(&invalid).is_err());
            assert_eq!(set.content_id(), id);
            assert_eq!(set.template_content_id(), template);
            assert_eq!(point(set), vector);
            assert_eq!(programs(set), ir);
        }
    }
}
#[test]
fn complete_template_compiles_once_and_binds_atomically() {
    let (family, p) = family();
    let continued = family
        .continue_symbolically(&GenerationOptions {
            mode: GenerationMode::Symbolic,
            max_order: 1,
            ..Default::default()
        })
        .unwrap();
    let staging = tempfile::tempdir().unwrap();
    let mut set = KernelSet::compile_threshold_family(
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
    assert!(
        set.threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
    assert_eq!(
        set.threshold_metadata()
            .unwrap()
            .lineage()
            .preparation
            .runtime_chamber
            .as_ref()
            .unwrap()
            .integration_axes,
        [1]
    );
    drop(continued);
    drop(family);
    exercise(&mut set, p);
}
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
    if let Some(path) = std::env::var_os("FASTSECDEC_PARAMETER_EXPORT") {
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
        .bind_parameters(&BTreeMap::from([(p, 2.375)]))
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
            .bind_parameters(&BTreeMap::from([(p, 2.375)]))
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
    assert!(exact.bind_parameters(&BTreeMap::from([(p, 2.25)])).is_err());
    assert!(!exact.parameters_bound());
    exact
        .bind_parameters(&BTreeMap::from([(p, 2.375)]))
        .unwrap();
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
            "kernel::parameter_template_tests::restore_child",
            "--nocapture",
        ])
        .env("FASTSECDEC_PARAMETER_CHILD", directory.path())
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
        .bind_parameters(&BTreeMap::from([(p, 2.375)]))
        .unwrap();
    assert_eq!(point(&mut refreshed), point(&mut set));
    if let Some(path) = std::env::var_os("FASTSECDEC_PARAMETER_EXPORT") {
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
    let path = std::path::PathBuf::from(std::env::var_os("FASTSECDEC_PARAMETER_CHILD").unwrap());
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
        assert_eq!(p.get_name(), "compiled_uniform::p");
        assert!(set.bind_parameters(&BTreeMap::from([(p, 2.5)])).is_err());
        assert!(!set.parameters_bound());
        set.bind_parameters(&BTreeMap::from([(p, 2.375)])).unwrap();
        assert_eq!(set.content_id(), row["point"].as_str().unwrap());
        let expected: Vec<f64> = serde_json::from_value(row["value"].clone()).unwrap();
        assert_eq!(point(&mut set), expected);
    }
}

mod science;

mod transport;
