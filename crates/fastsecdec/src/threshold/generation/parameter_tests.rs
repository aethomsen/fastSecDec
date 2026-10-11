use super::*;
use crate::threshold::regularization::secant;

fn input(projective: bool) -> (Arc<ParametricIntegrand>, PreparationOptions, Symbol) {
    let (p, x, y, e, t) = symbol!(
        "auto_uniform::p",
        "auto_uniform::x",
        "auto_uniform::y",
        "auto_uniform::eps",
        "auto_uniform::t"
    );
    let input = if projective {
        let model = Arc::new(
            crate::Model::from_json(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/models/scalar.json"
            )))
            .unwrap(),
        );
        let kin = crate::Kinematics::in_dimension(&symbolica::parse!("D"))
            .unwrap()
            .with_mass_squared(
                &feynkit_graph::symbols::external_momentum().call(1),
                Atom::var(p),
            )
            .unwrap();
        let gamma = symbolica::transcendental::gamma();
        let multiplier = gamma.call(Atom::one() - Atom::num(2) * Atom::var(e))
            / (gamma.call(Atom::one() + Atom::var(e))
                * gamma.call(Atom::one() - Atom::var(e)).pow(2));
        let graph = crate::input::GraphIntegral::from_dot(
            model,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/graphs/bubble.dot"
            )),
            &kin,
        )
        .unwrap()
        .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::num(3).sqrt())]))
        .unwrap()
        .with_measure_multiplier(multiplier);
        ParametricIntegrand::from_graph(
            &graph,
            vec![x, y],
            e,
            Atom::num(4) - Atom::num(2) * Atom::var(e),
        )
        .unwrap()
    } else {
        ParametricIntegrand::new(
            vec![y],
            e,
            ParametricDomain::UnitCube,
            vec![ParametricTerm::new(
                Atom::one(),
                vec![Atom::zero()],
                vec![
                    PolynomialFactor::new(
                        Atom::var(y).pow(5) + Atom::var(y) - Atom::var(p) + Atom::num(2),
                        -Atom::one() - Atom::var(e),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Causal),
                ],
            )],
        )
        .unwrap()
    };
    let mut options = PreparationOptions::new(t);
    options.generation.max_order = 1;
    options.regular_family = Some(RegularFamilyPreparation::default());
    options.threshold.solver.order = vec!["v0".into(), "v1".into()];
    let (a, b) = if projective {
        (Rational::from(15), Rational::from(18))
    } else {
        (Rational::from((9, 4)), Rational::from((5, 2)))
    };
    options.threshold.kinematics = gcad::GcadKinematics {
        runtime_parameters: vec![p],
        strict_positive: vec![Atom::var(p) - Atom::num(a), Atom::num(b) - Atom::var(p)],
        ..Default::default()
    };
    (Arc::new(input), options, p)
}
fn point(kernels: &mut KernelSet) -> Vec<f64> {
    let mut out = kernels.exact_coefficients().to_vec();
    for sector in kernels.sectors_mut() {
        let mut row = vec![0.; out.len()];
        assert!(
            sector
                .evaluate_with_diagnostics(&[0.37], &mut row)
                .unwrap()
                .checked
        );
        for (x, y) in out.iter_mut().zip(row) {
            *x += y;
        }
    }
    out
}

