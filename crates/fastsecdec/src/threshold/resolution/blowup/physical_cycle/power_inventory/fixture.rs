use super::*;
use std::sync::Arc;
use symbolica::domains::rational::Rational;
use symbolica::symbol;
pub(super) fn budget() -> Budget {
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
pub(super) fn cusp_center(py: usize, b: &mut Budget) -> Arc<CompanionCenter> {
    let order = initial_order(3, py, b);
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
            let k = if py == 5 { 2 } else { 6 };
            assert_eq!(power.power(), k);
            assert_eq!(presentation.effective_source().mark(), 6 / k);
            assert_eq!(
                center.child().invariant_pairs(),
                &[Rational::from((py as i64, 3))]
            );
            assert_eq!(
                presentation.original_maximum_order(2 * py / k).unwrap(),
                2 * py
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

pub(super) fn next_center(
    state: &Arc<OriginalPhysicalState>,
    ns: &str,
    b: &mut Budget,
) -> Option<(Arc<EmbeddedPresentation>, Arc<RelativeRecursiveBlowup>)> {
    let presentations = state.active_presentations(ns, b).unwrap();
    for (i, p) in presentations.iter().enumerate() {
        let factors = complete_factors(
            p.history().clone(),
            p.incidence_sum().clone(),
            &format!("{ns}_factor{i}"),
            b,
        );
        for (j, leaf) in factors
            .nodes()
            .values()
            .filter_map(|n| n.leaf())
            .enumerate()
        {
            match produce_embedded_monomial_center(p.clone(), leaf.clone(), b).unwrap() {
                EmbeddedCenterProduction::Center(c) => {
                    let checked =
                        CheckedRecursiveCenter::new(RecursiveCenterOrigin::EmbeddedMonomial(c), b)
                            .unwrap();
                    let adapted =
                        adapt_recursive_center(checked, &format!("{ns}_adapt{i}_{j}"), b).unwrap();
                    return Some((
                        p.clone(),
                        blowup_recursive_center(adapted, &format!("{ns}_blow{i}_{j}"), b).unwrap(),
                    ));
                }
                EmbeddedCenterProduction::Empty(_) => println!("CENTER_EMPTY {i}/{j}"),
                EmbeddedCenterProduction::NeedsLowerRecursion { .. } => {
                    println!("CENTER_NEEDS_RECURSION {i}/{j}")
                }
                EmbeddedCenterProduction::NeedsLocalization { .. } => {
                    println!("CENTER_NEEDS_LOCALIZATION {i}/{j}")
                }
            }
        }
    }
    None
}

pub(super) fn initial(
    py: usize,
) -> (
    Arc<OriginalPhysicalProblem>,
    Arc<RelativeRecursiveBlowup>,
    PhysicalChartFrontier,
) {
    let mut b = budget();
    let center = cusp_center(py, &mut b);
    let problem = OriginalPhysicalProblem::new(center.clone(), &mut b).unwrap();
    let checked =
        CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(center), &mut b).unwrap();
    let adapted = adapt_recursive_center(checked, &format!("aux_initial{py}"), &mut b).unwrap();
    let maps = blowup_recursive_center(adapted, &format!("aux_initial_maps{py}"), &mut b).unwrap();
    let frontier = PhysicalChartFrontier::from_problem(
        problem.clone(),
        maps.clone(),
        format!("aux_first{py}"),
        &mut b,
    )
    .unwrap();
    (problem, maps, frontier)
}
pub(super) fn finish_physical(frontier: &mut PhysicalChartFrontier) {
    for index in 0..frontier.work().len() {
        let mut b = budget();
        for _ in 0..1000 {
            match frontier
                .advance(index, &ComponentFactorLimits::default(), &mut b)
                .unwrap()
            {
                PhysicalChartAdvance::Progress => {}
                PhysicalChartAdvance::Complete | PhysicalChartAdvance::AlreadyComplete => break,
                x => panic!("physical {x:?}"),
            }
        }
    }
    assert!(frontier.complete_inventory());
}
pub(super) fn finish_aux(frontier: &mut PhysicalChartFrontier, reverse: bool) {
    let count = frontier.work().len();
    for step in 0..count {
        let index = if reverse { count - 1 - step } else { step };
        for _ in 0..1000 {
            match frontier.advance_auxiliary(index, &mut budget()).unwrap() {
                PhysicalAuxiliaryAdvance::Progress => {}
                PhysicalAuxiliaryAdvance::Accounted
                | PhysicalAuxiliaryAdvance::AlreadyAccounted => break,
                x => panic!("auxiliary {x:?}"),
            }
        }
    }
    assert!(frontier.auxiliary_accounting_complete());
}
pub(super) fn record(
    frontier: &PhysicalChartFrontier,
    index: usize,
) -> &Arc<PhysicalTerminalInventory> {
    match &frontier.work()[index] {
        PhysicalChartWork::Complete(t) => t,
        _ => panic!(),
    }
}

pub(super) fn reset_center(
    order: Arc<ComponentResidualOrder>,
    cycle: Option<Arc<CycleSnapshot>>,
    ns: &str,
    b: &mut Budget,
) -> Arc<CompanionCenter> {
    for oi in 0..order.upper_order_cover().opens().len() {
        let CompanionOpenProduction::Contact(o) =
            produce_companion_open(order.clone(), oi, &format!("{ns}_o{oi}"), b).unwrap()
        else {
            continue;
        };
        let o = Arc::new(*o);
        for ci in 0..o.contact().candidates().len() {
            let symbols = [
                symbol!(format!("{ns}_q{oi}_{ci}")),
                symbol!(format!("{ns}_i{oi}_{ci}")),
            ];
            let q = match &cycle {
                Some(c) => {
                    construct_companion_coefficient_for_cycle(o.clone(), ci, symbols, c.clone(), b)
                }
                None => construct_companion_coefficient(o.clone(), ci, symbols, b),
            }
            .unwrap();
            let CompanionCoefficientProduction::Coefficient(q) = q else {
                continue;
            };
            let mut d = CompanionFirstCenter::new(Arc::new(*q), &format!("{ns}_r{oi}_{ci}"), false)
                .unwrap();
            for _ in 0..100 {
                let step = d.advance(b).unwrap();
                if !matches!(step, RecursiveAdvance::Progress) {
                    println!("PLANE step {ns} {oi}/{ci} {step:?}");
                    break;
                }
            }
            if let CompanionCenterOutcome::Center(c) = d.outcome() {
                return c.clone();
            }
            println!(
                "PLANE outcome {}",
                match d.outcome() {
                    CompanionCenterOutcome::Pending => "pending",
                    CompanionCenterOutcome::ChildComponentOpensRequired(_) => "component",
                    CompanionCenterOutcome::ChildNeedsMoreOpens(_) => "contactcover",
                    CompanionCenterOutcome::EmptyChild(_) => "empty",
                    _ => "center",
                }
            );
        }
    }
    panic!("no plane center")
}
pub(super) fn reset_maps(
    c: Arc<CompanionCenter>,
    ns: &str,
    b: &mut Budget,
) -> Arc<RelativeRecursiveBlowup> {
    let checked = CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(c), b).unwrap();
    let adapted = adapt_recursive_center(checked, &format!("{ns}_adapt"), b).unwrap();
    blowup_recursive_center(adapted, &format!("{ns}_maps"), b).unwrap()
}
