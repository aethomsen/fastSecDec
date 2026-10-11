use super::*;
use crate::threshold::resolution::{Guard, Limits, Ring};
use std::sync::Arc;
use symbolica::{atom::Atom, symbol};

fn b() -> Budget {
    Budget::new(Limits::default())
}
fn ring() -> Arc<Ring> {
    Arc::new(
        Ring::new(
            vec![
                symbol!("prepared_member::x"),
                symbol!("prepared_member::y"),
                symbol!("prepared_member::z"),
                symbol!("prepared_member::u"),
            ],
            vec![],
        )
        .unwrap(),
    )
}
fn p(r: &Ring, s: &str) -> Poly {
    r.atom(&Atom::parse(s, "prepared_member", Default::default()).unwrap())
        .unwrap()
}

#[test]
fn prepared_membership_reuses_verified_basis_and_matches_one_off() {
    let r = ring();
    for equations in [
        vec![p(&r, "x+y"), p(&r, "x-y")],
        vec![p(&r, "x^2+y^2"), p(&r, "x*y+1")],
        vec![p(&r, "x^2-x-z"), p(&r, "u-x*u-1"), p(&r, "2*x*y-y-1")],
    ] {
        let ideal = Ideal::new(r.clone(), equations, &mut b()).unwrap();
        let mut budget = b();
        let mut prepared = PreparedMembership::new(&ideal, &[], &mut budget).unwrap();
        assert!(prepared.basis.is_none());
        // Literal original generators still use the constructive direct path.
        assert!(
            prepared
                .contains(&ideal.generators()[0], &mut budget)
                .unwrap()
        );
        assert!(prepared.basis.is_none());
        for query in [
            p(&r, "y"),
            p(&r, "z"),
            r.one(),
            p(&r, "(4*z+1)*y^2-1"),
            p(&r, "x*y+1"),
        ] {
            assert_eq!(
                prepared.contains(&query, &mut budget).unwrap(),
                ideal.contains(&query, &[], &mut b()).unwrap()
            );
        }
        assert!(prepared.basis.is_some());
        let pointer = prepared.basis.as_ref().unwrap().as_ptr();
        let before = budget.operations();
        let slots = budget.ideal_slots();
        assert!(!prepared.contains(&r.one(), &mut budget).unwrap());
        assert_eq!(before + 2, budget.operations());
        assert_eq!(slots, budget.ideal_slots());
        assert_eq!(pointer, prepared.basis.as_ref().unwrap().as_ptr());
    }
    let ideal = Ideal::new(r.clone(), vec![p(&r, "x+y"), p(&r, "x-y")], &mut b()).unwrap();
    let query = p(&r, "y");
    assert!(!query.reduce(ideal.generators()).is_zero());
    assert!(
        PreparedMembership::new(&ideal, &[], &mut b())
            .unwrap()
            .contains(&query, &mut b())
            .unwrap()
    );
}

#[test]
fn prepared_membership_preserves_admission_caps_and_atomic_retry() {
    let r = ring();
    let ideal = Ideal::new(r.clone(), vec![p(&r, "x^2+y^2"), p(&r, "x*y+1")], &mut b()).unwrap();
    let foreign = Ring::new(vec![symbol!("prepared_member::foreign")], vec![]).unwrap();
    assert!(matches!(
        PreparedMembership::new(&ideal, &[foreign.one()], &mut b()),
        Err(Error::Invalid("polynomial variable map"))
    ));
    let mut budget = b();
    let mut prepared = PreparedMembership::new(&ideal, &[], &mut budget).unwrap();
    assert!(matches!(
        prepared.contains(&foreign.one().zero(), &mut budget),
        Err(Error::Invalid("polynomial variable map"))
    ));
    let mut fail = Budget::new(Limits {
        max_operations: 2,
        ..Limits::default()
    });
    assert!(matches!(
        prepared.contains(&r.one(), &mut fail),
        Err(Error::ResourceIncomplete("operation budget"))
    ));
    assert!(prepared.basis.is_none());
    assert!(!prepared.contains(&r.one(), &mut budget).unwrap());
    assert!(
        prepared
            .basis
            .as_ref()
            .unwrap()
            .iter()
            .any(|f| (0..f.nvars()).any(|i| f.degree(i) > 2))
    );
    let mut tighter = Budget::new(Limits {
        max_degree_per_axis: 2,
        ..Limits::default()
    });
    assert!(matches!(
        prepared.contains(&r.one(), &mut tighter),
        Err(Error::ResourceIncomplete("polynomial degree budget"))
    ));
    let mut terms = Budget::new(Limits {
        max_terms: 1,
        ..Limits::default()
    });
    assert!(matches!(
        prepared.contains(&r.one().zero(), &mut terms),
        Err(Error::ResourceIncomplete("polynomial term budget"))
    ));
    let mut no_slots = Budget::new(Limits {
        max_total_ideal_slots: 0,
        ..Limits::default()
    });
    assert!(matches!(
        PreparedMembership::new(&ideal, &[], &mut no_slots),
        Err(Error::ResourceIncomplete("total ideal allocation budget"))
    ));
    assert!(!prepared.contains(&r.one(), &mut budget).unwrap());
}