#[test]
fn automatic_uniform_native_graph_and_translated_cube_compile_once_and_recover() {
    for projective in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (input, options, p) = input(projective);
        let mut progress = Vec::new();
        let receipt = prepare(input, options, root.path(), |s| {
            if let Some(d) = s.regular_family {
                progress.push(d);
            }
            ControlFlow::Continue(())
        })
        .unwrap()
        .receipt;
        assert_eq!(receipt.original_dimensions, if projective { 2 } else { 1 });
        assert_eq!(receipt.integration_dimensions, 1);
        assert_eq!(receipt.cells, if projective { 3 } else { 2 });
        assert_eq!(receipt.endpoint_charts, if projective { 4 } else { 2 });
        assert_eq!(
            progress.last().unwrap().admitted_charts,
            Some(receipt.endpoint_charts)
        );
        let (stored, route) =
            configuration::read(root.path(), &receipt.configuration, u64::MAX).unwrap();
        assert_eq!(route, regular::Route::Uniform);
        assert_eq!(
            stored.regular_family.unwrap().coordinates,
            Some(vec![stored.unit])
        );
        let mut stages = Vec::new();
        let replay = resume(
            root.path(),
            &receipt,
            "replay",
            &receipt.publication.source_identity,
            &receipt.publication.prepared_identity,
            32 * 1024 * 1024,
            |p| {
                stages.push(p.stage);
                ControlFlow::Continue(())
            },
        )
        .unwrap()
        .receipt;
        assert!(!stages.contains(&Stage::Solve));
        assert!(stages.contains(&Stage::Verify));
        assert_eq!(
            receipt.publication.prepared_identity,
            replay.publication.prepared_identity
        );
        let mut kernels = compile(root.path(), &receipt);
        let mut second = compile(root.path(), &replay);
        assert_eq!(kernels.content_id(), second.content_id());
        assert!(!kernels.parameters_bound());
        let template = kernels.template_content_id().to_owned();
        let values = if projective {
            [16., 17., 16.]
        } else {
            [2.375, 2.3125, 2.375]
        };
        let mut first = None;
        for value in values {
            let values = BTreeMap::from([(p, value)]);
            kernels.bind_parameters(&values).unwrap();
            second.bind_parameters(&values).unwrap();
            let row = point(&mut kernels);
            assert_eq!(row, point(&mut second));
            assert_eq!(kernels.template_content_id(), template);
            assert_eq!(row.len(), 6);
            assert!(row.iter().all(|x| x.is_finite()));
            if projective {
                assert!((row[0] - 1.).abs() < 1e-11 && row[1].abs() < 1e-11);
                assert!((row[3] - std::f64::consts::PI * (1. - 12. / value).sqrt()).abs() < 1e-11);
            }
            if value == if projective { 16. } else { 2.375 } {
                if let Some((id, prior)) = &first {
                    assert_eq!(kernels.content_id(), id);
                    assert_eq!(&row, prior);
                } else {
                    first = Some((kernels.content_id().to_owned(), row.clone()));
                }
            }
            let id = kernels.content_id().to_owned();
            assert!(kernels.bind_parameters(&BTreeMap::from([(p, 0.)])).is_err());
            assert_eq!(kernels.content_id(), id);
            assert_eq!(point(&mut kernels), row);
        }
    }
}

#[test]
fn uniform_raw_cancel_and_fresh_process_recovery_keep_native_roles() {
    let root = tempfile::tempdir().unwrap();
    let (input, options, _) = input(true);
    let mut checkpoint = None;
    let error = prepare(input, options, root.path(), |p| {
        checkpoint = p.checkpoint().or(checkpoint.take());
        if p.regular_family.is_some_and(|d| d.endpoint_queries >= 3) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .err()
    .unwrap();
    assert!(matches!(error, Error::Cancelled));
    let checkpoint = checkpoint.unwrap();
    std::fs::write(
        root.path().join("checkpoint.json"),
        serde_json::to_vec(&checkpoint).unwrap(),
    )
    .unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "threshold::generation::tests::parameter_tests::uniform_recovery_child",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("FSD_UNIFORM_PREPARATION_CHILD", root.path())
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
}

#[test]
#[ignore = "invoked by the uniform raw-recovery parent in a fresh native process"]
fn uniform_recovery_child() {
    let Some(root) = std::env::var_os("FSD_UNIFORM_PREPARATION_CHILD") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    for n in 0..31 {
        let _ = symbol!(format!("uniform_recovery_noise::s{n}"));
    }
    let checkpoint: PreparationCheckpoint =
        serde_json::from_slice(&std::fs::read(root.join("checkpoint.json")).unwrap()).unwrap();
    let mut stages = Vec::new();
    let recovered = resume_evidence(
        &root,
        &checkpoint,
        "cold",
        &checkpoint.evidence.request.source_identity,
        32 * 1024 * 1024,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&Stage::Solve));
    assert!(stages.contains(&Stage::Verify));
    let mut complete_stages = Vec::new();
    let complete = resume(
        &root,
        &recovered.receipt,
        "cold-complete",
        &recovered.receipt.publication.source_identity,
        &recovered.receipt.publication.prepared_identity,
        32 * 1024 * 1024,
        |p| {
            complete_stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!complete_stages.contains(&Stage::Solve));
    assert_eq!(
        complete.receipt.publication.prepared_identity,
        recovered.receipt.publication.prepared_identity
    );
    let mut kernels = compile(&root, &recovered.receipt);
    kernels
        .bind_parameters(&BTreeMap::from([(symbol!("auto_uniform::p"), 16.)]))
        .unwrap();
    let row = point(&mut kernels);
    assert!((row[0] - 1.).abs() < 1e-11);
    assert!((row[3] - std::f64::consts::FRAC_PI_2).abs() < 1e-11);
}

#[test]
fn uniform_capability_missing_and_immutable_density_fail_without_retry() {
    let root = tempfile::tempdir().unwrap();
    let (source, mut options, _) = input(false);
    options.regular_family = None;
    let mut solved = false;
    assert!(matches!(
        prepare(source, options, root.path(), |p| {
            solved |= p.stage == Stage::Solve;
            ControlFlow::Continue(())
        }),
        Err(Error::Unsupported(_))
    ));
    assert!(!solved);
    let root = tempfile::tempdir().unwrap();
    let (input, mut options, p) = input(false);
    // This control isolates an immutable density failure AFTER an already
    // admissible geometric proposal; the default larger margin can miss first.
    options.regular_family.as_mut().unwrap().limits.unit_margin = Rational::from((1, 8));
    let term = &input.terms()[0];
    let bad = ParametricIntegrand::new(
        input.parameters().to_vec(),
        input.regulator(),
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::var(p).pow(-1),
            term.monomial_powers().to_vec(),
            term.factors().to_vec(),
        )],
    )
    .unwrap();
    let mut attempts = 0;
    let error = prepare(Arc::new(bad), options, root.path(), |p| {
        if let Some(d) = p.regular_family {
            attempts = d.family_attempts;
        }
        ControlFlow::Continue(())
    })
    .err()
    .unwrap();
    assert!(matches!(
        error,
        Error::RegularFamily(secant::Error::Unsupported(_))
    ));
    assert_eq!(attempts, 1);
}

