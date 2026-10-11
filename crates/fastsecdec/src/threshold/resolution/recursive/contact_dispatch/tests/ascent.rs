use super::*;
#[test]
fn contact_dispatch_all_branches_keep_two_levels_and_component_physical_cover() {
    let mut b = Budget::new(Limits::default());
    let mut driver = live(&mut b);
    for _ in 0..10 {
        if matches!(driver.advance(&mut b).unwrap(), RecursiveAdvance::Complete) {
            break;
        }
    }
    let origin = driver.contact_dispatch_origin(&mut b).unwrap().unwrap();
    let cover = ContactDispatchCover::prepare(origin.clone(), &mut b).unwrap();
    let mut counts = [0usize; 3];
    for ci in 0..4 {
        let branch = match ContactDispatchBranch::prepare(
            cover.clone(),
            ci,
            [
                symbol!(format!("dispatch_branch_{ci}::z")),
                symbol!(format!("dispatch_branch_{ci}::i")),
            ],
            &mut b,
        )
        .unwrap()
        {
            ContactBranchProduction::EmptyContact { owner, evidence } => {
                assert!(Arc::ptr_eq(&owner, &cover));
                assert!(matches!(
                    evidence,
                    ContactProduction::EmptyContactOpen { .. }
                ));
                counts[0] += 1;
                continue;
            }
            ContactBranchProduction::Incomplete { .. } => panic!("unexpected incomplete branch"),
            ContactBranchProduction::Branch(q) => q,
        };
        assert_eq!(branch.embedding().codimension(), 2);
        assert!(Arc::ptr_eq(
            branch.embedding().ambient(),
            origin.auxiliary().frame()
        ));
        let components = ContactTerminalComponents::prepare(
            branch.clone(),
            &format!("dispatch_components_{ci}"),
            &mut b,
        )
        .unwrap();
        assert_eq!(components.cover().full_cover().opens().len(), 3);
        assert!(Arc::ptr_eq(
            components.cover().source(),
            origin.auxiliary().source()
        ));
        let mut complete = 0;
        let mut zeros = 0;
        for oi in 0..3 {
            let open = match components
                .localize(oi, &format!("dispatch_original_{ci}_{oi}"), &mut b)
                .unwrap()
            {
                CoverLocalizationProduction::Complete(o) => Arc::new(*o),
                CoverLocalizationProduction::Empty { .. } => {
                    complete += 1;
                    continue;
                }
                CoverLocalizationProduction::Incomplete { reason, .. } => {
                    panic!("unexpected physical stop {reason}")
                }
            };
            assert!(open.history().same_root(origin.auxiliary().history()));
            assert_eq!(open.history().stage(), 0);
            match components
                .cover()
                .restrict_support(
                    open,
                    &format!("dispatch_original_support_{ci}_{oi}"),
                    &mut b,
                )
                .unwrap()
            {
                PhysicalSupportRestriction::EmptySupport { .. } => {}
                PhysicalSupportRestriction::Support(s) => {
                    let (_, _, map) = s.physical_localization().unwrap();
                    let mut zero = true;
                    for f in branch
                        .contact()
                        .differential_coefficient()
                        .ideal()
                        .generators()
                    {
                        zero &= s
                            .frame()
                            .local()
                            .zero(&map.pull(f, &mut b).unwrap(), &mut b)
                            .unwrap();
                    }
                    zeros += usize::from(zero);
                }
            }
            complete += 1;
        }
        assert_eq!(complete, 3);
        if ci == 3 {
            assert_eq!(components.split().pattern(), ComponentPattern::Mixed);
            assert!(zeros > 0);
            counts[2] += 1;
        } else {
            assert_eq!(
                components.split().pattern(),
                ComponentPattern::NowhereIdenticallyZero
            );
            assert_eq!(zeros, 0);
            counts[1] += 1;
        }
    }
    assert_eq!(counts, [1, 2, 1]);
}

