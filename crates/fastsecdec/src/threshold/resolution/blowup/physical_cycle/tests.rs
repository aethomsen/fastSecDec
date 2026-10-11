use super::super::super::*;
use super::tests_fixture::{budget, second};
use std::sync::Arc;
use symbolica::symbol;

pub(super) fn complete(
    chart: &Arc<RelativeRecursiveChart>,
    name: &str,
    b: &mut Budget,
) -> Arc<CompletedComponentFactors> {
    let mut f = ComponentFactorFrontier::new(
        chart.history().clone(),
        chart.parent().target().clone(),
        name.into(),
        b,
    )
    .unwrap();
    for _ in 0..100 {
        let Some(path) = f.pending().next().cloned() else {
            break;
        };
        assert!(matches!(
            f.advance(&path, &ComponentFactorLimits::default(), b)
                .unwrap(),
            FactorAdvance::Progress { .. }
        ));
    }
    let ComponentFactorCompletion::Complete(factors) = f.try_complete().unwrap() else {
        panic!("incomplete actual factor inventory");
    };
    Arc::new(factors)
}

#[test]
fn original_factor_inventory_and_actual_ambient_drop_preserve_all_nine_charts() {
    let mut b = budget();
    let (center, charts) = second(false, &mut b);
    assert_eq!(charts.charts().len(), 9);
    let mut terminals = Vec::new();
    let mut unresolved = Vec::new();
    let foreign = complete(&charts.charts()[0], "terminal_foreign", &mut b);
    for (i, chart) in charts.charts().iter().enumerate() {
        let carry = AmbientCycleCarry::prepare(
            center.clone(),
            chart.clone(),
            &format!("terminal_carry{i}"),
            &mut b,
        )
        .unwrap();
        assert_eq!(carry.generators().len(), 1);
        if i == 0 {
            // An equal polynomial presentation is not the original physical
            // source owner. Its independently issued factor inventory must
            // not be spliced into this map lineage.
            let alias = Arc::new((**chart.parent().target()).clone());
            let mut alien = ComponentFactorFrontier::new(
                chart.history().clone(),
                alias,
                "terminal_equal_source_alias".into(),
                &mut b,
            )
            .unwrap();
            for _ in 0..100 {
                let Some(path) = alien.pending().next().cloned() else {
                    break;
                };
                alien
                    .advance(&path, &ComponentFactorLimits::default(), &mut b)
                    .unwrap();
            }
            let ComponentFactorCompletion::Complete(alien) = alien.try_complete().unwrap() else {
                panic!();
            };
            let alien = Arc::new(alien);
            assert!(matches!(
                OriginalTerminalInventory::prove(
                    carry.clone(),
                    alien.clone(),
                    "terminal_source_alias",
                    &mut b
                ),
                Err(Error::Invalid(_))
            ));
            assert!(matches!(
                AmbientDropInventory::prove(carry.clone(), alien, &mut b),
                Err(Error::Invalid(_))
            ));
        }
        assert!(carry.companions().iter().all(|g| g.cosupport().empty()));
        let factors = complete(chart, &format!("terminal_factors{i}"), &mut b);
        if i != 0 {
            assert!(matches!(
                OriginalTerminalInventory::prove(
                    carry.clone(),
                    foreign.clone(),
                    "wrong_terminal_factors",
                    &mut b
                ),
                Err(Error::Invalid(_))
            ));
            assert!(matches!(
                AmbientDropInventory::prove(carry.clone(), foreign.clone(), &mut b),
                Err(Error::Invalid(_))
            ));
        }
        let mut exhausted = Budget::new(Limits {
            max_operations: 0,
            ..Limits::default()
        });
        assert!(matches!(
            OriginalTerminalInventory::prove(
                carry.clone(),
                factors.clone(),
                "terminal_resource",
                &mut exhausted
            ),
            Err(Error::ResourceIncomplete(_))
        ));
        let terminal = OriginalTerminalInventory::prove(
            carry.clone(),
            factors.clone(),
            &format!("terminal_owner{i}"),
            &mut b,
        )
        .unwrap();
        assert!(Arc::ptr_eq(terminal.carry(), &carry));
        assert!(Arc::ptr_eq(terminal.factor_inventory(), &factors));
        if terminal.all_local_factors_snc() {
            terminals.push(i);
            for record in terminal.records() {
                let OriginalTerminalRecord::Terminal { proof, .. } = record else {
                    panic!();
                };
                assert_eq!(proof.factors().len(), 1);
                assert_eq!(proof.factors()[0].source(), carry.generators()[0].source());
                assert_eq!(
                    proof.factors()[0].boundary_powers().len(),
                    proof
                        .principal()
                        .factor()
                        .data()
                        .history()
                        .ledger()
                        .divisors()
                        .len()
                );
            }
        } else {
            unresolved.push(i);
            assert!(
                terminal
                    .records()
                    .iter()
                    .all(|r| matches!(r, OriginalTerminalRecord::FurtherResolution { .. }))
            );
            let inventory = AmbientDropInventory::prove(carry.clone(), factors, &mut b).unwrap();
            assert_eq!(inventory.proved_maximum(), Some(1));
            assert!(!inventory.unresolved_zero_component());
            let drop = inventory
                .records()
                .iter()
                .enumerate()
                .find_map(|(j, r)| {
                    matches!(r, AmbientDropRecord::Positive(_))
                        .then(|| inventory.positive_drop(j).unwrap())
                })
                .unwrap();
            let cycle = CycleSnapshot::after_ambient_drop(drop.clone(), &mut b).unwrap();
            assert!(
                matches!(cycle.origin(), CycleOrigin::AfterAmbientDrop(d) if Arc::ptr_eq(d,&drop))
            );
            assert!(Arc::ptr_eq(cycle.birth_history(), chart.history()));
            assert_eq!(cycle.birth_history().births(), chart.history().births());
            assert_eq!(
                cycle.old_ids(),
                chart
                    .history()
                    .ledger()
                    .divisors()
                    .iter()
                    .map(|d| d.id)
                    .collect::<Vec<_>>()
            );
            assert!(matches!(
                inventory.positive_drop(inventory.records().len()),
                Err(Error::Invalid(_))
            ));
        }
    }
    assert_eq!(terminals, [0, 1, 2, 3, 5, 6, 8]);
    assert_eq!(unresolved, [4, 7]);
}

