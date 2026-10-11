use super::*;
use symbolica::symbol;
fn budget() -> Budget {
    Budget::new(Limits::default())
}
fn complete_factors(
    history: Arc<ResolutionHistory>,
    source: Arc<MarkedIdeal>,
    ns: &str,
    b: &mut Budget,
) -> Arc<CompletedComponentFactors> {
    let mut f = ComponentFactorFrontier::new(history, source, ns.into(), b).unwrap();
    for _ in 0..100 {
        let Some(p) = f.pending().next().cloned() else {
            break;
        };
        f.advance(&p, &ComponentFactorLimits::default(), b).unwrap();
    }
    let ComponentFactorCompletion::Complete(f) = f.try_complete().unwrap() else {
        panic!()
    };
    Arc::new(f)
}
fn initial_order(px: usize, py: usize, b: &mut Budget) -> Arc<ComponentResidualOrder> {
    let ring = Arc::new(
        Ring::new(
            vec![
                symbol!("adapter_actual_cusp::x"),
                symbol!("adapter_actual_cusp::y"),
            ],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(ring.clone(), vec![], b).unwrap(),
        vec![0, 1],
        vec![],
        b,
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
        .verify(b)
        .unwrap(),
    );
    let SncProduction::Verified(ledger) = verify_initial_relative_snc(frame, vec![], b).unwrap()
    else {
        panic!()
    };
    let history = ResolutionHistory::initial(ledger).unwrap();
    let f = ring.coordinate(0).unwrap().pow(px) + ring.coordinate(1).unwrap().pow(py);
    let source = Arc::new(MarkedIdeal::new(Ideal::new(ring, vec![f], b).unwrap(), 1, b).unwrap());
    let factors = complete_factors(history, source, "plane_initial", b);
    let leaf = factors.nodes().values().find_map(|n| n.leaf()).unwrap();
    let ComponentResidualProduction::Order(order) =
        produce_component_residual_order(leaf.clone(), b).unwrap()
    else {
        panic!()
    };
    Arc::new(*order)
}
pub(crate) fn cusp_center(b: &mut Budget) -> Arc<CompanionCenter> {
    let order = initial_order(3, 5, b);
    for oi in 0..order.upper_order_cover().opens().len() {
        let CompanionOpenProduction::Contact(open) =
            produce_companion_open(order.clone(), oi, &format!("adapter_o{oi}"), b).unwrap()
        else {
            continue;
        };
        let open = Arc::new(*open);
        for ci in 0..open.contact().candidates().len() {
            let CompanionCoefficientProduction::Coefficient(q) = construct_companion_coefficient(
                open.clone(),
                ci,
                [
                    symbol!(format!("adapter_q{oi}_{ci}")),
                    symbol!(format!("adapter_i{oi}_{ci}")),
                ],
                b,
            )
            .unwrap() else {
                continue;
            };
            let q = Arc::new(*q);
            let mut original =
                CompanionFirstCenter::new(q.clone(), "adapter_unoptimized", false).unwrap();
            let mut limited = false;
            for _ in 0..32 {
                match original.advance(b).unwrap() {
                    RecursiveAdvance::Progress => {}
                    RecursiveAdvance::Incomplete { reason } => {
                        assert_eq!(reason, "coefficient mark LCM");
                        limited = true;
                        break;
                    }
                    RecursiveAdvance::Complete => break,
                }
            }
            if !limited {
                continue;
            }
            assert!(matches!(
                original.outcome(),
                CompanionCenterOutcome::Pending
            ));
            let mut driver = CompanionFirstCenter::new_with_principal_powers(
                q.clone(),
                "adapter_optimized",
                false,
            )
            .unwrap();
            for _ in 0..32 {
                match driver.advance(b).unwrap() {
                    RecursiveAdvance::Progress => {}
                    RecursiveAdvance::Complete => break,
                    other => panic!("optimized {other:?}"),
                }
            }
            let CompanionCenterOutcome::Center(center) = driver.outcome() else {
                panic!("missing optimized center {:?}", driver.outcome())
            };
            assert!(Arc::ptr_eq(center.coefficient(), &q));
            assert_eq!(center.child().source().mark(), 6);
            assert_eq!(
                center.child().source().ideal(),
                q.coefficient().coefficient().ideal()
            );
            let level = center.child().lift_receipt().unwrap().level();
            let presentation = level.presentation();
            let power = presentation.power().unwrap();
            assert_eq!(power.power(), 2);
            assert_eq!(presentation.effective_source().mark(), 3);
            assert_eq!(center.child().invariant_pairs(), &[Rational::from((5, 3))]);
            assert_eq!(presentation.original_maximum_order(5).unwrap(), 10);
            assert!(Arc::ptr_eq(
                presentation.original_source(),
                center.child().source()
            ));
            return center.clone();
        }
    }
    panic!("no actual original cusp coefficient");
}
#[test]
fn actual_cusp_initial_presentation_readmits_original_center() {
    let mut b = budget();
    let center = cusp_center(&mut b);
    let presentation = center
        .child()
        .lift_receipt()
        .unwrap()
        .level()
        .presentation();
    let original_problem = OriginalPhysicalProblem::new(center.clone(), &mut b).unwrap();
    let lower = EmbeddedChildCycle::new(center.clone(), &mut b).unwrap();
    assert!(Arc::ptr_eq(
        lower.initial_history(),
        presentation.origin().unwrap().history()
    ));
    assert_eq!(
        lower.original_tree().levels()[0]
            .companion()
            .unwrap()
            .mark(),
        10
    );
    assert_eq!(
        lower.original_tree().levels()[0]
            .effective_companion()
            .unwrap()
            .mark(),
        5
    );
    let checked =
        CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(center.clone()), &mut b)
            .unwrap();
    let adapted = adapt_recursive_center(checked, "adapter_actual_adapt", &mut b).unwrap();
    let maps = blowup_recursive_center(adapted, "adapter_actual_maps", &mut b).unwrap();
    let mut pivots = 0;
    let mut power_opens = 0;
    let mut empty_supports = 0;
    let mut wrong_units_refused = 0;
    for (i, chart) in maps.charts().iter().enumerate() {
        if chart.geometry().exceptional().is_some() {
            pivots += 1;
        }
        let state = original_problem.advance(chart.clone(), &mut b).unwrap();
        assert!(Arc::ptr_eq(state.root(), original_problem.root()));
        let tree = lower
            .original_tree()
            .pull_first(chart.clone(), &format!("adapter_tree{i}"), &mut b)
            .unwrap();
        assert_eq!(tree.levels().len(), lower.original_tree().levels().len());
        tree.histories(&mut b).unwrap();
        for level in tree.levels() {
            if level.opens().is_empty() {
                empty_supports += 1;
            }
            for open in level.opens() {
                if let Some(p) = open.power() {
                    power_opens += 1;
                    assert!(Arc::ptr_eq(p.presentation(), presentation));
                    assert_eq!(p.original().source().mark(), 6);
                    assert_eq!(p.effective().source().mark(), 3);
                    assert_eq!(p.original_companion().source().mark(), 10);
                    assert_eq!(p.effective_companion().source().mark(), 5);
                    assert_eq!(p.unit().exceptional_power(), 0);
                    let local = open.source().open().frame().local();
                    let g = &p.effective_companion().generators()[0].quotient;
                    if !local.zero(g, &mut b).unwrap() {
                        let wrong_unit = &p.unit().generators()[0].quotient + &local.ring().one();
                        let powered = b.power(g, presentation.power().unwrap().power()).unwrap();
                        let wrong = b.mul(&wrong_unit, &powered).unwrap();
                        let actual = &p.original_companion().generators()[0].quotient;
                        assert!(!local.zero(&(actual - &wrong), &mut b).unwrap());
                        wrong_units_refused += 1;
                    }
                }
            }
        }
    }
    assert!(pivots >= 2);
    assert!(power_opens > 0);
    assert!(empty_supports > 0);
    assert!(wrong_units_refused > 0);
    assert_eq!(b.limits.max_mark, 64);
}
