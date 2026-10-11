use super::super::*;
use super::*;
use symbolica::symbol;

fn budget() -> Budget {
    Budget::new(Limits::default())
}
fn setup(name: &str, guard: bool, quotient: bool, boundary: bool) -> Arc<ResolutionHistory> {
    let mut b = budget();
    let r = Arc::new(
        Ring::new(
            ["p", "x", "y", "q"]
                .iter()
                .map(|s| symbol!(format!("{name}::{s}")))
                .collect(),
            vec![0],
        )
        .unwrap(),
    );
    let x = r.coordinate(1).unwrap();
    let y = r.coordinate(2).unwrap();
    let equations = if quotient { vec![x.clone()] } else { vec![] };
    let local = LocalizedAlgebra::new(
        Ideal::new(r.clone(), equations.clone(), &mut b).unwrap(),
        vec![1, 2],
        if guard {
            vec![Guard {
                factor: &r.one() + &(&x * &x),
                inverse_axis: 3,
            }]
        } else {
            vec![]
        },
        &mut b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: if quotient { vec![0] } else { vec![] },
            dependent_axes: if quotient { vec![1] } else { vec![] },
            free_axes: if quotient { vec![2] } else { vec![1, 2] },
            determinant_inverse_axis: quotient.then_some(3),
        }
        .verify(&mut b)
        .unwrap(),
    );
    let SncProduction::Verified(ledger) = verify_initial_relative_snc(
        frame,
        if boundary {
            vec![InitialDivisor {
                id: BoundaryId(7),
                equation: y,
            }]
        } else {
            vec![]
        },
        &mut b,
    )
    .unwrap() else {
        panic!()
    };
    ResolutionHistory::initial(ledger).unwrap()
}
fn marked(h: &Arc<ResolutionHistory>, polys: Vec<Poly>, mark: usize) -> Arc<MarkedIdeal> {
    Arc::new(
        MarkedIdeal::new(
            Ideal::new(
                h.ledger().frame().local().ring().clone(),
                polys,
                &mut budget(),
            )
            .unwrap(),
            mark,
            &budget(),
        )
        .unwrap(),
    )
}
fn reduced(h: &Arc<ResolutionHistory>, s: &Arc<MarkedIdeal>) -> Arc<PrincipalPowerPresentation> {
    match produce_principal_power_presentation(h.clone(), s.clone(), &mut budget()).unwrap() {
        PrincipalPowerOutcome::Reduced(r) => r,
        o => panic!("{o:?}"),
    }
}
fn unchanged(
    h: &Arc<ResolutionHistory>,
    s: &Arc<MarkedIdeal>,
    reason: PrincipalPowerUnchangedReason,
) {
    let PrincipalPowerOutcome::Unchanged(p) =
        produce_principal_power_presentation(h.clone(), s.clone(), &mut budget()).unwrap()
    else {
        panic!()
    };
    assert_eq!(p.reason(), reason);
    assert!(Arc::ptr_eq(p.source(), s));
    assert!(Arc::ptr_eq(p.history(), h));
}
fn identity(r: &Arc<PrincipalPowerPresentation>) {
    let local = r.history().ledger().frame().local();
    let product = r.unit() * &r.base().pow(r.power());
    assert!(
        local
            .zero(
                &(&product - &r.source().ideal().generators()[0]),
                &mut budget()
            )
            .unwrap()
    );
    assert_eq!(r.power() * r.reduced().mark(), r.source().mark());
    assert_eq!(r.reduced().ideal().generators(), &[r.base().clone()]);
    assert!(r.progress().recombination_checked);
}

