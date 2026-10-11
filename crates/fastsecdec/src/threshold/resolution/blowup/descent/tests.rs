use super::*;
use symbolica::symbol;

fn budget() -> Budget {
    Budget::new(Limits {
        max_operations: 1_000_000,
        max_total_ideal_slots: 1_000_000,
        ..Limits::default()
    })
}
fn fixture(
    name: &str,
    disconnected: bool,
) -> (
    Arc<ResolutionHistory>,
    Arc<MarkedIdeal>,
    Arc<VerifiedOpenCover>,
) {
    let mut b = budget();
    let r = Arc::new(
        Ring::new(
            ["p", "x", "y", "z", "u"]
                .iter()
                .map(|s| symbol!(format!("{name}::{s}")))
                .collect(),
            vec![0],
        )
        .unwrap(),
    );
    let x = r.coordinate(1).unwrap();
    let y = r.coordinate(2).unwrap();
    let z = r.coordinate(3).unwrap();
    let equations = if disconnected {
        vec![&x * &(&x - &r.one())]
    } else {
        vec![]
    };
    let local = LocalizedAlgebra::new(
        Ideal::new(r.clone(), equations, &mut b).unwrap(),
        vec![1, 2, 3],
        vec![],
        &mut b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: if disconnected { vec![0] } else { vec![] },
            dependent_axes: if disconnected { vec![1] } else { vec![] },
            free_axes: if disconnected {
                vec![2, 3]
            } else {
                vec![1, 2, 3]
            },
            determinant_inverse_axis: disconnected.then_some(4),
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
    let history = ResolutionHistory::initial(ledger).unwrap();
    let source = Arc::new(
        MarkedIdeal::new(
            Ideal::new(r.clone(), vec![&(&y * &y) * &(&(&z * &z) * &z)], &mut b).unwrap(),
            2,
            &b,
        )
        .unwrap(),
    );
    let cover = Arc::new(
        OpenCoverCertificate {
            algebra: history.ledger().frame().local().clone(),
            support: Ideal::new(r.clone(), vec![], &mut b).unwrap(),
            opens: vec![x, &r.one() - &r.coordinate(1).unwrap(), &r.one() - &r.one()],
        }
        .verify(&mut b)
        .unwrap(),
    );
    (history, source, cover)
}
fn atlas(name: &str, disconnected: bool) -> Arc<PrincipalCenterAtlas> {
    let (h, s, c) = fixture(name, disconnected);
    PrincipalCenterAtlas::prepare(h, s, c, &format!("{name}_opens"), &mut budget()).unwrap()
}
fn centers(atlas: &Arc<PrincipalCenterAtlas>) -> Vec<Arc<LocalCenter>> {
    (0..atlas.len())
        .map(|i| match atlas.open(i) {
            Some(o) => LocalCenter::monomial(
                atlas.clone(),
                i,
                vec![2, 3],
                vec![o.history().ledger().frame().local().ring().one()],
                &mut budget(),
            )
            .unwrap(),
            None => LocalCenter::empty_ambient(atlas.clone(), i).unwrap(),
        })
        .collect()
}
fn overlaps(c: &[Arc<LocalCenter>], name: &str) -> Vec<Arc<CenterOverlap>> {
    let mut v = Vec::new();
    for i in 0..c.len() {
        for j in i + 1..c.len() {
            v.push(
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
    v
}
#[test]
fn principal_cover_actual_monomial_centers_and_unit_presentations() {
    let a = atlas("descent_actual", false);
    let mut c = centers(&a);
    for center in &c[..2] {
        let p = center.monomial_proof().unwrap();
        assert_eq!(p.indices(), [0]);
        let h = a.open(center.index()).unwrap().history();
        assert!(h.same_root(a.history()));
        assert!(h.ledger().frame().local().ring().is_parameter(0));
    }
    let o = a.open(0).unwrap();
    let r = o.history().ledger().frame().local().ring();
    c[0] = c[0]
        .with_presentation(
            Ideal::new(
                r.clone(),
                vec![&r.coordinate(1).unwrap() * &r.coordinate(2).unwrap()],
                &mut budget(),
            )
            .unwrap(),
            &mut budget(),
        )
        .unwrap();
    let mut pairs = overlaps(&c, "descent_actual_pair");
    assert_eq!(pairs[0].frame().unwrap().free_axes().len(), 3);
    let mut reversed = c.clone();
    reversed.reverse();
    pairs.reverse();
    let proof = GluedAdmissibleCenter::verify(a.clone(), reversed, pairs, &mut budget()).unwrap();
    assert_eq!(proof.centers().len(), 3);
    assert_eq!(proof.overlaps().len(), 3);
    assert!(proof.overlaps()[1].includes_empty_ambient());
    let weak = Arc::downgrade(&a);
    drop(proof);
    drop(c);
    drop(a);
    assert!(weak.upgrade().is_none());
}
#[test]
fn ambient_intersection_empty_is_retained_with_all_relations() {
    let a = atlas("descent_disconnected", true);
    let c = centers(&a);
    let pairs = overlaps(&c, "descent_disconnected_pair");
    let (equations, relations) = pairs[0].empty_intersection().unwrap();
    assert!(!equations.generators().is_empty());
    assert!(relations.len() >= 3);
    let g = GluedAdmissibleCenter::verify(a, c, pairs, &mut budget()).unwrap();
    assert_eq!(g.overlaps().len(), 3);
}
#[test]
fn omitted_inventory_foreign_source_history_and_bad_center_refuse() {
    let a = atlas("descent_mutations", false);
    let c = centers(&a);
    let pairs = overlaps(&c, "descent_mutations_pair");
    assert!(matches!(
        GluedAdmissibleCenter::verify(a.clone(), c[..2].to_vec(), pairs.clone(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        GluedAdmissibleCenter::verify(a.clone(), c.clone(), pairs[..2].to_vec(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut duplicate = pairs.clone();
    duplicate[2] = duplicate[0].clone();
    assert!(matches!(
        GluedAdmissibleCenter::verify(a.clone(), c.clone(), duplicate, &mut budget()),
        Err(Error::Invalid(_))
    ));
    let r = a.open(0).unwrap().history().ledger().frame().local().ring();
    assert!(matches!(
        c[0].with_presentation(
            Ideal::new(r.clone(), vec![r.coordinate(3).unwrap()], &mut budget()).unwrap(),
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
    let changed_history = ResolutionHistory::initial(a.history().ledger().clone()).unwrap();
    let foreign = PrincipalCenterAtlas::prepare(
        changed_history,
        a.source().clone(),
        a.cover().clone(),
        "descent_foreign",
        &mut budget(),
    )
    .unwrap();
    let d = centers(&foreign);
    assert!(matches!(
        CenterOverlap::verify(
            c[0].clone(),
            d[1].clone(),
            "descent_foreign_pair",
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
    let rr = a.source().ideal().ring();
    let two = &rr.one() + &rr.one();
    let different = Arc::new(
        MarkedIdeal::new(
            Ideal::new(
                rr.clone(),
                a.source()
                    .ideal()
                    .generators()
                    .iter()
                    .map(|g| g * &two)
                    .collect(),
                &mut budget(),
            )
            .unwrap(),
            2,
            &budget(),
        )
        .unwrap(),
    );
    let aa = PrincipalCenterAtlas::prepare(
        a.history().clone(),
        different,
        a.cover().clone(),
        "descent_source",
        &mut budget(),
    )
    .unwrap();
    let r2 = aa
        .open(1)
        .unwrap()
        .history()
        .ledger()
        .frame()
        .local()
        .ring();
    let foreign_source = LocalCenter::monomial(
        aa.clone(),
        1,
        vec![2, 3],
        vec![&r2.one() + &r2.one()],
        &mut budget(),
    )
    .unwrap();
    assert!(matches!(
        CenterOverlap::verify(
            c[0].clone(),
            foreign_source,
            "descent_source_pair",
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
}
#[test]
fn full_ambient_association_and_budget_retry_are_explicit() {
    let (h, s, c) = fixture("descent_retry", false);
    let local = h.ledger().frame().local();
    let r = local.ring();
    let omitted = OpenCoverCertificate {
        algebra: local.clone(),
        support: Ideal::new(r.clone(), vec![], &mut budget()).unwrap(),
        opens: vec![r.coordinate(1).unwrap()],
    }
    .verify(&mut budget());
    assert!(matches!(omitted, Err(Error::Invalid(_))));
    let support_only = Arc::new(
        OpenCoverCertificate {
            algebra: local.clone(),
            support: Ideal::new(r.clone(), vec![r.coordinate(2).unwrap()], &mut budget()).unwrap(),
            opens: vec![r.one()],
        }
        .verify(&mut budget())
        .unwrap(),
    );
    assert!(matches!(
        PrincipalCenterAtlas::prepare(
            h.clone(),
            s.clone(),
            support_only,
            "descent_lost_ambient",
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
    let mut tiny = Budget::new(Limits {
        max_total_ideal_slots: 0,
        ..Limits::default()
    });
    assert!(matches!(
        PrincipalCenterAtlas::prepare(
            h.clone(),
            s.clone(),
            c.clone(),
            "descent_retry_open",
            &mut tiny
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    let a = PrincipalCenterAtlas::prepare(h, s, c, "descent_retry_open", &mut budget()).unwrap();
    let cs = centers(&a);
    let mut tiny = Budget::new(Limits {
        max_total_ideal_slots: 0,
        ..Limits::default()
    });
    assert!(matches!(
        CenterOverlap::verify(
            cs[0].clone(),
            cs[1].clone(),
            "descent_retry_pair",
            &mut tiny
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    let ps = overlaps(&cs, "descent_retry_pair_ok");
    GluedAdmissibleCenter::verify(a, cs, ps, &mut budget()).unwrap();
}

#[test]
fn triple_overlap_and_actual_incompatible_local_maxima() {
    let (h, s, _) = fixture("descent_three", false);
    let local = h.ledger().frame().local();
    let r = local.ring();
    let cover = Arc::new(
        OpenCoverCertificate {
            algebra: local.clone(),
            support: Ideal::new(r.clone(), vec![], &mut budget()).unwrap(),
            opens: vec![
                r.coordinate(1).unwrap(),
                &r.one() - &r.coordinate(1).unwrap(),
                r.one(),
            ],
        }
        .verify(&mut budget())
        .unwrap(),
    );
    let a = PrincipalCenterAtlas::prepare(
        h.clone(),
        s.clone(),
        cover,
        "descent_three_open",
        &mut budget(),
    )
    .unwrap();
    let c = centers(&a);
    let ps = overlaps(&c, "descent_three_pair");
    assert!(ps.iter().all(|p| p.frame().is_some()));
    GluedAdmissibleCenter::verify(a, c, ps, &mut budget()).unwrap();
    // Both center owners below are actually produced, admissible local BM
    // monomial maxima. Their mismatch is why global scheduling is separate.
    let cover = Arc::new(
        OpenCoverCertificate {
            algebra: local.clone(),
            support: Ideal::new(r.clone(), vec![], &mut budget()).unwrap(),
            opens: vec![
                r.coordinate(2).unwrap(),
                &r.one() - &r.coordinate(2).unwrap(),
            ],
        }
        .verify(&mut budget())
        .unwrap(),
    );
    let a = PrincipalCenterAtlas::prepare(h, s, cover, "descent_incompatible_open", &mut budget())
        .unwrap();
    let c = centers(&a);
    assert_eq!(c[0].monomial_proof().unwrap().indices(), [1]);
    assert_eq!(c[1].monomial_proof().unwrap().indices(), [0]);
    assert!(matches!(
        CenterOverlap::verify(
            c[0].clone(),
            c[1].clone(),
            "descent_incompatible_pair",
            &mut budget()
        ),
        Err(Error::Invalid("ambient center ideal equality"))
    ));
}

#[test]
fn boundary_label_swap_and_empty_cosupport_are_distinct() {
    let a = atlas("descent_labels", false);
    let c = centers(&a);
    let ledger = a.history().ledger();
    let r = ledger.frame().local().ring();
    let SncProduction::Verified(swapped) = verify_initial_relative_snc(
        ledger.frame().clone(),
        vec![
            InitialDivisor {
                id: BoundaryId(0),
                equation: r.coordinate(3).unwrap(),
            },
            InitialDivisor {
                id: BoundaryId(1),
                equation: r.coordinate(2).unwrap(),
            },
        ],
        &mut budget(),
    )
    .unwrap() else {
        panic!()
    };
    let foreign = PrincipalCenterAtlas::prepare(
        ResolutionHistory::initial(swapped).unwrap(),
        a.source().clone(),
        a.cover().clone(),
        "descent_swapped",
        &mut budget(),
    )
    .unwrap();
    let rr = foreign
        .open(1)
        .unwrap()
        .history()
        .ledger()
        .frame()
        .local()
        .ring();
    let other = LocalCenter::monomial(
        foreign.clone(),
        1,
        vec![3, 2],
        vec![rr.one()],
        &mut budget(),
    )
    .unwrap();
    assert!(matches!(
        CenterOverlap::verify(c[0].clone(), other, "descent_swap_pair", &mut budget()),
        Err(Error::Invalid(_))
    ));
    let unit = Arc::new(
        MarkedIdeal::new(
            Ideal::new(r.clone(), vec![r.one()], &mut budget()).unwrap(),
            2,
            &budget(),
        )
        .unwrap(),
    );
    let a = PrincipalCenterAtlas::prepare(
        a.history().clone(),
        unit,
        a.cover().clone(),
        "descent_unit",
        &mut budget(),
    )
    .unwrap();
    let c = (0..a.len())
        .map(|i| match a.open(i) {
            Some(o) => LocalCenter::monomial(
                a.clone(),
                i,
                vec![0, 0],
                vec![o.history().ledger().frame().local().ring().one()],
                &mut budget(),
            )
            .unwrap(),
            None => LocalCenter::empty_ambient(a.clone(), i).unwrap(),
        })
        .collect::<Vec<_>>();
    assert!(c[0].empty_cosupport().is_some());
    let ps = overlaps(&c, "descent_unit_pair");
    GluedAdmissibleCenter::verify(a, c, ps, &mut budget()).unwrap();
}
