use super::super::super::*;
use super::tests::complete;
use super::tests_fixture::{budget, second};
use std::sync::Arc;
use symbolica::symbol;
#[test]
fn physical_lineage_preserves_all_original_factors_through_four_maps() {
    let mut b = budget();
    let (center, charts) = second(false, &mut b);
    let chart = charts.charts()[7].clone();
    let carry =
        AmbientCycleCarry::prepare(center, chart.clone(), "next_actual_carry", &mut b).unwrap();
    let factors = complete(&chart, "next_actual_factors", &mut b);
    let state = OriginalPhysicalState::from_ambient(carry.clone(), &mut b).unwrap();
    let inventory = AmbientDropInventory::prove(carry, factors, &mut b).unwrap();
    let index = inventory
        .records()
        .iter()
        .position(|r| matches!(r, AmbientDropRecord::Positive(_)))
        .unwrap();
    let drop = inventory.positive_drop(index).unwrap();
    let cycle = CycleSnapshot::after_ambient_drop(drop.clone(), &mut b).unwrap();
    for oi in 0..drop.current().upper_order_cover().opens().len() {
        let CompanionOpenProduction::Contact(open) = produce_companion_open(
            drop.current().clone(),
            oi,
            &format!("next_actual_open{oi}"),
            &mut b,
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
                        symbol!(format!("next_actual_q{oi}_{ci}")),
                        symbol!(format!("next_actual_inv{oi}_{ci}")),
                    ],
                    cycle.clone(),
                    &mut b,
                )
                .unwrap()
            else {
                continue;
            };
            let mut driver = CompanionFirstCenter::new(
                Arc::new(*q),
                &format!("next_actual_recursion{oi}_{ci}"),
                false,
            )
            .unwrap();
            for _ in 0..100 {
                if !matches!(driver.advance(&mut b).unwrap(), RecursiveAdvance::Progress) {
                    break;
                }
            }
            let CompanionCenterOutcome::Center(c) = driver.outcome() else {
                continue;
            };
            if c.normals().len() != 2 {
                continue;
            }

            let owner =
                AmbientContinuationCenter::prepare(drop.clone(), c.clone(), &mut b).unwrap();
            assert!(matches!(
                AmbientContinuationCenter::prepare(
                    inventory.positive_drop(index).unwrap(),
                    c.clone(),
                    &mut b
                ),
                Err(Error::Invalid(_))
            ));
            assert!(matches!(
                AmbientNextChart::prepare(owner.clone(), chart.clone(), &mut b),
                Err(Error::Invalid(_))
            ));
            let start = state.begin_ambient_cycle(owner.clone(), &mut b).unwrap();
            let checked =
                CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(c.clone()), &mut b)
                    .unwrap();
            let adapted =
                adapt_recursive_center(checked, &format!("next_actual_adapt{oi}_{ci}"), &mut b)
                    .unwrap();
            let next =
                blowup_recursive_center(adapted, &format!("next_actual_blowup{oi}_{ci}"), &mut b)
                    .unwrap();
            assert_eq!(next.charts().len(), 10);
            let mut count = 0;
            for mapped in next.charts() {
                let receipt =
                    AmbientNextChart::prepare(owner.clone(), mapped.clone(), &mut b).unwrap();
                assert!(Arc::ptr_eq(receipt.owner(), &owner));
                assert!(Arc::ptr_eq(receipt.chart(), mapped));
                assert_eq!(receipt.generators().len(), 1);
                let local = mapped.geometry().frame().local();
                for original in receipt.generators() {
                    assert!(
                        local
                            .zero(
                                &(original.total()
                                    - &b.mul(
                                        original.exceptional_factor(),
                                        original.current_quotient()
                                    )
                                    .unwrap()),
                                &mut b
                            )
                            .unwrap()
                    );
                }
                let old = owner
                    .source_open()
                    .open()
                    .extension()
                    .pull(
                        owner
                            .residual_drop()
                            .inventory()
                            .carry()
                            .original_coordinate_jacobian(),
                        &mut b,
                    )
                    .unwrap();
                let pulled = mapped.geometry().pull(&old, &mut b).unwrap();
                let expected = b
                    .mul(&pulled, mapped.geometry().original_coordinate_jacobian())
                    .unwrap();
                assert!(
                    local
                        .zero(
                            &(receipt.original_coordinate_jacobian() - &expected),
                            &mut b
                        )
                        .unwrap()
                );
                let state3 = start
                    .advance(mapped.clone(), &format!("generic_third{count}"), &mut b)
                    .unwrap();
                assert_eq!(state3.depth(), 3);
                assert_eq!(state3.retired_count(), 1);
                assert!(Arc::ptr_eq(state.root(), state3.root()));
                let factors = complete(mapped, &format!("generic_third_factors{count}"), &mut b);
                if count == 7 {
                    assert!(matches!(
                        PhysicalDropInventory::prove(
                            state3.clone(),
                            factors.clone(),
                            "wrong_unfinished_active_drop",
                            &mut b
                        ),
                        Err(Error::Invalid(_))
                    ));
                }
                assert!(Arc::ptr_eq(state3.retired().unwrap().source(), &state));
                assert_eq!(state3.retired().unwrap().count(), 1);
                assert!(state3.retired().unwrap().previous().is_none());
                let terminal = PhysicalTerminalInventory::prove(
                    state3.clone(),
                    factors,
                    &format!("generic_third_terminal{count}"),
                    &mut b,
                )
                .unwrap();
                println!(
                    "GENERIC depth3 map{count} original_terminal{}",
                    terminal.all_local_factors_snc()
                );
                if count == 7 {
                    assert!(!terminal.all_local_factors_snc());
                    let presentations = state3
                        .active_presentations("generic_active7", &mut b)
                        .unwrap();
                    assert_eq!(presentations.len(), 5);
                    for (si, presentation) in presentations.iter().enumerate() {
                        let mut f = ComponentFactorFrontier::new(
                            presentation.history().clone(),
                            presentation.incidence_sum().clone(),
                            format!("generic_lower7_{si}"),
                            &mut b,
                        )
                        .unwrap();
                        for _ in 0..100 {
                            let Some(path) = f.pending().next().cloned() else {
                                break;
                            };
                            f.advance(&path, &ComponentFactorLimits::default(), &mut b)
                                .unwrap();
                        }
                        let ComponentFactorCompletion::Complete(f) = f.try_complete().unwrap()
                        else {
                            panic!()
                        };
                        for (path, node) in f.nodes() {
                            let Some(leaf) = node.leaf() else { continue };
                            let c = produce_embedded_monomial_center(
                                presentation.clone(),
                                leaf.clone(),
                                &mut b,
                            )
                            .unwrap();
                            let EmbeddedCenterProduction::Center(c) = c else {
                                println!("GENERIC support{si} path{path:?} noncenter");
                                continue;
                            };
                            let checked = CheckedRecursiveCenter::new(
                                RecursiveCenterOrigin::EmbeddedMonomial(c),
                                &mut b,
                            )
                            .unwrap();
                            let adapted = adapt_recursive_center(
                                checked,
                                &format!("generic_fourth_adapt{si}"),
                                &mut b,
                            )
                            .unwrap();
                            let maps = blowup_recursive_center(
                                adapted,
                                &format!("generic_fourth_maps{si}"),
                                &mut b,
                            )
                            .unwrap();
                            println!("GENERIC fourth maps{}", maps.charts().len());
                            let mut frontier = PhysicalChartFrontier::from_embedded(
                                state3.clone(),
                                presentation.clone(),
                                maps.clone(),
                                format!("generic_fourth{si}"),
                                &mut b,
                            )
                            .unwrap();
                            assert!(!frontier.complete_inventory());
                            let mut zero = Budget::new(Limits {
                                max_operations: 0,
                                ..Limits::default()
                            });
                            assert!(matches!(
                                frontier
                                    .advance(0, &ComponentFactorLimits::default(), &mut zero)
                                    .unwrap(),
                                PhysicalChartAdvance::Incomplete { .. }
                            ));
                            assert!(matches!(frontier.work()[0], PhysicalChartWork::Pending));
                            for mi in (0..maps.charts().len()).rev() {
                                for _ in 0..100 {
                                    if matches!(
                                        frontier
                                            .advance(mi, &ComponentFactorLimits::default(), &mut b)
                                            .unwrap(),
                                        PhysicalChartAdvance::Complete
                                    ) {
                                        break;
                                    }
                                }
                                let PhysicalChartWork::Complete(t) = &frontier.work()[mi] else {
                                    panic!("pending fourth chart")
                                };
                                let state4 = t.state();
                                assert_eq!(state4.depth(), 4);
                                assert_eq!(state4.retired_count(), 1);
                                assert!(Arc::ptr_eq(state3.root(), state4.root()));
                                assert!(Arc::ptr_eq(
                                    state3.retired().unwrap(),
                                    state4.retired().unwrap()
                                ));
                                assert!(t.all_local_factors_snc());
                                println!(
                                    "GENERIC depth4 map{mi} original_terminal{}",
                                    t.all_local_factors_snc()
                                );
                                let mut zero = Budget::new(Limits {
                                    max_operations: 0,
                                    ..Limits::default()
                                });
                                assert_eq!(
                                    frontier
                                        .advance(mi, &ComponentFactorLimits::default(), &mut zero)
                                        .unwrap(),
                                    PhysicalChartAdvance::AlreadyComplete
                                );
                            }
                            assert!(frontier.complete_inventory());
                            assert!(frontier.all_local_factors_snc());
                        }
                    }
                } else {
                    assert!(terminal.all_local_factors_snc());
                }
                count += 1;
            }
            assert_eq!(count, next.charts().len());
            return;
        }
    }
    panic!("no actual non-Cartier next ambient center");
}

mod coefficients;
mod retirement;

mod initial;
