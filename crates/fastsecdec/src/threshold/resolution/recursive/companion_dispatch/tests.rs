use super::*;
use symbolica::symbol;
fn live(old: bool, b: &mut Budget) -> (CompanionFirstCenter, Arc<ComponentResidualOrder>) {
    live_boundary(old.then_some(0), b)
}
fn live_boundary(
    boundary: Option<usize>,
    b: &mut Budget,
) -> (CompanionFirstCenter, Arc<ComponentResidualOrder>) {
    let ring = Arc::new(
        Ring::new(
            vec![
                symbol!("outer_test::z"),
                symbol!("outer_test::x"),
                symbol!("outer_test::y"),
            ],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(ring.clone(), vec![], b).unwrap(),
        vec![0, 1, 2],
        vec![],
        b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            free_axes: vec![0, 1, 2],
            determinant_inverse_axis: None,
        }
        .verify(b)
        .unwrap(),
    );
    let z = ring.coordinate(0).unwrap();
    let x = ring.coordinate(1).unwrap();
    let y = ring.coordinate(2).unwrap();
    let divisors = boundary.map_or_else(Vec::new, |axis| {
        vec![InitialDivisor {
            id: BoundaryId(1),
            equation: ring.coordinate(axis).unwrap(),
        }]
    });
    let SncProduction::Verified(ledger) = verify_initial_relative_snc(frame, divisors, b).unwrap()
    else {
        panic!("SNC")
    };
    let history = ResolutionHistory::initial(ledger).unwrap();
    let p = &x.pow(2) + &(&y.pow(3) * &(&y - &ring.one()).pow(4));
    let f = &z.pow(2) + &p.pow(2);
    let source = Arc::new(MarkedIdeal::new(Ideal::new(ring, vec![f], b).unwrap(), 1, b).unwrap());
    let mut factors =
        ComponentFactorFrontier::new(history, source, "outer_test_factor".into(), b).unwrap();
    for _ in 0..32 {
        let Some(path) = factors.pending().next().cloned() else {
            break;
        };
        factors
            .advance(&path, &ComponentFactorLimits::default(), b)
            .unwrap();
    }
    let ComponentFactorCompletion::Complete(factors) = factors.try_complete().unwrap() else {
        panic!("factor incomplete")
    };
    for leaf in factors.nodes().values().filter_map(|n| n.leaf()) {
        let ComponentResidualProduction::Order(order) =
            produce_component_residual_order(leaf.clone(), b).unwrap()
        else {
            panic!("order incomplete")
        };
        let order = Arc::new(*order);
        assert_eq!(order.algebraic_maximum_on_cosupport(), 2);
        assert_eq!(order.upper_order_cover().opens().len(), 8);
        for oi in 0..order.upper_order_cover().opens().len() {
            let CompanionOpenProduction::Contact(open) =
                produce_companion_open(order.clone(), oi, &format!("outer_test_open{oi}"), b)
                    .unwrap()
            else {
                continue;
            };
            let open = Arc::new(*open);
            for ci in 0..open.contact().candidates().len() {
                let local = open.contact().frame().local();
                let d = &open.contact().candidates()[ci].differential;
                let unit = OpenCoverCertificate {
                    algebra: local.clone(),
                    support: Ideal::new(local.ring().clone(), vec![], b).unwrap(),
                    opens: vec![d.clone()],
                }
                .verify(b);
                if matches!(unit, Err(Error::Invalid(_))) {
                    continue;
                }
                unit.unwrap();
                let CompanionCoefficientProduction::Coefficient(q) =
                    construct_companion_coefficient(
                        open.clone(),
                        ci,
                        [
                            symbol!(format!("outer_test_q{oi}_{ci}")),
                            symbol!(format!("outer_test_i{oi}_{ci}")),
                        ],
                        b,
                    )
                    .unwrap()
                else {
                    panic!("contact")
                };
                let q = Arc::new(*q);
                assert_eq!(
                    q.coefficient().algebraic_maximum_old_count(),
                    usize::from(boundary.is_some())
                );
                let mut child =
                    CompanionFirstCenter::new_with_principal_powers(q, "outer_test_child", false)
                        .unwrap();
                for _ in 0..16 {
                    match child.advance(b).unwrap() {
                        RecursiveAdvance::Progress => {}
                        RecursiveAdvance::Complete => break,
                        RecursiveAdvance::Incomplete { reason } => panic!("child {reason}"),
                    }
                }
                assert!(matches!(
                    child.outcome(),
                    CompanionCenterOutcome::ChildNeedsMoreOpens(
                        RecursiveOutcome::MultipleContactOpensRequired { .. }
                    )
                ));
                return (child, order);
            }
        }
    }
    panic!("no actual source opening")
}
fn indices(n: usize, reverse: bool) -> Vec<usize> {
    let mut v = (0..n).collect::<Vec<_>>();
    if reverse {
        v.reverse();
    }
    v
}
fn lower(d: &mut CompanionContactDispatcher, reverse: bool, b: &mut Budget) {
    for i in indices(d.child().ambient().len(), reverse) {
        assert!(!matches!(
            d.advance_child_ambient(i, b).unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
    }
    for i in indices(d.child().branches().len(), reverse) {
        assert!(!matches!(
            d.advance_child_branch(i, b).unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
        let n = match &d.child().branches()[i] {
            ContactBranchWork::Components(f) => f.work().len(),
            ContactBranchWork::Empty(_) => continue,
            _ => panic!("pending"),
        };
        for j in indices(n, reverse) {
            assert!(!matches!(
                d.advance_child_component(i, j, b).unwrap(),
                ContactDispatchAdvance::Incomplete { .. }
            ));
        }
    }
    assert!(d.child().complete_inventory());
}
fn complete(d: &mut CompanionContactDispatcher, reverse: bool, b: &mut Budget) {
    for i in indices(d.outer().len(), reverse) {
        assert!(!matches!(
            d.advance_outer(i, b).unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
    }
    lower(d, reverse, b);
    for i in indices(d.child().branches().len(), reverse) {
        assert!(!matches!(
            d.prepare_outer_branch(i, b).unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
        let Some(f) = d.branches()[i].as_ref() else {
            continue;
        };
        let n = f.work().len();
        for j in indices(n, reverse) {
            assert!(!matches!(
                d.advance_outer_component(i, j, b).unwrap(),
                ContactDispatchAdvance::Incomplete { .. }
            ));
        }
    }
    assert!(d.complete_inventory());
    for work in d.branches().iter().flatten().flat_map(|f| f.work()) {
        assert!(!matches!(work, CompanionOuterWork::Pending));
        if !matches!(
            work,
            CompanionOuterWork::Center(_) | CompanionOuterWork::EmptyPrincipal(_)
        ) {
            assert!(
                work.is_deferred_physical_work(),
                "auxiliary emptiness never drops ambient density"
            );
        }
    }
}
fn centers(d: &CompanionContactDispatcher) -> Vec<Arc<CompanionDispatchedCenter>> {
    d.branches()
        .iter()
        .flatten()
        .flat_map(|f| f.work())
        .filter_map(|w| {
            if let CompanionOuterWork::Center(c) = w {
                Some(c.clone())
            } else {
                None
            }
        })
        .collect()
}
fn pivots(c: &Arc<CompanionDispatchedCenter>, expected: usize, b: &mut Budget) {
    assert_eq!(c.source().mark(), 1);
    assert_eq!(c.companion().mark(), 2);
    assert_eq!(c.orders()[0].source().mark(), 1);
    assert_eq!(c.orders()[1].source().mark(), 2);
    let checked =
        CheckedRecursiveCenter::new(RecursiveCenterOrigin::DispatchedCompanion(c.clone()), b)
            .unwrap()
            .with_simplified_normals(b)
            .unwrap();
    let adapted = adapt_recursive_center(checked, "outer_test_adapt", b).unwrap();
    let inventory = adapted
        .opens()
        .iter()
        .map(|o| {
            if o.identity_side().is_some() {
                1
            } else {
                o.center_axes().len()
            }
        })
        .sum::<usize>();
    assert_eq!(inventory, expected);
    let maps = blowup_recursive_center(adapted, "outer_test_blowup", b).unwrap();
    assert_eq!(maps.charts().len(), expected);
    for map in maps.charts() {
        assert!(map.history().same_root(c.history()));
        assert!(Arc::ptr_eq(map.parent().source(), c.source()));
        let local = map.geometry().frame().local();
        let factor = map
            .geometry()
            .exceptional()
            .map_or_else(|| Ok(local.ring().one()), |e| b.power(e, c.source().mark()))
            .unwrap();
        for row in map.parent().generators() {
            assert!(
                local
                    .zero(&(&row.total - &b.mul(&factor, &row.quotient).unwrap()), b)
                    .unwrap()
            );
        }
        // The ACTIVE companion has its distinct mark on these same maps.
        // Reuse native localized Cartier division; never divide by I's mark.
        for (i, f) in c.companion().ideal().generators().iter().enumerate() {
            let total = map.geometry().pull(f, b).unwrap();
            let mut quotient = total.clone();
            if let Some(exceptional) = map.geometry().exceptional() {
                for j in 0..c.companion().mark() {
                    quotient = super::super::super::companion::divide_regular_in_localization(
                        local,
                        exceptional,
                        &quotient,
                        &format!("outer_active_g_{i}_{j}"),
                        b,
                    )
                    .unwrap()
                    .0;
                }
                let factor = b.power(exceptional, c.companion().mark()).unwrap();
                assert!(
                    local
                        .zero(&(&total - &b.mul(&factor, &quotient).unwrap()), b)
                        .unwrap()
                );
            }
        }
    }
}
#[test]
fn companion_dispatch_actual_outer_ascent_inventory_and_pivots() {
    let mut b = Budget::new(Limits::default());
    let (child, order) = live(false, &mut b);
    let mut d = child
        .finite_contact_dispatcher("outer_forward".into(), &mut b)
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(
        d.origin().coefficient().source().order(),
        &order
    ));
    assert_eq!(d.outer().len(), 15);
    assert_eq!(d.child().ambient().len(), 11);
    assert_eq!(d.child().branches().len(), 4);
    complete(&mut d, false, &mut b);
    let cs = centers(&d);
    assert_eq!(cs.len(), 1);
    assert_eq!(
        d.branches()
            .iter()
            .flatten()
            .flat_map(|f| f.work())
            .filter(|w| w.is_deferred_physical_work())
            .count(),
        24
    );
    pivots(&cs[0], 6, &mut b);
    // A second traversal of the same issued source keeps the ambient center.
    let mut reverse = child
        .finite_contact_dispatcher("outer_forward".into(), &mut b)
        .unwrap()
        .unwrap();
    complete(&mut reverse, true, &mut b);
    let other = centers(&reverse);
    assert_eq!(other.len(), 1);
    assert_eq!(
        cs[0].frame().local().ring(),
        other[0].frame().local().ring()
    );
    assert_eq!(
        cs[0].frame().local().ideal(),
        other[0].frame().local().ideal()
    );
    assert_eq!(
        cs[0].frame().local().guards(),
        other[0].frame().local().guards()
    );
    let local = cs[0].frame().local();
    for (source, target) in [
        (cs[0].ideal(), other[0].ideal()),
        (other[0].ideal(), cs[0].ideal()),
    ] {
        let total = local.ideal().sum(target, &mut b).unwrap();
        for f in source.generators() {
            assert!(total.contains(f, local.unit_relations(), &mut b).unwrap());
        }
    }
}
#[test]
fn companion_dispatch_old_physical_history_not_auxiliary_reset() {
    let mut b = Budget::new(Limits::default());
    let (child, _) = live(true, &mut b);
    let mut d = child
        .finite_contact_dispatcher("outer_old".into(), &mut b)
        .unwrap()
        .unwrap();
    assert_eq!(d.origin().history().old_snapshot(), &[BoundaryId(1)]);
    assert!(
        d.origin()
            .child()
            .auxiliary()
            .history()
            .ledger()
            .divisors()
            .is_empty()
    );
    assert!(
        !d.origin()
            .history()
            .same_root(d.origin().child().auxiliary().history())
    );
    complete(&mut d, false, &mut b);
    let cs = centers(&d);
    assert_eq!(cs.len(), 1);
    assert_eq!(cs[0].orders().len(), 3);
    assert!(cs[0].history().same_root(d.origin().history()));
    assert_eq!(cs[0].history().old_snapshot(), &[BoundaryId(1)]);
    // This z divisor vanishes on the selected z contact: it tests actual old
    // ancestry/transversality, not a nonzero restricted incidence ideal.
    let old = cs[0]
        .origin()
        .coefficient()
        .coefficient()
        .boundary_ideal()
        .unwrap();
    for f in old.ideal().generators() {
        assert!(
            cs[0]
                .origin()
                .coefficient()
                .contact()
                .contact()
                .local()
                .zero(f, &mut b)
                .unwrap()
        );
    }
    pivots(&cs[0], 9, &mut b);
}
#[test]
fn companion_dispatch_retry_omitted_cover_and_foreign_owners_refuse() {
    let mut b = Budget::new(Limits::default());
    let (child, _) = live(false, &mut b);
    let mut d = child
        .finite_contact_dispatcher("outer_refuse".into(), &mut b)
        .unwrap()
        .unwrap();
    lower(&mut d, false, &mut b);
    assert!(!d.complete_inventory());
    for i in 0..d.child().branches().len() {
        d.prepare_outer_branch(i, &mut b).unwrap();
    }
    let (branch, index) = d
        .branches()
        .iter()
        .enumerate()
        .filter_map(|(i, f)| f.as_ref().map(|f| (i, f)))
        .find_map(|(i, f)| {
            f.child_work()
                .iter()
                .position(|w| matches!(w, ContactComponentWork::Center(_)))
                .map(|j| (i, j))
        })
        .unwrap();
    let mut none = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        d.advance_outer_component(branch, index, &mut none).unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    assert!(matches!(
        d.branches()[branch].as_ref().unwrap().work()[index],
        CompanionOuterWork::Pending
    ));
    complete(&mut d, false, &mut b);
    let c = centers(&d).pop().unwrap();
    let mut none = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        d.advance_outer_component(branch, index, &mut none).unwrap(),
        ContactDispatchAdvance::Accepted
    ));
    let (other, _) = live(false, &mut b);
    let foreign = other
        .finite_contact_dispatcher("outer_foreign".into(), &mut b)
        .unwrap()
        .unwrap();
    assert!(matches!(
        CompanionDispatchedCenter::prepare(
            foreign.origin().clone(),
            c.lower().clone(),
            c.cover().clone(),
            c.localization().clone(),
            c.matched().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        c.cover().restrict_matched_lower(
            c.localization().clone(),
            c.localization().clone(),
            "wrong_lower",
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        CompanionDispatchOrigin::prepare(
            c.origin().coefficient().clone(),
            c.origin().original(),
            c.origin().child().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn companion_dispatch_nonzero_old_incidence_retains_mark_limit() {
    let mut b = Budget::new(Limits::default());
    let (child, _) = live_boundary(Some(1), &mut b);
    let mut d = child
        .finite_contact_dispatcher("outer_nonzero_old".into(), &mut b)
        .unwrap()
        .unwrap();
    let old = d
        .origin()
        .coefficient()
        .coefficient()
        .boundary_ideal()
        .unwrap();
    let local = d.origin().coefficient().contact().contact().local();
    assert!(
        old.ideal()
            .generators()
            .iter()
            .any(|p| !local.zero(p, &mut b).unwrap())
    );
    assert_eq!(d.origin().history().old_snapshot(), &[BoundaryId(1)]);
    assert_eq!(d.outer().len(), 25);
    assert_eq!(d.child().ambient().len(), 21);
    assert_eq!(d.child().branches().len(), 8);
    for i in 0..d.outer().len() {
        assert!(matches!(
            d.advance_outer(i, &mut b).unwrap(),
            ContactDispatchAdvance::Progress
        ));
    }
    for i in 0..d.child().ambient().len() {
        assert!(matches!(
            d.advance_child_ambient(i, &mut b).unwrap(),
            ContactDispatchAdvance::Progress
        ));
    }
    assert!(matches!(
        d.advance_child_branch(0, &mut b).unwrap(),
        ContactDispatchAdvance::Incomplete {
            reason: "coefficient mark LCM"
        }
    ));
    assert!(matches!(
        d.child().branches()[0],
        ContactBranchWork::Pending
    ));
    assert!(!d.complete_inventory());
    assert!(!d.child().complete_inventory());
    assert!(
        d.outer()
            .iter()
            .all(|v| matches!(v, ContactAmbientWork::Accounted(_)))
    );
    assert_eq!(d.origin().original().mark(), 1);
    assert_eq!(d.origin().companion().mark(), 2);
}