#[test]
fn local_batches_do_not_exchange_guards_or_support() {
    let r = ring();
    let ideal = Ideal::new(r.clone(), vec![p(&r, "x*y")], &mut b()).unwrap();
    let xopen = LocalizedAlgebra::new(
        ideal.clone(),
        vec![0, 1, 2],
        vec![Guard {
            factor: p(&r, "x"),
            inverse_axis: 3,
        }],
        &mut b(),
    )
    .unwrap();
    let yopen = LocalizedAlgebra::new(
        ideal,
        vec![0, 1, 2],
        vec![Guard {
            factor: p(&r, "y"),
            inverse_axis: 3,
        }],
        &mut b(),
    )
    .unwrap();
    let mut xp = PreparedLocalZero::new(&xopen, &mut b()).unwrap();
    let mut yp = PreparedLocalZero::new(&yopen, &mut b()).unwrap();
    assert!(xp.is_for(&xopen));
    assert!(!xp.is_for(&yopen));
    assert!(xp.zero(&p(&r, "y"), &mut b()).unwrap());
    assert!(!yp.zero(&p(&r, "y"), &mut b()).unwrap());
    assert!(yp.zero(&p(&r, "x"), &mut b()).unwrap());
    assert!(!xp.zero(&p(&r, "x"), &mut b()).unwrap());
    let plain = LocalizedAlgebra::new(
        Ideal::new(r.clone(), vec![], &mut b()).unwrap(),
        vec![0, 1, 2],
        vec![],
        &mut b(),
    )
    .unwrap();
    assert!(
        PreparedLocalZero::new(&plain, &mut b())
            .unwrap()
            .zero(&p(&r, "u"), &mut b())
            .is_err()
    );
    assert!(
        !PreparedLocalZero::new(&plain, &mut b())
            .unwrap()
            .zero(&p(&r, "x"), &mut b())
            .unwrap()
    );
}

#[test]
fn zero_source_remainder_caps_and_batch_lifetime() {
    let r = ring();
    let weak = Arc::downgrade(&r);
    let empty = Ideal::new(r.clone(), vec![r.one().zero()], &mut b()).unwrap();
    {
        let count = Arc::strong_count(&r);
        let mut prepared = PreparedMembership::new(&empty, &[], &mut b()).unwrap();
        assert!(prepared.contains(&r.one().zero(), &mut b()).unwrap());
        assert!(!prepared.contains(&r.one(), &mut b()).unwrap());
        assert_eq!(Arc::strong_count(&r), count + 1);
    }
    assert_eq!(Arc::strong_count(&r), 2);
    let ideal = Ideal::new(r.clone(), vec![p(&r, "x-y^2")], &mut b()).unwrap();
    let mut prepared = PreparedMembership::new(&ideal, &[], &mut b()).unwrap();
    let mut capped = Budget::new(Limits {
        max_degree_per_axis: 2,
        ..Limits::default()
    });
    assert!(matches!(
        prepared.contains(&p(&r, "x^2"), &mut capped),
        Err(Error::ResourceIncomplete("polynomial degree budget"))
    ));
    drop(prepared);
    drop(ideal);
    drop(empty);
    drop(r);
    assert!(weak.upgrade().is_none());
}