#[path = "parameter_tests/science.rs"]
mod science;

#[test]
fn uniform_private_proposals_preserve_miss_failure_and_public_error_boundary() {
    let (input, options, _) = input(true);
    let owner = Arc::new(
        options
            .threshold
            .gcad_first_request(&input, &options.generation)
            .unwrap()
            .solve_verified()
            .unwrap(),
    );
    let proposals =
        [(1, 3), (5, 7)].map(|(a, b)| crate::threshold::maps::regular::BracketProposal {
            lower: Rational::from((a, 8)),
            upper: Rational::from((b, 8)),
            derivative_margin: Rational::from(10000),
        });
    let mut limits = secant::Limits {
        unit_margin: Rational::from((1, 8)),
        ..Default::default()
    };
    assert!(matches!(
        secant::attempt_parameter_affine_shifted(
            owner.clone(),
            vec![options.unit],
            Some(proposals.clone()),
            limits.clone()
        ),
        Err(crate::threshold::maps::regular::ProposalError::Miss(_))
    ));
    assert!(matches!(
        secant::NormalizedFamily::admit_parameter_pair_affine_shifted(
            owner.clone(),
            vec![options.unit],
            proposals.clone(),
            limits.clone()
        ),
        Err(secant::Error::Unsupported(_))
    ));
    limits.cancellation = Some(Arc::new(std::sync::atomic::AtomicBool::new(true)));
    assert!(matches!(
        secant::attempt_parameter_affine_shifted(
            owner,
            vec![options.unit],
            Some(proposals),
            limits
        ),
        Err(crate::threshold::maps::regular::ProposalError::Failure(
            secant::Error::Cancelled
        ))
    ));
}

#[test]
fn automatic_uniform_preserves_permuted_parameter_prefix_and_unused_guard_role() {
    let root = tempfile::tempdir().unwrap();
    let (input, mut options, p) = input(false);
    let q = symbol!("auto_uniform::q");
    options.threshold.kinematics.runtime_parameters.push(q);
    options
        .threshold
        .kinematics
        .strict_positive
        .extend([Atom::var(q) - Atom::num(3), Atom::num(4) - Atom::var(q)]);
    options.threshold.solver.order = vec!["v1".into(), "v0".into(), "v2".into()];
    let receipt = prepare(input, options, root.path(), |_| ControlFlow::Continue(()))
        .unwrap()
        .receipt;
    let mut kernels = compile(root.path(), &receipt);
    assert_eq!(kernels.runtime_parameters(), [p, q]);
    let chamber = kernels
        .threshold_metadata()
        .unwrap()
        .lineage()
        .preparation
        .runtime_chamber
        .as_ref()
        .unwrap();
    assert_eq!(chamber.runtime_parameters.len(), 2);
    assert_eq!(chamber.prefix_to_public, [1, 0]);
    assert_eq!(chamber.integration_axes, [2]);
    kernels
        .bind_parameters(&BTreeMap::from([(p, 2.375), (q, 3.5)]))
        .unwrap();
    let prior = point(&mut kernels);
    let id = kernels.content_id().to_owned();
    assert!(
        kernels
            .bind_parameters(&BTreeMap::from([(p, 2.375), (q, 2.5)]))
            .is_err()
    );
    assert_eq!(kernels.content_id(), id);
    assert_eq!(point(&mut kernels), prior);
}
