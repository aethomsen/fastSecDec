use super::*;
#[path = "fixture.rs"]
mod fixture;
use fixture::*;
#[test]
fn physical_auxiliary_actual_frontier_terminal_first_and_deeper_cycle() {
    for py in [6, 9] {
        let (problem, maps, mut front) = initial(py);
        let mut plain = PhysicalChartFrontier::from_problem(
            problem,
            maps.clone(),
            format!("aux_default{py}"),
            &mut budget(),
        )
        .unwrap();
        assert!(front.auxiliary_work().is_none());
        assert!(!front.auxiliary_accounting_complete());
        front.enable_initial_auxiliary(&mut budget()).unwrap();
        assert_eq!(
            front.advance_auxiliary(0, &mut budget()).unwrap(),
            PhysicalAuxiliaryAdvance::PhysicalPending
        );
        finish_physical(&mut plain);
        finish_physical(&mut front);
        assert!(plain.auxiliary_work().is_none());
        for i in 0..maps.charts().len() {
            let a = record(&plain, i);
            let z = record(&front, i);
            assert_eq!(a.all_local_factors_snc(), z.all_local_factors_snc());
            assert_eq!(
                a.state().original_coordinate_jacobian(),
                z.state().original_coordinate_jacobian()
            );
            assert_eq!(a.state().generators().len(), z.state().generators().len());
            for (a, z) in a.state().generators().iter().zip(z.state().generators()) {
                assert_eq!(a.total(), z.total());
                assert_eq!(a.current_quotient(), z.current_quotient());
            }
        }
        assert!(!front.all_local_factors_snc());
        finish_aux(&mut front, false);
        assert_eq!(
            front
                .auxiliary_work()
                .unwrap()
                .iter()
                .filter(|w| matches!(w, PhysicalAuxiliaryWork::PhysicalTerminal(_)))
                .count(),
            3
        );
        let index = front
            .work()
            .iter()
            .position(|w| matches!(w,PhysicalChartWork::Complete(t)if !t.all_local_factors_snc()))
            .unwrap();
        let mut inventory = front.ready_auxiliary_inventory(index).unwrap().clone();
        assert!(inventory.complete_accounting());
        assert_eq!(inventory.active_sources(), 1);
        assert!(inventory.first_tree().is_some());
        assert_eq!(
            front
                .advance(index, &ComponentFactorLimits::default(), &mut budget())
                .unwrap(),
            PhysicalChartAdvance::AlreadyComplete
        );
        let mut reached_all_snc = false;
        for depth in 0..2 {
            let state = inventory.state().clone();
            let (presentation, maps) =
                next_center(&state, &format!("aux_next{py}_{depth}"), &mut budget()).unwrap();
            let mut next = PhysicalChartFrontier::from_embedded(
                state,
                presentation,
                maps,
                format!("aux_follow{py}_{depth}"),
                &mut budget(),
            )
            .unwrap();
            next.enable_carried_auxiliary(inventory.clone(), &mut budget())
                .unwrap();
            finish_physical(&mut next);
            finish_aux(&mut next, true);
            let terminal = next
                .auxiliary_work()
                .unwrap()
                .iter()
                .filter(|w| matches!(w, PhysicalAuxiliaryWork::PhysicalTerminal(_)))
                .count();
            assert_eq!(next.work().len(), 6);
            assert_eq!(terminal, if py == 9 && depth == 0 { 5 } else { 6 });
            for w in next.auxiliary_work().unwrap() {
                if let PhysicalAuxiliaryWork::PhysicalTerminal(t) = w {
                    assert!(t.terminal().all_local_factors_snc());
                    assert!(Arc::ptr_eq(t.prior_inventory().unwrap(), &inventory));
                    assert!(t.original_tree().is_none());
                }
            }
            if next.all_local_factors_snc() {
                reached_all_snc = true;
                break;
            }
            let index = next
                .work()
                .iter()
                .position(
                    |w| matches!(w,PhysicalChartWork::Complete(t)if !t.all_local_factors_snc()),
                )
                .unwrap();
            let carried = next.ready_auxiliary_inventory(index).unwrap().clone();
            assert!(Arc::ptr_eq(carried.prior().unwrap(), &inventory));
            assert_eq!(carried.active_sources(), 1);
            assert!(
                carried
                    .slots()
                    .iter()
                    .any(|s| matches!(s.kind(),PhysicalPowerSlotKind::Active(p)if p.depth()==1))
            );
            inventory = carried;
        }
        assert!(reached_all_snc, "full local chart inventory must reach SNC");
    }
}
#[test]
fn physical_auxiliary_complete_inventory_and_retry_fences() {
    let (_, _, mut front) = initial(9);
    front.enable_initial_auxiliary(&mut budget()).unwrap();
    finish_physical(&mut front);
    let index = front
        .work()
        .iter()
        .position(|w| matches!(w,PhysicalChartWork::Complete(t)if !t.all_local_factors_snc()))
        .unwrap();
    let mut limited = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        front.advance_auxiliary(index, &mut limited).unwrap(),
        PhysicalAuxiliaryAdvance::Incomplete { .. }
    ));
    assert!(matches!(
        front.auxiliary_work().unwrap()[index],
        PhysicalAuxiliaryWork::Pending
    ));
    assert_eq!(
        front.advance_auxiliary(index, &mut budget()).unwrap(),
        PhysicalAuxiliaryAdvance::Progress
    );
    let PhysicalAuxiliaryWork::Preparing(p) = &front.auxiliary_work().unwrap()[index] else {
        panic!()
    };
    let old = p.clone();
    let mut limited = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        front.advance_auxiliary(index, &mut limited).unwrap(),
        PhysicalAuxiliaryAdvance::Incomplete { .. }
    ));
    assert!(
        matches!(&front.auxiliary_work().unwrap()[index],PhysicalAuxiliaryWork::Preparing(p)if Arc::ptr_eq(p,&old))
    );
    finish_aux(&mut front, true);
    let inv = front.ready_auxiliary_inventory(index).unwrap().clone();
    let mut omitted = inv.as_ref().clone();
    omitted.slots.pop();
    assert!(!omitted.complete_accounting());
    let mut repeated = inv.as_ref().clone();
    assert!(repeated.slots.len() > 1);
    repeated.slots[1] = repeated.slots[0].clone();
    assert!(!repeated.complete_accounting());
    let state = inv.state().clone();
    let (presentation, maps) = next_center(&state, "aux_mutation_next", &mut budget()).unwrap();
    let mut next = PhysicalChartFrontier::from_embedded(
        state,
        presentation,
        maps,
        "aux_mutation_follow".into(),
        &mut budget(),
    )
    .unwrap();
    assert!(matches!(
        next.enable_carried_auxiliary(Arc::new(omitted), &mut budget()),
        Err(Error::Invalid(_))
    ));
    assert!(next.auxiliary_work().is_none());
    assert!(matches!(
        next.enable_initial_auxiliary(&mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut swapped = inv.as_ref().clone();
    let foreign = front
        .work()
        .iter()
        .enumerate()
        .find(|(i, _)| *i != index)
        .unwrap()
        .0;
    swapped.state = record(&front, foreign).state().clone();
    assert!(matches!(
        next.enable_carried_auxiliary(Arc::new(swapped), &mut budget()),
        Err(Error::Invalid(_))
    ));
    next.enable_carried_auxiliary(inv, &mut budget()).unwrap();
    assert!(matches!(
        next.enable_initial_auxiliary(&mut budget()),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn physical_auxiliary_outer_reset_retires_exact_prior_without_reactivation() {
    let (_, _, mut front) = initial(5);
    front.enable_initial_auxiliary(&mut budget()).unwrap();
    finish_physical(&mut front);
    finish_aux(&mut front, false);
    let i = front
        .work()
        .iter()
        .position(|w| matches!(w,PhysicalChartWork::Complete(t)if !t.all_local_factors_snc()))
        .unwrap();
    let prior = front.ready_auxiliary_inventory(i).unwrap().clone();
    assert_eq!(prior.active_sources(), 0);
    assert!(
        prior
            .slots()
            .iter()
            .any(|s| matches!(s.kind(), PhysicalPowerSlotKind::SourceCompleted(_)))
    );
    let state = prior.state().clone();
    let mut b = budget();
    let drop = PhysicalDropInventory::prove(
        state.clone(),
        record(&front, i).factor_inventory().clone(),
        "aux_reset_drop",
        &mut b,
    )
    .unwrap();
    let ri = drop
        .records()
        .iter()
        .position(|r| matches!(r, AmbientDropRecord::Positive(_)))
        .unwrap();
    let drop = drop.positive_drop(ri).unwrap();
    let cycle = CycleSnapshot::after_physical_drop(drop.clone(), &mut b).unwrap();
    let center = reset_center(
        drop.current().clone(),
        Some(cycle),
        "aux_reset_center",
        &mut b,
    );
    let owner = PhysicalContinuationCenter::prepare(drop, center.clone(), &mut b).unwrap();
    let start = state.begin_cycle(owner, &mut b).unwrap();
    let maps = reset_maps(center, "aux_reset_maps", &mut b);
    let mut next =
        PhysicalChartFrontier::from_start(start.clone(), maps, "aux_reset_front".into(), &mut b)
            .unwrap();
    assert!(matches!(
        next.enable_carried_auxiliary(prior.clone(), &mut b),
        Err(Error::Invalid(_))
    ));
    let mut wrong = prior.as_ref().clone();
    wrong.state = record(&front, (i + 1) % front.work().len()).state().clone();
    assert!(matches!(
        next.enable_restarted_auxiliary(Arc::new(wrong), &mut b),
        Err(Error::Invalid(_))
    ));
    assert!(next.auxiliary_work().is_none());
    next.enable_restarted_auxiliary(prior.clone(), &mut b)
        .unwrap();
    finish_physical(&mut next);
    finish_aux(&mut next, true);
    assert_eq!(next.work().len(), 6);
    let mut terminals = 0;
    let mut ready = 0;
    for w in next.auxiliary_work().unwrap() {
        let retired = match w {
            PhysicalAuxiliaryWork::PhysicalTerminal(t) => {
                terminals += 1;
                assert!(t.original_tree().is_some());
                t.retirement().unwrap()
            }
            PhysicalAuxiliaryWork::Ready(r) => {
                ready += 1;
                assert!(r.first_tree().is_some());
                assert!(r.prior().is_none());
                assert!(r.complete_accounting());
                r.retired().unwrap()
            }
            _ => panic!("unexpected unaccounted reset"),
        };
        assert!(Arc::ptr_eq(retired.source(), &prior));
        assert!(Arc::ptr_eq(retired.start(), &start));
        assert!(Arc::ptr_eq(retired.start().source(), prior.state()));
    }
    assert_eq!((terminals, ready), (5, 1));
    assert!(prior.complete_accounting());
    assert_eq!(prior.active_sources(), 0);
}
