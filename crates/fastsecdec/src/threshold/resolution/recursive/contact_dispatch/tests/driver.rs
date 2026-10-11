use super::*;
#[test]
fn contact_dispatch_driver_complete_original_cover() {
    for reverse in [false, true] {
        let mut b = Budget::new(Limits::default());
        let mut driver = live_order(&mut b, reverse);
        for _ in 0..10 {
            if matches!(driver.advance(&mut b).unwrap(), RecursiveAdvance::Complete) {
                break;
            }
        }
        let mut dispatch = driver
            .finite_contact_dispatcher(format!("dispatch_whole_{reverse}"), &mut b)
            .unwrap()
            .unwrap();
        assert!(!dispatch.complete_inventory());
        let mut ai = (0..dispatch.ambient().len()).collect::<Vec<_>>();
        if reverse {
            ai.reverse();
        }
        for i in ai {
            assert!(matches!(
                dispatch.advance_ambient(i, &mut b).unwrap(),
                ContactDispatchAdvance::Progress
            ));
        }
        let mut bi = (0..dispatch.branches().len()).collect::<Vec<_>>();
        if reverse {
            bi.reverse();
        }
        let mut centers = Vec::new();
        for i in bi {
            assert!(matches!(
                dispatch.advance_branch(i, &mut b).unwrap(),
                ContactDispatchAdvance::Progress
            ));
            let count = match &dispatch.branches()[i] {
                ContactBranchWork::Components(f) => f.work().len(),
                ContactBranchWork::Empty(_) => continue,
                _ => panic!("branch pending"),
            };
            let mut oi = (0..count).collect::<Vec<_>>();
            if reverse {
                oi.reverse();
            }
            for j in oi {
                assert!(matches!(
                    dispatch.advance_component(i, j, &mut b).unwrap(),
                    ContactDispatchAdvance::Progress
                ));
            }
            let ContactBranchWork::Components(f) = &dispatch.branches()[i] else {
                panic!()
            };
            assert!(f.complete_inventory());
            for w in f.work() {
                if let ContactComponentWork::Center(c) = w {
                    centers.push(c.clone());
                }
            }
        }
        assert!(dispatch.complete_inventory());
        assert_eq!(dispatch.ambient().len(), 9);
        assert_eq!(centers.len(), 1);
        let c = &centers[0];
        let checked = CheckedRecursiveCenter::new(
            RecursiveCenterOrigin::DispatchedContact(c.clone()),
            &mut b,
        )
        .unwrap();
        assert!(Arc::ptr_eq(checked.history(), c.history()));
    }
}