#[test]
fn contact_dispatch_original_ancestor_ascent_all_components() {
    let mut b = Budget::new(Limits::default());
    let mut driver = live(&mut b);
    for _ in 0..10 {
        if matches!(driver.advance(&mut b).unwrap(), RecursiveAdvance::Complete) {
            break;
        }
    }
    let origin = driver.contact_dispatch_origin(&mut b).unwrap().unwrap();
    let cover = ContactDispatchCover::prepare(origin.clone(), &mut b).unwrap();
    let mut centers = 0;
    let mut units = 0;
    let mut empty = 0;
    for ci in [3, 2, 1, 0] {
        let branch = match ContactDispatchBranch::prepare(
            cover.clone(),
            ci,
            [
                symbol!(format!("dispatch_ascent_{ci}::z")),
                symbol!(format!("dispatch_ascent_{ci}::i")),
            ],
            &mut b,
        )
        .unwrap()
        {
            ContactBranchProduction::Branch(q) => q,
            ContactBranchProduction::EmptyContact { .. } => {
                empty += 1;
                continue;
            }
            _ => panic!("incomplete actual branch"),
        };
        let components = ContactTerminalComponents::prepare(
            branch,
            &format!("dispatch_ascent_split_{ci}"),
            &mut b,
        )
        .unwrap();
        let mut frontier =
            ContactComponentFrontier::new(components, format!("dispatch_ascent_work_{ci}"), &mut b)
                .unwrap();
        for i in (0..frontier.work().len()).rev() {
            assert!(matches!(
                frontier.advance(i, &mut b).unwrap(),
                ContactDispatchAdvance::Progress
            ));
        }
        assert!(frontier.complete_inventory());
        for work in frontier.work() {
            match work {
                ContactComponentWork::Center(c) => {
                    assert_eq!(ci, 3);
                    assert_eq!(c.ancestors().len(), 2);
                    assert_eq!(
                        c.ancestors()
                            .iter()
                            .map(|a| a.maximum())
                            .collect::<Vec<_>>(),
                        [2, 4]
                    );
                    assert_eq!(
                        c.ancestors()
                            .iter()
                            .map(|a| a.presentation().original_source().mark())
                            .collect::<Vec<_>>(),
                        [1, 2]
                    );
                    assert!(c.history().same_root(origin.auxiliary().history()));
                    assert_eq!(c.history().stage(), 0);
                    let checked = CheckedRecursiveCenter::new(
                        RecursiveCenterOrigin::DispatchedContact(c.clone()),
                        &mut b,
                    )
                    .unwrap();
                    assert!(Arc::ptr_eq(checked.history(), c.history()));
                    let r = c.frame().local().ring();
                    let expected = Ideal::new(
                        r.clone(),
                        vec![
                            r.coordinate(0).unwrap(),
                            &r.coordinate(1).unwrap() - &r.one(),
                        ],
                        &mut b,
                    )
                    .unwrap();
                    for (a, z) in [(c.ideal(), &expected), (&expected, c.ideal())] {
                        let full = c.frame().local().ideal().sum(z, &mut b).unwrap();
                        for f in a.generators() {
                            assert!(
                                full.contains(f, c.frame().local().unit_relations(), &mut b)
                                    .unwrap()
                            );
                        }
                    }
                    centers += 1;
                }
                ContactComponentWork::UnitChild { .. } => units += 1,
                ContactComponentWork::EmptyPrincipal(_) | ContactComponentWork::EmptySupport(_) => {
                    empty += 1
                }
                ContactComponentWork::Pending => panic!("pending map lost"),
            }
        }
    }
    assert!(centers > 0);
    assert!(units > 0);
    assert!(empty > 0);
}

#[test]
fn contact_dispatch_ascent_rejects_foreign_component_owner() {
    let mut b = Budget::new(Limits::default());
    let mut source = live(&mut b);
    for _ in 0..10 {
        if matches!(source.advance(&mut b).unwrap(), RecursiveAdvance::Complete) {
            break;
        }
    }
    let mut d = source
        .finite_contact_dispatcher("dispatch_foreign_center".into(), &mut b)
        .unwrap()
        .unwrap();
    for ci in [1, 3] {
        assert!(matches!(
            d.advance_branch(ci, &mut b).unwrap(),
            ContactDispatchAdvance::Progress
        ));
        let ContactBranchWork::Components(f) = &d.branches()[ci] else {
            panic!()
        };
        let n = f.work().len();
        for j in 0..n {
            assert!(matches!(
                d.advance_component(ci, j, &mut b).unwrap(),
                ContactDispatchAdvance::Progress
            ));
        }
    }
    let ContactBranchWork::Components(other) = &d.branches()[1] else {
        panic!()
    };
    let ContactBranchWork::Components(valid) = &d.branches()[3] else {
        panic!()
    };
    let c = valid
        .work()
        .iter()
        .find_map(|w| {
            if let ContactComponentWork::Center(c) = w {
                Some(c)
            } else {
                None
            }
        })
        .unwrap();
    assert!(matches!(
        DispatchedContactCenter::prepare(
            other.owner().clone(),
            c.localization().clone(),
            c.support().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    assert!(valid.complete_inventory());
    assert!(!d.complete_inventory());
}
