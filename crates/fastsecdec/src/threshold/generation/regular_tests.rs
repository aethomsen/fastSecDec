use super::*;
use crate::threshold::regularization::secant;

fn regular_input(paired: bool) -> (Arc<ParametricIntegrand>, PreparationOptions) {
    let (x, y, eps, t) = symbol!(
        "auto_regular::x",
        "auto_regular::y",
        "auto_regular::eps",
        "auto_regular::t"
    );
    let polynomial = if paired {
        Atom::var(y).pow(6) - Atom::var(y) + (Atom::one() + Atom::var(x)) / Atom::num(16)
    } else {
        Atom::var(y).pow(5) + Atom::var(y) - Atom::var(x)
    };
    let input = ParametricIntegrand::new(
        vec![x, y],
        eps,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(
                    polynomial,
                    -Atom::one() - Atom::var(eps),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let mut options = PreparationOptions::new(t);
    options.regular_family = Some(RegularFamilyPreparation::default());
    options.threshold.solver.order = vec!["v0".into(), "v1".into()];
    options.threshold.gcad_limits.wall_time_secs = 30.;
    options.threshold.gcad_limits.memory_mib = 1024;
    options.generation.max_order = 0;
    (Arc::new(input), options)
}

#[test]
fn automatic_regular_families_prepare_replay_compile() {
    for paired in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (input, options) = regular_input(paired);
        let mut snapshots = Vec::new();
        let receipt = prepare(input, options, directory.path(), |p| {
            if let Some(d) = p.regular_family {
                snapshots.push(d);
            }
            ControlFlow::Continue(())
        })
        .unwrap()
        .receipt;
        assert_eq!(receipt.integration_dimensions, 2);
        assert_eq!(receipt.cells, if paired { 3 } else { 2 });
        assert_eq!(receipt.endpoint_charts, if paired { 4 } else { 2 });
        let resolved =
            configuration::read(directory.path(), &receipt.configuration, 16 * 1024 * 1024)
                .unwrap()
                .0;
        let coordinates = resolved.regular_family.unwrap().coordinates.unwrap();
        assert_eq!(coordinates.len(), 2);
        assert_eq!(coordinates[1], resolved.unit);
        assert_eq!(
            snapshots.last().unwrap().admitted_charts,
            Some(receipt.endpoint_charts)
        );
        if paired {
            assert_eq!(snapshots.last().unwrap().endpoint_queries, 127);
        }
        let mut first = compile(directory.path(), &receipt);
        assert_eq!(
            first.algebraic_callback_recipe(),
            Some(crate::kernel::AlgebraicCallbackRecipe::AffineCoefficientShiftV3)
        );
        let mut stages = Vec::new();
        let replay = resume(
            directory.path(),
            &receipt,
            "replayed",
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
        let mut second = compile(directory.path(), &replay);
        assert_eq!(first.content_id(), second.content_id());
        assert_eq!(first.sectors().len(), receipt.endpoint_charts);
        for point in [
            [0.23, 0.37],
            [0.71, 0.61],
            // The original automatic narrow-bracket V2 failure.
            [0.9999325137757114, 0.9990361877224441],
        ] {
            for (a, b) in first.sectors_mut().iter_mut().zip(second.sectors_mut()) {
                let mut av = vec![0.; a.output_count()];
                let mut bv = vec![0.; b.output_count()];
                a.evaluate(&point, &mut av).unwrap();
                b.evaluate(&point, &mut bv).unwrap();
                assert_eq!(av, bv);
            }
        }
    }
}

#[test]
fn automatic_regular_bounded_miss_and_cancel_keep_raw_checkpoint() {
    let directory = tempfile::tempdir().unwrap();
    let (input, mut options) = regular_input(true);
    options
        .regular_family
        .as_mut()
        .unwrap()
        .discovery
        .max_dyadic_level = 4;
    let mut checkpoint = None;
    let error = prepare(input, options, directory.path(), |p| {
        checkpoint = p.checkpoint().or(checkpoint.take());
        ControlFlow::Continue(())
    })
    .err()
    .unwrap();
    assert!(matches!(
        error,
        Error::DiscoveryIncomplete {
            progress: DiscoveryProgress {
                endpoint_queries: 15,
                ..
            },
            ..
        }
    ));
    assert!(checkpoint.is_some());
    let directory = tempfile::tempdir().unwrap();
    let (input, options) = regular_input(true);
    let mut checkpoint = None;
    let error = prepare(input, options, directory.path(), |p| {
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
    assert!(checkpoint.is_some());
}

#[test]
fn automatic_regular_fences_roles_parameters_and_order() {
    let directory = tempfile::tempdir().unwrap();
    let (input, mut options) = regular_input(false);
    options.regular_family.as_mut().unwrap().coordinates =
        Some(vec![input.parameters()[0], options.unit]);
    assert!(matches!(
        prepare(input, options, directory.path(), |_| ControlFlow::Continue(
            ()
        )),
        Err(Error::RegularFamily(secant::Error::Invalid(_)))
    ));
    let directory = tempfile::tempdir().unwrap();
    let (input, mut options) = regular_input(false);
    options
        .fixed_fiber
        .insert(symbol!("auto_regular::unused"), Rational::one());
    assert!(matches!(
        prepare(input, options, directory.path(), |_| ControlFlow::Continue(
            ()
        )),
        Err(Error::Unsupported(_))
    ));
    let directory = tempfile::tempdir().unwrap();
    let (input, mut options) = regular_input(false);
    options.threshold.solver.order = vec!["v1".into(), "v0".into()];
    assert!(matches!(
        prepare(input, options, directory.path(), |_| ControlFlow::Continue(
            ()
        )),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn automatic_regular_replays_resolved_roles_in_a_fresh_child() {
    let directory = tempfile::tempdir().unwrap();
    let (input, mut options) = regular_input(true);
    options.regular_family.as_mut().unwrap().limits.unit_margin = Rational::from((1, 4));
    let receipt = prepare(input, options, directory.path(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap()
    .receipt;
    std::fs::write(
        directory.path().join("receipt.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "threshold::generation::tests::regular_tests::regular_replay_child",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("FSD_REGULAR_FACTORY_CHILD", directory.path())
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
#[ignore = "invoked in a fresh native process by automatic_regular_replays_resolved_roles_in_a_fresh_child"]
fn regular_replay_child() {
    let Some(root) = std::env::var_os("FSD_REGULAR_FACTORY_CHILD") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    for i in 0..19 {
        let _ = symbol!(format!("regular_replay_noise::s{i}"));
    }
    let receipt: PreparationReceipt =
        serde_json::from_slice(&std::fs::read(root.join("receipt.json")).unwrap()).unwrap();
    let stored = configuration::read(&root, &receipt.configuration, 32 * 1024 * 1024)
        .unwrap()
        .0;
    let config = stored.regular_family.unwrap();
    assert_eq!(config.limits.unit_margin, Rational::from((1, 4)));
    assert_eq!(
        config.coordinates.as_ref().unwrap().last(),
        Some(&stored.unit)
    );
    let mut stages = Vec::new();
    let replay = resume(
        &root,
        &receipt,
        "cold-vectors",
        &receipt.publication.source_identity,
        &receipt.publication.prepared_identity,
        32 * 1024 * 1024,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&Stage::Solve));
    assert!(stages.contains(&Stage::Verify));
    assert_eq!(
        replay.receipt.publication.prepared_identity,
        receipt.publication.prepared_identity
    );
    let mut first = compile(&root, &receipt);
    let mut second = compile(&root, &replay.receipt);
    assert_eq!(first.content_id(), second.content_id());
    for (a, b) in first.sectors_mut().iter_mut().zip(second.sectors_mut()) {
        let mut av = vec![0.; a.output_count()];
        let mut bv = vec![0.; b.output_count()];
        a.evaluate(&[0.31, 0.57], &mut av).unwrap();
        b.evaluate(&[0.31, 0.57], &mut bv).unwrap();
        assert_eq!(av, bv);
    }
}

#[test]
fn automatic_regular_declines_inapplicable_shape_without_searching_margins() {
    let directory = tempfile::tempdir().unwrap();
    let (input, options) = regular_input(false);
    let x = input.parameters()[0];
    let replacement = ParametricIntegrand::new(
        input.parameters().to_vec(),
        input.regulator(),
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(
                    Atom::var(x) - Atom::num((1, 2)),
                    -Atom::one() - Atom::var(input.regulator()),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let mut attempts = 0;
    let result = prepare(Arc::new(replacement), options, directory.path(), |p| {
        if let Some(d) = p.regular_family {
            attempts = d.family_attempts;
        }
        ControlFlow::Continue(())
    });
    assert!(matches!(
        result,
        Err(Error::RegularFamily(secant::Error::Unsupported(_)))
    ));
    assert_eq!(attempts, 0);
}

#[test]
fn automatic_regular_does_not_retry_immutable_density_failures() {
    let directory = tempfile::tempdir().unwrap();
    let (input, options) = regular_input(false);
    let mut factors = input.terms()[0].factors().to_vec();
    factors.push(PolynomialFactor::new(
        Atom::var(input.parameters()[0]) / (Atom::one() + Atom::var(input.regulator())),
        Atom::one(),
        FactorRole::Polynomial,
    ));
    let input = ParametricIntegrand::new(
        input.parameters().to_vec(),
        input.regulator(),
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            factors,
        )],
    )
    .unwrap();
    let mut attempts = 0;
    let result = prepare(Arc::new(input), options, directory.path(), |progress| {
        if let Some(discovery) = progress.regular_family {
            attempts = discovery.family_attempts;
        }
        ControlFlow::Continue(())
    });
    assert!(matches!(
        result,
        Err(Error::RegularFamily(secant::Error::Unsupported(_)))
    ));
    assert_eq!(attempts, 1);
}
