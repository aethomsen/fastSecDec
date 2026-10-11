use super::*;
use symbolica::symbol;

#[test]
fn recursive_matched_frame_next_cover_seam() {
    let started = std::time::Instant::now();
    let mut d = prepared_lower(false);
    let mut b = Budget::new(Limits::default());
    assert!(!matches!(
        d.prepare_recursive_ascent(4, &mut b).unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    assert!(!matches!(
        d.advance_recursive_ascent_branch(4, 1, &mut b).unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    let count = match d.recursive_branch(4).unwrap().ascent().unwrap() {
        RecursiveBranchAscent::Finite(f) => match &f.branches()[1] {
            RecursiveAscentBranch::Components(c) => c.work().len(),
            _ => panic!("actual component"),
        },
        _ => panic!("finite"),
    };
    for i in 0..count {
        assert!(!matches!(
            d.advance_recursive_ascent_open(4, 1, i, &mut Budget::new(Limits::default()))
                .unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
    }
    let c = centers(&d).into_iter().next().unwrap();
    let matched = c.matched().clone();
    let lower = matched.lower().history().ledger().frame();
    b = Budget::new(Limits::default());
    let t = &lower.local().ring().coordinate(2).unwrap() - &lower.local().ring().one();
    let other = &lower.local().ring().one() - &t;
    let next = Arc::new(
        OpenCoverCertificate {
            algebra: lower.local().clone(),
            support: Ideal::new(lower.local().ring().clone(), vec![], &mut b).unwrap(),
            opens: vec![t.clone(), other],
        }
        .verify(&mut b)
        .unwrap(),
    );
    let refined = SupportEmbedding::refine_matched(matched.clone(), &mut b).unwrap();
    assert!(refined.descends_from(matched.support()));
    assert!(Arc::ptr_eq(refined.frame(), matched.frame()));
    assert!(Arc::ptr_eq(refined.ambient(), c.frame()));
    assert_eq!(refined.codimension(), matched.support().codimension());
    assert!(matches!(
        PulledSupportCover::prepare_matched(matched.support().clone(), next.clone(), &mut b),
        Err(Error::Invalid(_))
    ));
    let pulled =
        PulledSupportCover::prepare_matched(refined.clone(), next.clone(), &mut b).unwrap();
    assert_eq!(pulled.pulled().opens().len(), 2);
    assert!(Arc::ptr_eq(pulled.original(), &next));
    let physical = refined
        .physical_cover(c.source().clone(), pulled.pulled().clone(), &mut b)
        .unwrap();
    assert_eq!(
        physical.opens().len(),
        2 + physical.cosupport().generators().len()
    );
    let mut outcomes = [0usize; 4];
    let mut foreign_checked = false;
    for index in 0..physical.opens().len() {
        b = Budget::new(Limits::default());
        let outer = match physical
            .localize(
                c.history().clone(),
                index,
                &format!("matched_next_outer{index}"),
                &mut b,
            )
            .unwrap()
        {
            CoverLocalizationProduction::Complete(o) => Arc::new(*o),
            CoverLocalizationProduction::Empty { .. } => {
                outcomes[0] += 1;
                continue;
            }
            other => panic!("{other:?}"),
        };
        assert!(outer.history().same_root(c.history()));
        assert_eq!(outer.history().stage(), c.history().stage());
        let lower_index = match physical.opens()[index] {
            PhysicalSupportOpen::Lower { index, .. } => index,
            PhysicalSupportOpen::BelowMark { .. } => {
                outcomes[1] += 1;
                continue;
            }
        };
        let lower_open = match localize_verified_cover_open(
            matched.lower().history().clone(),
            next.clone(),
            lower_index,
            &format!("matched_next_lower{index}"),
            &mut b,
        )
        .unwrap()
        {
            CoverLocalizationProduction::Complete(o) => Arc::new(*o),
            other => panic!("actual lower unexpectedly unavailable {other:?}"),
        };
        if !foreign_checked {
            let foreign =
                ResolutionHistory::initial(matched.lower().history().ledger().clone()).unwrap();
            assert!(!foreign.same_root(matched.lower().history()));
            let foreign_open = match localize_verified_cover_open(
                foreign,
                next.clone(),
                lower_index,
                "matched_next_foreign",
                &mut b,
            )
            .unwrap()
            {
                CoverLocalizationProduction::Complete(o) => Arc::new(*o),
                _ => panic!("same-frame negative open"),
            };
            assert!(matches!(
                physical.restrict_pulled_lower(
                    outer.clone(),
                    foreign_open,
                    pulled.clone(),
                    "matched_next_bad",
                    &mut b
                ),
                Err(Error::Invalid(_))
            ));
            foreign_checked = true;
        }
        match physical
            .restrict_pulled_lower(
                outer,
                lower_open,
                pulled.clone(),
                &format!("matched_next_join{index}"),
                &mut b,
            )
            .unwrap()
        {
            MatchedPhysicalRestriction::EmptySupport(_) => outcomes[2] += 1,
            MatchedPhysicalRestriction::Support(m) => {
                let retained = SupportEmbedding::refine_matched(m.clone(), &mut b).unwrap();
                assert!(retained.descends_from(&refined));
                assert!(m.lower_map().checked_derivatives() > 0);
                outcomes[3] += 1;
            }
        }
    }
    assert_eq!(outcomes.iter().sum::<usize>(), physical.opens().len());
    assert!(outcomes[3] > 0 && foreign_checked);
    assert!(!d.complete_inventory());
    println!(
        "MATCHED_NEXT_COVER_PASS root_contacts={} root_ambient={} original_history=true native_cover=2 physical={} outcomes={outcomes:?} source_mark={} seconds={}",
        d.branches().len(),
        d.ambient().len(),
        physical.opens().len(),
        c.source().mark(),
        started.elapsed().as_secs_f64()
    );
}

fn indices(n: usize, reverse: bool) -> Vec<usize> {
    let mut v = (0..n).collect::<Vec<_>>();
    if reverse {
        v.reverse();
    }
    v
}
fn prepared_lower(reverse: bool) -> FiniteContactDispatcher {
    prepared_lower_order(reverse, false)
}
fn prepared_lower_order(reverse: bool, late_ambient: bool) -> FiniteContactDispatcher {
    let mut b = Budget::new(Limits::default());
    let ring = Arc::new(
        Ring::new(
            vec![
                symbol!("global_quartic::x"),
                symbol!("global_quartic::y"),
                symbol!("global_quartic::z"),
            ],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(ring.clone(), vec![], &mut b).unwrap(),
        vec![0, 1, 2],
        vec![],
        &mut b,
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
        .verify(&mut b)
        .unwrap(),
    );
    let x = ring.coordinate(0).unwrap();
    let y = ring.coordinate(1).unwrap();
    let z = ring.coordinate(2).unwrap();
    let g = |t: &Poly| (&t.pow(2) - &ring.one()).pow(2);
    let f = &(&x.pow(2) + &g(&y)) + &g(&z);
    let source =
        Arc::new(MarkedIdeal::new(Ideal::new(ring, vec![f], &mut b).unwrap(), 1, &b).unwrap());
    let mut driver = BoundaryFreeFirstCenter::new_with_principal_powers(
        frame,
        source,
        "global_quartic_actual",
        false,
    )
    .unwrap();
    for _ in 0..12 {
        let next = driver.advance(&mut b).unwrap();
        if !matches!(next, RecursiveAdvance::Progress) {
            break;
        }
    }
    let Some(mut finite) = driver
        .recursive_contact_dispatcher("global_quartic_dispatch".into(), &mut b)
        .unwrap()
    else {
        panic!("actual multiple contact required")
    };
    let mut recursive = 0;
    let mut empty = 0;
    let mut completed = 0;
    let mut centers = 0;
    for a in indices(finite.ambient().len(), reverse) {
        assert!(!matches!(
            finite.advance_ambient(a, &mut b).unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
    }
    for i in indices(finite.branches().len(), reverse) {
        assert!(!matches!(
            finite.advance_branch(i, &mut b).unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
        if finite.recursive_branch(i).is_none() {
            continue;
        }
        recursive += 1;
        for _ in 0..12 {
            let q = finite.recursive_branch(i).unwrap();
            if q.lower().is_some() || q.child_inventory_complete() {
                break;
            }
            assert!(!matches!(
                finite.advance_recursive_child(i, &mut b).unwrap(),
                ContactDispatchAdvance::Incomplete { .. }
            ));
        }
        let q = finite.recursive_branch(i).unwrap();
        if q.lower().is_none() {
            assert!(matches!(
                q.child().outcome(),
                RecursiveOutcome::EmptyMarkedCosupport { .. }
            ));
            assert!(q.child_inventory_complete());
            empty += 1;
            continue;
        }
        let lower = q.lower().unwrap();
        assert!(Arc::ptr_eq(
            lower.cover().origin().auxiliary().source(),
            q.source()
        ));
        let ambient = lower.ambient().len();
        let branches = lower.branches().len();
        for a in indices(if late_ambient { 0 } else { ambient }, reverse) {
            assert!(!matches!(
                finite.advance_nested_ambient(&[i], a, &mut b).unwrap(),
                ContactDispatchAdvance::Incomplete { .. }
            ));
        }
        for j in indices(branches, reverse) {
            assert!(!matches!(
                finite.advance_nested_branch(&[i], j, &mut b).unwrap(),
                ContactDispatchAdvance::Incomplete { .. }
            ));
            let count = match &finite
                .recursive_branch(i)
                .unwrap()
                .lower()
                .unwrap()
                .branches()[j]
            {
                ContactBranchWork::Components(f) => f.work().len(),
                ContactBranchWork::Empty(_) => 0,
                _ => panic!("unexpected lower branch"),
            };
            for c in indices(count, reverse) {
                assert!(!matches!(
                    finite.advance_nested_component(&[i], j, c, &mut b).unwrap(),
                    ContactDispatchAdvance::Incomplete { .. }
                ));
            }
            if let ContactBranchWork::Components(f) = &finite
                .recursive_branch(i)
                .unwrap()
                .lower()
                .unwrap()
                .branches()[j]
            {
                centers += f
                    .work()
                    .iter()
                    .filter(|w| matches!(w, ContactComponentWork::Center(_)))
                    .count();
            }
        }
        assert_eq!(
            finite
                .recursive_branch(i)
                .unwrap()
                .child_inventory_complete(),
            !late_ambient
        );
        completed += 1;
    }
    assert_eq!((recursive, empty, completed, centers), (8, 4, 4, 6));
    assert!(!finite.complete_inventory());
    finite
}
fn finish(d: &mut FiniteContactDispatcher, reverse: bool) {
    finish_ascent(d, reverse, true)
}
fn finish_ascent(d: &mut FiniteContactDispatcher, reverse: bool, complete: bool) {
    for i in indices(d.branches().len(), reverse) {
        if d.recursive_branch(i).is_none() {
            continue;
        }
        assert!(!matches!(
            d.prepare_recursive_ascent(i, &mut Budget::new(Limits::default()))
                .unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
        let n = match d.recursive_branch(i).unwrap().ascent().unwrap() {
            RecursiveBranchAscent::Finite(f) => f.branches().len(),
            _ => 0,
        };
        for j in indices(n, reverse) {
            assert!(!matches!(
                d.advance_recursive_ascent_branch(i, j, &mut Budget::new(Limits::default()))
                    .unwrap(),
                ContactDispatchAdvance::Incomplete { .. }
            ));
            let count = match d.recursive_branch(i).unwrap().ascent().unwrap() {
                RecursiveBranchAscent::Finite(f) => match &f.branches()[j] {
                    RecursiveAscentBranch::Components(c) => c.work().len(),
                    _ => 0,
                },
                _ => 0,
            };
            for k in indices(count, reverse) {
                assert!(!matches!(
                    d.advance_recursive_ascent_open(i, j, k, &mut Budget::new(Limits::default()))
                        .unwrap(),
                    ContactDispatchAdvance::Incomplete { .. }
                ));
            }
        }
    }
    assert_eq!(d.complete_inventory(), complete);
}

#[test]
fn recursive_partial_refresh_after_last_ambient_slot() {
    let mut d = prepared_lower_order(false, true);
    finish_ascent(&mut d, false, false);
    let mut refreshed = 0;
    for i in 0..d.branches().len() {
        let Some(child) = d.recursive_branch(i) else {
            continue;
        };
        let Some(lower) = child.lower() else {
            continue;
        };
        let count = lower.ambient().len();
        assert!(!lower.complete_inventory());
        let RecursiveBranchAscent::Finite(f) = child.ascent().unwrap() else {
            panic!("finite");
        };
        let initial = f.lower().clone();
        assert!(!f.complete_inventory());
        for a in 0..count {
            assert!(!matches!(
                d.advance_nested_ambient(&[i], a, &mut Budget::new(Limits::default()))
                    .unwrap(),
                ContactDispatchAdvance::Incomplete { .. }
            ));
        }
        assert!(d.recursive_branch(i).unwrap().child_inventory_complete());
        assert!(
            !d.recursive_branch(i)
                .unwrap()
                .construction_inventory_complete()
        );
        let mut no_slots = Budget::new(Limits {
            max_total_ideal_slots: 0,
            ..Limits::default()
        });
        assert!(matches!(
            d.advance_recursive_ascent_branch(i, 0, &mut no_slots)
                .unwrap(),
            ContactDispatchAdvance::Incomplete { .. }
        ));
        let RecursiveBranchAscent::Finite(f) = d.recursive_branch(i).unwrap().ascent().unwrap()
        else {
            unreachable!();
        };
        assert!(Arc::ptr_eq(f.lower(), &initial));
        assert!(!f.complete_inventory());
        assert!(matches!(
            d.advance_recursive_ascent_branch(i, 0, &mut Budget::new(Limits::default()))
                .unwrap(),
            ContactDispatchAdvance::Progress
        ));
        let RecursiveBranchAscent::Finite(f) = d.recursive_branch(i).unwrap().ascent().unwrap()
        else {
            unreachable!();
        };
        assert!(!Arc::ptr_eq(f.lower(), &initial));
        assert!(Arc::ptr_eq(f.lower().cover(), initial.cover()));
        assert!(f.lower().complete_inventory() && f.complete_inventory());
        assert!(
            initial
                .ambient()
                .iter()
                .all(|w| matches!(w, ContactAmbientWork::Pending))
        );
        refreshed += 1;
    }
    assert_eq!(refreshed, 4);
    assert!(d.complete_inventory());
    assert_eq!(centers(&d).len(), 6);
}
fn centers(d: &FiniteContactDispatcher) -> Vec<Arc<RecursiveContactCenter>> {
    let mut out = Vec::new();
    for i in 0..d.branches().len() {
        if let Some(RecursiveBranchAscent::Finite(f)) =
            d.recursive_branch(i).and_then(|q| q.ascent())
        {
            for branch in f.branches() {
                if let RecursiveAscentBranch::Components(c) = branch {
                    for work in c.work() {
                        if let RecursiveLiftWork::Center(c) = work {
                            out.push(c.clone());
                        }
                    }
                }
            }
        }
    }
    out
}
#[test]
fn recursive_contact_complete_quartic_original_ascent_reordered() {
    let mut reference = None;
    for reverse in [false, true] {
        let mut d = prepared_lower(reverse);
        finish(&mut d, reverse);
        assert_eq!((d.branches().len(), d.ambient().len()), (10, 18));
        let cs = centers(&d);
        assert_eq!(cs.len(), 6);
        let mut counts = [0usize; 2];
        for i in 0..d.branches().len() {
            if let Some(RecursiveBranchAscent::Finite(f)) =
                d.recursive_branch(i).and_then(|q| q.ascent())
            {
                for branch in f.branches() {
                    if let RecursiveAscentBranch::Components(c) = branch {
                        for w in c.work() {
                            counts[0] += 1;
                            counts[1] += usize::from(!matches!(w, RecursiveLiftWork::Center(_)));
                        }
                    }
                }
            }
        }
        assert_eq!(counts, [42, 36]);
        let mut signatures = Vec::new();
        for c in cs {
            let mut b = Budget::new(Limits::default());
            assert_eq!(c.source().mark(), 1);
            assert_eq!(c.ancestors().len(), 2);
            assert!(
                c.history()
                    .same_root(d.cover().origin().auxiliary().history())
            );
            assert_eq!(c.history().stage(), 0);
            let l = c.frame().local();
            let r = l.ring();
            let expected = Ideal::new(
                r.clone(),
                vec![
                    r.coordinate(0).unwrap(),
                    &r.coordinate(1).unwrap().pow(2) - &r.one(),
                    &r.coordinate(2).unwrap().pow(2) - &r.one(),
                ],
                &mut b,
            )
            .unwrap();
            for (a, z) in [(c.ideal(), &expected), (&expected, c.ideal())] {
                let full = l.ideal().sum(z, &mut b).unwrap();
                for f in a.generators() {
                    assert!(full.contains(f, l.unit_relations(), &mut b).unwrap());
                }
            }
            signatures.push((c.source().ideal().clone(), c.ideal().clone()));
        }
        if let Some(old) = &reference {
            assert_eq!(&signatures, old);
        } else {
            reference = Some(signatures);
        }
    }
}
#[test]
fn recursive_contact_retry_and_wrong_original_owners() {
    let mut d = prepared_lower(false);
    let zero = Limits {
        max_operations: 0,
        ..Limits::default()
    };
    assert!(matches!(
        d.prepare_recursive_ascent(4, &mut Budget::new(zero))
            .unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    assert!(d.recursive_branch(4).unwrap().ascent().is_none());
    assert!(!d.complete_inventory());
    assert!(matches!(
        d.prepare_recursive_ascent(4, &mut Budget::new(Limits::default()))
            .unwrap(),
        ContactDispatchAdvance::Progress
    ));
    assert!(matches!(
        d.advance_recursive_ascent_branch(4, 1, &mut Budget::new(Limits::default()))
            .unwrap(),
        ContactDispatchAdvance::Progress
    ));
    let first_pending = match d.recursive_branch(4).unwrap().ascent().unwrap() {
        RecursiveBranchAscent::Finite(f) => match &f.branches()[1] {
            RecursiveAscentBranch::Components(c) => c.work().len(),
            _ => panic!(),
        },
        _ => panic!(),
    };
    assert_eq!(first_pending, 7);
    assert!(matches!(
        d.advance_recursive_ascent_open(
            4,
            1,
            0,
            &mut Budget::new(Limits {
                max_operations: 0,
                ..Limits::default()
            })
        )
        .unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    let RecursiveBranchAscent::Finite(f) = d.recursive_branch(4).unwrap().ascent().unwrap() else {
        panic!("accepted finite ascent missing");
    };
    let RecursiveAscentBranch::Components(c) = &f.branches()[1] else {
        panic!("accepted component ascent missing");
    };
    assert!(
        c.work()
            .iter()
            .all(|w| matches!(w, RecursiveLiftWork::Pending))
    );
    assert!(
        !d.complete_inventory(),
        "omitted ascent opens cannot complete construction"
    );
    finish(&mut d, false);
    let cs = centers(&d);
    let c = &cs[0];
    let q = d.recursive_branch(4).unwrap();
    let mut b = Budget::new(Limits::default());
    let prepare = |branch, source, first, cover, open, matched, b: &mut Budget| {
        RecursiveContactCenter::prepare(
            (branch, source),
            c.lower().clone(),
            first,
            cover,
            open,
            matched,
            b,
        )
    };
    let bad_source = Arc::new(q.source().as_ref().clone());
    assert!(matches!(
        prepare(
            c.branch().clone(),
            &bad_source,
            c.source_open().clone(),
            c.cover().clone(),
            c.localization().clone(),
            c.matched().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        prepare(
            d.recursive_branch(6).unwrap().branch().clone(),
            q.source(),
            c.source_open().clone(),
            c.cover().clone(),
            c.localization().clone(),
            c.matched().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        prepare(
            c.branch().clone(),
            q.source(),
            cs[1].source_open().clone(),
            c.cover().clone(),
            c.localization().clone(),
            c.matched().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    let old = d.cover().origin().auxiliary().history();
    let foreign_history = ResolutionHistory::initial(old.ledger().clone()).unwrap();
    assert!(!foreign_history.same_root(old));
    let foreign_open = match localize_verified_cover_open(
        foreign_history,
        d.cover().physical_cover().clone(),
        c.source_open().open().index(),
        "foreign_same_frame",
        &mut b,
    )
    .unwrap()
    {
        CoverLocalizationProduction::Complete(o) => Arc::new(*o),
        _ => panic!(),
    };
    assert!(matches!(
        prepare(
            c.branch().clone(),
            q.source(),
            foreign_open,
            c.cover().clone(),
            c.localization().clone(),
            c.matched().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    let mut mismatched_index_tested = false;
    for i in 0..c.lower().components().cover().opens().len() {
        if i == c.lower().localization().open().index() {
            continue;
        }
        if let CoverLocalizationProduction::Complete(other) = c
            .lower()
            .components()
            .localize(i, &format!("foreign_index{i}"), &mut b)
            .unwrap()
        {
            assert!(matches!(
                c.cover().restrict_pulled_lower(
                    c.localization().clone(),
                    Arc::new(*other),
                    c.matched().pulled_cover().unwrap().clone(),
                    "wrong_lower_index",
                    &mut b
                ),
                Err(Error::Invalid(_))
            ));
            mismatched_index_tested = true;
            break;
        }
    }
    assert!(mismatched_index_tested);
    let foreign = cs[1].matched().pulled_cover().unwrap().clone();
    assert!(matches!(
        c.cover().restrict_pulled_lower(
            c.localization().clone(),
            c.lower().localization().clone(),
            foreign,
            "wrong_pull",
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    let first = c.matched().pulled_cover().unwrap();
    assert!(matches!(
        PulledSupportCover::prepare(
            first.embedding().clone(),
            c.cover().full_cover().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    assert!(Arc::ptr_eq(
        c.source_open().open().source(),
        d.cover().origin().auxiliary().history()
    ));
    assert!(matches!(
        d.prepare_recursive_ascent(
            4,
            &mut Budget::new(Limits {
                max_operations: 0,
                ..Limits::default()
            })
        )
        .unwrap(),
        ContactDispatchAdvance::Accepted
    ));
    assert!(d.complete_inventory());
}
#[test]
fn recursive_contact_actual_locality_native_adaptation() {
    let mut d = prepared_lower(false);
    finish(&mut d, false);
    let c = centers(&d).remove(0);
    let mut b = Budget::new(Limits::default());
    let checked =
        CheckedRecursiveCenter::new(RecursiveCenterOrigin::RecursiveContact(c.clone()), &mut b)
            .unwrap()
            .with_simplified_normals(&mut b)
            .unwrap();
    assert!(Arc::ptr_eq(checked.history(), c.history()));
    assert_eq!(checked.ideal(), c.ideal());
    let adapted = adapt_recursive_center(checked, "recursive_maintained_adapt", &mut b).unwrap();
    assert_eq!(adapted.opens().len(), 4);
}
