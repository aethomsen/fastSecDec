use super::*;
#[test]
fn native_normalization_preserves_keyed_physical_coefficient_probe() {
    use super::super::super::general_transform::controlled_marked;
    let mut b = budget();
    let ring = Arc::new(
        Ring::new(
            vec![
                symbol!("lineage_coefficient::x"),
                symbol!("lineage_coefficient::y"),
            ],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(ring.clone(), vec![], &mut b).unwrap(),
        vec![0, 1],
        vec![],
        &mut b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            free_axes: vec![0, 1],
            determinant_inverse_axis: None,
        }
        .verify(&mut b)
        .unwrap(),
    );
    let SncProduction::Verified(ledger) =
        verify_initial_relative_snc(frame.clone(), vec![], &mut b).unwrap()
    else {
        panic!()
    };
    let history = ResolutionHistory::initial(ledger).unwrap();
    let x = ring.coordinate(0).unwrap();
    let original = Arc::new(
        MarkedIdeal::new(
            Ideal::new(ring.clone(), vec![&x + &x], &mut b).unwrap(),
            1,
            &b,
        )
        .unwrap(),
    );
    let normalizer = QuotientNormalizer::prepare(frame.local().clone(), &mut b).unwrap();
    let NormalizationOutcome::Complete(equivalence) =
        normalizer.normalize(original.clone(), &mut b).unwrap()
    else {
        panic!()
    };
    assert!(Arc::ptr_eq(equivalence.source(), &original));
    assert_ne!(
        equivalence.source().ideal().generators(),
        equivalence.normalized().ideal().generators()
    );
    let mut driver = BoundaryFreeFirstCenter::new(
        frame.clone(),
        equivalence.normalized().clone(),
        "lineage_coefficient",
        false,
    )
    .unwrap();
    for _ in 0..100 {
        if !matches!(driver.advance(&mut b).unwrap(), RecursiveAdvance::Progress) {
            break;
        }
    }
    let RecursiveOutcome::Center(center) = driver.outcome() else {
        panic!()
    };
    let checked = CheckedRecursiveCenter::new(
        RecursiveCenterOrigin::BoundaryFree {
            center: center.clone(),
            history,
        },
        &mut b,
    )
    .unwrap();
    let adapted = adapt_recursive_center(checked, "lineage_coefficient_adapt", &mut b).unwrap();
    let maps = blowup_recursive_center(adapted, "lineage_coefficient_maps", &mut b).unwrap();
    assert!(!maps.charts().is_empty());
    for (i, chart) in maps.charts().iter().enumerate() {
        assert!(
            chart
                .parent()
                .source()
                .ideal()
                .contains(
                    &original.ideal().generators()[0],
                    frame.local().unit_relations(),
                    &mut b
                )
                .unwrap()
        );
        let tracked = controlled_marked(
            chart.geometry().clone(),
            original.clone(),
            &format!("lineage_keyed{i}"),
            &mut b,
        )
        .unwrap();
        assert_eq!(tracked.generators().len(), 1);
        let exceptional = super::super::AccumulatedExceptional::first(chart, &mut b).unwrap();
        let original_factor = super::super::OriginalGeneratorPullback::checked(
            tracked.generators()[0].source.clone(),
            tracked.generators()[0].total.clone(),
            exceptional.product().clone(),
            tracked.generators()[0].quotient.clone(),
            chart.geometry().frame().local(),
            &mut b,
        )
        .unwrap();
        let factors = complete(chart, &format!("coefficient_terminal_factors{i}"), &mut b);
        let records = super::super::terminal::prove_original_terminal(
            super::super::terminal::OriginalTerminalInput {
                source: chart.parent().target(),
                history: chart.history(),
                exceptional: &exceptional,
                generators: &[original_factor],
                jacobian: chart.geometry().original_coordinate_jacobian(),
            },
            &factors,
            &format!("coefficient_terminal{i}"),
            &mut b,
        )
        .unwrap();
        assert!(!records.is_empty());
        assert!(
            records
                .iter()
                .all(|r| matches!(r, OriginalTerminalRecord::Terminal { .. }))
        );

        assert_eq!(
            tracked.generators()[0].source,
            original.ideal().generators()[0]
        );
        let local = chart.geometry().frame().local();
        let two = local.ring().one() + local.ring().one();
        assert_eq!(chart.parent().generators().len(), 1);
        let expected = b
            .mul(&two, &chart.parent().generators()[0].quotient)
            .unwrap();
        assert!(
            local
                .zero(&(&tracked.generators()[0].quotient - &expected), &mut b)
                .unwrap()
        );
        assert!(
            local
                .zero(
                    &(&tracked.generators()[0].total
                        - &chart
                            .geometry()
                            .pull(&original.ideal().generators()[0], &mut b)
                            .unwrap()),
                    &mut b
                )
                .unwrap()
        );
    }
    println!(
        "PASS: native normalization2x→x, same issued charts retain keyed quotient ratio2 including identity complements"
    );
}