#[test]
fn principal_ideal_does_not_certify_each_original_factor() {
    let mut b = budget();
    let (center, charts) = second(true, &mut b);
    let mut refusals = Vec::new();
    for (i, chart) in charts.charts().iter().enumerate() {
        let carry = AmbientCycleCarry::prepare(
            center.clone(),
            chart.clone(),
            &format!("individual_carry{i}"),
            &mut b,
        )
        .unwrap();
        assert_eq!(carry.generators().len(), 2);
        let factors = complete(chart, &format!("individual_factors{i}"), &mut b);
        let terminal = OriginalTerminalInventory::prove(
            carry,
            factors,
            &format!("individual_terminal{i}"),
            &mut b,
        )
        .unwrap();
        if terminal.records().iter().any(|r| {
            matches!(
                r,
                OriginalTerminalRecord::IndividualFactorsUnresolved { .. }
            )
        }) {
            assert!(!terminal.all_local_factors_snc());
            refusals.push(i);
        }
    }
    assert_eq!(refusals, [3, 5, 6, 8]);
}

#[test]
fn actual_ambient_cycle_next_center_all_pivots_and_owner_mutations() {
    let mut b = budget();
    let (center, charts) = second(false, &mut b);
    let chart = charts.charts()[7].clone();
    let carry =
        AmbientCycleCarry::prepare(center, chart.clone(), "next_actual_carry", &mut b).unwrap();
    let factors = complete(&chart, "next_actual_factors", &mut b);
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
                count += 1;
            }
            assert_eq!(count, next.charts().len());
            return;
        }
    }
    panic!("no actual non-Cartier next ambient center");
}
