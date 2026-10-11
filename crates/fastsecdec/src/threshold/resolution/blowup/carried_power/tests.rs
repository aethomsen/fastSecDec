use super::*;
#[path = "fixture.rs"]
mod fixture;
use fixture::*;
struct Case {
    state: Arc<OriginalPhysicalState>,
    presentation: Arc<EmbeddedPresentation>,
    powers: Vec<Arc<CarriedPowerState>>,
    maps: Arc<RelativeRecursiveBlowup>,
}
fn case() -> Case {
    let mut b = budget();
    let center = center(6, &mut b);
    let original = OriginalPhysicalProblem::new(center.clone(), &mut b).unwrap();
    let lower = original.root().owner().clone();
    let checked =
        CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(center), &mut b).unwrap();
    let adapted = adapt_recursive_center(checked, "carried_owner_first_adapt", &mut b).unwrap();
    let maps = blowup_recursive_center(adapted, "carried_owner_first_maps", &mut b).unwrap();
    for (i, chart) in maps.charts().iter().enumerate() {
        let tree = lower
            .original_tree()
            .pull_first(chart.clone(), &format!("carried_owner_tree{i}"), &mut b)
            .unwrap();
        let mut powers = Vec::new();
        for (li, level) in tree.levels().iter().enumerate() {
            for (oi, slot) in level.opens().iter().enumerate() {
                if slot.power().is_some() {
                    powers
                        .push(CarriedPowerState::from_first(tree.clone(), li, oi, &mut b).unwrap());
                }
            }
        }
        let carry =
            carry_companion_chart(chart.clone(), &format!("carried_owner_carry{i}"), &mut b)
                .unwrap();
        for (si, open) in carry.opens().iter().enumerate() {
            let induced = induce_child_chart(
                lower.clone(),
                carry.clone(),
                si,
                &format!("carried_owner_induce{i}_{si}"),
                &mut b,
            )
            .unwrap();
            let factors = complete_factors(
                induced.history().clone(),
                open.incidence_sum().target().clone(),
                &format!("carried_owner_factor{i}_{si}"),
                &mut b,
            );
            for leaf in factors.nodes().values().filter_map(|n| n.leaf()) {
                if let InducedCenterProduction::Center(c) =
                    produce_induced_monomial_center(induced.clone(), leaf.clone(), &mut b).unwrap()
                {
                    let checked = CheckedRecursiveCenter::new(
                        RecursiveCenterOrigin::CarriedMonomial(c),
                        &mut b,
                    )
                    .unwrap();
                    let adapted =
                        adapt_recursive_center(checked, "carried_owner_next_adapt", &mut b)
                            .unwrap();
                    return Case {
                        state: original.advance(chart.clone(), &mut b).unwrap(),
                        presentation: EmbeddedPresentation::from_induced(induced, &mut b).unwrap(),
                        powers,
                        maps: blowup_recursive_center(adapted, "carried_owner_next_maps", &mut b)
                            .unwrap(),
                    };
                }
            }
        }
    }
    panic!("actual surviving lower J missing");
}
#[test]
fn carried_power_actual_six_maps_keep_original_factors_and_completed_mates() {
    let case = case();
    assert_eq!(case.maps.charts().len(), 6);
    assert_eq!(case.powers.iter().filter(|p| p.completed()).count(), 1);
    assert_eq!(case.powers.iter().filter(|p| !p.completed()).count(), 1);
    let mut active = 0;
    let mut completed = 0;
    let mut empty = 0;
    let mut target_opens = 0;
    for (i, chart) in case.maps.charts().iter().enumerate() {
        for (j, state) in case.powers.iter().enumerate() {
            match state
                .advance(
                    chart.clone(),
                    &format!("carried_owner_step{i}_{j}"),
                    &mut budget(),
                )
                .unwrap()
            {
                CarriedPowerAdvance::SourceCompleted(owner) => {
                    assert!(Arc::ptr_eq(&owner, state));
                    assert!(owner.source_cosupport().original().empty());
                    assert!(owner.source_cosupport().effective().empty());
                    completed += 1;
                }
                CarriedPowerAdvance::Transformed(t) => {
                    assert!(t.complete_inventory());
                    active += 1;
                    empty += usize::from(t.support().saturation().empty());
                    target_opens += t.states().len();
                    for s in t.states() {
                        assert!(Arc::ptr_eq(s.initial(), state.initial()));
                        assert!(s.history().same_root(state.history()));
                        assert_eq!(s.depth(), 1);
                        assert!(s.companion().completed());
                        assert!(Arc::ptr_eq(
                            s.companion().proof(),
                            state.companion().proof()
                        ));
                        assert!(Arc::ptr_eq(s.prior().unwrap().0, state));
                        check_identity(
                            s.open().frame(),
                            s.normalized(),
                            s.effective(),
                            s.unit(),
                            s.power().unwrap(),
                            &mut budget(),
                        )
                        .unwrap();
                    }
                }
                CarriedPowerAdvance::Incomplete(p) => {
                    panic!("active actual fixture {}", p.reason())
                }
            }
        }
        let mut b = budget();
        let transition = EmbeddedTransition::prepare(
            case.presentation.clone(),
            chart.clone(),
            &format!("carried_owner_physical{i}"),
            &mut b,
        )
        .unwrap();
        let state = case
            .state
            .continue_embedded(transition, &format!("carried_owner_lineage{i}"), &mut b)
            .unwrap();
        let factors = complete_factors(
            chart.history().clone(),
            chart.parent().target().clone(),
            &format!("carried_owner_terminal_f{i}"),
            &mut b,
        );
        assert!(
            PhysicalTerminalInventory::prove(
                state,
                factors,
                &format!("carried_owner_terminal{i}"),
                &mut b
            )
            .unwrap()
            .all_local_factors_snc()
        );
    }
    assert_eq!((active, completed, empty, target_opens), (6, 6, 3, 8));
}
#[test]
fn carried_power_refuses_foreign_owner_reactivation_and_keeps_retry() {
    let c = case();
    let active = c.powers.iter().find(|s| !s.completed()).unwrap();
    let done = c.powers.iter().find(|s| s.completed()).unwrap();
    let chart = c.maps.charts()[1].clone();
    assert!(matches!(
        active.advance(
            c.state.chart().clone(),
            "carried_owner_wrong_chart",
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
    let mut reactivated = done.as_ref().clone();
    reactivated.source_cosupport = active.source_cosupport().clone();
    assert!(matches!(
        Arc::new(reactivated).advance(chart.clone(), "carried_owner_reactivate", &mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut limited = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    let CarriedPowerAdvance::Incomplete(pending) = active
        .advance(chart.clone(), "carried_owner_retry", &mut limited)
        .unwrap()
    else {
        panic!()
    };
    assert!(Arc::ptr_eq(pending.source(), active));
    assert!(Arc::ptr_eq(pending.chart(), &chart));
    let CarriedPowerAdvance::Transformed(t) = pending.retry(&mut budget()).unwrap() else {
        panic!()
    };
    assert!(t.complete_inventory());
    assert_eq!(active.depth(), 0);
    let mut wrong = active.as_ref().clone();
    wrong.unit = wrong.open.frame().local().ring().one() + wrong.unit.clone();
    assert!(matches!(
        Arc::new(wrong).advance(chart.clone(), "carried_owner_wrong_unit", &mut budget()),
        Err(Error::Invalid(_))
    ));
    let wrong = MarkedIdeal::new(
        Ideal::new(
            active.open.frame().local().ring().clone(),
            vec![active.open.frame().local().ring().one()],
            &mut budget(),
        )
        .unwrap(),
        active.effective.mark(),
        &budget(),
    )
    .unwrap();
    assert!(matches!(
        PowerCosupportProof::prove(
            active.open.frame().clone(),
            active.original.clone(),
            Arc::new(wrong),
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
    let (tree, li, oi) = active.first_slot().unwrap();
    assert!(matches!(
        CarriedPowerState::from_first(tree.clone(), li, oi + 100, &mut budget()),
        Err(Error::Invalid(_))
    ));
}
#[test]
fn carried_power_x3_y5_completes_original_and_effective_j_at_last_frame() {
    let mut b = budget();
    let center = center(5, &mut b);
    let origin = EmbeddedChildCycle::new(center.clone(), &mut b).unwrap();
    let checked =
        CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(center), &mut b).unwrap();
    let adapted = adapt_recursive_center(checked, "carried_owner_completed_adapt", &mut b).unwrap();
    let maps = blowup_recursive_center(adapted, "carried_owner_completed_maps", &mut b).unwrap();
    let mut count = 0;
    for (i, chart) in maps.charts().iter().enumerate() {
        let tree = origin
            .original_tree()
            .pull_first(
                chart.clone(),
                &format!("carried_owner_completed_tree{i}"),
                &mut b,
            )
            .unwrap();
        for (li, level) in tree.levels().iter().enumerate() {
            for (oi, slot) in level.opens().iter().enumerate() {
                if slot.power().is_some() {
                    let state =
                        CarriedPowerState::from_first(tree.clone(), li, oi, &mut b).unwrap();
                    assert!(state.completed());
                    assert!(state.companion().completed());
                    assert!(Arc::ptr_eq(
                        state.source_cosupport().frame(),
                        state.open().frame()
                    ));
                    assert!(Arc::ptr_eq(
                        state.source_cosupport().original().source(),
                        state.original()
                    ));
                    assert!(Arc::ptr_eq(
                        state.source_cosupport().effective().source(),
                        state.effective()
                    ));
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 3);
}

#[test]
fn carried_power_positive_polynomial_is_not_an_unproved_unit() {
    let c = case();
    let state = c.powers.iter().find(|s| !s.completed()).unwrap();
    let frame = state.open().frame();
    let ring = frame.local().ring();
    let x = ring.coordinate(frame.free_axes()[0]).unwrap();
    let unproved_unit = ring.one() + x.pow(2);
    assert!(!unit(frame.local(), &unproved_unit, &mut budget()).unwrap());
    let k = state.power().unwrap();
    let p = &unproved_unit * &principal(state.effective()).unwrap().pow(k);
    let p = MarkedIdeal::new(
        Ideal::new(ring.clone(), vec![p], &mut budget()).unwrap(),
        state.original().mark(),
        &budget(),
    )
    .unwrap();
    assert!(matches!(
        check_identity(
            frame,
            &p,
            state.effective(),
            &unproved_unit,
            k,
            &mut budget()
        ),
        Err(Error::Invalid("carried power unit"))
    ));
}

#[test]
fn carried_power_does_not_invent_parameter_family_recursion() {
    parametric_refusal(&mut budget());
}
