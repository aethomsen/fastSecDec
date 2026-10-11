//! Genuine shared-recursion fixture; no manufactured center/history receipts.
use super::super::super::*;
use std::sync::Arc;
use symbolica::symbol;

pub(super) fn budget() -> Budget {
    Budget::new(Limits {
        max_operations: 2_000_000,
        max_total_ideal_slots: 2_000_000,
        ..Limits::default()
    })
}
pub(super) fn second(
    extra: bool,
    b: &mut Budget,
) -> (Arc<ContinuedSupportedCenter>, Arc<RelativeRecursiveBlowup>) {
    let outer = super::super::relative_tests::original_probe::original_with_factors(b, extra);
    let origin = EmbeddedChildCycle::new(outer, b).unwrap();
    let mut recursion = origin
        .initial_problem()
        .unwrap()
        .recursion("original_nested_problem".into(), b)
        .unwrap();
    for _ in 0..1000 {
        if matches!(
            recursion
                .advance(&ComponentFactorLimits::default(), b)
                .unwrap(),
            RecursiveAdvance::Complete
        ) {
            break;
        }
    }
    let ready = recursion
        .records()
        .iter()
        .find_map(|r| {
            if let ProblemRecursionRecord::Center(c) = r {
                Some(c.clone())
            } else {
                None
            }
        })
        .unwrap();
    let EmbeddedProblemProduction::OriginalCenter(physical) = ready.ascend(b).unwrap() else {
        panic!()
    };
    let anchored =
        AnchoredOriginalTree::new(OriginalAnchorCenter::Original(physical.clone()), b).unwrap();
    let checked =
        CheckedRecursiveCenter::new(RecursiveCenterOrigin::OriginalProblem(physical), b).unwrap();
    let adapted = adapt_recursive_center(checked, "original_nested_adapt", b).unwrap();
    let first = blowup_recursive_center(adapted, "original_nested_blowup", b).unwrap();
    for (pi, chart) in first.charts().iter().enumerate() {
        if pi != 2 {
            continue;
        }
        let carried = SupportedProblemChart::prepare(
            anchored.clone(),
            chart.clone(),
            &format!("original_current{pi}"),
            b,
        )
        .unwrap();
        let _ancestors = carried
            .ancestor_presentations(&format!("original_ancestors{pi}"), b)
            .unwrap();
        for (oi, current) in carried.opens().iter().enumerate() {
            let mut factors = ComponentFactorFrontier::new(
                current.history().clone(),
                current.source().target().clone(),
                format!("original_drop_factors{pi}_{oi}"),
                b,
            )
            .unwrap();
            for _ in 0..100 {
                let Some(path) = factors.pending().next().cloned() else {
                    break;
                };
                factors
                    .advance(&path, &ComponentFactorLimits::default(), b)
                    .unwrap();
            }
            let ComponentFactorCompletion::Complete(factors) = factors.try_complete().unwrap()
            else {
                panic!()
            };
            let inventory = SupportedDropInventory::prove(
                carried.clone(),
                current.clone(),
                Arc::new(factors),
                b,
            )
            .unwrap();
            for (ri, record) in inventory.records().iter().enumerate() {
                if !matches!(record, SupportedDropRecord::Positive(_)) {
                    continue;
                }
                let drop = inventory.positive_drop(ri).unwrap();
                let cycle = CycleSnapshot::after_supported_drop(drop.clone(), b).unwrap();
                for ni in 0..drop.current().upper_order_cover().opens().len() {
                    let CompanionOpenProduction::Contact(open) = produce_companion_open(
                        drop.current().clone(),
                        ni,
                        &format!("next_open{pi}_{oi}_{ri}_{ni}"),
                        b,
                    )
                    .unwrap() else {
                        continue;
                    };
                    let open = Arc::new(*open);
                    for ci in 0..open.contact().candidates().len() {
                        let CompanionCoefficientProduction::Coefficient(q) =
                            construct_companion_coefficient_for_cycle(
                                open.clone(),
                                ci,
                                [
                                    symbol!(format!("next_q{pi}_{oi}_{ri}_{ni}_{ci}")),
                                    symbol!(format!("next_inv{pi}_{oi}_{ri}_{ni}_{ci}")),
                                ],
                                cycle.clone(),
                                b,
                            )
                            .unwrap()
                        else {
                            continue;
                        };
                        let mut driver = CompanionFirstCenter::new(
                            Arc::new(*q),
                            &format!("next_center{pi}_{oi}_{ri}_{ni}_{ci}"),
                            false,
                        )
                        .unwrap();
                        for _ in 0..100 {
                            if !matches!(driver.advance(b).unwrap(), RecursiveAdvance::Progress) {
                                break;
                            }
                        }
                        let CompanionCenterOutcome::Center(next) = driver.outcome() else {
                            continue;
                        };
                        let ContinuedSupportedProduction::Center(next) =
                            ContinuedSupportedCenter::prepare(
                                drop.clone(),
                                next.clone(),
                                &format!("next_ancestor{pi}_{oi}_{ri}_{ni}_{ci}"),
                                b,
                            )
                            .unwrap()
                        else {
                            continue;
                        };
                        let checked = CheckedRecursiveCenter::new(
                            RecursiveCenterOrigin::SupportedProblem(next.clone()),
                            b,
                        )
                        .unwrap();
                        let adapted = adapt_recursive_center(
                            checked,
                            &format!("next_adapt{pi}_{oi}_{ri}_{ni}_{ci}"),
                            b,
                        )
                        .unwrap();
                        let result = blowup_recursive_center(
                            adapted,
                            &format!("next_blowup{pi}_{oi}_{ri}_{ni}_{ci}"),
                            b,
                        )
                        .unwrap();
                        return (next, result);
                    }
                }
            }
        }
    }
    panic!("no actual original-source second physical chart");
}