#[test]
#[ignore = "bounded native all-pivot diagnostic: prior600s cap incomplete"]
fn contact_dispatch_native_pivots_bounded() {
    for reverse in [false, true] {
        let mut b = Budget::new(Limits::default());
        let mut driver = live_order(&mut b, reverse);
        for _ in 0..10 {
            if matches!(driver.advance(&mut b).unwrap(), RecursiveAdvance::Complete) {
                break;
            }
        }
        let mut dispatch = driver
            .finite_contact_dispatcher(format!("dispatch_whole_{reverse}"), &mut b)
            .unwrap()
            .unwrap();
        assert!(!dispatch.complete_inventory());
        let mut ai = (0..dispatch.ambient().len()).collect::<Vec<_>>();
        if reverse {
            ai.reverse();
        }
        for i in ai {
            assert!(matches!(
                dispatch.advance_ambient(i, &mut b).unwrap(),
                ContactDispatchAdvance::Progress
            ));
        }
        let mut bi = (0..dispatch.branches().len()).collect::<Vec<_>>();
        if reverse {
            bi.reverse();
        }
        let mut centers = Vec::new();
        for i in bi {
            assert!(matches!(
                dispatch.advance_branch(i, &mut b).unwrap(),
                ContactDispatchAdvance::Progress
            ));
            let count = match &dispatch.branches()[i] {
                ContactBranchWork::Components(f) => f.work().len(),
                ContactBranchWork::Empty(_) => continue,
                _ => panic!("branch pending"),
            };
            let mut oi = (0..count).collect::<Vec<_>>();
            if reverse {
                oi.reverse();
            }
            for j in oi {
                assert!(matches!(
                    dispatch.advance_component(i, j, &mut b).unwrap(),
                    ContactDispatchAdvance::Progress
                ));
            }
            let ContactBranchWork::Components(f) = &dispatch.branches()[i] else {
                panic!()
            };
            assert!(f.complete_inventory());
            for w in f.work() {
                if let ContactComponentWork::Center(c) = w {
                    centers.push(c.clone());
                }
            }
        }
        assert!(dispatch.complete_inventory());
        assert_eq!(dispatch.ambient().len(), 9);
        assert_eq!(centers.len(), 1);
        let c = &centers[0];
        let checked = CheckedRecursiveCenter::new(
            RecursiveCenterOrigin::DispatchedContact(c.clone()),
            &mut b,
        )
        .unwrap();
        eprintln!("DISPATCH_ADAPT_BEGIN reverse={reverse}");
        let adapted =
            adapt_recursive_center(checked, &format!("dispatch_native_adapt_{reverse}"), &mut b)
                .unwrap();
        assert!(!adapted.opens().is_empty());
        let expected = adapted
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
        eprintln!(
            "DISPATCH_ADAPT_COMPLETE opens={} expected_pivots={expected}",
            adapted.opens().len()
        );
        let maps = blowup_recursive_center(
            adapted,
            &format!("dispatch_native_blowup_{reverse}"),
            &mut b,
        )
        .unwrap();
        eprintln!("DISPATCH_PIVOTS_COMPLETE count={}", maps.charts().len());
        assert_eq!(maps.charts().len(), expected);
        assert!(expected >= 2);
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
                        .zero(
                            &(&row.total - &b.mul(&factor, &row.quotient).unwrap()),
                            &mut b
                        )
                        .unwrap()
                );
            }
        }
    }
}

#[test]
fn contact_dispatch_missing_winning_open_and_budget_do_not_complete() {
    let mut b = Budget::new(Limits::default());
    let mut source = live(&mut b);
    for _ in 0..10 {
        if matches!(source.advance(&mut b).unwrap(), RecursiveAdvance::Complete) {
            break;
        }
    }
    let origin = source.contact_dispatch_origin(&mut b).unwrap().unwrap();
    let opens = origin.cover().cover().opens()[..3].to_vec();
    assert!(matches!(
        OpenCoverCertificate {
            algebra: origin.cover().frame().local().clone(),
            support: origin.cover().cover().support().clone(),
            opens
        }
        .verify(&mut b),
        Err(Error::Invalid(_))
    ));
    let mut d = source
        .finite_contact_dispatcher("dispatch_refuse_inventory".into(), &mut b)
        .unwrap()
        .unwrap();
    let mut none = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        d.advance_branch(3, &mut none).unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    assert!(matches!(d.branches()[3], ContactBranchWork::Pending));
    assert!(!d.complete_inventory());
    for i in 0..d.ambient().len() {
        assert!(matches!(
            d.advance_ambient(i, &mut b).unwrap(),
            ContactDispatchAdvance::Progress
        ));
    }
    for i in 0..3 {
        assert!(matches!(
            d.advance_branch(i, &mut b).unwrap(),
            ContactDispatchAdvance::Progress
        ));
        let n = match &d.branches()[i] {
            ContactBranchWork::Components(f) => f.work().len(),
            ContactBranchWork::Empty(_) => 0,
            _ => panic!(),
        };
        for j in 0..n {
            assert!(matches!(
                d.advance_component(i, j, &mut b).unwrap(),
                ContactDispatchAdvance::Progress
            ));
        }
    }
    assert!(!d.complete_inventory());
    assert!(matches!(d.branches()[3], ContactBranchWork::Pending));
    assert!(matches!(
        d.advance_component(3, 0, &mut b),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        d.advance_branch(3, &mut b).unwrap(),
        ContactDispatchAdvance::Progress
    ));
    let ContactBranchWork::Components(f) = &d.branches()[3] else {
        panic!()
    };
    let n = f.work().len();
    for j in 0..n {
        assert!(matches!(
            d.advance_component(3, j, &mut b).unwrap(),
            ContactDispatchAdvance::Progress
        ));
    }
    assert!(d.complete_inventory());
}
