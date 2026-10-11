use super::*;
use symbolica::symbol;

fn budget() -> Budget {
    Budget::new(Limits {
        max_operations: 1_000_000,
        max_total_ideal_slots: 1_000_000,
        ..Limits::default()
    })
}
fn fixture(name: &str, unit: bool) -> Arc<PrincipalCenterAtlas> {
    let mut b = budget();
    let r = Arc::new(
        Ring::new(
            ["p", "y", "z"]
                .iter()
                .map(|s| symbol!(format!("{name}::{s}")))
                .collect(),
            vec![0],
        )
        .unwrap(),
    );
    let y = r.coordinate(1).unwrap();
    let z = r.coordinate(2).unwrap();
    let local = LocalizedAlgebra::new(
        Ideal::new(r.clone(), vec![], &mut b).unwrap(),
        vec![1, 2],
        vec![],
        &mut b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            free_axes: vec![1, 2],
            determinant_inverse_axis: None,
        }
        .verify(&mut b)
        .unwrap(),
    );
    let SncProduction::Verified(ledger) = verify_initial_relative_snc(
        frame,
        vec![
            InitialDivisor {
                id: BoundaryId(0),
                equation: y.clone(),
            },
            InitialDivisor {
                id: BoundaryId(1),
                equation: z.clone(),
            },
        ],
        &mut b,
    )
    .unwrap() else {
        panic!()
    };
    let h = ResolutionHistory::initial(ledger).unwrap();
    let f = if unit {
        r.one()
    } else {
        &(&y * &y) * &(&(&z * &z) * &z)
    };
    let s =
        Arc::new(MarkedIdeal::new(Ideal::new(r.clone(), vec![f], &mut b).unwrap(), 2, &b).unwrap());
    let c = Arc::new(
        OpenCoverCertificate {
            algebra: h.ledger().frame().local().clone(),
            support: Ideal::new(r.clone(), vec![], &mut b).unwrap(),
            opens: vec![y.clone(), &r.one() - &y, &r.one() - &r.one()],
        }
        .verify(&mut b)
        .unwrap(),
    );
    PrincipalCenterAtlas::prepare(h, s, c, &format!("{name}_opens"), &mut b).unwrap()
}
fn selection(a: &Arc<PrincipalCenterAtlas>, unit: bool) -> Arc<MonomialMaximumSelection> {
    MonomialMaximumSelection::produce(
        a.clone(),
        if unit { vec![0, 0] } else { vec![2, 3] },
        vec![a.source().ideal().ring().one()],
        &mut budget(),
    )
    .unwrap()
}
fn restrictions(s: &Arc<MonomialMaximumSelection>) -> Vec<Arc<LocalCenter>> {
    (0..s.atlas().len())
        .map(|i| s.restrict(i, &mut budget()).unwrap())
        .collect()
}
fn pairs(c: &[Arc<LocalCenter>], name: &str) -> Vec<Arc<CenterOverlap>> {
    let mut result = vec![];
    for i in 0..c.len() {
        for j in i + 1..c.len() {
            result.push(
                CenterOverlap::verify(
                    c[i].clone(),
                    c[j].clone(),
                    &format!("{name}_{i}_{j}"),
                    &mut budget(),
                )
                .unwrap(),
            );
        }
    }
    result
}
#[test]
fn global_selection_restricts_without_recomputing_local_maximum() {
    let a = fixture("selection_restriction", false);
    let s = selection(&a, false);
    let MonomialProduction::Center(p) = s.native_proof() else {
        panic!()
    };
    assert_eq!(p.indices(), [0]);
    assert!(Arc::ptr_eq(&p.witness().source, a.source()));
    assert!(Arc::ptr_eq(p.ledger(), a.history().ledger()));
    let mut c = restrictions(&s);
    let states: Vec<_> = c
        .iter()
        .map(|c| c.selection_restriction().unwrap().state())
        .collect();
    assert_eq!(
        states,
        [
            SelectedCenterState::EmptySelectedCenter,
            SelectedCenterState::SelectedCenter,
            SelectedCenterState::EmptyAmbient
        ]
    );
    for center in &c[..2] {
        let r = center.selection_restriction().unwrap();
        assert!(!r.cosupport().unwrap().empty());
        let h = a.open(center.index()).unwrap().history();
        assert!(h.same_root(a.history()));
        assert_eq!(h.births(), a.history().births());
        assert_eq!(h.old_snapshot(), a.history().old_snapshot());
        assert_eq!(h.stage(), a.history().stage());
        assert!(h.ledger().frame().local().ring().is_parameter(0));
    }
    let local = a.open(0).unwrap().history().ledger().frame().local();
    let independently_selected = LocalCenter::monomial(
        a.clone(),
        0,
        vec![2, 3],
        vec![local.ring().one()],
        &mut budget(),
    )
    .unwrap();
    assert_eq!(
        independently_selected.monomial_proof().unwrap().indices(),
        [1]
    );
    let overlap = pairs(&c, "selection_restriction_pair");
    assert!(overlap[0].frame().is_some());
    let mut foreign = c.clone();
    foreign[0] = independently_selected;
    assert!(matches!(
        s.verify_cover(foreign, overlap.clone(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    // Unit-scaled generators retain this exact selection event.
    let local = a.open(1).unwrap().history().ledger().frame().local();
    let y = local.ring().coordinate(1).unwrap();
    c[1] = c[1]
        .with_presentation(
            Ideal::new(
                local.ring().clone(),
                vec![&y * &(&local.ring().one() - &y)],
                &mut budget(),
            )
            .unwrap(),
            &mut budget(),
        )
        .unwrap();
    assert!(Arc::ptr_eq(
        c[1].selection_restriction().unwrap().selection(),
        &s
    ));
    let mut overlap = pairs(&c, "selection_scaled_pair");
    c.reverse();
    overlap.reverse();
    let proof = s.verify_cover(c, overlap, &mut budget()).unwrap();
    assert_eq!(proof.descent().centers().len(), 3);
    assert_eq!(proof.descent().overlaps().len(), 3);
    assert!(Arc::ptr_eq(proof.selection(), &s));
}
#[test]
fn selected_event_source_history_and_complete_inventory_are_bound() {
    let a = fixture("selection_association", false);
    let s = selection(&a, false);
    let c = restrictions(&s);
    let o = pairs(&c, "selection_association_pair");
    let event = selection(&a, false);
    let mut mixed = c.clone();
    mixed[1] = event.restrict(1, &mut budget()).unwrap();
    assert!(matches!(
        s.verify_cover(mixed, o.clone(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    for (i, (h, source)) in [
        (a.history().clone(), Arc::new(a.source().as_ref().clone())),
        (
            ResolutionHistory::initial(a.history().ledger().clone()).unwrap(),
            a.source().clone(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let foreign = PrincipalCenterAtlas::prepare(
            h,
            source,
            a.cover().clone(),
            &format!("selection_foreign_{i}"),
            &mut budget(),
        )
        .unwrap();
        let ss = selection(&foreign, false);
        let mut mixed = c.clone();
        mixed[1] = ss.restrict(1, &mut budget()).unwrap();
        assert!(matches!(
            s.verify_cover(mixed, o.clone(), &mut budget()),
            Err(Error::Invalid(_))
        ));
    }
    assert!(matches!(
        s.restrict(3, &mut budget()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        s.verify_cover(c[..2].to_vec(), o.clone(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        s.verify_cover(c.clone(), o[..2].to_vec(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut duplicate = c.clone();
    duplicate[2] = duplicate[0].clone();
    assert!(matches!(
        s.verify_cover(duplicate, o.clone(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut duplicate = o.clone();
    duplicate[2] = duplicate[0].clone();
    assert!(matches!(
        s.verify_cover(c.clone(), duplicate, &mut budget()),
        Err(Error::Invalid(_))
    ));
    // Same event/index but another issued restriction cannot replace the exact
    // local center owner to which a saved overlap was bound.
    let mut rebound = c.clone();
    rebound[1] = s.restrict(1, &mut budget()).unwrap();
    assert!(matches!(
        s.verify_cover(rebound, o.clone(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    s.verify_cover(c, o, &mut budget()).unwrap();
}
#[test]
fn empty_cosupport_is_proved_separately_and_zero_source_is_not_a_unit() {
    let a = fixture("selection_empty", true);
    let s = selection(&a, true);
    assert!(matches!(
        s.native_proof(),
        MonomialProduction::EmptyCosupport { .. }
    ));
    let c = restrictions(&s);
    for v in &c[..2] {
        let p = v.selection_restriction().unwrap();
        assert_eq!(p.state(), SelectedCenterState::EmptyCosupport);
        assert!(p.cosupport().unwrap().empty());
        assert_eq!(p.cosupport().unwrap().source().mark(), a.source().mark());
    }
    assert_eq!(
        c[2].selection_restriction().unwrap().state(),
        SelectedCenterState::EmptyAmbient
    );
    assert!(c[2].selection_restriction().unwrap().cosupport().is_none());
    s.verify_cover(c.clone(), pairs(&c, "selection_empty_pair"), &mut budget())
        .unwrap();
    let source = Arc::new(
        MarkedIdeal::new(
            Ideal::new(a.source().ideal().ring().clone(), vec![], &mut budget()).unwrap(),
            2,
            &budget(),
        )
        .unwrap(),
    );
    let zero = PrincipalCenterAtlas::prepare(
        a.history().clone(),
        source,
        a.cover().clone(),
        "selection_zero",
        &mut budget(),
    )
    .unwrap();
    assert!(matches!(
        MonomialMaximumSelection::produce(zero, vec![0, 0], vec![], &mut budget()),
        Err(Error::Invalid("residual ideal is not unit"))
    ));
}
#[test]
fn bounded_retry_and_shared_owner_lifetime() {
    fn empty_budget() -> Budget {
        Budget::new(Limits {
            max_total_ideal_slots: 0,
            ..Limits::default()
        })
    }
    let a = fixture("selection_retry", false);
    let r = a.source().ideal().ring();
    assert!(matches!(
        MonomialMaximumSelection::produce(
            a.clone(),
            vec![2, 3],
            vec![r.one()],
            &mut empty_budget()
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        MonomialMaximumSelection::produce(a.clone(), vec![2], vec![r.one()], &mut budget()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        MonomialMaximumSelection::produce(
            a.clone(),
            vec![2, 3],
            vec![&r.one() + &r.one()],
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
    let s = selection(&a, false);
    for i in 0..3 {
        assert!(matches!(
            s.restrict(i, &mut empty_budget()),
            Err(Error::ResourceIncomplete(_))
        ));
    }
    let c = restrictions(&s);
    let o = pairs(&c, "selection_retry_pair");
    assert!(matches!(
        s.verify_cover(c.clone(), o.clone(), &mut empty_budget()),
        Err(Error::ResourceIncomplete(_))
    ));
    let weak_atlas = Arc::downgrade(&a);
    let weak_selection = Arc::downgrade(&s);
    let proof = s.verify_cover(c.clone(), o.clone(), &mut budget()).unwrap();
    drop(c);
    drop(o);
    drop(s);
    drop(a);
    assert!(weak_atlas.upgrade().is_some());
    assert!(weak_selection.upgrade().is_some());
    drop(proof);
    assert!(weak_atlas.upgrade().is_none());
    assert!(weak_selection.upgrade().is_none());
}
