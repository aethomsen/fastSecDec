use super::*;
use std::sync::Arc;
use symbolica::domains::rational::Rational;
use symbolica::symbol;
pub(super) fn budget() -> Budget {
    Budget::new(Limits::default())
}
pub(super) fn complete_factors(
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
fn initial_order(px: usize, py: usize, mode: u8, b: &mut Budget) -> Arc<ComponentResidualOrder> {
    let mut variables = vec![
        symbol!("adapter_actual_cusp::x"),
        symbol!("adapter_actual_cusp::y"),
    ];
    if mode == 1 {
        variables.extend([
            symbol!("adapter_actual_cusp::p"),
            symbol!("adapter_actual_cusp::pi"),
        ]);
    }
    let ring = Arc::new(Ring::new(variables, if mode == 1 { vec![2] } else { vec![] }).unwrap());
    let unit = if mode > 0 {
        ring.one()
            + ring
                .coordinate(if mode == 1 { 2 } else { 1 })
                .unwrap()
                .pow(2)
    } else {
        ring.one()
    };
    let local = LocalizedAlgebra::new(
        Ideal::new(ring.clone(), vec![], b).unwrap(),
        vec![0, 1],
        if mode > 0 {
            vec![Guard {
                factor: unit.clone(),
                inverse_axis: if mode == 1 { 3 } else { 2 },
            }]
        } else {
            vec![]
        },
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
    let f = &unit * &(ring.coordinate(0).unwrap().pow(px) + ring.coordinate(1).unwrap().pow(py));
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
pub(super) fn center(py: usize, b: &mut Budget) -> Arc<CompanionCenter> {
    center_inner(py, 0, b)
}
fn center_inner(py: usize, mode: u8, b: &mut Budget) -> Arc<CompanionCenter> {
    let order = initial_order(3, py, mode, b);
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
            assert_eq!(power.power(), if py == 6 { 6 } else { 2 });
            assert_eq!(
                presentation.effective_source().mark(),
                if py == 6 { 1 } else { 3 }
            );
            assert_eq!(
                center.child().invariant_pairs(),
                &[if py == 6 {
                    Rational::from(2)
                } else {
                    Rational::from((5, 3))
                }]
            );
            assert_eq!(
                presentation
                    .original_maximum_order(if py == 6 { 2 } else { 5 })
                    .unwrap(),
                if py == 6 { 12 } else { 10 }
            );
            assert!(Arc::ptr_eq(
                presentation.original_source(),
                center.child().source()
            ));
            return center.clone();
        }
    }
    panic!("no actual original cusp coefficient");
}

pub(super) fn parametric_refusal(b: &mut Budget) {
    let order = initial_order(3, 6, 1, b);
    for oi in 0..order.upper_order_cover().opens().len() {
        let CompanionOpenProduction::Contact(open) =
            produce_companion_open(order.clone(), oi, &format!("carried_parametric_o{oi}"), b)
                .unwrap()
        else {
            continue;
        };
        let open = Arc::new(*open);
        for ci in 0..open.contact().candidates().len() {
            let CompanionCoefficientProduction::Coefficient(q) = construct_companion_coefficient(
                open.clone(),
                ci,
                [
                    symbol!(format!("carried_parametric_q{oi}_{ci}")),
                    symbol!(format!("carried_parametric_i{oi}_{ci}")),
                ],
                b,
            )
            .unwrap() else {
                continue;
            };
            assert!(matches!(
                CompanionFirstCenter::new_with_principal_powers(
                    Arc::new(*q),
                    "carried_parametric_refused",
                    false
                ),
                Err(Error::Invalid("first-center fixed-parameter source/frame"))
            ));
            return;
        }
    }
    panic!("missing genuine parameter coefficient");
}