#[test]
fn principal_power_preserves_actual_source_history_and_integer_marks() {
    let h = setup("principal_integer", false, false, true);
    let r = h.ledger().frame().local().ring();
    let y = r.coordinate(2).unwrap();
    let s = marked(&h, vec![y.pow(10)], 6);
    let p = reduced(&h, &s);
    identity(&p);
    assert_eq!(p.power(), 2);
    assert_eq!(p.base(), &y.pow(5));
    assert_eq!(p.reduced().mark(), 3);
    assert!(Arc::ptr_eq(p.source(), &s));
    assert!(Arc::ptr_eq(p.history(), &h));
    assert_eq!(p.history().births(), h.births());
    assert_eq!(p.history().old_snapshot(), h.old_snapshot());
    assert!(p.reduced().ideal().ring().is_parameter(0));
    let xy = &r.coordinate(1).unwrap() + &y;
    let minus_two = -(&r.one() + &r.one());
    let s = marked(&h, vec![&minus_two * &xy.pow(12)], 8);
    let p = reduced(&h, &s);
    identity(&p);
    assert_eq!(p.power(), 4);
    assert_eq!(p.reduced().mark(), 2);
    assert_eq!(p.unit(), &minus_two);
    assert_eq!(p.base(), &xy.pow(3));
}
#[test]
fn nonconstant_units_require_actual_guards_and_do_not_change_physical_data() {
    let h = setup("principal_guard", true, false, false);
    let r = h.ledger().frame().local().ring();
    let x = r.coordinate(1).unwrap();
    let y = r.coordinate(2).unwrap();
    let unit = &r.one() + &x.pow(2);
    let polynomial = &unit * &y.pow(10);
    let s = marked(&h, vec![polynomial.clone()], 6);
    let p = reduced(&h, &s);
    identity(&p);
    assert_eq!(p.power(), 2);
    assert_eq!(p.unit(), &unit);
    assert_eq!(s.ideal().generators(), &[polynomial]);
    assert!(
        p.progress()
            .checked_factors
            .iter()
            .any(|f| !f.polynomial().is_constant() && f.is_unit())
    );
    let h = setup("principal_no_guard", false, false, false);
    let r = h.ledger().frame().local().ring();
    let x = r.coordinate(1).unwrap();
    let y = r.coordinate(2).unwrap();
    let s = marked(&h, vec![&(&r.one() + &x.pow(2)) * &y.pow(10)], 6);
    unchanged(&h, &s, PrincipalPowerUnchangedReason::NoCommonPower);
}
#[test]
fn zero_and_unit_are_classified_in_the_actual_quotient() {
    let h = setup("principal_zero", false, true, false);
    let r = h.ledger().frame().local().ring();
    let x = r.coordinate(1).unwrap();
    let s = marked(&h, vec![x.clone()], 4);
    assert!(!s.ideal().generators().is_empty());
    unchanged(&h, &s, PrincipalPowerUnchangedReason::ZeroIdeal);
    unchanged(
        &h,
        &marked(&h, vec![], 4),
        PrincipalPowerUnchangedReason::ZeroIdeal,
    );
    let s = marked(&h, vec![&r.one() + &x], 4);
    unchanged(&h, &s, PrincipalPowerUnchangedReason::UnitIdeal);
    let h = setup("principal_unit_guard", true, false, false);
    let r = h.ledger().frame().local().ring();
    let s = marked(&h, vec![&r.one() + &r.coordinate(1).unwrap().pow(2)], 4);
    unchanged(&h, &s, PrincipalPowerUnchangedReason::UnitIdeal);
    unchanged(
        &h,
        &marked(&h, vec![r.one()], 4),
        PrincipalPowerUnchangedReason::UnitIdeal,
    );
}
#[test]
fn mixed_generators_and_nondivisible_marks_are_never_radicalized() {
    let h = setup("principal_mixed", false, false, false);
    let r = h.ledger().frame().local().ring();
    let x = r.coordinate(1).unwrap();
    let y = r.coordinate(2).unwrap();
    let s = marked(&h, vec![x.pow(2), y.pow(2)], 2);
    unchanged(&h, &s, PrincipalPowerUnchangedReason::MultipleGenerators);
    assert!(!s.ideal().contains(&(&x * &y), &[], &mut budget()).unwrap());
    let s = marked(&h, vec![y.pow(5)], 3);
    unchanged(&h, &s, PrincipalPowerUnchangedReason::NoCommonPower);
}
#[test]
fn source_refusal_and_resource_prefix_are_explicit_and_retryable() {
    let h = setup("principal_budget", false, false, false);
    let r = h.ledger().frame().local().ring();
    let y = r.coordinate(2).unwrap();
    let s = marked(&h, vec![y.pow(10)], 6);
    let foreign = setup("principal_foreign", false, false, false);
    assert!(matches!(
        produce_principal_power_presentation(foreign, s.clone(), &mut budget()),
        Err(Error::Invalid(_))
    ));
    for limits in [
        Limits {
            max_operations: 0,
            ..Limits::default()
        },
        Limits {
            max_total_ideal_slots: 0,
            ..Limits::default()
        },
        Limits {
            max_degree_per_axis: 4,
            ..Limits::default()
        },
        Limits {
            max_mark: 3,
            ..Limits::default()
        },
    ] {
        let PrincipalPowerOutcome::Incomplete {
            source,
            history,
            reason,
            progress,
        } = produce_principal_power_presentation(h.clone(), s.clone(), &mut Budget::new(limits))
            .unwrap()
        else {
            panic!()
        };
        assert!(Arc::ptr_eq(&source, &s));
        assert!(Arc::ptr_eq(&history, &h));
        assert!(!reason.is_empty());
        assert_ne!(progress.stage, "complete");
    }
    let p = reduced(&h, &s);
    identity(&p);
    assert_eq!(h.stage(), 0);
}
#[test]
fn checked_actual_blowup_preserves_the_controlled_power_identity() {
    let h = setup("principal_transform", false, false, true);
    let r = h.ledger().frame().local().ring();
    let y = r.coordinate(2).unwrap();
    let unit = -(&r.one() + &r.one());
    let s = marked(&h, vec![&unit * &y.pow(10)], 6);
    let presentation = reduced(&h, &s);
    let MonomialProduction::Center(center) = produce_monomial_center(
        h.ledger().clone(),
        MonomialWitness {
            source: s,
            powers: vec![10],
            quotients: vec![unit],
        },
        &mut budget(),
    )
    .unwrap() else {
        panic!()
    };
    let center = Arc::new(*center);
    let AdaptedProduction::Complete(cover) = produce_adapted_cover(
        h.ledger().clone(),
        "principal_transform_adapt",
        &mut budget(),
    )
    .unwrap() else {
        panic!()
    };
    let BlowupProduction::Complete(blowup) = produce_monomial_blowup(
        center,
        h.clone(),
        cover,
        "principal_transform_blow",
        &mut budget(),
    )
    .unwrap() else {
        panic!()
    };
    let mut actual = 0;
    for chart in blowup.charts() {
        let mut b = budget();
        let local = chart.frame().local();
        let pulled = chart.pull_from_source(presentation.base(), &mut b).unwrap();
        let q = if let Some(e) = chart.exceptional() {
            actual += 1;
            let ledger = chart.history().ledger();
            let index = ledger
                .divisors()
                .iter()
                .position(|d| &d.equation == e)
                .unwrap();
            let mut q = pulled;
            for i in 0..presentation.reduced().mark() {
                let CartierDivision::Quotient(receipt) = divide_cartier(
                    ledger.clone(),
                    index,
                    q,
                    &format!("principal_transform_div{actual}_{i}"),
                    &mut b,
                )
                .unwrap() else {
                    panic!()
                };
                q = receipt.quotient().clone();
            }
            q
        } else {
            pulled
        };
        let u = chart.pull_from_source(presentation.unit(), &mut b).unwrap();
        let expected = &u * &q.pow(presentation.power());
        assert_eq!(chart.target().ideal().generators().len(), 1);
        assert!(
            local
                .zero(
                    &(&chart.target().ideal().generators()[0] - &expected),
                    &mut b
                )
                .unwrap()
        );
        assert_eq!(chart.target().mark(), presentation.source().mark());
        assert!(chart.history().same_root(&h));
    }
    assert!(actual > 0);
    assert_eq!(presentation.history().stage(), 0);
}
